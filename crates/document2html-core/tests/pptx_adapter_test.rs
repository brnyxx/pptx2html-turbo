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
            name: "pptx2html-core",
            version: env!("CARGO_PKG_VERSION"),
        }
    );
}

#[test]
fn core_converter_rejects_native_only_format_explicitly() {
    // Given
    let input = DocumentInput::detect(b"%PDF-1.7\n", Some("report.pdf"));

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
