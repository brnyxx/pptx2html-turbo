from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path
from typing import cast

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_security_snapshot import (
    SecuritySnapshotError,
    generate_security_snapshot,
)
from evaluate.multiformat_security_snapshot_validation import (
    validate_security_snapshot,
)

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class ValidateMultiFormatSecuritySourcesTests(unittest.TestCase):
    def test_validates_exact_seventy_source_snapshot(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            manifest = self._snapshot(Path(temp_dir))

            result = validate_security_snapshot(CONTRACT, manifest)

            self.assertEqual(
                result.counts,
                {
                    "doc": 10,
                    "docx": 10,
                    "pdf": 10,
                    "ppt": 10,
                    "pptx": 10,
                    "xls": 10,
                    "xlsx": 10,
                },
            )
            self.assertEqual(result.files, 71)
            self.assertEqual(result.status, "GENERATED")
            self.assertEqual(result.manifest_sha256, sha256_file(manifest))

    def test_rejects_noncanonical_and_duplicate_key_manifests(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            manifest = self._snapshot(Path(temp_dir))
            manifest.write_bytes(manifest.read_bytes() + b" ")

            with self.assertRaises(SecuritySnapshotError):
                validate_security_snapshot(CONTRACT, manifest)

        with tempfile.TemporaryDirectory() as temp_dir:
            manifest = self._snapshot(Path(temp_dir))
            manifest.write_text(
                manifest.read_text().replace(
                    '"schema_version": 1,',
                    '"schema_version": 1, "schema_version": 1,',
                    1,
                ),
                encoding="utf-8",
            )

            with self.assertRaises(SecuritySnapshotError):
                validate_security_snapshot(CONTRACT, manifest)

    def test_rejects_manifest_record_and_hash_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            manifest = self._snapshot(Path(temp_dir))
            values = self._values(manifest)
            source = self._source(values, "doc", 0)
            source["expected_outcome"] = "safe-convert"
            write_canonical_json(manifest, values)

            with self.assertRaises(SecuritySnapshotError):
                validate_security_snapshot(CONTRACT, manifest)

        with tempfile.TemporaryDirectory() as temp_dir:
            manifest = self._snapshot(Path(temp_dir))
            values = self._values(manifest)
            source = self._source(values, "doc", 0)
            (manifest.parent / string_value(source, "path")).write_bytes(b"tampered")

            with self.assertRaises(SecuritySnapshotError):
                validate_security_snapshot(CONTRACT, manifest)

    def test_rejects_semantically_substituted_source(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            manifest = self._snapshot(Path(temp_dir))
            values = self._values(manifest)
            source = self._source(values, "pdf", 0)
            replacement = self._source(values, "pdf", 1)
            source_path = manifest.parent / string_value(source, "path")
            source_path.write_bytes(
                (manifest.parent / string_value(replacement, "path")).read_bytes()
            )
            source["sha256"] = sha256_file(source_path)
            write_canonical_json(manifest, values)

            with self.assertRaises(SecuritySnapshotError):
                validate_security_snapshot(CONTRACT, manifest)

    def test_rejects_extra_files_symlinks_and_hard_links(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            manifest = self._snapshot(Path(temp_dir))
            (manifest.parent / "extra").write_bytes(b"extra")

            with self.assertRaises(SecuritySnapshotError):
                validate_security_snapshot(CONTRACT, manifest)

        with tempfile.TemporaryDirectory() as temp_dir:
            manifest = self._snapshot(Path(temp_dir))
            values = self._values(manifest)
            first = self._source(values, "pdf", 0)
            second = self._source(values, "pdf", 1)
            source = manifest.parent / string_value(first, "path")
            source.unlink()
            source.symlink_to(manifest.parent / string_value(second, "path"))

            with self.assertRaises(SecuritySnapshotError):
                validate_security_snapshot(CONTRACT, manifest)

        with tempfile.TemporaryDirectory() as temp_dir:
            manifest = self._snapshot(Path(temp_dir))
            values = self._values(manifest)
            first = self._source(values, "pdf", 0)
            second = self._source(values, "pdf", 1)
            source = manifest.parent / string_value(first, "path")
            source.unlink()
            os.link(manifest.parent / string_value(second, "path"), source)

            with self.assertRaises(SecuritySnapshotError):
                validate_security_snapshot(CONTRACT, manifest)

    def test_requires_canonical_manifest_name_and_exact_file_set(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            manifest = self._snapshot(Path(temp_dir))
            alternate = manifest.with_name("alternate.json")
            alternate.write_bytes(manifest.read_bytes())

            with self.assertRaises(SecuritySnapshotError):
                validate_security_snapshot(CONTRACT, alternate)

            manifest.unlink()
            with self.assertRaises(SecuritySnapshotError):
                validate_security_snapshot(CONTRACT, manifest)

    def _snapshot(self, root: Path) -> Path:
        output = root / "security"
        generate_security_snapshot(
            CONTRACT,
            output,
            validator=lambda contract, manifest: None,
        )
        return output / "security-sources.json"

    def _values(self, manifest: Path) -> dict[str, JsonValue]:
        return cast(
            dict[str, JsonValue],
            json.loads(manifest.read_text(encoding="utf-8")),
        )

    def _source(
        self,
        values: dict[str, JsonValue],
        document_format: str,
        index: int,
    ) -> dict[str, JsonValue]:
        formats = object_value(values, "formats")
        format_value = object_value(formats, document_format)
        return object_list(
            format_value,
            "sources",
            "test.sources",
        )[index]


if __name__ == "__main__":
    unittest.main()
