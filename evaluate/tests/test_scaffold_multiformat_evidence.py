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
            second_output = Path(temp_dir) / "second-wave"

            # When
            scaffold_evidence(PROJECT_ROOT, CONTRACT_PATH, output)
            scaffold_evidence(PROJECT_ROOT, CONTRACT_PATH, second_output)

            # Then
            first_files = {
                path.relative_to(output): path.read_bytes()
                for path in output.rglob("*")
                if path.is_file()
            }
            second_files = {
                path.relative_to(second_output): path.read_bytes()
                for path in second_output.rglob("*")
                if path.is_file()
            }
            self.assertEqual(first_files, second_files)
            contract = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
            evaluator = output / "evidence" / "evaluator-manifest.json"
            evaluator_data = json.loads(evaluator.read_text(encoding="utf-8"))
            self.assertEqual(evaluator_data["schema_version"], 2)
            self.assertEqual(
                evaluator_data["algorithm_parameters"],
                contract["metric_parameters"],
            )
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
                metrics = json.loads(
                    (output / "metrics" / f"{document_format}.json").read_text(
                        encoding="utf-8"
                    )
                )
                self.assertEqual(metrics["schema_version"], 2)
                self.assertEqual(metrics["status"], "INCOMPLETE")

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
                    "browser": {
                        "chromium": "revision",
                        "executable_sha256": "b" * 64,
                        "playwright": "1.62.0",
                        "viewport_width": 1920,
                        "viewport_height": 2400,
                        "device_scale_factor": 1,
                        "locale": "en-US",
                        "timezone": "UTC",
                        "color_profile": "srgb",
                        "reduced_motion": "reduce",
                        "animations": "disabled",
                        "os": "test-os",
                        "architecture": "test-architecture",
                        "font_environment_sha256": "c" * 64,
                    },
                    "candidate_runtime": {
                        "build_revision": "1" * 40,
                        "converter_sha256": "1" * 64,
                        "converter_version": "converter-test",
                        "soffice_sha256": "2" * 64,
                        "soffice_version": "soffice-test",
                        "pdftohtml_sha256": "3" * 64,
                        "pdftohtml_version": "pdftohtml-test",
                        "pdfinfo_sha256": "4" * 64,
                        "pdfinfo_version": "pdfinfo-test",
                        "receipt_signer_sha256": "5" * 64,
                        "receipt_signer_version": "receipt-signer-test",
                    },
                    "sandbox_verifier": {
                        "algorithm": "ed25519",
                        "verifier_id": "test-verifier",
                        "public_key_sha256": "8" * 64,
                        "openssl_sha256": "9" * 64,
                    },
                    "font_bundle_sha256": "a" * 64,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        return lock
