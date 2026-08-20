use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessIsolation {
    StrictAuto,
    Explicit(IsolationLauncher),
    AllowUnisolated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsolationLauncher {
    pub executable: PathBuf,
    pub argument_prefix: Vec<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeBackendConfig {
    pub soffice_path: Option<PathBuf>,
    pub pdftohtml_path: Option<PathBuf>,
    pub pdfinfo_path: Option<PathBuf>,
    pub stage_timeout: Duration,
    pub max_input_bytes: u64,
    pub max_output_bytes: u64,
    pub max_log_bytes: usize,
    pub process_isolation: ProcessIsolation,
}

impl Default for NativeBackendConfig {
    fn default() -> Self {
        Self {
            soffice_path: None,
            pdftohtml_path: None,
            pdfinfo_path: None,
            stage_timeout: Duration::from_secs(120),
            max_input_bytes: 512 * 1024 * 1024,
            max_output_bytes: 1024 * 1024 * 1024,
            max_log_bytes: 1024 * 1024,
            process_isolation: ProcessIsolation::StrictAuto,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{NativeBackendConfig, ProcessIsolation};

    #[test]
    fn defaults_enforce_strict_bounded_conversion() {
        // Given
        let expected_timeout = Duration::from_secs(120);

        // When
        let config = NativeBackendConfig::default();

        // Then
        assert_eq!(config.process_isolation, ProcessIsolation::StrictAuto);
        assert_eq!(config.stage_timeout, expected_timeout);
        assert_eq!(config.max_input_bytes, 512 * 1024 * 1024);
        assert_eq!(config.max_output_bytes, 1024 * 1024 * 1024);
        assert_eq!(config.max_log_bytes, 1024 * 1024);
        assert!(config.soffice_path.is_none());
        assert!(config.pdftohtml_path.is_none());
        assert!(config.pdfinfo_path.is_none());
    }
}
