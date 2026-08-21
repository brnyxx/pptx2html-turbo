from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path

from evaluate.finalize_multiformat_office_oracles import (
    OfficeOracleFinalizeError,
    finalize_office_oracle,
)
from evaluate.multiformat_capture_manifest import validate_capture_manifest
from evaluate.multiformat_inventory import parse_inventory
from evaluate.multiformat_metric_links import load_metric_spec
from evaluate.multiformat_office_oracle_batch import load_office_oracle_batch
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.multiformat_metric_artifact_fixture import write_png
from evaluate.tests.multiformat_office_oracle_finalize_fixture import (
    office_batch_binding,
    write_finalizer_fixture,
)


class FinalizeMultiFormatOfficeOraclesTests(unittest.TestCase):
    def test_signed_batch_becomes_product_gate_capture_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = write_finalizer_fixture(root)

            capture = finalize_office_oracle(**fixture)

            spec = load_metric_spec(fixture["corpus_manifest"])
            validated = validate_capture_manifest(
                capture,
                "oracle",
                spec,
                sha256_file(fixture["contract"]),
                sha256_file(fixture["corpus_manifest"]),
                sha256_file(fixture["evaluator_manifest"]),
                sha256_file(fixture["oracle_lock"]),
                fixture["project_revision"],
                fixture["output_dir"],
                fixture["oracle_lock"],
            )
            self.assertEqual(len(validated.units), len(spec.capture_identities()))
            first = next(iter(validated.units.values()))
            inventory = parse_inventory(
                fixture["output_dir"] / first.inventory.path,
                first.unit_id,
            )
            self.assertTrue(inventory.texts)

    def test_missing_corpus_source_is_rejected_before_signing(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = write_finalizer_fixture(root)
            batch = json.loads(fixture["batch_manifest"].read_text(encoding="utf-8"))
            batch["files"].pop()
            fixture["batch_manifest"].write_text(
                json.dumps(batch, sort_keys=True),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(
                OfficeOracleFinalizeError,
                "office oracle",
            ):
                finalize_office_oracle(**fixture)

    def test_batch_schema_errors_are_wrapped(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = write_finalizer_fixture(root)
            batch = json.loads(fixture["batch_manifest"].read_text(encoding="utf-8"))
            batch["unexpected"] = True
            fixture["batch_manifest"].write_text(
                json.dumps(batch, sort_keys=True),
                encoding="utf-8",
            )

            with self.assertRaises(OfficeOracleFinalizeError):
                finalize_office_oracle(**fixture)

    def test_mixed_format_batch_is_scoped_before_signing(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = write_finalizer_fixture(root)
            batch_root = fixture["batch_manifest"].parent
            extra = batch_root / "extra-pdf"
            extra.mkdir()
            png = extra / "unit-1.png"
            pdf = extra / "reference.pdf"
            semantic = extra / "semantic.json"
            layout = extra / "layout.xml"
            write_png(png, 192, 192, (120, 80, 40))
            pdf.write_bytes(b"%PDF-1.7\n%%EOF\n")
            semantic.write_text("{}", encoding="utf-8")
            layout.write_text(
                '<doc><page width="192" height="192"/></doc>',
                encoding="utf-8",
            )
            batch = json.loads(fixture["batch_manifest"].read_text(encoding="utf-8"))
            batch["files"].append(
                {
                    "id": "extra-pdf",
                    "format": "pdf",
                    "source_sha256": "5" * 64,
                    "pdf": office_batch_binding(batch_root, pdf),
                    "semantic": office_batch_binding(batch_root, semantic),
                    "layout": office_batch_binding(batch_root, layout),
                    "visual_units": [
                        {
                            "png": office_batch_binding(batch_root, png),
                            "width": 192,
                            "height": 192,
                        }
                    ],
                }
            )
            fixture["batch_manifest"].write_text(
                json.dumps(batch, sort_keys=True),
                encoding="utf-8",
            )

            capture = finalize_office_oracle(**fixture)

            capture_values = json.loads(capture.read_text(encoding="utf-8"))
            scoped_batch = load_office_oracle_batch(
                fixture["output_dir"] / capture_values["office_batch_manifest"]["path"]
            )
            self.assertNotIn("extra-pdf", scoped_batch.files)

    def test_runtime_executable_suffixes_are_preserved(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = write_finalizer_fixture(root)
            signer = root / "receipt-signer.exe"
            openssl = root / "openssl.exe"
            shutil.copy2(fixture["receipt_signer"], signer)
            shutil.copy2(fixture["openssl"], openssl)
            fixture["receipt_signer"] = signer
            fixture["openssl"] = openssl

            finalize_office_oracle(**fixture)

            runtime = fixture["output_dir"] / "runtime"
            self.assertTrue((runtime / "receipt-signer.exe").is_file())
            self.assertTrue((runtime / "openssl.exe").is_file())
