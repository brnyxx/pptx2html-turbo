use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::{NativeError, NativeResult};

mod output_limit;
mod spec;
use output_limit::enforce_output_limit;
pub(crate) use spec::CommandSpec;

const OBSERVATION_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProcessLimits {
    pub(crate) timeout: Duration,
    pub(crate) max_log_bytes: usize,
    pub(crate) max_output_bytes: u64,
}

#[derive(Debug)]
pub(crate) struct ProcessOutput {
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

pub(crate) struct SystemCommandRunner;

impl SystemCommandRunner {
    pub(crate) fn run(
        spec: &CommandSpec,
        limits: &ProcessLimits,
        monitored_root: &Path,
    ) -> NativeResult<ProcessOutput> {
        let mut command = Command::new(&spec.executable);
        command
            .args(&spec.arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear();
        command.envs(spec.environment.iter().cloned());
        if let Some(directory) = &spec.working_directory {
            command.current_dir(directory);
        }
        let mut child = command
            .spawn()
            .map_err(|source| NativeError::ProcessLaunch {
                executable: spec.executable.clone(),
                source,
            })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| reader_error("missing stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| reader_error("missing stderr"))?;
        let (sender, receiver) = mpsc::channel();
        let stdout_reader = spawn_reader(Stream::Stdout, stdout, sender.clone());
        let stderr_reader = spawn_reader(Stream::Stderr, stderr, sender);
        let deadline = Instant::now() + limits.timeout;
        let mut captured = CapturedStreams::default();

        let execution = (|| {
            loop {
                if let Some(status) = child.try_wait()? {
                    return Ok(status);
                }
                if Instant::now() >= deadline {
                    terminate(&mut child);
                    return Err(NativeError::Timeout {
                        executable: spec.executable.clone(),
                        seconds: limits.timeout.as_secs(),
                    });
                }
                if let Err(error) = enforce_output_limit(monitored_root, limits.max_output_bytes) {
                    terminate(&mut child);
                    return Err(error);
                }
                match receiver.recv_timeout(OBSERVATION_INTERVAL) {
                    Ok(event) => captured.accept(event, limits, &mut child)?,
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => {}
                }
            }
        })();
        if execution.is_err() {
            terminate(&mut child);
        }
        join_reader(stdout_reader)?;
        join_reader(stderr_reader)?;
        let status = execution?;
        drain_events(&receiver, &mut captured, limits, &mut child)?;
        enforce_output_limit(monitored_root, limits.max_output_bytes)?;
        if !status.success() {
            return Err(NativeError::ProcessFailed {
                executable: spec.executable.clone(),
                status: status.code(),
                stderr: String::from_utf8_lossy(&captured.stderr).into_owned(),
            });
        }
        Ok(ProcessOutput {
            stdout: captured.stdout,
            stderr: captured.stderr,
        })
    }
}

#[derive(Clone, Copy)]
enum Stream {
    Stdout,
    Stderr,
}

enum StreamEvent {
    Data(Stream, Vec<u8>),
    ReadError(std::io::Error),
}

#[derive(Default)]
struct CapturedStreams {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl CapturedStreams {
    fn accept(
        &mut self,
        event: StreamEvent,
        limits: &ProcessLimits,
        child: &mut std::process::Child,
    ) -> NativeResult<()> {
        let (resource, target, chunk) = match event {
            StreamEvent::Data(Stream::Stdout, chunk) => ("stdout", &mut self.stdout, chunk),
            StreamEvent::Data(Stream::Stderr, chunk) => ("stderr", &mut self.stderr, chunk),
            StreamEvent::ReadError(error) => return Err(NativeError::Io(error)),
        };
        if target.len().saturating_add(chunk.len()) > limits.max_log_bytes {
            terminate(child);
            return Err(NativeError::ResourceLimitExceeded {
                resource,
                limit: limits.max_log_bytes as u64,
            });
        }
        target.extend_from_slice(&chunk);
        Ok(())
    }
}

fn spawn_reader<R: Read + Send + 'static>(
    stream: Stream,
    mut reader: R,
    sender: Sender<StreamEvent>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    if sender
                        .send(StreamEvent::Data(stream, buffer[..count].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(error) => {
                    let _send_result = sender.send(StreamEvent::ReadError(error));
                    break;
                }
            }
        }
    })
}

fn drain_events(
    receiver: &Receiver<StreamEvent>,
    captured: &mut CapturedStreams,
    limits: &ProcessLimits,
    child: &mut std::process::Child,
) -> NativeResult<()> {
    for event in receiver.try_iter() {
        captured.accept(event, limits, child)?;
    }
    Ok(())
}

fn terminate(child: &mut std::process::Child) {
    if let Err(error) = child.kill()
        && error.kind() != std::io::ErrorKind::InvalidInput
    {
        log::warn!("failed to kill native process {}: {error}", child.id());
    }
    if let Err(error) = child.wait() {
        log::warn!("failed to reap native process {}: {error}", child.id());
    }
}

fn join_reader(handle: thread::JoinHandle<()>) -> NativeResult<()> {
    handle
        .join()
        .map_err(|_| NativeError::ProcessReader("reader thread panicked".to_owned()))
}

fn reader_error(reason: &str) -> NativeError {
    NativeError::ProcessReader(reason.to_owned())
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
