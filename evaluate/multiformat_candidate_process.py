from __future__ import annotations

import os
import signal
import subprocess
import threading
import time
from enum import StrEnum
from pathlib import Path
from typing import BinaryIO

from evaluate.multiformat_candidate_types import CandidateCaptureError


class CandidateProcessFailure(StrEnum):
    PIPES_UNAVAILABLE = "pipes-unavailable"
    TIMEOUT = "timeout"
    READER_TIMEOUT = "reader-timeout"
    LOG_OVERSIZE = "log-oversize"


class CandidateProcessError(CandidateCaptureError):
    failure: CandidateProcessFailure

    def __init__(self, failure: CandidateProcessFailure) -> None:
        self.failure = failure
        messages = {
            CandidateProcessFailure.PIPES_UNAVAILABLE: "converter pipes are unavailable",
            CandidateProcessFailure.TIMEOUT: "converter timeout",
            CandidateProcessFailure.READER_TIMEOUT: "converter reader timeout",
            CandidateProcessFailure.LOG_OVERSIZE: "converter log exceeds limit",
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
        process = subprocess.Popen(
            command,
            cwd=cwd,
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
        if process.stdout is None or process.stderr is None:
            raise CandidateProcessError(CandidateProcessFailure.PIPES_UNAVAILABLE)
        overflow = threading.Event()
        timeout_error: subprocess.TimeoutExpired | None = None
        readers = [
            threading.Thread(
                target=_drain_bounded,
                args=(process, process.stdout, stdout, overflow, max_log_bytes),
            ),
            threading.Thread(
                target=_drain_bounded,
                args=(process, process.stderr, stderr, overflow, max_log_bytes),
            ),
        ]
        for reader in readers:
            reader.start()
        deadline = time.monotonic() + timeout_seconds
        exit_code = 0
        try:
            exit_code = process.wait(timeout=timeout_seconds)
        except subprocess.TimeoutExpired as error:
            timeout_error = error
            _ = _kill_process_group(process)
            _ = process.wait()
        finally:
            if process.poll() is not None:
                _ = _kill_process_group(process)
            for reader in readers:
                reader.join(timeout=max(0.0, deadline - time.monotonic()))
            if any(reader.is_alive() for reader in readers):
                process.stdout.close()
                process.stderr.close()
                if timeout_error is None:
                    raise CandidateProcessError(CandidateProcessFailure.READER_TIMEOUT)
            process.stdout.close()
            process.stderr.close()
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
                _ = _kill_process_group(process)
                break
    except (OSError, ValueError):
        overflow.set()
        _ = _kill_process_group(process)


def _kill_process_group(process: subprocess.Popen[bytes]) -> bool:
    try:
        os.killpg(process.pid, signal.SIGKILL)
        return True
    except OSError:
        try:
            _ = process.kill()
        except OSError:
            return process.poll() is not None
        return True
