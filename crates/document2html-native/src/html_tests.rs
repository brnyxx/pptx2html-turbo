use std::path::Path;

use super::{count_page_containers, normalize_generated_metadata, validate_safe_html};

#[test]
fn removes_poppler_path_and_timestamp_metadata() {
    // Given
    let root = Path::new("/tmp/document2html-1");
    let html = r#"<head>
<title>/tmp/document2html-1/poppler/output</title>
<meta name="generator" content="pdftohtml 0.36"/>
<meta name="date" content="2026-08-20T06:54:04+00:00"/>
</head>"#;

    // When
    let normalized =
        normalize_generated_metadata(html, root).expect("known metadata should normalize");

    // Then
    assert!(normalized.contains("<title>document</title>"));
    assert!(normalized.contains(r#"<meta name="generator" content="pdftohtml 0.36"/>"#));
    assert!(!normalized.contains("/tmp/document2html-1"));
    assert!(!normalized.contains(r#"<meta name="date""#));
}

#[test]
fn rejects_active_remote_and_temporary_content() {
    for html in [
        r#"<script>alert(1)</script>"#,
        r#"<img src="https://example.test/image.png"/>"#,
        r#"<title>/tmp/document2html-1/poppler/other</title>"#,
    ] {
        // Given
        let root = Path::new("/tmp/document2html-1");

        // When
        let result = validate_safe_html(html, root);

        // Then
        assert!(result.is_err(), "{html:?} should be rejected");
    }
}

#[test]
fn page_containers_must_be_strictly_sequential() {
    // Given
    let valid = r#"<div id="page1-div"></div><div id="page2-div"></div>"#;
    let invalid = r#"<div id="page1-div"></div><div id="page3-div"></div>"#;

    // When
    let valid_count = count_page_containers(valid).expect("valid pages should count");
    let invalid_result = count_page_containers(invalid);

    // Then
    assert_eq!(valid_count, 2);
    assert!(invalid_result.is_err());
}
