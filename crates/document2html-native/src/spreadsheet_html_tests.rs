use document2html_core::{SpreadsheetCell, SpreadsheetSemantics};

use super::annotate_spreadsheet_html;

#[test]
fn annotates_unique_unicode_and_escaped_values() {
    let html = r#"<html><head><style>.x{position:absolute}</style></head><body><div id="page1-div"><p class="x">반복 &amp; café<br/>東京</p></div></body></html>"#;
    let semantics = SpreadsheetSemantics {
        cells: vec![
            cell("First & <sheet>", "A1", "반복 & café"),
            cell("東京", "C3", "東京"),
        ],
    };

    let annotated = annotate_spreadsheet_html(html, &semantics);

    assert_eq!(annotated.html.matches("data-cell-coordinate=").count(), 2);
    assert!(annotated.html.contains(
        r#"data-cell-coordinate="A1" data-worksheet="First &amp; &lt;sheet&gt;">반복 &amp; café</span>"#
    ));
    assert!(
        annotated
            .html
            .contains(r#"data-cell-coordinate="C3" data-worksheet="東京">東京</span>"#)
    );
    assert_eq!(remove_semantic_spans(&annotated.html), html);
}

#[test]
fn ambiguous_or_missing_rendered_values_are_not_assigned_coordinates() {
    let html = r#"<html><body><div id="page1-div"><p>same same same extra</p><p>prefix</p></div></body></html>"#;
    let semantics = SpreadsheetSemantics {
        cells: vec![cell("One", "A1", "same"), cell("One", "A2", "same")],
    };

    let annotated = annotate_spreadsheet_html(html, &semantics);

    assert_eq!(annotated.html, html);
    assert!(
        annotated
            .diagnostics
            .iter()
            .any(|value| value.code == "SPREADSHEET_CELL_AMBIGUOUS")
    );
}

#[test]
fn overlapping_source_values_fail_closed_without_changing_visual_html() {
    let html = r#"<html><body><div id="page1-div"><p>alpha beta</p></div></body></html>"#;
    let semantics = SpreadsheetSemantics {
        cells: vec![cell("One", "A1", "alpha beta"), cell("One", "A2", "alpha")],
    };

    let annotated = annotate_spreadsheet_html(html, &semantics);

    assert_eq!(annotated.html, html);
    assert!(
        annotated
            .diagnostics
            .iter()
            .any(|value| value.code == "SPREADSHEET_CELLS_UNMAPPED")
    );
}

/// Raw-text element bodies are not rendered content. Annotating them would
/// inject markup into a stylesheet or script and corrupt the document.
#[test]
fn raw_text_element_bodies_are_never_annotated() {
    let html = concat!(
        "<html><body>",
        "<style>div { color : red }</style>",
        "<script>var x = 42 ;</script>",
        "<textarea>draft note</textarea>",
        "<title>sheet title</title>",
        "<p>visible</p>",
        "</body></html>"
    );
    let semantics = SpreadsheetSemantics {
        cells: vec![
            cell("S", "A1", "red"),
            cell("S", "A2", "42"),
            cell("S", "A3", "draft note"),
            cell("S", "A4", "sheet title"),
            cell("S", "A5", "visible"),
        ],
    };

    let annotated = annotate_spreadsheet_html(html, &semantics);

    // Only the paragraph is rendered content, so exactly one span appears.
    assert_eq!(annotated.html.matches("data-cell-coordinate=").count(), 1);
    assert!(
        annotated
            .html
            .contains(r#"data-cell-coordinate="A5" data-worksheet="S">visible</span>"#)
    );
    for raw in [
        "<style>div { color : red }</style>",
        "<script>var x = 42 ;</script>",
        "<textarea>draft note</textarea>",
        "<title>sheet title</title>",
    ] {
        assert!(annotated.html.contains(raw), "{raw} must stay verbatim");
    }
}

/// Case and attribute variations must not defeat raw-text skipping, and an
/// unterminated raw element must not leak its body into matching.
#[test]
fn raw_text_skipping_survives_case_attributes_and_unterminated_elements() {
    let mixed = r#"<html><body><STYLE type="text/css">p { color : blue }</STYLE><p>shown</p></body></html>"#;
    let semantics = SpreadsheetSemantics {
        cells: vec![cell("S", "A1", "blue"), cell("S", "A2", "shown")],
    };

    let annotated = annotate_spreadsheet_html(mixed, &semantics);

    assert_eq!(annotated.html.matches("data-cell-coordinate=").count(), 1);
    assert!(annotated.html.contains(r#""A2" data-worksheet="S">shown<"#));
    assert!(annotated.html.contains("p { color : blue }"));

    let unterminated = r#"<html><body><script>var leaked = 7 ;</body></html>"#;
    let leaked = SpreadsheetSemantics {
        cells: vec![cell("S", "A1", "7")],
    };

    let annotated = annotate_spreadsheet_html(unterminated, &leaked);

    assert_eq!(annotated.html, unterminated);
}

/// Duplicate values must never be zipped positionally onto duplicate
/// occurrences: HTML order is not proven to match workbook order. Reversing
/// the worksheet order must not change the (refused) outcome.
#[test]
fn duplicate_values_are_refused_regardless_of_worksheet_order() {
    let html = r#"<html><body><div id="page1-div"><p>dup</p><p>dup</p></div></body></html>"#;
    let forward = SpreadsheetSemantics {
        cells: vec![cell("First", "A1", "dup"), cell("Second", "Z9", "dup")],
    };
    let reversed = SpreadsheetSemantics {
        cells: vec![cell("Second", "Z9", "dup"), cell("First", "A1", "dup")],
    };

    let forward_annotated = annotate_spreadsheet_html(html, &forward);
    let reversed_annotated = annotate_spreadsheet_html(html, &reversed);

    // Neither ordering may invent a coordinate, and both must agree.
    assert_eq!(forward_annotated.html, html);
    assert_eq!(reversed_annotated.html, html);
    assert_eq!(forward_annotated.html, reversed_annotated.html);
    for annotated in [&forward_annotated, &reversed_annotated] {
        assert!(
            annotated
                .diagnostics
                .iter()
                .any(|value| value.code == "SPREADSHEET_CELL_AMBIGUOUS")
        );
    }
}

/// Unique values still resolve when other cells are ambiguous, and worksheet
/// order does not influence which unique cell wins.
#[test]
fn unique_values_resolve_independently_of_worksheet_order() {
    let html =
        r#"<html><body><div id="page1-div"><p>dup</p><p>dup</p><p>solo</p></div></body></html>"#;
    let forward = SpreadsheetSemantics {
        cells: vec![
            cell("First", "A1", "dup"),
            cell("Second", "Z9", "dup"),
            cell("First", "B2", "solo"),
        ],
    };
    let reversed = SpreadsheetSemantics {
        cells: vec![
            cell("First", "B2", "solo"),
            cell("Second", "Z9", "dup"),
            cell("First", "A1", "dup"),
        ],
    };

    let forward_annotated = annotate_spreadsheet_html(html, &forward);
    let reversed_annotated = annotate_spreadsheet_html(html, &reversed);

    assert_eq!(forward_annotated.html, reversed_annotated.html);
    assert_eq!(
        forward_annotated
            .html
            .matches("data-cell-coordinate=")
            .count(),
        1
    );
    assert!(
        forward_annotated
            .html
            .contains(r#"data-cell-coordinate="B2" data-worksheet="First">solo</span>"#)
    );
}

/// A cell whose number format cannot be reproduced must not block conversion
/// or claim a coordinate. The HTML stays intact and a typed diagnostic says so.
#[test]
fn unreproducible_number_formats_keep_html_and_report_a_diagnostic() {
    let html = r#"<html><body><div id="page1-div"><p>50.00%</p><p>plain</p></div></body></html>"#;
    let semantics = SpreadsheetSemantics {
        cells: vec![unattributable_cell("S", "A1"), cell("S", "A2", "plain")],
    };

    let annotated = annotate_spreadsheet_html(html, &semantics);

    // The unreproducible cell claims nothing, but the reproducible one still
    // resolves, so conversion is never blocked by an exotic format.
    assert_eq!(annotated.html.matches("data-cell-coordinate=").count(), 1);
    assert!(
        annotated
            .html
            .contains(r#"data-cell-coordinate="A2" data-worksheet="S">plain</span>"#)
    );
    assert!(annotated.html.contains("<p>50.00%</p>"));
    let diagnostic = annotated
        .diagnostics
        .iter()
        .find(|value| value.code == "SPREADSHEET_CELL_FORMAT_UNREPRODUCED")
        .expect("unreproducible formats must be reported");
    assert_eq!(diagnostic.family, "spreadsheet-semantics");
    assert_eq!(diagnostic.fallback_kind, "cell-coordinate-omitted");
}

/// Matching cost must scale with document size, not with `cells * nodes`.
/// Asserted through a deterministic probe count rather than wall-clock time.
#[test]
fn large_sheet_matching_cost_stays_near_linear() {
    let small = probe_cost(400);
    let large = probe_cost(1600);

    // Input grew 4x. A cells x nodes matcher would grow ~16x; an indexed scan
    // grows ~4x. Allow generous slack while still excluding quadratic growth.
    let ratio = large.probes as f64 / small.probes.max(1) as f64;
    assert!(
        ratio < 8.0,
        "probe growth {ratio} indicates worse-than-linear matching \
         (small={} large={})",
        small.probes,
        large.probes
    );

    // Every distinct value must still be annotated exactly once.
    assert_eq!(large.annotations, 1600);
}

/// A truncated scan must discard every annotation.
///
/// Uniqueness is only provable when the whole document was inspected. Here a
/// value appears once before the token cutoff and again after it, so a matcher
/// that kept pre-cutoff results would wrongly annotate it as unique.
#[test]
fn truncated_scan_discards_all_annotations() {
    let cutoff = super::matching_max_tokens();
    // "dup" sits in the first node; the filler crosses the cutoff, and the
    // second "dup" lands in a node the scan never reaches.
    let filler: String = (0..=cutoff).map(|index| format!("f{index} ")).collect();
    let html = format!(
        concat!(
            r#"<html><body><div id="page1-div">"#,
            "<p>dup</p><p>{}</p><p>dup</p><p>solo</p>",
            "</div></body></html>"
        ),
        filler
    );
    let semantics = SpreadsheetSemantics {
        cells: vec![cell("S", "A1", "dup"), cell("S", "B2", "solo")],
    };

    let annotated = annotate_spreadsheet_html(&html, &semantics);

    // Nothing may be annotated, not even the value that looked unique.
    assert_eq!(annotated.html, html);
    let codes: Vec<&str> = annotated
        .diagnostics
        .iter()
        .map(|value| value.code.as_str())
        .collect();
    assert_eq!(codes, vec!["SPREADSHEET_CELL_SCAN_TRUNCATED"]);
}

/// Many distinct phrase widths must not multiply the matching cost.
///
/// A widths-by-windows matcher performs one lookup per (token, width) pair, so
/// adding widths alone multiplies its work. The trie walk advances only along
/// live prefixes, so widths that share no leading token cost one failed
/// transition each.
#[test]
fn distinct_phrase_widths_do_not_multiply_matching_cost() {
    let tokens = 4_000usize;
    let narrow = width_cost(tokens, 1);
    let wide = width_cost(tokens, 150);

    // 150x more widths must not cost anything close to 150x more work.
    let ratio = wide.probes as f64 / narrow.probes.max(1) as f64;
    assert!(
        ratio < 2.0,
        "probe growth {ratio} indicates width-dependent matching \
         (narrow={} wide={})",
        narrow.probes,
        wide.probes
    );
    // The bound must hold per token, independent of the width count.
    assert!(
        wide.probes < tokens * 4,
        "probes {} exceed the token-linear bound",
        wide.probes
    );
}

/// Adversarial input: every cell value shares a long common prefix with the
/// rendered text, forcing the deepest legal walk at every token position.
/// The cost must still be bounded by the longest live prefix, not by width
/// count times token count.
#[test]
fn shared_prefix_phrases_stay_within_the_depth_bound() {
    let depth = 40usize;
    let tokens = 2_000usize;
    let body: String = (0..tokens).map(|_| "p ").collect();
    let html = format!(r#"<html><body><div id="page1-div"><p>{body}</p></div></body></html>"#);
    // Values "p", "p p", ... "p p ... p": every width is a live prefix.
    let cells: Vec<_> = (1..=depth)
        .map(|width| {
            let phrase = vec!["p"; width].join(" ");
            cell("S", &format!("A{width}"), &phrase)
        })
        .collect();
    let semantics = SpreadsheetSemantics { cells };

    let annotated = annotate_spreadsheet_html(&html, &semantics);

    // Walks stop at the deepest stored phrase, so work is tokens * depth and
    // never tokens * tokens.
    assert!(
        annotated.probes <= tokens * (depth + 1),
        "probes {} exceed the depth bound",
        annotated.probes
    );
    // Every value repeats, so all are ambiguous and nothing is annotated.
    assert_eq!(annotated.html, html);
    assert!(
        annotated
            .diagnostics
            .iter()
            .any(|value| value.code == "SPREADSHEET_CELL_AMBIGUOUS")
    );
}

fn width_cost(tokens: usize, widths: usize) -> ProbeCost {
    let body: String = (0..tokens).map(|index| format!("w{index} ")).collect();
    let html = format!(r#"<html><body><div id="page1-div"><p>{body}</p></div></body></html>"#);
    // Each cell uses a distinct width and tokens absent from the document.
    let cells: Vec<_> = (1..=widths)
        .map(|width| {
            let phrase = (0..width)
                .map(|part| format!("z{width}_{part}"))
                .collect::<Vec<_>>()
                .join(" ");
            cell("S", &format!("A{width}"), &phrase)
        })
        .collect();
    let semantics = SpreadsheetSemantics { cells };

    let annotated = annotate_spreadsheet_html(&html, &semantics);

    ProbeCost {
        probes: annotated.probes,
        annotations: annotated.html.matches("data-cell-coordinate=").count(),
    }
}

struct ProbeCost {
    probes: usize,
    annotations: usize,
}

fn probe_cost(cells: usize) -> ProbeCost {
    let body: String = (0..cells).map(|index| format!("<p>v{index}</p>")).collect();
    let html = format!(r#"<html><body><div id="page1-div">{body}</div></body></html>"#);
    let semantics = SpreadsheetSemantics {
        cells: (0..cells)
            .map(|index| cell("S", &format!("A{}", index + 1), &format!("v{index}")))
            .collect(),
    };

    let annotated = annotate_spreadsheet_html(&html, &semantics);

    ProbeCost {
        probes: annotated.probes,
        annotations: annotated.html.matches("data-cell-coordinate=").count(),
    }
}

fn cell(worksheet: &str, coordinate: &str, displayed_value: &str) -> SpreadsheetCell {
    SpreadsheetCell {
        worksheet: worksheet.to_owned(),
        coordinate: coordinate.to_owned(),
        displayed_value: displayed_value.to_owned(),
        attributable: true,
    }
}

/// A cell that converted but whose displayed text could not be reproduced.
fn unattributable_cell(worksheet: &str, coordinate: &str) -> SpreadsheetCell {
    SpreadsheetCell {
        worksheet: worksheet.to_owned(),
        coordinate: coordinate.to_owned(),
        displayed_value: String::new(),
        attributable: false,
    }
}

fn remove_semantic_spans(value: &str) -> String {
    let mut output = value.replace("</span>", "");
    while let Some(start) = output.find("<span data-cell-coordinate=") {
        let end = output[start..].find('>').expect("test span is closed") + start + 1;
        output.replace_range(start..end, "");
    }
    output
}
