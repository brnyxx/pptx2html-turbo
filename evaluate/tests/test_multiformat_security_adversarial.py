from __future__ import annotations

import struct
import tempfile
import unittest
import zipfile
from pathlib import Path

from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_security_fixture import validate_security_fixture
from evaluate.tests.multiformat_security_cfb_fixture import (
    LINK_STREAM_OFFSET,
    PRIMARY_ENTRY_OFFSET,
    ROOT_ENTRY_OFFSET,
    SEMANTIC_ENTRY_OFFSET,
    CHILD_ENTRY_OFFSET,
    _directory_entry,
)
from evaluate.tests.multiformat_security_source_fixture import write_security_source
from evaluate.tests.multiformat_source_fixture import (
    END_OF_CHAIN,
    FREE_SECTOR,
    OLE_SIGNATURE,
    write_positive_source,
)


class MultiFormatSecurityAdversarialTests(unittest.TestCase):
    def test_ooxml_trigger_must_be_reachable_from_office_document(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "unreferenced.docx"
            write_positive_source(source, "docx", "plain")
            with zipfile.ZipFile(source, "a") as archive:
                archive.writestr(
                    "security/entity.xml",
                    b'<!DOCTYPE root [<!ENTITY x "expanded">]><root>&x;</root>',
                )

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.DOCX,
                    "entity-expansion",
                )

    def test_malformed_cfbf_requires_coherent_container(self) -> None:
        value = bytearray(1024)
        struct.pack_into("<HHH", value, 26, 3, 0xFFFE, 9)
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "header-only.doc"
            source.write_bytes(value)

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.DOC,
                    "malformed-cfbf",
                )

    def test_legacy_primary_stream_must_be_at_root(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "nested-primary.doc"
            write_positive_source(source, "doc", "nested-primary")
            value = bytearray(source.read_bytes())
            value[0] ^= 0xFF
            value[PRIMARY_ENTRY_OFFSET : PRIMARY_ENTRY_OFFSET + 128] = _directory_entry(
                "Nested", 1, child=3, right=2
            )
            value[CHILD_ENTRY_OFFSET : CHILD_ENTRY_OFFSET + 128] = _directory_entry(
                "WordDocument",
                2,
                start_sector=1,
                stream_size=4096,
            )
            source.write_bytes(value)

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.DOC,
                    "malformed-cfbf",
                )

    def test_difat_overflow_requires_coherent_container(self) -> None:
        value = bytearray(1024)
        value[:8] = OLE_SIGNATURE
        struct.pack_into("<HHH", value, 26, 3, 0xFFFE, 9)
        struct.pack_into("<II", value, 68, 999, 1)
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "header-only.doc"
            source.write_bytes(value)

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.DOC,
                    "difat-overflow",
                )

    def test_malformed_fixture_must_match_declared_office_subtype(self) -> None:
        cases = [
            ("xlsx", DocumentFormat.DOCX, "malformed-zip"),
            ("xls", DocumentFormat.DOC, "malformed-cfbf"),
        ]
        for source_format, declared_format, family in cases:
            with self.subTest(source_format=source_format):
                with tempfile.TemporaryDirectory() as temp_dir:
                    source = Path(temp_dir) / f"source.{source_format}"
                    renamed = Path(temp_dir) / f"renamed.{declared_format.value}"
                    write_security_source(source, source_format, family)
                    renamed.write_bytes(source.read_bytes())

                    with self.assertRaisesRegex(CorpusError, "security.fixture"):
                        validate_security_fixture(
                            renamed,
                            declared_format,
                            family,
                        )

    def test_malformed_zip_subtype_cannot_be_forged_with_filename_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "source.xlsx"
            renamed = Path(temp_dir) / "renamed.docx"
            write_security_source(source, "xlsx", "malformed-zip")
            renamed.write_bytes(source.read_bytes() + b"word/document.xml")

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    renamed,
                    DocumentFormat.DOCX,
                    "malformed-zip",
                )

    def test_cfb_external_target_must_belong_to_link_stream(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "disconnected.doc"
            write_security_source(source, "doc", "external-link")
            value = bytearray(source.read_bytes())
            target = b"https://example.invalid/security-link"
            value[LINK_STREAM_OFFSET : LINK_STREAM_OFFSET + len(target)] = (
                b"\x00" * len(target)
            )
            value[-len(target) :] = target
            source.write_bytes(value)

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.DOC,
                    "external-link",
                )

    def test_cfb_security_stream_must_be_root_reachable(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "detached.doc"
            write_security_source(source, "doc", "external-link")
            value = bytearray(source.read_bytes())
            struct.pack_into(
                "<I",
                value,
                PRIMARY_ENTRY_OFFSET + 72,
                FREE_SECTOR,
            )
            source.write_bytes(value)

            with self.assertRaisesRegex(CorpusError, "security.fixture"):
                validate_security_fixture(
                    source,
                    DocumentFormat.DOC,
                    "external-link",
                )

    def test_cfb_storage_trigger_must_own_child_stream(self) -> None:
        for family in ("macro-storage", "embedded-object"):
            with self.subTest(family=family):
                with tempfile.TemporaryDirectory() as temp_dir:
                    source = Path(temp_dir) / "sibling.doc"
                    write_security_source(source, "doc", family)
                    value = bytearray(source.read_bytes())
                    struct.pack_into(
                        "<I",
                        value,
                        SEMANTIC_ENTRY_OFFSET + 76,
                        FREE_SECTOR,
                    )
                    struct.pack_into(
                        "<I",
                        value,
                        SEMANTIC_ENTRY_OFFSET + 72,
                        3,
                    )
                    source.write_bytes(value)

                    with self.assertRaisesRegex(CorpusError, "security.fixture"):
                        validate_security_fixture(
                            source,
                            DocumentFormat.DOC,
                            family,
                        )

    def test_cfb_mini_stream_corruption_checks_declared_chain(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "mini.doc"
            write_security_source(source, "doc", "external-link")
            value = bytearray(source.read_bytes())
            struct.pack_into("<II", value, 60, 9, 1)
            struct.pack_into("<I", value, ROOT_ENTRY_OFFSET + 116, 1)
            struct.pack_into("<Q", value, ROOT_ENTRY_OFFSET + 120, 4096)
            struct.pack_into("<I", value, SEMANTIC_ENTRY_OFFSET + 116, 999)
            struct.pack_into("<Q", value, SEMANTIC_ENTRY_OFFSET + 120, 64)
            source.write_bytes(value)

            validate_security_fixture(
                source,
                DocumentFormat.DOC,
                "mini-stream-corruption",
            )

    def test_cfb_nested_reachable_stream_is_semantically_checked(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            source = Path(temp_dir) / "nested.doc"
            write_positive_source(source, "doc", "nested")
            value = bytearray(source.read_bytes())
            value[SEMANTIC_ENTRY_OFFSET : SEMANTIC_ENTRY_OFFSET + 128] = (
                _directory_entry("Nested", 1, child=3)
            )
            value[CHILD_ENTRY_OFFSET : CHILD_ENTRY_OFFSET + 128] = _directory_entry(
                "Broken",
                2,
                start_sector=END_OF_CHAIN,
                stream_size=8192,
            )
            source.write_bytes(value)

            validate_security_fixture(
                source,
                DocumentFormat.DOC,
                "truncated-stream",
            )
