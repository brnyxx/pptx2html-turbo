use std::path::PathBuf;

use thiserror::Error;

pub type NativeResult<T> = Result<T, NativeError>;

#[derive(Debug, Error)]
pub enum NativeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Malformed {backend} output: {reason}")]
    MalformedBackendOutput {
        backend: &'static str,
        reason: String,
    },

    #[error("Failed to launch {executable}: {source}")]
    ProcessLaunch {
        executable: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{executable} exited with status {status:?}: {stderr}")]
    ProcessFailed {
        executable: PathBuf,
        status: Option<i32>,
        stderr: String,
    },

    #[error("{executable} exceeded its {seconds} second deadline")]
    Timeout { executable: PathBuf, seconds: u64 },

    #[error("{resource} exceeded the {limit} byte limit")]
    ResourceLimitExceeded { resource: &'static str, limit: u64 },

    #[error("Native process reader failed: {0}")]
    ProcessReader(String),

    #[error("Unsafe output entry: {0}")]
    UnsafeOutput(PathBuf),
}

impl NativeError {
    pub fn is_resource_limit(&self, resource: &str, limit: u64) -> bool {
        matches!(
            self,
            Self::ResourceLimitExceeded {
                resource: actual_resource,
                limit: actual_limit,
            } if *actual_resource == resource && *actual_limit == limit
        )
    }
}
