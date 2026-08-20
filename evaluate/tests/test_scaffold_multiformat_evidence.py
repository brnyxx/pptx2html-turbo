import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_contract import GateStatus, evaluate_reports
from evaluate.scaffold_multiformat_evidence import ScaffoldError, scaffold_evidence

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class ScaffoldMultiFormatEvidenceTests(unittest.TestCase):
    def test_scaffold_is_deterministic_incomplete_and_hash_bound(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "wave"

            # When
            scaffold_evidence(PROJECT_ROOT, CONTRACT_PATH, output)

            # Then
            contract = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
            evaluator = output / "evidence" / "evaluator-manifest.json"
            evaluator_data = json.loads(evaluator.read_text(encoding="utf-8"))
            self.assertEqual(evaluator_data["schema_version"], 1)
            self.assertGreaterEqual(len(evaluator_data["files"]), 4)
            for entry in evaluator_data["files"]:
                source = PROJECT_ROOT / entry["path"]
                self.assertEqual(
                    entry["sha256"],
                    hashlib.sha256(source.read_bytes()).hexdigest(),
                )
            for document_format in contract["required_formats"]:
                report = json.loads(
                    (output / "reports" / f"{document_format}.json").read_text(
                        encoding="utf-8",
                    )
                )
                self.assertEqual(report["status"], "INCOMPLETE")
                self.assertNotIn("conformance", report)
                self.assertNotIn("blind", report)
                corpus = json.loads(
                    (output / "corpora" / document_format / "manifest.json").read_text(
                        encoding="utf-8"
                    )
                )
                self.assertEqual(corpus["format"], document_format)
                self.assertEqual(corpus["tracks"]["conformance"]["expected_count"], 100)
                self.assertEqual(corpus["tracks"]["blind"]["expected_count"], 75)
                self.assertEqual(corpus["tracks"]["security"]["expected_count"], 10)

            lock = self._write_valid_lock(output)
            summary = evaluate_reports(
                CONTRACT_PATH,
                output / "reports",
                lock,
                output,
            )
            self.assertEqual(summary.status, GateStatus.INCOMPLETE)
            self.assertTrue(
                all(
                    result.status is GateStatus.INCOMPLETE for result in summary.formats
                )
            )

    def test_refuses_to_overlay_existing_output(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "wave"
            output.mkdir()
            (output / "keep.txt").write_text("user data", encoding="utf-8")

            # When and Then
            with self.assertRaises(ScaffoldError):
                scaffold_evidence(PROJECT_ROOT, CONTRACT_PATH, output)
            self.assertEqual(
                (output / "keep.txt").read_text(encoding="utf-8"),
                "user data",
            )

    @staticmethod
    def _write_valid_lock(output: Path) -> Path:
        lock = output / "oracle-lock.json"
        lock.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "status": "locked",
                    "office": {
                        "os": "Windows 11",
                        "word": "build",
                        "excel": "build",
                        "powerpoint": "build",
                    },
                    "pdf": {"primary": "mupdf", "secondary": "renderer"},
                    "browser": {"chromium": "revision"},
                    "font_bundle_sha256": "a" * 64,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        return lock
