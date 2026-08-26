from __future__ import annotations

import os
import sys

from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    CandidateProcessFailure,
    run_bounded_process,
)
from evaluate.multiformat_native_unit_io import no_follow
from evaluate.multiformat_native_unit_snapshot_validation import (
    verify_executable_binding,
)
from evaluate.multiformat_native_unit_types import NativeProcessRequest


def run_borrowed_snapshot(request: NativeProcessRequest) -> int:
    binding = request.executable_snapshot
    if binding is None:
        raise CandidateProcessError(CandidateProcessFailure.EXECUTABLE_UNTRUSTED)
    stdin_fd: int | None = None
    verify_executable_binding(binding, full_content=False)
    try:
        if binding.shell_script:
            stdin_fd = os.open(binding.path, os.O_RDONLY | no_follow())
            command = ("/bin/sh", "-c", ". /dev/stdin", *request.command)
            executable = None
        else:
            command = request.command
            executable = binding.path
        return run_bounded_process(
            command,
            request.cwd,
            dict(request.environment),
            request.stdout_path,
            request.stderr_path,
            timeout_seconds=request.timeout_seconds,
            max_log_bytes=request.max_log_bytes,
            executable=executable,
            stdin_fd=stdin_fd,
        )
    finally:
        active = sys.exception()
        if stdin_fd is not None:
            try:
                os.close(stdin_fd)
            except OSError as error:
                if active is None:
                    raise
                active.add_note(str(error))
        try:
            verify_executable_binding(binding, full_content=False)
        except CandidateProcessError as error:
            if active is None:
                raise
            active.add_note(str(error))
