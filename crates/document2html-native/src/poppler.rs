use std::path::Path;

use document2html_core::AssetMode;

use crate::config::NativeBackendConfig;
use crate::html::{NormalizedHtml, normalize_poppler_html};
use crate::pdfinfo::parse_pdfinfo_pages;
use crate::process::CommandSpec;
use crate::runtime::NativeRuntimeInfo;
use crate::stage::run_stage;
use crate::workspace::TemporaryWorkspace;
use crate::{NativeError, NativeResult};

pub(crate) fn convert_pdf_to_html(
    pdf_path: &Path,
    asset_mode: AssetMode,
    config: &NativeBackendConfig,
    runtime: &NativeRuntimeInfo,
    workspace: &TemporaryWorkspace,
) -> NativeResult<NormalizedHtml> {
    let expected_pages = page_count(pdf_path, config, runtime, workspace)?;
    let poppler_dir = workspace.root().join("poppler");
    let html_path = poppler_dir.join("output.html");
    let command = CommandSpec::new(&runtime.pdftohtml.executable)
        .args([
            "-c",
            "-s",
            "-noframes",
            "-hidden",
            "-enc",
            "UTF-8",
            "-fmt",
            "png",
        ])
        .arg(pdf_path)
        .arg(&html_path)
        .working_directory(workspace.root());
    run_stage(command, config, workspace.root(), &poppler_dir)?;
    normalize_poppler_html(
        &poppler_dir,
        &html_path,
        asset_mode,
        expected_pages,
        workspace.root(),
    )
}

fn page_count(
    pdf_path: &Path,
    config: &NativeBackendConfig,
    runtime: &NativeRuntimeInfo,
    workspace: &TemporaryWorkspace,
) -> NativeResult<usize> {
    let command = CommandSpec::new(&runtime.pdfinfo.executable)
        .arg(pdf_path)
        .working_directory(workspace.root());
    let output = run_stage(
        command,
        config,
        workspace.root(),
        pdf_path
            .parent()
            .ok_or_else(|| malformed_error("PDF path has no parent"))?,
    )?;
    let stdout = std::str::from_utf8(&output.stdout)
        .map_err(|_| malformed_error("pdfinfo stdout is not UTF-8"))?;
    parse_pdfinfo_pages(stdout)
}

fn malformed_error(reason: &str) -> NativeError {
    NativeError::MalformedBackendOutput {
        backend: "poppler",
        reason: reason.to_owned(),
    }
}
