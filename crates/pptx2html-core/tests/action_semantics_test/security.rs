use pptx2html_core::model::{
    Presentation, Shape, TextBody, TextParagraph, TextRun, is_safe_external_uri,
};
use pptx2html_core::renderer::HtmlRenderer;

use super::{MinimalPptx, convert_bytes_with_metadata};

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
