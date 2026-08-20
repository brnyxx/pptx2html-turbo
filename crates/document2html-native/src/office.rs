use std::fs;
use std::path::{Path, PathBuf};

use document2html_core::{DocumentFormat, DocumentInput};

use crate::config::NativeBackendConfig;
use crate::process::CommandSpec;
use crate::runtime::NativeRuntimeInfo;
use crate::stage::run_stage;
use crate::workspace::TemporaryWorkspace;
use crate::{NativeError, NativeResult};

pub(crate) fn convert_office_to_pdf(
    input: &DocumentInput<'_>,
    format: DocumentFormat,
    config: &NativeBackendConfig,
    runtime: &NativeRuntimeInfo,
    workspace: &TemporaryWorkspace,
) -> NativeResult<PathBuf> {
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
    let office_dir = workspace.root().join("office");
    let profile = file_uri(&workspace.root().join("profile"));
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
    validate_office_output(&office_dir)
}

fn validate_office_output(office_dir: &Path) -> NativeResult<PathBuf> {
    let expected = office_dir.join("input.pdf");
    let mut entries = fs::read_dir(office_dir)?;
    let Some(entry) = entries.next().transpose()? else {
        return malformed("LibreOffice did not create input.pdf");
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
