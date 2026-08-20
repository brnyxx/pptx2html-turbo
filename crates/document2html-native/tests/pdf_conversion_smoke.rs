use document2html_core::{DocumentConversionOptions, DocumentFormat, DocumentInput, UnitKind};
use document2html_native::{NativeBackendConfig, NativeDocumentConverter};

#[path = "support/docx.rs"]
mod docx_support;
#[path = "support/legacy.rs"]
mod legacy_support;
use docx_support::build_minimal_docx;
use legacy_support::convert_fixture_with_libreoffice;

#[test]
#[ignore = "requires LibreOffice and Poppler executables"]
fn converts_pdf_through_the_native_public_surface() {
    // Given
    let docx = build_minimal_docx("Universal PDF");
    let data = convert_fixture_with_libreoffice(&docx, "docx", "pdf");
    let input = DocumentInput::detect(&data, Some("sample.pdf"));
    let converter =
        NativeDocumentConverter::new(NativeBackendConfig::default()).expect("probe runtime");

    // When
    let result = converter
        .convert(&input, &DocumentConversionOptions::default())
        .expect("convert PDF");

    // Then
    assert_eq!(result.format, DocumentFormat::Pdf);
    assert_eq!(result.unit_kind, UnitKind::Page);
    assert_eq!(result.unit_count, 1);
    assert_eq!(result.backend.name, "poppler");
    assert!(result.html.contains("Universal"));
    assert!(result.html.contains("PDF"));
}
