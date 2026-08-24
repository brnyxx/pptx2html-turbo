from __future__ import annotations

import os
import signal
import subprocess
import threading
import time
from enum import StrEnum
from pathlib import Path
from typing import BinaryIO, Protocol

from evaluate.multiformat_candidate_types import CandidateCaptureError


class CandidateProcessFailure(StrEnum):
    PIPES_UNAVAILABLE = "pipes-unavailable"
    TIMEOUT = "timeout"
    READER_TIMEOUT = "reader-timeout"
    READER_FAILED = "reader-failed"
    LOG_OVERSIZE = "log-oversize"
    TERMINATION_FAILED = "termination-failed"


class CandidateProcessError(CandidateCaptureError):
    failure: CandidateProcessFailure

    def __init__(self, failure: CandidateProcessFailure) -> None:
        self.failure = failure
        messages = {
            CandidateProcessFailure.PIPES_UNAVAILABLE: "converter pipes are unavailable",
            CandidateProcessFailure.TIMEOUT: "converter timeout",
            CandidateProcessFailure.READER_TIMEOUT: "converter reader timeout",
            CandidateProcessFailure.READER_FAILED: "converter reader failed",
            CandidateProcessFailure.LOG_OVERSIZE: "converter log exceeds limit",
            CandidateProcessFailure.TERMINATION_FAILED: "converter termination failed",
        }
        super().__init__(messages[failure])


def run_bounded_process(
    command: tuple[str, ...],
    cwd: Path,
    environment: dict[str, str],
    stdout_path: Path,
    stderr_path: Path,
    *,
    timeout_seconds: float,
    max_log_bytes: int,
) -> int:
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        try:
            process = subprocess.Popen(
                command,
                cwd=cwd,
                env=environment,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
        except (OSError, ValueError) as error:
            raise CandidateProcessError(
                CandidateProcessFailure.PIPES_UNAVAILABLE
            ) from error
        if process.stdout is None or process.stderr is None:
            if not _kill_process_group(process):
                raise CandidateProcessError(CandidateProcessFailure.TERMINATION_FAILED)
            try:
                _ = process.wait(timeout=_termination_timeout(timeout_seconds))
            except (OSError, subprocess.TimeoutExpired) as error:
                raise CandidateProcessError(
                    CandidateProcessFailure.TERMINATION_FAILED
                ) from error
            raise CandidateProcessError(CandidateProcessFailure.PIPES_UNAVAILABLE)
        overflow = threading.Event()
        reader_failures: list[Exception] = []
        termination_failures: list[Exception] = []
        timeout_error: subprocess.TimeoutExpired | None = None
        readers = [
            threading.Thread(
                target=_drain_bounded,
                args=(
                    process,
                    process.stdout,
                    stdout,
                    overflow,
                    reader_failures,
                    termination_failures,
                    max_log_bytes,
                ),
                daemon=True,
            ),
            threading.Thread(
                target=_drain_bounded,
                args=(
                    process,
                    process.stderr,
                    stderr,
                    overflow,
                    reader_failures,
                    termination_failures,
                    max_log_bytes,
                ),
                daemon=True,
            ),
        ]
        for reader in readers:
            reader.start()
        exit_code = 0
        try:
            exit_code = process.wait(timeout=timeout_seconds)
            if not _kill_process_group(process):
                termination_failures.append(RuntimeError("process group survived exit"))
        except subprocess.TimeoutExpired as error:
            timeout_error = error
            if not _kill_process_group(process):
                termination_failures.append(RuntimeError("process group survived kill"))
            else:
                try:
                    _ = process.wait(timeout=_termination_timeout(timeout_seconds))
                except (OSError, subprocess.TimeoutExpired) as wait_error:
                    termination_failures.append(wait_error)
        except (OSError, ValueError) as error:
            termination_failures.append(error)
        finally:
            deadline = time.monotonic() + _termination_timeout(timeout_seconds)
            for reader in readers:
                reader.join(timeout=max(0.0, deadline - time.monotonic()))
            if not any(reader.is_alive() for reader in readers):
                _close_pipe(process.stdout)
                _close_pipe(process.stderr)
        if termination_failures:
            raise CandidateProcessError(
                CandidateProcessFailure.TERMINATION_FAILED
            ) from termination_failures[0]
        if any(reader.is_alive() for reader in readers):
            raise CandidateProcessError(CandidateProcessFailure.READER_TIMEOUT)
        if reader_failures:
            raise CandidateProcessError(
                CandidateProcessFailure.READER_FAILED
            ) from reader_failures[0]
        if timeout_error is not None:
            raise CandidateProcessError(
                CandidateProcessFailure.TIMEOUT
            ) from timeout_error
        if overflow.is_set():
            raise CandidateProcessError(CandidateProcessFailure.LOG_OVERSIZE)
        return exit_code


def _drain_bounded(
    process: subprocess.Popen[bytes],
    source: BinaryIO,
    target: BinaryIO,
    overflow: threading.Event,
    reader_failures: list[Exception],
    termination_failures: list[Exception],
    max_log_bytes: int,
) -> None:
    written = 0
    try:
        for chunk in iter(lambda: source.read(8192), b""):
            remaining = max_log_bytes - written
            if remaining > 0:
                _ = target.write(chunk[:remaining])
                written += min(len(chunk), remaining)
            if len(chunk) > remaining:
                overflow.set()
                if not _kill_process_group(process):
                    termination_failures.append(
                        RuntimeError("process group survived log overflow")
                    )
                break
    except (OSError, ValueError, RuntimeError, TypeError) as error:
        reader_failures.append(error)
        if not _kill_process_group(process):
            termination_failures.append(
                RuntimeError("process group survived reader failure")
            )


def _termination_timeout(timeout_seconds: float) -> float:
    return min(1.0, max(0.01, timeout_seconds))


class _Closeable(Protocol):
    def close(self) -> None: ...


def _close_pipe(pipe: _Closeable) -> None:
    try:
        pipe.close()
    except (OSError, ValueError):
        return


def _kill_process_group(process: subprocess.Popen[bytes]) -> bool:
    try:
        os.killpg(process.pid, signal.SIGKILL)
        return True
    except OSError:
        try:
            _ = process.kill()
        except (OSError, ValueError):
            return process.poll() is not None
        return True
