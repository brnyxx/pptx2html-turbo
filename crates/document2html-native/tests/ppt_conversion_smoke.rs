use document2html_core::{DocumentConversionOptions, DocumentFormat, DocumentInput, UnitKind};
use document2html_native::{NativeBackendConfig, NativeDocumentConverter};

#[path = "support/legacy.rs"]
mod legacy_support;
use legacy_support::convert_fixture_with_libreoffice;

const SINGLE_SLIDE_PPTX: &[u8] =
    include_bytes!("../../pptx2html-cli/tests/fixtures/single-slide.pptx");

#[test]
#[ignore = "requires LibreOffice and Poppler executables"]
fn converts_legacy_ppt_through_the_native_public_surface() {
    // Given
    let data = convert_fixture_with_libreoffice(SINGLE_SLIDE_PPTX, "pptx", "ppt");
    let input = DocumentInput::detect(&data, Some("sample.ppt"));
    let converter =
        NativeDocumentConverter::new(NativeBackendConfig::default()).expect("probe runtime");

    // When
    let result = converter
        .convert(&input, &DocumentConversionOptions::default())
        .expect("convert PPT");

    // Then
    assert_eq!(result.format, DocumentFormat::Ppt);
    assert_eq!(result.unit_kind, UnitKind::SlidePage);
    assert_eq!(result.unit_count, 1);
    assert!(result.html.contains(r#"id="page1-div""#));
}
