from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_corpus import CorpusError, validate_corpus_manifest
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_security_fixture import validate_security_fixture
from evaluate.multiformat_security_source import write_security_source
from evaluate.multiformat_source_fixture import write_positive_source
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class MultiFormatSecurityFixtureTests(unittest.TestCase):
    def test_every_contract_family_has_machine_verifiable_semantics(self) -> None:
        contract = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
        for format_name, families in contract["security_case_outcomes"].items():
            document_format = DocumentFormat(format_name)
            for family in families:
                with self.subTest(document_format=format_name, family=family):
                    with tempfile.TemporaryDirectory() as temp_dir:
                        source = Path(temp_dir) / f"fixture.{format_name}"
                        write_security_source(
                            source, DocumentFormat(format_name), family
                        )

                        validate_security_fixture(
                            source,
                            document_format,
                            family,
                        )

    def test_safe_convert_fixtures_remain_valid_source_formats(self) -> None:
        contract = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
        for format_name, families in contract["security_case_outcomes"].items():
            for family, outcome in families.items():
                if outcome != "safe-convert":
                    continue
                with self.subTest(document_format=format_name, family=family):
                    with tempfile.TemporaryDirectory() as temp_dir:
                        root = Path(temp_dir)
                        source = root / f"fixture.{format_name}"
                        write_security_source(
                            source, DocumentFormat(format_name), family
                        )

                        validate_source(
                            {
                                "id": "security-fixture",
                                "path": source.name,
                                "sha256": hashlib.sha256(
                                    source.read_bytes()
                                ).hexdigest(),
                            },
                            root,
                            DocumentFormat(format_name),
                            require_valid_format=True,
                        )

    def test_fixture_cannot_be_relabelled_as_another_family(self) -> None:
        contract = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
        for format_name, families in contract["security_case_outcomes"].items():
            names = list(families)
            for index, family in enumerate(names):
                with self.subTest(document_format=format_name, family=family):
                    with tempfile.TemporaryDirectory() as temp_dir:
                        source = Path(temp_dir) / f"fixture.{format_name}"
                        write_security_source(
                            source, DocumentFormat(format_name), family
                        )

                        with self.assertRaisesRegex(
                            CorpusError,
                            "security.fixture",
                        ):
                            validate_security_fixture(
                                source,
                                DocumentFormat(format_name),
                                names[(index + 1) % len(names)],
                            )

    def test_arbitrary_hostile_bytes_are_not_a_security_fixture(self) -> None:
        cases = [
            (DocumentFormat.DOCX, "malformed-zip"),
            (DocumentFormat.DOC, "malformed-cfbf"),
            (DocumentFormat.PDF, "malformed-xref"),
        ]
        for document_format, family in cases:
            with self.subTest(document_format=document_format):
                with tempfile.TemporaryDirectory() as temp_dir:
                    source = Path(temp_dir) / f"arbitrary.{document_format.value}"
                    source.write_bytes(b"hostile")

                    with self.assertRaisesRegex(CorpusError, "security.fixture"):
                        validate_security_fixture(
                            source,
                            document_format,
                            family,
                        )

    def test_safe_fixture_must_contain_its_dangerous_construct(self) -> None:
        cases = [
            (DocumentFormat.DOCX, "external-relationship"),
            (DocumentFormat.DOC, "macro-storage"),
            (DocumentFormat.PDF, "javascript-action"),
        ]
        for document_format, family in cases:
            with self.subTest(document_format=document_format):
                with tempfile.TemporaryDirectory() as temp_dir:
                    source = Path(temp_dir) / f"plain.{document_format.value}"
                    write_positive_source(
                        source,
                        document_format.value,
                        "plain",
                    )

                    with self.assertRaisesRegex(CorpusError, "security.fixture"):
                        validate_security_fixture(
                            source,
                            document_format,
                            family,
                        )

    def test_ready_corpus_rejects_unproved_security_labels(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            contract, manifest = ready_fixture(Path(temp_dir))
            value = json.loads(manifest.read_text(encoding="utf-8"))
            item = value["tracks"]["security"]["items"][0]
            source = manifest.parent / item["path"]
            source.write_bytes(b"hostile")
            item["sha256"] = hashlib.sha256(source.read_bytes()).hexdigest()
            manifest.write_text(
                json.dumps(value, sort_keys=True),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_corpus_manifest(contract, manifest)
