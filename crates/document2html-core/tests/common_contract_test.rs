use document2html_core::{
    AssetMode, DocumentConversionOptions, DocumentFormat, RuntimeSupport, core_runtime_capabilities,
};

#[test]
fn conversion_options_embed_assets_by_default() {
    // Given
    let options = DocumentConversionOptions::default();

    // When
    let asset_mode = options.asset_mode;

    // Then
    assert_eq!(asset_mode, AssetMode::Embed);
}

#[test]
fn core_runtime_reports_all_formats_without_overstating_support() {
    // Given
    let expected_formats = [
        DocumentFormat::Pptx,
        DocumentFormat::Docx,
        DocumentFormat::Doc,
        DocumentFormat::Xlsx,
        DocumentFormat::Xls,
        DocumentFormat::Ppt,
        DocumentFormat::Pdf,
    ];

    // When
    let capabilities = core_runtime_capabilities();

    // Then
    assert_eq!(
        capabilities
            .iter()
            .map(|capability| capability.format)
            .collect::<Vec<_>>(),
        expected_formats
    );
    for capability in capabilities {
        let expected = if capability.format == DocumentFormat::Pptx {
            RuntimeSupport::Available
        } else {
            RuntimeSupport::BackendUnavailable
        };
        assert_eq!(capability.support, expected);
    }
}
