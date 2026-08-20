use document2html_core::{
    AssetMode, DocumentConversionOptions, DocumentFormat, DocumentInput, UnitKind,
};
use document2html_native::{NativeBackendConfig, NativeDocumentConverter};

#[path = "support/docx.rs"]
mod docx_support;
use docx_support::build_minimal_docx;

#[test]
#[ignore = "requires LibreOffice and Poppler executables"]
fn converts_docx_through_the_native_public_surface() {
    // Given
    let data = build_minimal_docx("Universal DOCX");
    let input = DocumentInput::detect(&data, Some("sample.docx"));
    let config = NativeBackendConfig::default();
    let converter = NativeDocumentConverter::new(config).expect("probe native runtime");

    // When
    let result = converter
        .convert(&input, &DocumentConversionOptions::default())
        .expect("convert DOCX");
    let repeated = converter
        .convert(&input, &DocumentConversionOptions::default())
        .expect("repeat DOCX conversion");
    let external = converter
        .convert(
            &input,
            &DocumentConversionOptions {
                asset_mode: AssetMode::External,
            },
        )
        .expect("convert DOCX with external assets");

    // Then
    assert_eq!(result.format, DocumentFormat::Docx);
    assert_eq!(result.unit_kind, UnitKind::Page);
    assert_eq!(result.unit_count, 1);
    assert!(result.html.contains("Universal"));
    assert!(result.html.contains("DOCX"));
    assert!(result.html.contains("data:image/png;base64,"));
    assert!(result.external_assets.is_empty());
    assert_eq!(result.html, repeated.html);
    assert_eq!(result.external_assets, repeated.external_assets);
    assert!(external.html.contains("assets/asset-0001.png"));
    assert!(!external.html.contains("data:image/png;base64,"));
    assert_eq!(external.external_assets.len(), 1);
    assert_eq!(
        external.external_assets[0].relative_path,
        "assets/asset-0001.png"
    );
    assert_eq!(result.backend.name, "libreoffice+poppler");
    assert_eq!(result.diagnostics[0].code, "NATIVE_BACKEND_OPAQUE");
    assert!(
        result
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "NATIVE_NETWORK_ISOLATION_DISABLED")
    );
}
