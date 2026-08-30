#![forbid(unsafe_code)]

mod config;
mod converter;
mod error;
mod fonts;
mod html;
mod isolation;
mod office;
mod pdfinfo;
mod poppler;
mod process;
mod runtime;
mod sha256;
mod spreadsheet_html;
mod stage;
mod workspace;
mod xlsx_freeze;
mod xlsx_workbook;
mod xlsx_xml;

pub use config::{IsolationLauncher, NativeBackendConfig, ProcessIsolation};
pub use converter::NativeDocumentConverter;
pub use error::{NativeError, NativeResult};
pub use fonts::{EastAsianFontPolicy, EastAsianSubstitute};
pub use pdfinfo::parse_pdfinfo_pages;
pub use runtime::{NativeRuntime, NativeRuntimeInfo, NativeToolInfo};
