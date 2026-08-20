use crate::{
    AssetMode, BackendIdentity, DocumentAsset, DocumentConversionOptions, DocumentConversionResult,
    DocumentDiagnostic, DocumentError, DocumentFormat, DocumentInput, DocumentResult, UnitKind,
    core_runtime_capabilities, detect_format,
};

pub struct CoreDocumentConverter;

impl CoreDocumentConverter {
    pub fn convert(
        input: &DocumentInput<'_>,
        options: &DocumentConversionOptions,
    ) -> DocumentResult<DocumentConversionResult> {
        let pptx_options = pptx2html_core::ConversionOptions {
            embed_images: options.asset_mode == AssetMode::Embed,
            ..Default::default()
        };
        Self::convert_pptx_with_options(input, &pptx_options)
    }

    pub fn convert_pptx_with_options(
        input: &DocumentInput<'_>,
        options: &pptx2html_core::ConversionOptions,
    ) -> DocumentResult<DocumentConversionResult> {
        let format = detect_format(input)?;
        if format != DocumentFormat::Pptx {
            return Err(DocumentError::BackendUnavailable {
                format,
                runtime: "core",
            });
        }
        let result = pptx2html_core::convert_bytes_with_options_metadata(input.data, options)?;
        Ok(DocumentConversionResult {
            format,
            html: result.html,
            external_assets: result
                .external_assets
                .into_iter()
                .map(|asset| DocumentAsset {
                    relative_path: asset.relative_path,
                    content_type: asset.content_type,
                    data: asset.data,
                })
                .collect(),
            diagnostics: result
                .diagnostics
                .into_iter()
                .map(|diagnostic| DocumentDiagnostic {
                    code: diagnostic.code,
                    family: diagnostic.family.as_str().to_owned(),
                    support_tier: diagnostic.support_tier.as_str().to_owned(),
                    stage: diagnostic.stage.map(|stage| stage.as_str().to_owned()),
                    raw_reference: diagnostic.raw_reference,
                    fallback_kind: diagnostic.fallback_kind.as_str().to_owned(),
                    reason: diagnostic.reason,
                })
                .collect(),
            unit_count: result.slide_count,
            unit_kind: UnitKind::Slide,
            backend: BackendIdentity {
                name: "pptx2html-core",
                version: env!("CARGO_PKG_VERSION"),
            },
            capabilities: core_runtime_capabilities(),
        })
    }
}
