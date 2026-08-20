import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_corpus import (
    CorpusError,
    CorpusStatus,
    validate_corpus_manifest,
)
from evaluate.multiformat_schema import JsonValue
from evaluate.scaffold_multiformat_evidence import scaffold_evidence
from evaluate.tests.multiformat_source_fixture import write_positive_source

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class MultiFormatCorpusTests(unittest.TestCase):
    def test_ready_manifest_enforces_exact_tracks_and_sources(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, manifest = self._ready_fixture(root)

            result = validate_corpus_manifest(contract, manifest)

            self.assertEqual(result.status, CorpusStatus.READY)
            self.assertEqual(result.conformance_units, 2)
            self.assertEqual(result.blind_files, 5)
            self.assertEqual(result.security_cases, 2)
            self.assertEqual(result.blind_producers, 5)

    def test_incomplete_scaffold_stays_incomplete(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "wave"
            scaffold_evidence(PROJECT_ROOT, CONTRACT_PATH, output)

            contract = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
            for document_format in contract["required_formats"]:
                with self.subTest(document_format=document_format):
                    result = validate_corpus_manifest(
                        CONTRACT_PATH,
                        output / "corpora" / document_format / "manifest.json",
                    )
                    self.assertEqual(result.status, CorpusStatus.INCOMPLETE)
                    self.assertEqual(result.conformance_units, 0)
                    self.assertEqual(result.blind_files, 0)
                    self.assertEqual(result.security_cases, 0)

    def test_duplicate_blind_source_or_template_is_rejected(self) -> None:
        for option, reason in [
            ({"duplicate_blind_hash": True}, "blind.sha256"),
            ({"duplicate_template": True}, "blind.template_family"),
        ]:
            with self.subTest(reason=reason):
                with tempfile.TemporaryDirectory() as temp_dir:
                    root = Path(temp_dir)
                    contract, manifest_path = self._ready_fixture(root, **option)

                    with self.assertRaisesRegex(CorpusError, reason):
                        validate_corpus_manifest(contract, manifest_path)

    def test_blind_track_requires_five_independent_producers(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, manifest = self._ready_fixture(root, few_producers=True)

            with self.assertRaisesRegex(CorpusError, "blind.producers"):
                validate_corpus_manifest(contract, manifest)

    def test_source_path_and_hash_are_bound(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, traversal = self._ready_fixture(root, traversal=True)
            with self.assertRaisesRegex(CorpusError, "source.path"):
                validate_corpus_manifest(contract, traversal)

        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, bad_hash = self._ready_fixture(root, bad_hash=True)
            with self.assertRaisesRegex(CorpusError, "source.sha256"):
                validate_corpus_manifest(contract, bad_hash)

    def test_positive_sources_must_match_the_declared_format(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, manifest = self._ready_fixture(root, invalid_signature=True)

            with self.assertRaisesRegex(CorpusError, "source.format"):
                validate_corpus_manifest(contract, manifest)

    def test_primary_stratum_quotas_are_exact(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, manifest = self._ready_fixture(root, wrong_quota=True)

            with self.assertRaisesRegex(CorpusError, "conformance.stratum"):
                validate_corpus_manifest(contract, manifest)

    @classmethod
    def _ready_fixture(
        cls,
        root: Path,
        *,
        duplicate_blind_hash: bool = False,
        duplicate_template: bool = False,
        few_producers: bool = False,
        traversal: bool = False,
        bad_hash: bool = False,
        invalid_signature: bool = False,
        wrong_quota: bool = False,
    ) -> tuple[Path, Path]:
        contract = root / "contract.json"
        contract_value: dict[str, JsonValue] = {
            "schema_version": 1,
            "required_formats": ["docx"],
            "corpus": {
                "conformance_units": 2,
                "blind_files": 5,
                "security_cases": 2,
                "reviewers": 2,
                "deterministic_runs": 2,
            },
            "thresholds": {},
            "strata": {"docx": ["text", "tables"]},
            "stratum_quotas": {"docx": {"text": 1, "tables": 1}},
            "legacy_paired_stratum_quotas": {},
            "security_case_outcomes": {
                "docx": {"case-0": "reject", "case-1": "reject"}
            },
        }
        cls._write_json(contract, contract_value)
        corpus_root = root / "corpus"
        sources = corpus_root / "sources"
        sources.mkdir(parents=True)

        conformance_path = sources / "conformance.docx"
        cls._write_docx(conformance_path, "conformance")
        first_stratum = "text"
        second_stratum = "text" if wrong_quota else "tables"
        conformance = {
            "id": "conformance",
            "path": "sources/conformance.docx",
            "sha256": cls._sha256(conformance_path),
            "paired_source": None,
            "provenance": None,
            "units": [
                {
                    "id": "unit-1",
                    "ordinal": 1,
                    "primary_stratum": first_stratum,
                    "paired_stratum": None,
                    "secondary_features": [],
                },
                {
                    "id": "unit-2",
                    "ordinal": 2,
                    "primary_stratum": second_stratum,
                    "paired_stratum": None,
                    "secondary_features": [],
                },
            ],
        }

        blind: list[dict[str, JsonValue]] = []
        first_hash = ""
        for index in range(5):
            path = sources / f"blind-{index}.docx"
            if invalid_signature and index == 0:
                path.write_bytes(b"not an OOXML package")
            else:
                marker = (
                    "blind-0"
                    if duplicate_blind_hash and index == 1
                    else f"blind-{index}"
                )
                cls._write_docx(path, marker)
            digest = cls._sha256(path)
            if index == 0:
                first_hash = digest
            if duplicate_blind_hash and index == 1:
                digest = first_hash
            blind.append(
                {
                    "id": f"blind-{index}",
                    "path": f"sources/blind-{index}.docx",
                    "sha256": digest,
                    "producer": (
                        f"producer-{index}" if not few_producers else "producer-0"
                    ),
                    "source_uri": f"https://example.test/blind-{index}.docx",
                    "template_family": (
                        "template-0"
                        if duplicate_template and index == 1
                        else f"template-{index}"
                    ),
                    "unit_count": 1,
                }
            )

        security: list[dict[str, JsonValue]] = []
        for index in range(2):
            path = sources / f"security-{index}.docx"
            path.write_bytes(f"hostile-{index}".encode())
            security.append(
                {
                    "id": f"security-{index}",
                    "path": f"sources/security-{index}.docx",
                    "sha256": cls._sha256(path),
                    "case_family": f"case-{index}",
                    "expected_outcome": "reject",
                }
            )

        if traversal:
            conformance["path"] = "../escape.docx"
        if bad_hash:
            conformance["sha256"] = "0" * 64
        manifest: dict[str, JsonValue] = {
            "schema_version": 2,
            "status": "READY",
            "format": "docx",
            "contract_sha256": cls._sha256(contract),
            "stratum_quotas": {"text": 1, "tables": 1},
            "tracks": {
                "conformance": {"expected_count": 2, "items": [conformance]},
                "blind": {"expected_count": 5, "items": blind},
                "security": {"expected_count": 2, "items": security},
            },
        }
        manifest_path = corpus_root / "manifest.json"
        cls._write_json(manifest_path, manifest)
        return contract, manifest_path

    @staticmethod
    def _write_docx(path: Path, marker: str) -> None:
        write_positive_source(path, "docx", marker)

    @staticmethod
    def _write_json(path: Path, value: dict[str, JsonValue]) -> None:
        path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

    @staticmethod
    def _sha256(path: Path) -> str:
        return hashlib.sha256(path.read_bytes()).hexdigest()
