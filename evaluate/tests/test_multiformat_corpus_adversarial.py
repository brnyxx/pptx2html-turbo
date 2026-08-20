import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_corpus import CorpusError, validate_corpus_manifest
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_package_validation import MAX_SOURCE_BYTES
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture
from evaluate.tests.multiformat_source_fixture import (
    write_ambiguous_legacy_source,
    write_ambiguous_ooxml_source,
    write_positive_source,
)


class MultiFormatCorpusAdversarialTests(unittest.TestCase):
    def test_blind_source_cannot_reuse_conformance_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, manifest_path = ready_fixture(root)
            source_root = manifest_path.parent
            conformance = source_root / "sources" / "conformance.docx"
            blind = source_root / "sources" / "blind-0.docx"
            blind.write_bytes(conformance.read_bytes())
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["tracks"]["blind"]["items"][0]["sha256"] = hashlib.sha256(
                blind.read_bytes()
            ).hexdigest()
            manifest_path.write_text(
                json.dumps(manifest, sort_keys=True),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(CorpusError, "tracks.sha256"):
                validate_corpus_manifest(contract, manifest_path)

    def test_legacy_container_subtype_is_enforced(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "renamed.xls"
            write_positive_source(source, "doc", "word-container")
            item = {
                "id": "renamed",
                "path": "renamed.xls",
                "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
            }

            with self.assertRaisesRegex(CorpusError, "source.format"):
                validate_source(
                    item,
                    root,
                    DocumentFormat.XLS,
                    require_valid_format=True,
                )

    def test_legacy_polyglot_container_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "ambiguous.doc"
            write_ambiguous_legacy_source(source, "doc")
            item = {
                "id": "ambiguous",
                "path": "ambiguous.doc",
                "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
            }

            with self.assertRaisesRegex(CorpusError, "source.format"):
                validate_source(
                    item,
                    root,
                    DocumentFormat.DOC,
                    require_valid_format=True,
                )

    def test_ooxml_polyglot_package_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "ambiguous.docx"
            write_ambiguous_ooxml_source(source)
            item = {
                "id": "ambiguous",
                "path": "ambiguous.docx",
                "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
            }

            with self.assertRaisesRegex(CorpusError, "source.format"):
                validate_source(
                    item,
                    root,
                    DocumentFormat.DOCX,
                    require_valid_format=True,
                )

    def test_pdf_requires_a_bounded_structural_trailer(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "truncated.pdf"
            source.write_bytes(b"%PDF-1.7\nno trailer")
            item = {
                "id": "truncated",
                "path": "truncated.pdf",
                "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
            }

            with self.assertRaisesRegex(CorpusError, "source.format"):
                validate_source(
                    item,
                    root,
                    DocumentFormat.PDF,
                    require_valid_format=True,
                )

    def test_unit_ordinals_must_cover_each_source_without_gaps(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, manifest_path = ready_fixture(root)
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["tracks"]["conformance"]["items"][0]["units"][1]["ordinal"] = 3
            manifest_path.write_text(
                json.dumps(manifest, sort_keys=True),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(CorpusError, "conformance.unit.ordinal"):
                validate_corpus_manifest(contract, manifest_path)

    def test_security_case_family_and_outcome_are_contract_bound(self) -> None:
        for field, value, reason in [
            ("case_family", "undeclared", "security.expected_outcome"),
            ("expected_outcome", "safe-convert", "security.expected_outcome"),
        ]:
            with self.subTest(field=field):
                with tempfile.TemporaryDirectory() as temp_dir:
                    root = Path(temp_dir)
                    contract, manifest_path = ready_fixture(root)
                    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
                    manifest["tracks"]["security"]["items"][0][field] = value
                    manifest_path.write_text(
                        json.dumps(manifest, sort_keys=True),
                        encoding="utf-8",
                    )

                    with self.assertRaisesRegex(CorpusError, reason):
                        validate_corpus_manifest(contract, manifest_path)

    def test_oversized_source_is_rejected_before_hashing(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "oversized.pdf"
            with source.open("wb") as output:
                output.seek(MAX_SOURCE_BYTES)
                output.write(b"x")
            item = {
                "id": "oversized",
                "path": "oversized.pdf",
                "sha256": "0" * 64,
            }

            with self.assertRaisesRegex(CorpusError, "source.size"):
                validate_source(
                    item,
                    root,
                    DocumentFormat.PDF,
                    require_valid_format=True,
                )

    def test_nested_duplicate_contract_keys_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, manifest = ready_fixture(root)
            text = contract.read_text(encoding="utf-8")
            contract.write_text(
                text.replace(
                    '"blind_files": 5',
                    '"blind_files": 5, "blind_files": 5',
                    1,
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(CorpusError, "manifest.schema"):
                validate_corpus_manifest(contract, manifest)
