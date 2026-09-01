use document2html_core::{DocumentConversionOptions, DocumentFormat, DocumentInput, UnitKind};
use document2html_native::{NativeBackendConfig, NativeDocumentConverter};

#[path = "support/legacy.rs"]
mod legacy_support;
#[path = "support/xlsx.rs"]
mod xlsx_support;
use legacy_support::convert_fixture_with_libreoffice;
use xlsx_support::build_minimal_xlsx;

#[test]
#[ignore = "requires LibreOffice and Poppler executables"]
fn converts_legacy_xls_through_the_native_public_surface() {
    // Given
    let xlsx = build_minimal_xlsx("Universal XLS");
    let data = convert_fixture_with_libreoffice(&xlsx, "xlsx", "xls");
    let input = DocumentInput::detect(&data, Some("sample.xls"));
    let converter =
        NativeDocumentConverter::new(NativeBackendConfig::default()).expect("probe runtime");

    // When
    let result = converter
        .convert(&input, &DocumentConversionOptions::default())
        .expect("convert XLS");

    // Then
    assert_eq!(result.format, DocumentFormat::Xls);
    assert_eq!(result.unit_kind, UnitKind::SheetPage);
    assert_eq!(result.unit_count, 1);
    assert!(result.html.contains("Universal"));
    assert!(result.html.contains("XLS"));
    assert!(result.html.contains(r#"data-cell-coordinate="A1""#));
    assert!(result.html.contains(r#"data-worksheet="Sheet1""#));
}
