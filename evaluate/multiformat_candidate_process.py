from __future__ import annotations

import os
import signal
import subprocess
import threading
import time
from pathlib import Path
from typing import BinaryIO

from evaluate.multiformat_candidate_types import CandidateCaptureError


class CandidateProcessError(CandidateCaptureError):
    pass


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
            raise CandidateProcessError("converter pipes are unavailable")
        overflow = threading.Event()
        readers = [
            threading.Thread(
                target=_drain_bounded,
                args=(
                    process,
                    process.stdout,
                    stdout,
                    overflow,
                    max_log_bytes,
                ),
            ),
            threading.Thread(
                target=_drain_bounded,
                args=(
                    process,
                    process.stderr,
                    stderr,
                    overflow,
                    max_log_bytes,
                ),
            ),
        ]
        for reader in readers:
            reader.start()
        deadline = time.monotonic() + timeout_seconds
        try:
            exit_code = process.wait(timeout=timeout_seconds)
        except subprocess.TimeoutExpired as error:
            _kill_process_group(process)
            process.wait()
            raise CandidateProcessError("converter timeout") from error
        finally:
            if process.poll() is not None:
                _kill_process_group(process)
            for reader in readers:
                reader.join(timeout=max(0.0, deadline - time.monotonic()))
            if any(reader.is_alive() for reader in readers):
                process.stdout.close()
                process.stderr.close()
                raise CandidateProcessError("converter reader timeout")
            process.stdout.close()
            process.stderr.close()
        if overflow.is_set():
            raise CandidateProcessError("converter log exceeds limit")
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
                target.write(chunk[:remaining])
                written += min(len(chunk), remaining)
            if len(chunk) > remaining:
                overflow.set()
                _kill_process_group(process)
                break
    except (OSError, ValueError):
        overflow.set()
        _kill_process_group(process)


def _kill_process_group(process: subprocess.Popen[bytes]) -> bool:
    try:
        os.killpg(process.pid, signal.SIGKILL)
        return True
    except OSError:
        try:
            process.kill()
        except OSError:
            return process.poll() is not None
        return True
