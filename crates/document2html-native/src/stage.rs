use std::ffi::OsStr;
use std::path::Path;

use crate::config::NativeBackendConfig;
use crate::isolation::isolate;
use crate::process::{CommandSpec, ProcessLimits, ProcessOutput, SystemCommandRunner};
use crate::{NativeResult, ProcessIsolation};

pub(crate) fn run_stage(
    command: CommandSpec,
    config: &NativeBackendConfig,
    workspace_root: &Path,
    monitored_root: &Path,
) -> NativeResult<ProcessOutput> {
    let command = with_controlled_environment(command, workspace_root);
    let command = isolate(command, &config.process_isolation, workspace_root)?;
    SystemCommandRunner::run(
        &command,
        &ProcessLimits {
            timeout: config.stage_timeout,
            max_log_bytes: config.max_log_bytes,
            max_output_bytes: config.max_output_bytes,
        },
        monitored_root,
    )
}

fn with_controlled_environment(mut command: CommandSpec, workspace_root: &Path) -> CommandSpec {
    command = command
        .environment("HOME", workspace_root.join("home"))
        .environment("TMPDIR", workspace_root.join("tmp"))
        .environment("TEMP", workspace_root.join("tmp"))
        .environment("TMP", workspace_root.join("tmp"))
        .environment("LANG", "C.UTF-8")
        .environment("LC_ALL", "C.UTF-8")
        .environment("TZ", "UTC");
    copy_parent_environment(command, "PATH")
}

fn copy_parent_environment(command: CommandSpec, key: impl AsRef<OsStr>) -> CommandSpec {
    let Some(value) = std::env::var_os(&key) else {
        return command;
    };
    command.environment(key, value)
}

pub(crate) fn isolation_diagnostic(isolation: &ProcessIsolation) -> Option<&'static str> {
    match isolation {
        ProcessIsolation::AllowUnisolated => Some("NATIVE_NETWORK_ISOLATION_DISABLED"),
        ProcessIsolation::StrictAuto | ProcessIsolation::Explicit(_) => None,
    }
}
