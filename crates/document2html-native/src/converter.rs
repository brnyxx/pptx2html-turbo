use std::fs;

use document2html_core::{
    BackendIdentity, CoreDocumentConverter, DocumentConversionOptions, DocumentConversionResult,
    DocumentDiagnostic, DocumentFormat, DocumentInput, RuntimeCapability, RuntimeSupport, UnitKind,
    detect_format, parse_xlsx_semantics,
};

use crate::config::NativeBackendConfig;
use crate::office::convert_office_to_pdf;
use crate::poppler::{PdfHtmlScale, convert_pdf_to_html};
use crate::runtime::{NativeRuntime, NativeRuntimeInfo};
use crate::spreadsheet_html::annotate_spreadsheet_html;
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
            DocumentFormat::Ppt => self.convert_office(input, format, UnitKind::SlidePage, options),
            DocumentFormat::Pptx => unreachable!("PPTX returns through the core adapter"),
            DocumentFormat::Pdf => self.convert_pdf(input, options),
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
        let office = convert_office_to_pdf(input, format, &self.config, &self.runtime, &workspace)?;
        let mut normalized = convert_pdf_to_html(
            &office.pdf,
            options.asset_mode,
            if format == DocumentFormat::Ppt {
                PdfHtmlScale::Presentation
            } else {
                PdfHtmlScale::Paged
            },
            &self.config,
            &self.runtime,
            &workspace,
        )?;
        let mut diagnostics = native_diagnostics(&self.config);
        if matches!(format, DocumentFormat::Xlsx | DocumentFormat::Xls) {
            let semantics = if let Some(path) = office.semantic_xlsx {
                parse_xlsx_semantics(&fs::read(path)?)?
            } else {
                parse_xlsx_semantics(input.data)?
            };
            let annotated = annotate_spreadsheet_html(&normalized.html, &semantics);
            normalized.html = annotated.html;
            diagnostics.extend(annotated.diagnostics);
        }
        Ok(DocumentConversionResult {
            format,
            html: normalized.html,
            external_assets: normalized.assets,
            diagnostics,
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

    fn convert_pdf(
        &self,
        input: &DocumentInput<'_>,
        options: &DocumentConversionOptions,
    ) -> NativeResult<DocumentConversionResult> {
        if input.data.len() as u64 > self.config.max_input_bytes {
            return Err(NativeError::ResourceLimitExceeded {
                resource: "input",
                limit: self.config.max_input_bytes,
            });
        }
        let workspace = TemporaryWorkspace::create()?;
        let pdf = workspace.root().join("input").join("input.pdf");
        fs::write(&pdf, input.data)?;
        let normalized = convert_pdf_to_html(
            &pdf,
            options.asset_mode,
            PdfHtmlScale::Paged,
            &self.config,
            &self.runtime,
            &workspace,
        )?;
        Ok(DocumentConversionResult {
            format: DocumentFormat::Pdf,
            html: normalized.html,
            external_assets: normalized.assets,
            diagnostics: native_diagnostics(&self.config),
            unit_count: normalized.page_count,
            unit_kind: UnitKind::Page,
            backend: BackendIdentity {
                name: "poppler".to_owned(),
                version: self.runtime.pdftohtml.version.clone(),
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
        available(DocumentFormat::Ppt, "libreoffice+poppler"),
        available(DocumentFormat::Pdf, "poppler"),
    ]
}

const fn available(format: DocumentFormat, backend: &'static str) -> RuntimeCapability {
    RuntimeCapability {
        format,
        support: RuntimeSupport::Available,
        backend: Some(backend),
    }
}
