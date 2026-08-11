use pptx2html_core::model::{
    Action, ActionSet, ActionTarget, ActionTrigger, GroupData, Presentation, Shape, ShapeType,
    TableCell, TableData, TableRow, TextBody, TextParagraph, TextRun, is_safe_external_uri,
};
use pptx2html_core::renderer::HtmlRenderer;

use super::{MinimalPptx, convert_bytes_with_metadata};

fn external_owner_action(uri: &str) -> ActionSet {
    ActionSet {
        click: Some(Action {
            trigger: ActionTrigger::Click,
            target: ActionTarget::ExternalUri(uri.to_owned()),
            relationship_id: None,
            relationship_type: None,
            relationship_mode: None,
            source_part: None,
            raw_action: None,
            anchor: None,
            tooltip: None,
            issue: None,
        }),
        hover: None,
    }
}

fn legacy_body(runs: Vec<TextRun>) -> TextBody {
    TextBody {
        paragraphs: vec![TextParagraph {
            runs,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn assert_no_nested_anchors(html: &str) {
    let mut depth = 0usize;
    for token in html.split('<').skip(1) {
        if token.starts_with("a ") {
            assert_eq!(depth, 0, "anchor must not contain another anchor");
            depth += 1;
        } else if token.starts_with("/a>") {
            depth = depth.saturating_sub(1);
        }
    }
    assert_eq!(depth, 0, "all anchors close");
}

#[test]
fn safe_legacy_links_override_shape_surfaces_without_enabling_plain_or_unsafe_runs() {
    let text_shape = Shape {
        id: 70,
        actions: external_owner_action("https://owner.example.test/text"),
        text_body: Some(legacy_body(vec![
            TextRun {
                text: "SAFE_LEGACY_TEXT".to_owned(),
                hyperlink: Some("https://legacy.example.test/text".to_owned()),
                ..Default::default()
            },
            TextRun {
                text: "PLAIN_TEXT".to_owned(),
                ..Default::default()
            },
            TextRun {
                text: "UNSAFE_LEGACY_TEXT".to_owned(),
                hyperlink: Some("javascript:secret0".to_owned()),
                ..Default::default()
            },
        ])),
        ..Default::default()
    };
    let table_shape = Shape {
        id: 71,
        actions: external_owner_action("https://owner.example.test/table"),
        shape_type: ShapeType::Table(TableData {
            rows: vec![TableRow {
                cells: vec![TableCell {
                    text_body: Some(legacy_body(vec![TextRun {
                        text: "SAFE_LEGACY_TABLE".to_owned(),
                        hyperlink: Some("mailto:legacy@example.test".to_owned()),
                        ..Default::default()
                    }])),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            col_widths: vec![1.0],
            ..Default::default()
        }),
        ..Default::default()
    };
    let group_shape = Shape {
        id: 72,
        actions: external_owner_action("https://owner.example.test/group"),
        shape_type: ShapeType::Group(
            vec![Shape {
                text_body: Some(legacy_body(vec![TextRun {
                    text: "SAFE_LEGACY_GROUP".to_owned(),
                    hyperlink: Some("https://legacy.example.test/group".to_owned()),
                    ..Default::default()
                }])),
                ..Default::default()
            }],
            GroupData::default(),
        ),
        ..Default::default()
    };
    let presentation = Presentation {
        slides: vec![pptx2html_core::model::Slide {
            shapes: vec![text_shape, table_shape, group_shape],
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = HtmlRenderer::render_with_options_metadata(
        &presentation,
        &pptx2html_core::ConversionOptions::default(),
    )
    .expect("public model renders");

    assert!(
        result
            .html
            .contains("href=\"https://legacy.example.test/text\"")
    );
    assert!(result.html.contains("href=\"mailto:legacy@example.test\""));
    assert!(
        result
            .html
            .contains("href=\"https://legacy.example.test/group\"")
    );
    assert!(!result.html.contains("javascript:"));
    assert!(!result.html.contains("secret0"));
    assert!(result.html.contains("SAFE_LEGACY_TEXT"));
    assert!(result.html.contains("PLAIN_TEXT"));
    assert!(result.html.contains("UNSAFE_LEGACY_TEXT"));
    assert!(result.html.contains(
        ".shape-action-surface~.text-body .run[href],.shape-action-surface~table .run[href],.shape-action-surface~.shape .run[href]"
    ));
    assert!(
        !result
            .html
            .contains(".shape-action-surface~.text-body .run{pointer-events:auto}")
    );
    assert_no_nested_anchors(&result.html);
}

#[test]
fn unsafe_legacy_hyperlink_is_visible_but_never_serialized_as_a_target() {
    let mut presentation = Presentation::default();
    let shape = Shape {
        text_body: Some(TextBody {
            paragraphs: vec![TextParagraph {
                runs: vec![TextRun {
                    text: "LEGACY_VISIBLE".to_owned(),
                    hyperlink: Some("javascript:secret0".to_owned()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    };
    presentation.slides.push(pptx2html_core::model::Slide {
        shapes: vec![shape],
        ..Default::default()
    });

    let result = HtmlRenderer::render_with_options_metadata(
        &presentation,
        &pptx2html_core::ConversionOptions::default(),
    )
    .expect("model renders");

    assert!(result.html.contains("LEGACY_VISIBLE"));
    assert!(!result.html.contains("secret0"));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|item| item.code == "ACTION_UNSAFE_URI")
            .count(),
        1
    );
}

#[test]
fn external_uri_policy_is_strict_and_case_insensitive() {
    for safe in [
        "HTTP://example.test",
        "https://example.test/path?q=1",
        "MAILTO:user@example.test",
    ] {
        assert!(is_safe_external_uri(safe), "expected safe: {safe}");
    }
    for blocked in [
        "javascript:secret0",
        "data:text/html,secret0",
        "vbscript:secret0",
        "file:///secret0",
        "about:blank",
        "//example.test",
        "/relative",
        "custom:secret0",
        "https:example.test",
        "https://user:secret0@example.test",
        "https://example.test/line\nbreak",
        "http ://example.test",
    ] {
        assert!(
            !is_safe_external_uri(blocked),
            "expected blocked: {blocked}"
        );
    }
}

#[test]
fn relationship_failures_are_typed_without_disclosing_targets() {
    let shapes = r#"
<p:sp><p:nvSpPr><p:cNvPr id="10" name="missing"><a:hlinkClick r:id="missing"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="11" name="duplicate"><a:hlinkClick r:id="duplicate"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="12" name="mismatch"><a:hlinkClick r:id="mismatch"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="13" name="spoof"><a:hlinkClick xmlns:r="urn:evil" r:id="safe"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>"#;
    let rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="duplicate" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://first.test" TargetMode="External"/>
<Relationship Id="duplicate" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://second.test/secret0" TargetMode="External"/>
<Relationship Id="mismatch" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="https://wrong.test/secret0" TargetMode="External"/>
<Relationship Id="safe" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://safe.test" TargetMode="External"/>
</Relationships>"#;

    let result =
        convert_bytes_with_metadata(&MinimalPptx::new(shapes).with_slide_rels(rels).build())
            .expect("fixture converts");
    let mut codes: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|item| item.code.starts_with("ACTION_"))
        .map(|item| item.code.as_str())
        .collect();
    codes.sort_unstable();

    assert_eq!(
        codes,
        [
            "ACTION_RELATIONSHIP_DUPLICATE",
            "ACTION_RELATIONSHIP_MISMATCH",
            "ACTION_RELATIONSHIP_MISMATCH",
            "ACTION_RELATIONSHIP_MISSING",
        ]
    );
    assert!(!result.html.contains("secret0"));
    assert!(!result.html.contains("safe.test"));
    assert!(
        result
            .diagnostics
            .iter()
            .all(|item| !item.reason.contains("secret0")
                && item.raw_reference.as_deref() != Some("secret0"))
    );
}
