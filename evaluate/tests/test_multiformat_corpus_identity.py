from __future__ import annotations

import json
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path

from evaluate.multiformat_contract import contract_digest
from evaluate.multiformat_corpus_identity import (
    admitted_corpus_digest,
    validate_admitted_corpus,
)
from evaluate.multiformat_corpus_types import CorpusError, CorpusStatus
from evaluate.multiformat_schema import JsonValue, sha256_file

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"
FORMATS = ("pptx", "docx", "doc", "xlsx", "xls", "ppt", "pdf")


class MultiFormatCorpusIdentityTests(unittest.TestCase):
    def test_digest_is_order_invariant_but_binds_names_and_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            manifest = self._write_manifest(root)
            original = json.loads(manifest.read_text(encoding="utf-8"))
            reordered = deepcopy(original)
            sources = reordered["sources"]
            assert isinstance(sources, list)
            sources.reverse()

            self.assertEqual(
                admitted_corpus_digest(original),
                admitted_corpus_digest(reordered),
            )

            mutations: tuple[tuple[str, str], ...] = (
                ("id", "renamed"),
                ("path", "sources/renamed.pdf"),
                ("sha256", "f" * 64),
            )
            for field, value in mutations:
                changed = deepcopy(original)
                records = changed["sources"]
                assert isinstance(records, list)
                record = records[0]
                assert isinstance(record, dict)
                record[field] = value
                with self.subTest(field=field):
                    self.assertNotEqual(
                        admitted_corpus_digest(original),
                        admitted_corpus_digest(changed),
                    )
            removed = deepcopy(original)
            removed_records = removed["sources"]
            assert isinstance(removed_records, list)
            removed_records.pop()
            added = deepcopy(original)
            added_records = added["sources"]
            assert isinstance(added_records, list)
            added_record = deepcopy(added_records[-1])
            assert isinstance(added_record, dict)
            added_record["id"] = "added"
            added_record["path"] = "sources/pdf/added.pdf"
            added_records.append(added_record)
            self.assertNotEqual(
                admitted_corpus_digest(original), admitted_corpus_digest(removed)
            )
            self.assertNotEqual(
                admitted_corpus_digest(original), admitted_corpus_digest(added)
            )

    def test_contract_digest_binds_quotas_algorithms_rounding_weights_and_thresholds(
        self,
    ) -> None:
        contract = json.loads(CONTRACT.read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            baseline = root / "baseline.json"
            baseline.write_text(json.dumps(contract), encoding="utf-8")
            expected = contract_digest(baseline)
            reordered = root / "reordered.json"
            reordered.write_text(json.dumps(contract, sort_keys=True), encoding="utf-8")
            self.assertEqual(contract_digest(reordered), expected)

            mutations = (
                (
                    "quota",
                    lambda value: value["stratum_quotas"]["pdf"].__setitem__(
                        "text-fonts", 19
                    ),
                ),
                (
                    "algorithm",
                    lambda value: value["metric_parameters"]["visual"].__setitem__(
                        "ms_ssim", 0.34
                    ),
                ),
                (
                    "rounding",
                    lambda value: value["metric_parameters"].__setitem__(
                        "rounding", "ROUND_DOWN"
                    ),
                ),
                (
                    "weight",
                    lambda value: value["metric_parameters"]["unit"].__setitem__(
                        "visual", 0.59
                    ),
                ),
                (
                    "threshold",
                    lambda value: value["thresholds"].__setitem__("format_score", 95.0),
                ),
            )
            for name, mutate in mutations:
                changed = deepcopy(contract)
                mutate(changed)
                path = root / f"{name}.json"
                path.write_text(json.dumps(changed), encoding="utf-8")
                with self.subTest(name=name):
                    self.assertNotEqual(contract_digest(path), expected)

    def test_ready_manifest_verifies_all_formats_counts_quotas_and_source_bytes(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            manifest = self._write_manifest(root)

            result = validate_admitted_corpus(CONTRACT, root)

            self.assertEqual(result.status, CorpusStatus.READY)
            self.assertEqual(
                result.aggregate_sha256,
                admitted_corpus_digest(
                    json.loads(manifest.read_text(encoding="utf-8"))
                ),
            )
            source = next((root / "sources").rglob("*.pptx"))
            source.write_bytes(b"replacement")
            with self.assertRaisesRegex(CorpusError, "source.sha256"):
                validate_admitted_corpus(CONTRACT, root)

    def test_duplicate_pair_and_duplicate_json_key_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            manifest = self._write_manifest(root)
            value = json.loads(manifest.read_text(encoding="utf-8"))
            records = value["sources"]
            assert isinstance(records, list)
            records.append(deepcopy(records[0]))
            value["aggregate_sha256"] = admitted_corpus_digest(value)
            manifest.write_text(json.dumps(value), encoding="utf-8")
            with self.assertRaisesRegex(CorpusError, "source.id"):
                validate_admitted_corpus(CONTRACT, root)

        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self._write_manifest(root)
            manifest = root / "manifest.json"
            text = manifest.read_text(encoding="utf-8")
            manifest.write_text(
                text.replace(
                    '"status": "READY"', '"status": "READY", "status": "READY"'
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(CorpusError, "manifest.schema"):
                validate_admitted_corpus(CONTRACT, root)

    def test_missing_ready_is_incomplete_but_post_ready_tampering_is_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            root.mkdir(exist_ok=True)
            result = validate_admitted_corpus(CONTRACT, root)
            self.assertEqual(result.status, CorpusStatus.INCOMPLETE)

            self._write_manifest(root)
            (
                root / "admission-evidence" / "pptx-conformance-0-extraction.json"
            ).unlink()
            with self.assertRaisesRegex(CorpusError, "evidence.path"):
                validate_admitted_corpus(CONTRACT, root)

    def _write_manifest(self, root: Path) -> Path:
        contract = json.loads(CONTRACT.read_text(encoding="utf-8"))
        records: list[dict[str, JsonValue]] = []
        evidence_root = root / "admission-evidence"
        source_root = root / "sources"
        evidence_root.mkdir(parents=True, exist_ok=True)
        source_root.mkdir(parents=True, exist_ok=True)
        for document_format in FORMATS:
            counts = (("conformance", 1, 100), ("blind", 75, 1), ("security", 10, 1))
            for track, count, unit_count in counts:
                for index in range(count):
                    item_id = f"{document_format}-{track}-{index}"
                    source = (
                        source_root / document_format / f"{item_id}.{document_format}"
                    )
                    source.parent.mkdir(exist_ok=True)
                    source.write_bytes(item_id.encode())
                    evidence: dict[str, JsonValue] = {}
                    for kind in ("extraction", "fonts", "render"):
                        proof = evidence_root / f"{item_id}-{kind}.json"
                        proof.write_bytes(f"{item_id}:{kind}".encode())
                        evidence[kind] = {
                            "path": proof.relative_to(root).as_posix(),
                            "sha256": sha256_file(proof),
                        }
                    records.append(
                        {
                            "format": document_format,
                            "id": item_id,
                            "track": track,
                            "path": source.relative_to(root).as_posix(),
                            "sha256": sha256_file(source),
                            "unit_count": unit_count,
                            "evidence": evidence,
                        }
                    )
        quotas = contract["stratum_quotas"]
        assert isinstance(quotas, dict)
        value: dict[str, JsonValue] = {
            "schema_version": 2,
            "status": "READY",
            "corpus_revision": "corpus-v1",
            "contract_sha256": contract_digest(CONTRACT),
            "aggregate_sha256": "0" * 64,
            "per_format_counts": {
                name: {"conformance": 100, "blind": 75, "security": 10}
                for name in FORMATS
            },
            "stratum_quotas": quotas,
            "sources": records,
            "admitted_at": "2026-08-21T00:00:00Z",
            "project_revision": "a" * 40,
        }
        value["aggregate_sha256"] = admitted_corpus_digest(value)
        manifest = root / "manifest.json"
        manifest.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")
        (root / "READY").write_text("READY\n", encoding="ascii")
        return manifest


if __name__ == "__main__":
    unittest.main()
