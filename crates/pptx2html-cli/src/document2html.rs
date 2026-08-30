#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use document2html_core::{
    AssetMode, CoreDocumentConverter, DocumentConversionOptions, DocumentFormat, DocumentInput,
    detect_format,
};
use document2html_native::{NativeBackendConfig, NativeDocumentConverter, ProcessIsolation};

#[path = "document2html/diagnostics.rs"]
mod diagnostics;
#[path = "document2html/output.rs"]
mod output;

pub(crate) use diagnostics::json_string;
use diagnostics::{diagnostics_json, report_diagnostics};
use output::{ensure_distinct_paths, write_assets, write_text};

const MAX_STAGE_TIMEOUT_SECONDS: u64 = 3_600;

#[derive(Debug, Parser)]
#[command(
    name = "document2html",
    version,
    about = "Convert Office documents and PDF to self-contained HTML"
)]
struct Cli {
    /// Input PPTX, DOCX, DOC, XLSX, XLS, PPT, or PDF path
    input: PathBuf,

    /// Output HTML path (default: input filename.html)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Explicit input format for ambiguous legacy Office containers
    #[arg(long, value_parser = ["pptx", "docx", "doc", "xlsx", "xls", "ppt", "pdf"])]
    input_format: Option<String>,

    /// Extract generated images beside the HTML instead of embedding them
    #[arg(long)]
    no_embed: bool,

    /// Print detected format and selected runtime as JSON, then exit
    #[arg(long)]
    info: bool,

    /// Write canonical conversion diagnostics JSON to this path
    #[arg(long, visible_alias = "diagnostics-json", value_name = "PATH")]
    diagnostics: Option<PathBuf>,

    /// Exit 2 after writing outputs when fallback diagnostics are present
    #[arg(long)]
    fail_on_fallback: bool,

    /// Permit native conversion without process-level network isolation
    #[arg(long)]
    allow_unisolated: bool,

    /// Override the LibreOffice executable path
    #[arg(long, value_name = "PATH")]
    soffice: Option<PathBuf>,

    /// Override the pdftohtml executable path
    #[arg(long, value_name = "PATH")]
    pdftohtml: Option<PathBuf>,

    /// Override the pdfinfo executable path
    #[arg(long, value_name = "PATH")]
    pdfinfo: Option<PathBuf>,

    /// Maximum duration in seconds for each native conversion stage
    #[arg(long, default_value_t = 120, value_parser = parse_positive_timeout_seconds)]
    stage_timeout_seconds: u64,

    /// Uniform PPTX scale used by deterministic presentation capture
    #[arg(long, value_parser = parse_positive_scale)]
    presentation_scale: Option<f64>,
}

