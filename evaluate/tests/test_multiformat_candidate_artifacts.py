from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from evaluate.multiformat_candidate_artifacts import (
    CandidateArtifactError,
    materialize_runtime_artifacts,
)
from evaluate.multiformat_candidate_capture import materialize_candidate_runtime
from evaluate.multiformat_candidate_types import (
    CandidateRuntimePaths,
    CandidateRuntimeSnapshotError,
)
from evaluate.multiformat_evaluator_portable_wave_files import PORTABLE_WAVE_TEST_FILES
from evaluate.multiformat_portable_package_inventory import (
    bind_package_executable_with_inventory,
)


class CandidateRuntimeArtifactTests(unittest.TestCase):
    def test_runtime_executes_the_sandbox_bound_office_and_browser_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            locked_soffice = root / "locked-soffice"
            locked_chromium = root / "locked-chromium"
            supplied = CandidateRuntimePaths(
                root / "converter",
                locked_soffice,
                root / "pdftohtml",
                root / "pdfinfo",
                locked_chromium,
                root / "receipt-signer",
                root / "fonts.conf",
                "test",
                30,
            )
            snapshots = {
                "converter_binary": root / "copied-converter",
                "soffice_binary": root / "copied-soffice",
                "pdftohtml_binary": root / "copied-pdftohtml",
                "pdfinfo_binary": root / "copied-pdfinfo",
                "receipt_signer_binary": root / "copied-receipt-signer",
                "font_config": root / "copied-fonts.conf",
            }
            preflight = SimpleNamespace(
                runtime_artifacts={},
                runtime=supplied,
                runtime_profile=SimpleNamespace(candidate_runtime_lock={}),
                project_revision="a" * 40,
            )

            with (
                mock.patch(
                    "evaluate.multiformat_candidate_capture.materialize_runtime_artifacts",
                    return_value=snapshots,
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_capture.validate_candidate_runtime"
                ),
            ):
                runtime, _artifacts = materialize_candidate_runtime(
                    preflight,
                    root,
                    root / "output",
                )

            self.assertEqual(runtime.soffice, locked_soffice)
            self.assertEqual(runtime.chromium, locked_chromium)

    def test_candidate_artifact_regressions_are_portable_wave_bound(self) -> None:
        # Given/When: the portable evaluator closure is inspected.
        path = "evaluate/tests/test_multiformat_candidate_artifacts.py"

        # Then: candidate runtime snapshot regressions are digest-bound.
        self.assertIn(path, PORTABLE_WAVE_TEST_FILES)

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

    def test_hardlinked_source_is_copied_to_a_private_snapshot(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            evidence_root = root / "evidence"
            evidence_root.mkdir()
            source = root / "tool"
            source.write_bytes(b"locked-runtime")
            alias = root / "tool-alias"
            alias.hardlink_to(source)

            artifacts = materialize_runtime_artifacts(
                {"converter_binary": source},
                evidence_root,
                evidence_root / "hardlink-snapshot",
            )
            snapshot = artifacts["converter_binary"]
            alias.write_bytes(b"changed-runtime")

            self.assertEqual(snapshot.read_bytes(), b"locked-runtime")
            self.assertEqual(snapshot.stat().st_nlink, 1)
            self.assertNotEqual(snapshot.stat().st_ino, source.stat().st_ino)

    def test_reuses_declared_package_for_poppler_candidate_tools(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: two locked Poppler tools in one inventoried package.
            root = Path(temporary)
            evidence_root = root / "evidence"
            evidence_root.mkdir()
            source_package = root / "Poppler.app"
            pdftohtml = source_package / "Contents/bin/pdftohtml"
            pdfinfo = source_package / "Contents/bin/pdfinfo"
            library = source_package / "Contents/lib/libpoppler.dylib"
            pdftohtml.parent.mkdir(parents=True)
            library.parent.mkdir(parents=True)
            pdftohtml.write_bytes(b"pdftohtml")
            pdfinfo.write_bytes(b"pdfinfo")
            library.write_bytes(b"library")
            bound_html, inventory = bind_package_executable_with_inventory(
                pdftohtml, evidence_root, evidence_root / "poppler-package"
            )
            self.assertIsNotNone(inventory)
            bound_info = bound_html.with_name("pdfinfo")

            # When: candidate runtime artifacts snapshot both package members.
            artifacts = materialize_runtime_artifacts(
                {
                    "pdftohtml_binary": bound_html,
                    "pdfinfo_binary": bound_info,
                },
                evidence_root,
                evidence_root / "candidate/runtime-inputs",
            )

            # Then: one private package copy preserves both tools and their library.
            html = artifacts["pdftohtml_binary"]
            info = artifacts["pdfinfo_binary"]
            self.assertEqual(html.parents[1], info.parents[1])
            self.assertEqual(
                (html.parents[1] / "lib/libpoppler.dylib").read_bytes(), b"library"
            )
            self.assertEqual(len(list(artifacts.root.glob("*-package"))), 1)
            artifacts.revalidate()

            # When: the snapshotted closure changes between candidate runs.
            copied_library = html.parents[1] / "lib/libpoppler.dylib"
            copied_library.chmod(0o644)
            copied_library.write_bytes(b"between-run mutation")

            # Then: interval revalidation fails before the next run.
            with self.assertRaises(CandidateRuntimeSnapshotError):
                artifacts.revalidate()

    def test_rejects_symlinks_and_escaping_package_descendants(self) -> None:
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
