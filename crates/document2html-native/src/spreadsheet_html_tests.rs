use document2html_core::{SpreadsheetCell, SpreadsheetSemantics};

use super::annotate_spreadsheet_html;

#[test]
fn annotates_repeated_unicode_and_escaped_values_in_source_order() {
    let html = r#"<html><head><style>.x{position:absolute}</style></head><body><div id="page1-div"><p class="x">반복 &amp; café&#160;반복 &amp; café<br/>東京</p></div></body></html>"#;
    let semantics = SpreadsheetSemantics {
        cells: vec![
            cell("First & <sheet>", "A1", "반복 & café"),
            cell("First & <sheet>", "B2", "반복 & café"),
            cell("東京", "C3", "東京"),
        ],
    };

    let annotated = annotate_spreadsheet_html(html, &semantics);

    assert_eq!(annotated.matches("data-cell-coordinate=").count(), 3);
    assert!(annotated.contains(
        r#"data-cell-coordinate="A1" data-worksheet="First &amp; &lt;sheet&gt;">반복 &amp; café</span>"#
    ));
    assert!(annotated.contains(
        r#"data-cell-coordinate="B2" data-worksheet="First &amp; &lt;sheet&gt;">반복 &amp; café</span>"#
    ));
    assert!(annotated.contains(r#"data-cell-coordinate="C3" data-worksheet="東京">東京</span>"#));
    assert_eq!(remove_semantic_spans(&annotated), html);
}

#[test]
fn ambiguous_or_missing_rendered_values_are_not_assigned_coordinates() {
    let html = r#"<html><body><div id="page1-div"><p>same same same extra</p><p>prefix</p></div></body></html>"#;
    let semantics = SpreadsheetSemantics {
        cells: vec![cell("One", "A1", "same"), cell("One", "A2", "same")],
    };

    let annotated = annotate_spreadsheet_html(html, &semantics);

    assert_eq!(annotated, html);
}

#[test]
fn overlapping_source_values_fail_closed_without_changing_visual_html() {
    let html = r#"<html><body><div id="page1-div"><p>alpha beta</p></div></body></html>"#;
    let semantics = SpreadsheetSemantics {
        cells: vec![cell("One", "A1", "alpha beta"), cell("One", "A2", "alpha")],
    };

    let annotated = annotate_spreadsheet_html(html, &semantics);

    assert_eq!(annotated, html);
}

fn cell(worksheet: &str, coordinate: &str, displayed_value: &str) -> SpreadsheetCell {
    SpreadsheetCell {
        worksheet: worksheet.to_owned(),
        coordinate: coordinate.to_owned(),
        displayed_value: displayed_value.to_owned(),
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
