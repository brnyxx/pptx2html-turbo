use std::time::Duration;

use super::{CommandSpec, ProcessLimits, SystemCommandRunner};
use crate::workspace::TemporaryWorkspace;

#[test]
#[cfg(unix)]
fn captures_bounded_stdout_from_argument_vector() {
    // Given
    let workspace = TemporaryWorkspace::create().expect("create workspace");
    let command = CommandSpec::new("/usr/bin/printf")
        .arg("native-runner")
        .working_directory(workspace.root());
    let limits = ProcessLimits {
        timeout: Duration::from_secs(2),
        max_log_bytes: 1024,
        max_output_bytes: 1024,
    };

    // When
    let output = SystemCommandRunner::run(&command, &limits, workspace.root())
        .expect("bounded process should succeed");

    // Then
    assert_eq!(output.stdout, b"native-runner");
    assert!(output.stderr.is_empty());
}

#[test]
#[cfg(unix)]
fn rejects_stdout_that_crosses_the_log_limit() {
    // Given
    let workspace = TemporaryWorkspace::create().expect("create workspace");
    let command = CommandSpec::new("/usr/bin/yes").working_directory(workspace.root());
    let limits = ProcessLimits {
        timeout: Duration::from_secs(2),
        max_log_bytes: 64,
        max_output_bytes: 1024,
    };

    // When
    let error = SystemCommandRunner::run(&command, &limits, workspace.root())
        .expect_err("unbounded stdout should fail");

    // Then
    assert!(error.is_resource_limit("stdout", 64));
}

#[test]
fn command_spec_never_uses_a_shell_string() {
    // Given
    let executable = std::path::Path::new("converter");

    // When
    let command = CommandSpec::new(executable)
        .arg("--output")
        .arg("file name.html");

    // Then
    assert_eq!(command.executable, executable);
    assert_eq!(command.arguments.len(), 2);
    assert_eq!(command.arguments[1], "file name.html");
}

#[test]
#[cfg(unix)]
fn timeout_terminates_spawned_descendants() {
    // Given
    let workspace = TemporaryWorkspace::create().expect("create workspace");
    let pid_path = workspace.root().join("descendant.pid");
    let script = format!(
        "sleep 60 & child=$!; echo $child > '{}'; wait",
        pid_path.display()
    );
    let command = CommandSpec::new("/bin/sh")
        .args(["-c", &script])
        .working_directory(workspace.root());
    let limits = ProcessLimits {
        timeout: Duration::from_millis(500),
        max_log_bytes: 1024,
        max_output_bytes: 1024,
    };

    // When
    let result = SystemCommandRunner::run(&command, &limits, workspace.root());

    // Then
    assert!(result.is_err());
    let pid = std::fs::read_to_string(&pid_path).expect("descendant pid");
    let status = std::process::Command::new("/bin/ps")
        .args(["-p", pid.trim(), "-o", "stat="])
        .output()
        .expect("inspect descendant");
    let state = String::from_utf8_lossy(&status.stdout);
    assert!(
        state.trim().is_empty() || state.trim_start().starts_with('Z'),
        "descendant survived with state {state}"
    );
}
