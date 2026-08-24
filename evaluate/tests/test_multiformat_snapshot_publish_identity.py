from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate import multiformat_snapshot_publish as snapshot_publish
from evaluate.multiformat_snapshot_publish import publish_snapshot


def _write_complete(staging: Path) -> None:
    (staging / "manifest.json").write_bytes(b"complete")


class MultiFormatSnapshotPublishIdentityTests(unittest.TestCase):
    def test_staging_lstat_boundary_avoids_raw_failure_and_leak(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            destination = root / "corpus"
            original_mkdtemp = snapshot_publish.tempfile.mkdtemp
            original_open = snapshot_publish.os.open
            original_lstat = Path.lstat
            created: Path | None = None
            opened = False

            def record_stage(*, prefix: str, dir: str) -> str:
                nonlocal created
                path = original_mkdtemp(prefix=prefix, dir=dir)
                created = Path(path)
                return path

            def track_open(
                path: str | os.PathLike[str],
                flags: int,
                mode: int = 0o777,
                *,
                dir_fd: int | None = None,
            ) -> int:
                nonlocal opened
                if created is not None and Path(path) == created:
                    opened = True
                    Path.lstat = original_lstat
                return original_open(path, flags, mode, dir_fd=dir_fd)

            def fail_stage_lstat() -> os.stat_result:
                if not opened:
                    raise OSError("injected staging lstat failure")
                assert created is not None
                return original_lstat(created)

            with (
                mock.patch.object(
                    snapshot_publish.tempfile, "mkdtemp", side_effect=record_stage
                ),
                mock.patch.object(snapshot_publish.os, "open", side_effect=track_open),
                mock.patch.object(Path, "lstat", side_effect=fail_stage_lstat),
            ):
                publish_snapshot(destination, _write_complete)

            self.assertEqual((destination / "manifest.json").read_bytes(), b"complete")
            self.assertIsNotNone(created)
            assert created is not None
            self.assertFalse(created.exists())
            self.assertEqual(tuple(root.glob(".corpus.stage-*")), ())
            self.assertFalse((root / ".corpus.snapshot.lock").exists())


if __name__ == "__main__":
    unittest.main()
