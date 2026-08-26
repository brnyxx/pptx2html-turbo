from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_files import stable_file
from evaluate.multiformat_native_unit_snapshot import materialize_binary
from evaluate.multiformat_native_unit_snapshot_pool import NativeProcessSnapshotPool
from evaluate.multiformat_native_unit_types import (
    NativeExecutableBinding,
    NativeProcessRequest,
    NativeUnitFailure,
)
from evaluate.tests.multiformat_native_unit_fixture import make_native_unit_fixture


class MultiFormatNativeUnitSnapshotPoolTests(unittest.TestCase):
    def test_pool_materializes_once_and_reuses_immutable_binding(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            expected = stable_file(
                fixture.soffice,
                fixture.request(root, DocumentFormat.DOC),
                NativeUnitFailure.TOOL_MISSING,
            )
            requests = tuple(
                NativeProcessRequest(
                    (fixture.soffice.as_posix(), "--version"),
                    root,
                    (("PATH", os.defpath),),
                    root / f"stdout-{index}",
                    root / f"stderr-{index}",
                    5,
                    1024,
                    expected,
                )
                for index in range(2)
            )
            bindings: list[NativeExecutableBinding | None] = []

            def runner(request: NativeProcessRequest) -> int:
                bindings.append(request.executable_snapshot)
                return 0

            snapshot_root = root / "snapshots"
            snapshot_root.mkdir()
            binding_path = root
            with (
                patch(
                    "evaluate.multiformat_native_unit_snapshot_pool.materialize_binary",
                    wraps=materialize_binary,
                ) as materialize,
                NativeProcessSnapshotPool(snapshot_root, runner) as pool,
            ):
                self.assertEqual([pool(request) for request in requests], [0, 0])
                self.assertEqual(materialize.call_count, 1)
                self.assertIsNotNone(bindings[0])
                self.assertEqual(bindings[0], bindings[1])
                binding_path = bindings[0].path if bindings[0] is not None else root
                self.assertTrue(binding_path.is_file())

            self.assertFalse(binding_path.exists())


if __name__ == "__main__":
    _ = unittest.main()
