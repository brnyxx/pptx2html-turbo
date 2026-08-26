use std::fs;
use std::path::{Path, PathBuf};

use document2html_core::{DocumentFormat, DocumentInput};

use crate::config::NativeBackendConfig;
use crate::fonts::EastAsianFontPolicy;
use crate::process::CommandSpec;
use crate::runtime::NativeRuntimeInfo;
use crate::stage::run_stage;
use crate::workspace::TemporaryWorkspace;
use crate::{NativeError, NativeResult};

pub(crate) struct OfficeOutput {
    pub(crate) pdf: PathBuf,
    pub(crate) semantic_xlsx: Option<PathBuf>,
}

pub(crate) fn convert_office_to_pdf(
    input: &DocumentInput<'_>,
    format: DocumentFormat,
    config: &NativeBackendConfig,
    runtime: &NativeRuntimeInfo,
    workspace: &TemporaryWorkspace,
) -> NativeResult<OfficeOutput> {
    if input.data.len() as u64 > config.max_input_bytes {
        return Err(NativeError::ResourceLimitExceeded {
            resource: "input",
            limit: config.max_input_bytes,
        });
    }
    let input_path = workspace
        .root()
        .join("input")
        .join(format!("input.{}", format.extension()));
    fs::write(&input_path, input.data)?;
    match &runtime.east_asian_fonts {
        EastAsianFontPolicy::Pinned(substitute) => {
            workspace.seed_font_substitution(&substitute.family)?;
        }
        EastAsianFontPolicy::PlatformDefault { .. } => {}
    }
    let profile = file_uri(&workspace.root().join("profile"));
    let semantic_xlsx = if format == DocumentFormat::Xls {
        Some(convert_xls_to_xlsx(
            &input_path,
            &profile,
            config,
            runtime,
            workspace,
        )?)
    } else {
        None
    };
    let office_dir = workspace.root().join("office");
    let command = CommandSpec::new(&runtime.libreoffice.executable)
        .arg("--headless")
        .arg(format!("-env:UserInstallation={profile}"))
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(&office_dir)
        .arg(&input_path)
        .working_directory(workspace.root());
    run_stage(command, config, workspace.root(), &office_dir)?;
    let pdf = validate_office_output(&office_dir, "pdf")?;
    Ok(OfficeOutput { pdf, semantic_xlsx })
}

fn convert_xls_to_xlsx(
    input_path: &Path,
    profile: &str,
    config: &NativeBackendConfig,
    runtime: &NativeRuntimeInfo,
    workspace: &TemporaryWorkspace,
) -> NativeResult<PathBuf> {
    let output_dir = workspace.root().join("spreadsheet");
    let command = CommandSpec::new(&runtime.libreoffice.executable)
        .arg("--headless")
        .arg(format!("-env:UserInstallation={profile}"))
        .arg("--convert-to")
        .arg("xlsx")
        .arg("--outdir")
        .arg(&output_dir)
        .arg(input_path)
        .working_directory(workspace.root());
    run_stage(command, config, workspace.root(), &output_dir)?;
    validate_office_output(&output_dir, "xlsx")
}

fn validate_office_output(office_dir: &Path, extension: &str) -> NativeResult<PathBuf> {
    let expected = office_dir.join(format!("input.{extension}"));
    let mut entries = fs::read_dir(office_dir)?;
    let Some(entry) = entries.next().transpose()? else {
        return malformed("LibreOffice did not create the expected output");
    };
    if entries.next().transpose()?.is_some() || entry.path() != expected {
        return malformed("LibreOffice emitted an unexpected output inventory");
    }
    let metadata = fs::symlink_metadata(&expected)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(NativeError::UnsafeOutput(expected));
    }
    Ok(expected)
}

fn file_uri(path: &Path) -> String {
    let value = path.to_string_lossy();
    let escaped = value
        .replace('%', "%25")
        .replace(' ', "%20")
        .replace('#', "%23");
    if escaped.starts_with('/') {
        format!("file://{escaped}")
    } else {
        format!("file:///{escaped}")
    }
}

fn malformed<T>(reason: &str) -> NativeResult<T> {
    Err(NativeError::MalformedBackendOutput {
        backend: "libreoffice",
        reason: reason.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::file_uri;

    #[test]
    fn profile_uri_escapes_path_delimiters() {
        // Given
        let path = std::path::Path::new("/tmp/profile #1");

        // When
        let uri = file_uri(path);

        // Then
        assert_eq!(uri, "file:///tmp/profile%20%231");
    }
}
