use document2html_core::{
    BackendIdentity, CoreDocumentConverter, DocumentConversionOptions, DocumentError,
    DocumentFormat, DocumentInput, UnitKind,
};

const SINGLE_SLIDE_PPTX: &[u8] =
    include_bytes!("../../pptx2html-cli/tests/fixtures/single-slide.pptx");

#[test]
fn generic_converter_routes_pptx_through_existing_core() {
    // Given
    let input = DocumentInput::detect(SINGLE_SLIDE_PPTX, Some("single-slide.pptx"));

    // When
    let result = CoreDocumentConverter::convert(&input, &DocumentConversionOptions::default())
        .expect("PPTX conversion should succeed");

    // Then
    assert_eq!(result.format, DocumentFormat::Pptx);
    assert_eq!(result.unit_kind, UnitKind::Slide);
    assert_eq!(result.unit_count, 1);
    assert!(result.html.contains("<!DOCTYPE html>"));
    assert_eq!(
        result.backend,
        BackendIdentity {
            name: "pptx2html-core".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    );
}

#[test]
fn core_converter_rejects_native_only_format_explicitly() {
    // Given
    let input = DocumentInput::detect(
        b"%PDF-1.7
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f\x20
0000000009 00000 n\x20
trailer
<< /Size 2 /Root 1 0 R >>
startxref
45
%%EOF
",
        Some("report.pdf"),
    );

    // When
    let error = CoreDocumentConverter::convert(&input, &DocumentConversionOptions::default())
        .expect_err("PDF requires a native backend");

    // Then
    assert!(matches!(
        error,
        DocumentError::BackendUnavailable {
            format: DocumentFormat::Pdf,
            runtime: "core",
        }
    ));
}
