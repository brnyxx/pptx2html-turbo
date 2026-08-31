use std::fs;
use std::path::Path;

use document2html_core::AssetMode;

use super::{
    count_page_containers, normalize_generated_metadata, normalize_poppler_html, validate_safe_html,
};

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
        r#"<a href="file:///tmp/secret">secret</a>"#,
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
fn permits_escaped_file_uri_as_document_text() {
    let html = r#"<p>Content-Location: file:///htmlDoc.html</p>"#;

    validate_safe_html(html, Path::new("/tmp/document2html-1"))
        .expect("escaped document text is inert");
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

#[test]
fn invalid_poppler_utf8_is_replaced_without_rejecting_the_document() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let output_dir = workspace.path().join("poppler");
    fs::create_dir(&output_dir).expect("create output directory");
    let html_path = output_dir.join("output.html");
    let mut html =
        b"<!doctype html><html><head><meta charset=\"utf-8\"><style>.ft{font-family:".to_vec();
    html.extend_from_slice(&[0xba, 0xda, 0xcc, 0xe5]);
    html.extend_from_slice(b"}</style></head><body><div id=\"page1-div\"></div></body></html>");
    fs::write(&html_path, html).expect("write Poppler HTML");

    let normalized = normalize_poppler_html(
        &output_dir,
        &html_path,
        AssetMode::Embed,
        1,
        workspace.path(),
    )
    .expect("invalid UTF-8 is replaced");

    assert!(normalized.html.contains(char::REPLACEMENT_CHARACTER));
    assert!(std::str::from_utf8(normalized.html.as_bytes()).is_ok());
    assert_eq!(normalized.page_count, 1);
}
