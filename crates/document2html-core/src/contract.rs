use crate::DocumentFormat;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AssetMode {
    #[default]
    Embed,
    External,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DocumentConversionOptions {
    pub asset_mode: AssetMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSupport {
    Available,
    BackendUnavailable,
}

impl RuntimeSupport {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::BackendUnavailable => "backend-unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeCapability {
    pub format: DocumentFormat,
    pub support: RuntimeSupport,
    pub backend: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    Page,
    SheetPage,
    Slide,
    SlidePage,
}

impl UnitKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::SheetPage => "sheet-page",
            Self::Slide => "slide",
            Self::SlidePage => "slide-page",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentAsset {
    pub relative_path: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentDiagnostic {
    pub code: String,
    pub family: String,
    pub support_tier: String,
    pub stage: Option<String>,
    pub raw_reference: Option<String>,
    pub fallback_kind: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct DocumentConversionResult {
    pub format: DocumentFormat,
    pub html: String,
    pub external_assets: Vec<DocumentAsset>,
    pub diagnostics: Vec<DocumentDiagnostic>,
    pub unit_count: usize,
    pub unit_kind: UnitKind,
    pub backend: BackendIdentity,
    pub capabilities: [RuntimeCapability; 7],
}

impl DocumentConversionResult {
    pub fn diagnostics_json(&self) -> String {
        let entries = self
            .diagnostics
            .iter()
            .map(DocumentDiagnostic::to_json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{entries}]")
    }
}

impl DocumentDiagnostic {
    fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"code\":{},",
                "\"family\":{},",
                "\"support_tier\":{},",
                "\"stage\":{},",
                "\"raw_reference\":{},",
                "\"fallback_kind\":{},",
                "\"reason\":{}",
                "}}"
            ),
            json_string(&self.code),
            json_string(&self.family),
            json_string(&self.support_tier),
            optional_json_string(self.stage.as_deref()),
            optional_json_string(self.raw_reference.as_deref()),
            json_string(&self.fallback_kind),
            json_string(&self.reason),
        )
    }
}

fn optional_json_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character <= '\u{001f}' => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

pub const fn core_runtime_capabilities() -> [RuntimeCapability; 7] {
    [
        RuntimeCapability {
            format: DocumentFormat::Pptx,
            support: RuntimeSupport::Available,
            backend: Some("pptx2html-core"),
        },
        unavailable(DocumentFormat::Docx),
        unavailable(DocumentFormat::Doc),
        unavailable(DocumentFormat::Xlsx),
        unavailable(DocumentFormat::Xls),
        unavailable(DocumentFormat::Ppt),
        unavailable(DocumentFormat::Pdf),
    ]
}

const fn unavailable(format: DocumentFormat) -> RuntimeCapability {
    RuntimeCapability {
        format,
        support: RuntimeSupport::BackendUnavailable,
        backend: None,
    }
}
