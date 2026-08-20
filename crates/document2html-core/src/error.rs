use thiserror::Error;

use crate::DocumentFormat;

pub type DocumentResult<T> = Result<T, DocumentError>;

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("The input format is not supported")]
    UnsupportedFormat,

    #[error("The legacy compound document requires a DOC, XLS, or PPT hint")]
    AmbiguousFormat,

    #[error("Detected {detected} input conflicts with the {hinted} hint")]
    ConflictingFormatHint {
        detected: DocumentFormat,
        hinted: DocumentFormat,
    },

    #[error("No {runtime} backend is available for {format}")]
    BackendUnavailable {
        format: DocumentFormat,
        runtime: &'static str,
    },

    #[error("Required package part is missing: {0}")]
    MissingPackagePart(String),

    #[error("Package metadata exceeds the {limit} byte limit: {part}")]
    PackageMetadataTooLarge { part: String, limit: u64 },

    #[error("ZIP archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("PPTX conversion error: {0}")]
    Pptx(#[from] pptx2html_core::error::PptxError),
}
