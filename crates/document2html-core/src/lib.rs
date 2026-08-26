#![forbid(unsafe_code)]

mod adapter;
mod contract;
mod error;
mod format;
mod spreadsheet;

pub use adapter::CoreDocumentConverter;
pub use contract::{
    AssetMode, BackendIdentity, DocumentAsset, DocumentConversionOptions, DocumentConversionResult,
    DocumentDiagnostic, RuntimeCapability, RuntimeSupport, UnitKind, core_runtime_capabilities,
};
pub use error::{DocumentError, DocumentResult};
pub use format::{DocumentFormat, DocumentInput, detect_format};
pub use spreadsheet::{SpreadsheetCell, SpreadsheetSemantics, parse_xlsx_semantics};
