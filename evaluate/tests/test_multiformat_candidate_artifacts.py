from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import (
    CandidateArtifactError,
    materialize_runtime_artifacts,
)


class CandidateRuntimeArtifactTests(unittest.TestCase):
    def test_in_root_runtime_is_a_private_snapshot_when_source_changes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            evidence_root = Path(temporary)
            source = evidence_root / "locked-tool"
            source.write_bytes(b"verified-runtime")

            artifacts = materialize_runtime_artifacts(
                {"converter_binary": source},
                evidence_root,
                evidence_root / "capture/runtime-inputs",
            )
            snapshot = artifacts["converter_binary"]
            source.write_bytes(b"mutated-runtime!")

            self.assertNotEqual(snapshot, source)
            self.assertEqual(snapshot.read_bytes(), b"verified-runtime")
            self.assertEqual(snapshot.stat().st_nlink, 1)

    def test_rejects_aliases_and_escaping_package_descendants(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            evidence_root = root / "evidence"
            evidence_root.mkdir()
            source = root / "tool"
            source.write_bytes(b"runtime")
            alias = root / "tool-alias"
            alias.hardlink_to(source)

            with self.assertRaisesRegex(CandidateArtifactError, "hardlinked"):
                materialize_runtime_artifacts(
                    {"converter_binary": source},
                    evidence_root,
                    evidence_root / "hardlink-snapshot",
                )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            evidence_root = root / "evidence"
            evidence_root.mkdir()
            source = root / "tool"
            source.write_bytes(b"runtime")
            symlink = root / "tool-symlink"
            symlink.symlink_to(source)

            with self.assertRaisesRegex(CandidateArtifactError, "symlinked"):
                materialize_runtime_artifacts(
                    {"converter_binary": symlink},
                    evidence_root,
                    evidence_root / "symlink-snapshot",
                )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            evidence_root = root / "evidence"
            evidence_root.mkdir()
            package = root / "chromium-package"
            package.mkdir()
            chromium = package / "chromium"
            chromium.write_bytes(b"runtime")
            outside = root / "outside"
            outside.write_bytes(b"attacker")
            (package / "escape").symlink_to(outside)

            with self.assertRaisesRegex(CandidateArtifactError, "symlink escapes"):
                materialize_runtime_artifacts(
                    {"chromium_binary": chromium},
                    evidence_root,
                    evidence_root / "package-snapshot",
                )


if __name__ == "__main__":
    unittest.main()
