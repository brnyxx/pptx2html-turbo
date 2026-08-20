use std::ffi::OsString;
use std::path::Path;

use crate::config::{IsolationLauncher, ProcessIsolation};
use crate::process::CommandSpec;
use crate::{NativeError, NativeResult};

#[cfg(target_os = "macos")]
const MACOS_PROFILE: &str = "(version 1)(allow default)(deny network-outbound (remote ip \"*:*\"))";

pub(crate) fn isolate(
    command: CommandSpec,
    isolation: &ProcessIsolation,
    workspace_root: &Path,
) -> NativeResult<CommandSpec> {
    match isolation {
        ProcessIsolation::AllowUnisolated => Ok(command),
        ProcessIsolation::Explicit(launcher) => Ok(wrap(command, launcher)),
        ProcessIsolation::StrictAuto => strict_platform_wrapper(command, workspace_root),
    }
}

fn wrap(command: CommandSpec, launcher: &IsolationLauncher) -> CommandSpec {
    let mut arguments = launcher.argument_prefix.clone();
    arguments.push(OsString::from("--"));
    arguments.push(command.executable.into_os_string());
    arguments.extend(command.arguments);
    CommandSpec {
        executable: launcher.executable.clone(),
        arguments,
        environment: command.environment,
        working_directory: command.working_directory,
    }
}

#[cfg(target_os = "macos")]
fn strict_platform_wrapper(
    command: CommandSpec,
    _workspace_root: &Path,
) -> NativeResult<CommandSpec> {
    let executable = std::path::PathBuf::from("/usr/bin/sandbox-exec");
    if !executable.is_file() {
        return unavailable("sandbox-exec is not installed");
    }
    Ok(wrap(
        command,
        &IsolationLauncher {
            executable,
            argument_prefix: vec![OsString::from("-p"), OsString::from(MACOS_PROFILE)],
        },
    ))
}

#[cfg(target_os = "linux")]
fn strict_platform_wrapper(
    command: CommandSpec,
    workspace_root: &Path,
) -> NativeResult<CommandSpec> {
    let executable = ["/usr/bin/bwrap", "/usr/local/bin/bwrap"]
        .into_iter()
        .map(std::path::PathBuf::from)
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| NativeError::BackendUnavailable("bwrap is not installed".to_owned()))?;
    Ok(wrap(
        command,
        &IsolationLauncher {
            executable,
            argument_prefix: vec![
                OsString::from("--unshare-net"),
                OsString::from("--ro-bind"),
                OsString::from("/"),
                OsString::from("/"),
                OsString::from("--bind"),
                workspace_root.as_os_str().to_owned(),
                workspace_root.as_os_str().to_owned(),
            ],
        },
    ))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn strict_platform_wrapper(
    _command: CommandSpec,
    _workspace_root: &Path,
) -> NativeResult<CommandSpec> {
    unavailable("strict process isolation requires an explicit launcher on this platform")
}

fn unavailable<T>(reason: &str) -> NativeResult<T> {
    Err(NativeError::BackendUnavailable(reason.to_owned()))
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::MACOS_PROFILE;

    #[test]
    fn strict_profile_blocks_remote_ip_without_disabling_local_ipc() {
        // Given
        let profile = MACOS_PROFILE;

        // When
        let blocks_remote_ip = profile.contains("deny network-outbound (remote ip");

        // Then
        assert!(blocks_remote_ip);
        assert!(!profile.contains("deny network*"));
    }
}
