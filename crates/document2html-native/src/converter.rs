use document2html_core::{
    BackendIdentity, CoreDocumentConverter, DocumentConversionOptions, DocumentConversionResult,
    DocumentDiagnostic, DocumentError, DocumentFormat, DocumentInput, RuntimeCapability,
    RuntimeSupport, UnitKind, detect_format,
};

use crate::config::NativeBackendConfig;
use crate::office::convert_office_to_pdf;
use crate::poppler::convert_pdf_to_html;
use crate::runtime::{NativeRuntime, NativeRuntimeInfo};
use crate::stage::isolation_diagnostic;
use crate::workspace::TemporaryWorkspace;
use crate::{NativeError, NativeResult};

#[derive(Debug, Clone)]
pub struct NativeDocumentConverter {
    config: NativeBackendConfig,
    runtime: NativeRuntimeInfo,
}

impl NativeDocumentConverter {
    pub fn new(config: NativeBackendConfig) -> NativeResult<Self> {
        let runtime = NativeRuntime::new(config.clone()).probe()?;
        Ok(Self { config, runtime })
    }

    pub fn runtime(&self) -> &NativeRuntimeInfo {
        &self.runtime
    }

    pub fn convert(
        &self,
        input: &DocumentInput<'_>,
        options: &DocumentConversionOptions,
    ) -> NativeResult<DocumentConversionResult> {
        let format = detect_format(input)?;
        if format == DocumentFormat::Pptx {
            let mut result = CoreDocumentConverter::convert(input, options)?;
            result.capabilities = native_runtime_capabilities();
            return Ok(result);
        }
        match format {
            DocumentFormat::Docx | DocumentFormat::Doc => {
                self.convert_office(input, format, UnitKind::Page, options)
            }
            DocumentFormat::Xlsx | DocumentFormat::Xls => {
                self.convert_office(input, format, UnitKind::SheetPage, options)
            }
            DocumentFormat::Pptx => unreachable!("PPTX returns through the core adapter"),
            DocumentFormat::Ppt | DocumentFormat::Pdf => {
                Err(NativeError::Document(DocumentError::BackendUnavailable {
                    format,
                    runtime: "native",
                }))
            }
        }
    }

    fn convert_office(
        &self,
        input: &DocumentInput<'_>,
        format: DocumentFormat,
        unit_kind: UnitKind,
        options: &DocumentConversionOptions,
    ) -> NativeResult<DocumentConversionResult> {
        let workspace = TemporaryWorkspace::create()?;
        let pdf = convert_office_to_pdf(input, format, &self.config, &self.runtime, &workspace)?;
        let normalized = convert_pdf_to_html(
            &pdf,
            options.asset_mode,
            &self.config,
            &self.runtime,
            &workspace,
        )?;
        Ok(DocumentConversionResult {
            format,
            html: normalized.html,
            external_assets: normalized.assets,
            diagnostics: native_diagnostics(&self.config),
            unit_count: normalized.page_count,
            unit_kind,
            backend: BackendIdentity {
                name: "libreoffice+poppler".to_owned(),
                version: format!(
                    "{}; {}",
                    self.runtime.libreoffice.version, self.runtime.pdftohtml.version
                ),
            },
            capabilities: native_runtime_capabilities(),
        })
    }
}

fn native_diagnostics(config: &NativeBackendConfig) -> Vec<DocumentDiagnostic> {
    let mut diagnostics = vec![DocumentDiagnostic {
        code: "NATIVE_BACKEND_OPAQUE".to_owned(),
        family: "native-document".to_owned(),
        support_tier: "approximate".to_owned(),
        stage: Some("render".to_owned()),
        raw_reference: None,
        fallback_kind: "native-renderer".to_owned(),
        reason: "The native backend cannot classify unsupported source elements individually"
            .to_owned(),
    }];
    if let Some(code) = isolation_diagnostic(&config.process_isolation) {
        diagnostics.push(DocumentDiagnostic {
            code: code.to_owned(),
            family: "native-runtime".to_owned(),
            support_tier: "fallback".to_owned(),
            stage: Some("render".to_owned()),
            raw_reference: None,
            fallback_kind: "runtime-policy".to_owned(),
            reason: "Native conversion ran without process-level network isolation".to_owned(),
        });
    }
    diagnostics
}

const fn native_runtime_capabilities() -> [RuntimeCapability; 7] {
    [
        available(DocumentFormat::Pptx, "pptx2html-core"),
        available(DocumentFormat::Docx, "libreoffice+poppler"),
        available(DocumentFormat::Doc, "libreoffice+poppler"),
        available(DocumentFormat::Xlsx, "libreoffice+poppler"),
        available(DocumentFormat::Xls, "libreoffice+poppler"),
        unavailable(DocumentFormat::Ppt),
        unavailable(DocumentFormat::Pdf),
    ]
}

const fn available(format: DocumentFormat, backend: &'static str) -> RuntimeCapability {
    RuntimeCapability {
        format,
        support: RuntimeSupport::Available,
        backend: Some(backend),
    }
}

const fn unavailable(format: DocumentFormat) -> RuntimeCapability {
    RuntimeCapability {
        format,
        support: RuntimeSupport::BackendUnavailable,
        backend: None,
    }
}
