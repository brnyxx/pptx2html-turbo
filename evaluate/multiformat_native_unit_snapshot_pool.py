from __future__ import annotations

import sys
import threading
from dataclasses import replace
from pathlib import Path
from types import TracebackType

from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    CandidateProcessFailure,
)
from evaluate.multiformat_native_unit_snapshot import (
    NativeExecutableSnapshot,
    materialize_binary,
    release_binary,
    verify_binary,
)
from evaluate.multiformat_native_unit_trusted import (
    close_trusted_executable,
    open_trusted_executable,
    verify_trusted_executable,
)
from evaluate.multiformat_native_unit_types import (
    NativeProcessRequest,
    NativeProcessRunner,
    NativeStableFile,
)


class NativeProcessSnapshotPool:
    def __init__(self, root: Path, runner: NativeProcessRunner) -> None:
        self._root = root
        self._runner = runner
        self._lock = threading.Lock()
        self._snapshots: dict[Path, NativeExecutableSnapshot] = {}
        self._sources: dict[Path, NativeStableFile] = {}

    def __enter__(self) -> NativeProcessSnapshotPool:
        return self

    def __exit__(
        self,
        exception_type: type[BaseException] | None,
        exception: BaseException | None,
        traceback: TracebackType | None,
    ) -> bool:
        _ = exception_type, traceback
        self.close(exception)
        return False

    def __call__(self, request: NativeProcessRequest) -> int:
        expected = request.executable_identity
        if expected is None:
            return self._runner(request)
        path = Path(request.command[0])
        snapshot = self._snapshot(path, expected)
        verify_trusted_executable(path, expected)
        try:
            return self._runner(replace(request, executable_snapshot=snapshot.binding))
        finally:
            active = sys.exception()
            try:
                verify_trusted_executable(path, expected)
            except CandidateProcessError as error:
                if active is None:
                    raise
                active.add_note(str(error))

    def close(self, active: BaseException | None = None) -> None:
        failures: list[CandidateProcessError] = []
        for snapshot in reversed(tuple(self._snapshots.values())):
            try:
                verify_binary(snapshot, full_content=True)
                release_binary(snapshot)
            except CandidateProcessError as error:
                failures.append(error)
        self._snapshots.clear()
        self._sources.clear()
        if not failures:
            return
        if active is not None:
            for failure in failures:
                active.add_note(str(failure))
            return
        raise failures[0]

    def _snapshot(
        self, path: Path, expected: NativeStableFile
    ) -> NativeExecutableSnapshot:
        with self._lock:
            existing = self._snapshots.get(path)
            if existing is not None:
                if self._sources[path] != expected:
                    raise CandidateProcessError(
                        CandidateProcessFailure.EXECUTABLE_UNTRUSTED
                    )
                return existing
            trusted = open_trusted_executable(path, expected)
            try:
                snapshot = materialize_binary(trusted, self._root)
            finally:
                close_trusted_executable(trusted.descriptor)
            self._snapshots[path] = snapshot
            self._sources[path] = expected
            return snapshot
