use std::ffi::OsStr;
use std::path::PathBuf;

use crate::config::NativeBackendConfig;
use crate::fonts::{EastAsianFontPolicy, resolve_policy};
use crate::process::{CommandSpec, ProcessLimits, ProcessOutput, SystemCommandRunner};
use crate::workspace::TemporaryWorkspace;
use crate::{NativeError, NativeResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeToolInfo {
    pub executable: PathBuf,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRuntimeInfo {
    pub libreoffice: NativeToolInfo,
    pub pdftohtml: NativeToolInfo,
    pub pdfinfo: NativeToolInfo,
    /// How this host resolves east-Asian families. `Pinned` carries the
    /// substitute family and the digest of the exact font file behind it, so
    /// evidence binds the artifact that produced a conversion.
    pub east_asian_fonts: EastAsianFontPolicy,
}

#[derive(Debug, Clone)]
pub struct NativeRuntime {
    config: NativeBackendConfig,
}

impl NativeRuntime {
    pub fn new(config: NativeBackendConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &NativeBackendConfig {
        &self.config
    }

    pub fn probe(&self) -> NativeResult<NativeRuntimeInfo> {
        let workspace = TemporaryWorkspace::create()?;
        let libreoffice_path = resolve_executable(
            self.config.soffice_path.as_ref(),
            "DOCUMENT2HTML_SOFFICE",
            "soffice",
        );
        let pdftohtml_path = resolve_executable(
            self.config.pdftohtml_path.as_ref(),
            "DOCUMENT2HTML_PDFTOHTML",
            "pdftohtml",
        );
        let pdfinfo_path = resolve_executable(
            self.config.pdfinfo_path.as_ref(),
            "DOCUMENT2HTML_PDFINFO",
            "pdfinfo",
        );
        let libreoffice =
            self.probe_tool(&workspace, libreoffice_path, "--version", "LibreOffice")?;
        let pdftohtml = self.probe_tool(&workspace, pdftohtml_path, "-v", "pdftohtml version")?;
        let pdfinfo = self.probe_tool(&workspace, pdfinfo_path, "-v", "pdfinfo version")?;
        if poppler_release(&pdftohtml.version) != poppler_release(&pdfinfo.version) {
            return malformed("pdftohtml and pdfinfo versions do not match");
        }
        let Some(east_asian_fonts) = resolve_policy() else {
            return Err(NativeError::BackendUnavailable(
                "no CJK-capable substitute font is installed, so east-Asian text \
                 would resolve through the nondeterministic CoreText fallback"
                    .to_owned(),
            ));
        };
        Ok(NativeRuntimeInfo {
            libreoffice,
            pdftohtml,
            pdfinfo,
            east_asian_fonts,
        })
    }

    fn probe_tool(
        &self,
        workspace: &TemporaryWorkspace,
        executable: PathBuf,
        version_argument: impl AsRef<OsStr>,
        expected_prefix: &str,
    ) -> NativeResult<NativeToolInfo> {
        let mut command = CommandSpec::new(&executable)
            .arg(version_argument)
            .working_directory(workspace.root())
            .environment("HOME", workspace.root().join("home"))
            .environment("TMPDIR", workspace.root().join("tmp"))
            .environment("LANG", "C.UTF-8")
            .environment("LC_ALL", "C.UTF-8")
            .environment("TZ", "UTC");
        if let Some(path) = std::env::var_os("PATH") {
            command = command.environment("PATH", path);
        }
        let output = SystemCommandRunner::run(
            &command,
            &ProcessLimits {
                timeout: self.config.stage_timeout,
                max_log_bytes: self.config.max_log_bytes,
                max_output_bytes: self.config.max_output_bytes,
            },
            workspace.root(),
        )?;
        let version = parse_version_output(&output, expected_prefix)?;
        Ok(NativeToolInfo {
            executable,
            version,
        })
    }
}

fn resolve_executable(
    configured: Option<&PathBuf>,
    environment_key: &str,
    fallback: &str,
) -> PathBuf {
    configured.cloned().unwrap_or_else(|| {
        std::env::var_os(environment_key)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(fallback))
    })
}

fn parse_version_output(output: &ProcessOutput, expected_prefix: &str) -> NativeResult<String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    stdout
        .lines()
        .chain(stderr.lines())
        .map(str::trim)
        .find(|line| line.starts_with(expected_prefix))
        .map(str::to_owned)
        .ok_or_else(|| NativeError::MalformedBackendOutput {
            backend: "runtime-probe",
            reason: format!("missing {expected_prefix} version line"),
        })
}

fn poppler_release(version: &str) -> Option<&str> {
    version
        .split_once("version ")
        .map(|(_, release)| release.trim())
}

fn malformed<T>(reason: &str) -> NativeResult<T> {
    Err(NativeError::MalformedBackendOutput {
        backend: "runtime-probe",
        reason: reason.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::{poppler_release, resolve_executable};

    #[test]
    fn extracts_poppler_release_from_verified_version_line() {
        // Given
        let version = "pdftohtml version 26.03.0";

        // When
        let release = poppler_release(version);

        // Then
        assert_eq!(release, Some("26.03.0"));
    }

    #[test]
    fn explicit_executable_path_wins_over_environment() {
        // Given
        let configured = std::path::PathBuf::from("/trusted/pdfinfo");

        // When
        let resolved =
            resolve_executable(Some(&configured), "DOCUMENT2HTML_TEST_MISSING", "pdfinfo");

        // Then
        assert_eq!(resolved, configured);
    }
}
