use document2html_core::{DocumentConversionOptions, DocumentFormat, DocumentInput, UnitKind};
use document2html_native::{NativeBackendConfig, NativeDocumentConverter};

#[path = "support/xlsx.rs"]
mod xlsx_support;
use xlsx_support::build_minimal_xlsx;

#[test]
#[ignore = "requires LibreOffice and Poppler executables"]
fn converts_xlsx_through_the_native_public_surface() {
    // Given
    let data = build_minimal_xlsx("Universal XLSX");
    let input = DocumentInput::detect(&data, Some("sample.xlsx"));
    let converter =
        NativeDocumentConverter::new(NativeBackendConfig::default()).expect("probe runtime");

    // When
    let result = converter
        .convert(&input, &DocumentConversionOptions::default())
        .expect("convert XLSX");

    // Then
    assert_eq!(result.format, DocumentFormat::Xlsx);
    assert_eq!(result.unit_kind, UnitKind::SheetPage);
    assert_eq!(result.unit_count, 1);
    assert!(result.html.contains("Universal"));
    assert!(result.html.contains("XLSX"));
}