fn main() {
    env_logger::init();
    let exit_code = match run(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Conversion failed: {error}");
            1
        }
    };
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn run(cli: Cli) -> Result<i32, String> {
    let data = std::fs::read(&cli.input)
        .map_err(|error| format!("failed to read {}: {error}", cli.input.display()))?;
    let source_name = cli.input.file_name().and_then(|name| name.to_str());
    let format_hint = cli.input_format.as_deref().map(parse_format).transpose()?;
    let input = match format_hint {
        Some(format) => DocumentInput::with_format(&data, source_name, format),
        None => DocumentInput::detect(&data, source_name),
    };
    let format = detect_format(&input).map_err(|error| error.to_string())?;
    if cli.info {
        let runtime = if format == DocumentFormat::Pptx {
            "core"
        } else {
            "native"
        };
        println!(
            r#"{{"format":{},"runtime":{}}}"#,
            json_string(format.as_str()),
            json_string(runtime)
        );
        return Ok(0);
    }

    let output = cli
        .output
        .clone()
        .unwrap_or_else(|| cli.input.with_extension("html"));
    ensure_distinct_paths(&cli.input, &output, "input", "HTML output")?;
    if let Some(sidecar) = &cli.diagnostics {
        ensure_distinct_paths(&cli.input, sidecar, "input", "diagnostics")?;
        ensure_distinct_paths(&output, sidecar, "HTML output", "diagnostics")?;
    }
    let options = DocumentConversionOptions {
        asset_mode: if cli.no_embed {
            AssetMode::External
        } else {
            AssetMode::Embed
        },
    };
    let result = if format == DocumentFormat::Pptx {
        let pptx_options = pptx2html_core::ConversionOptions {
            embed_images: options.asset_mode == AssetMode::Embed,
            scale: cli.presentation_scale.unwrap_or(1.0),
            ..Default::default()
        };
        CoreDocumentConverter::convert_pptx_with_options(&input, &pptx_options)
            .map_err(|error| error.to_string())?
    } else {
        if cli.presentation_scale.is_some() {
            return Err("--presentation-scale is only valid for PPTX input".to_owned());
        }
        let converter =
            NativeDocumentConverter::new(native_config(&cli)).map_err(|error| error.to_string())?;
        converter
            .convert(&input, &options)
            .map_err(|error| error.to_string())?
    };
    let asset_base = output.parent().unwrap_or_else(|| std::path::Path::new("."));
    for asset in &result.external_assets {
        ensure_distinct_paths(
            &output,
            &asset_base.join(&asset.relative_path),
            "HTML output",
            "external asset",
        )?;
    }
    write_text(&output, &result.html)?;
    write_assets(asset_base, &result.external_assets)?;
    let diagnostic_json = diagnostics_json(&result.diagnostics);
    if let Some(sidecar) = &cli.diagnostics {
        write_text(sidecar, &diagnostic_json)?;
    }
    let has_fallback = report_diagnostics(&result.diagnostics);
    println!(
        "Conversion complete: {} ({}) -> {}",
        cli.input.display(),
        format.as_str(),
        output.display()
    );
    Ok(i32::from(cli.fail_on_fallback && has_fallback) * 2)
}

fn native_config(cli: &Cli) -> NativeBackendConfig {
    NativeBackendConfig {
        soffice_path: cli.soffice.clone(),
        pdftohtml_path: cli.pdftohtml.clone(),
        pdfinfo_path: cli.pdfinfo.clone(),
        stage_timeout: Duration::from_secs(cli.stage_timeout_seconds),
        process_isolation: if cli.allow_unisolated {
            ProcessIsolation::AllowUnisolated
        } else {
            ProcessIsolation::StrictAuto
        },
        ..Default::default()
    }
}

fn parse_format(value: &str) -> Result<DocumentFormat, String> {
    match value {
        "pptx" => Ok(DocumentFormat::Pptx),
        "docx" => Ok(DocumentFormat::Docx),
        "doc" => Ok(DocumentFormat::Doc),
        "xlsx" => Ok(DocumentFormat::Xlsx),
        "xls" => Ok(DocumentFormat::Xls),
        "ppt" => Ok(DocumentFormat::Ppt),
        "pdf" => Ok(DocumentFormat::Pdf),
        _ => Err(format!("unsupported input format: {value}")),
    }
}

fn parse_positive_scale(value: &str) -> Result<f64, String> {
    let scale = value
        .parse::<f64>()
        .map_err(|_| "presentation scale must be a number".to_owned())?;
    if scale.is_finite() && scale > 0.0 {
        Ok(scale)
    } else {
        Err("presentation scale must be finite and greater than zero".to_owned())
    }
}

fn parse_positive_timeout_seconds(value: &str) -> Result<u64, String> {
    let timeout_seconds = value
        .parse::<u64>()
        .map_err(|_| "stage timeout seconds must be a positive integer".to_owned())?;
    if (1..=MAX_STAGE_TIMEOUT_SECONDS).contains(&timeout_seconds) {
        Ok(timeout_seconds)
    } else {
        Err(format!(
            "stage timeout seconds must be between 1 and {MAX_STAGE_TIMEOUT_SECONDS}"
        ))
    }
}

#[cfg(test)]
#[path = "document2html_tests.rs"]
mod tests;
