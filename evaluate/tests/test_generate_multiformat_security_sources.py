from __future__ import annotations

import json
import tempfile
import unittest
import zipfile
from pathlib import Path

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_schema import sha256_file
from evaluate.multiformat_security_snapshot import (
    SecuritySnapshotError,
    generate_security_snapshot,
)
from evaluate.multiformat_security_source import write_security_source

PROJECT_ROOT = Path(__file__).resolve().parents[2]
CONTRACT = PROJECT_ROOT / "evaluate" / "multiformat" / "contract.v1.json"


class GenerateMultiFormatSecuritySourcesTests(unittest.TestCase):
    def test_generates_exact_contract_snapshot(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "security"

            result = generate_security_snapshot(
                CONTRACT,
                output,
                validator=self._accept,
            )

            manifest = output / "security-sources.json"
            values = json.loads(manifest.read_text(encoding="utf-8"))
            contract = json.loads(CONTRACT.read_text(encoding="utf-8"))
            self.assertEqual(result.counts, {name: 10 for name in result.counts})
            self.assertEqual(result.files, 71)
            self.assertEqual(result.status, "GENERATED")
            self.assertEqual(result.manifest_sha256, sha256_file(manifest))
            self.assertEqual(values["contract_sha256"], sha256_file(CONTRACT))
            self.assertEqual(
                set(values["formats"]),
                set(contract["required_formats"]),
            )
            ids: set[str] = set()
            paths: set[str] = set()
            digests: set[str] = set()
            for format_name, format_value in values["formats"].items():
                outcomes = contract["security_case_outcomes"][format_name]
                self.assertEqual(format_value["expected_count"], 10)
                self.assertEqual(len(format_value["sources"]), 10)
                self.assertEqual(
                    [item["case_family"] for item in format_value["sources"]],
                    sorted(outcomes),
                )
                for item in format_value["sources"]:
                    family = item["case_family"]
                    self.assertEqual(
                        item,
                        {
                            "case_family": family,
                            "expected_outcome": outcomes[family],
                            "id": f"security-{format_name}-{family}",
                            "path": (f"sources/{format_name}/{family}.{format_name}"),
                            "sha256": item["sha256"],
                        },
                    )
                    ids.add(item["id"])
                    paths.add(item["path"])
                    digests.add(item["sha256"])
            self.assertEqual((len(ids), len(paths), len(digests)), (70, 70, 70))
            self.assertEqual(
                sum(1 for path in output.rglob("*") if path.is_file()),
                71,
            )

    def test_two_clean_generations_are_byte_identical(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            first = root / "first"
            second = root / "second"

            generate_security_snapshot(CONTRACT, first, validator=self._accept)
            generate_security_snapshot(CONTRACT, second, validator=self._accept)

            self.assertEqual(self._tree(first), self._tree(second))

    def test_every_ooxml_zip_entry_has_fixed_timestamp(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "security"
            generate_security_snapshot(CONTRACT, output, validator=self._accept)

            for document_format in ("docx", "pptx", "xlsx"):
                for source in sorted((output / "sources" / document_format).iterdir()):
                    if source.stem == "malformed-zip":
                        continue
                    with (
                        self.subTest(source=source),
                        zipfile.ZipFile(source) as archive,
                    ):
                        self.assertTrue(
                            all(
                                item.date_time == (1980, 1, 1, 0, 0, 0)
                                for item in archive.infolist()
                            )
                        )

    def test_existing_destination_and_lock_are_never_mutated(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            output = root / "security"
            output.mkdir()
            sentinel = output / "sentinel"
            sentinel.write_bytes(b"owned")

            with self.assertRaises(SecuritySnapshotError):
                generate_security_snapshot(
                    CONTRACT,
                    output,
                    validator=self._accept,
                )

            self.assertEqual(sentinel.read_bytes(), b"owned")

        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            output = root / "security"
            lock = root / ".security.security-sources.lock"
            lock.write_bytes(b"other")

            with self.assertRaises(SecuritySnapshotError):
                generate_security_snapshot(
                    CONTRACT,
                    output,
                    validator=self._accept,
                )

            self.assertEqual(lock.read_bytes(), b"other")
            self.assertFalse(output.exists())

    def test_writer_failure_leaves_destination_absent(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "security"
            calls = 0

            def fail_after_two(
                path: Path,
                document_format: DocumentFormat,
                family: str,
            ) -> None:
                nonlocal calls
                calls += 1
                if calls == 3:
                    raise OSError("injected")
                write_security_source(path, document_format, family)

            with self.assertRaises(SecuritySnapshotError):
                generate_security_snapshot(
                    CONTRACT,
                    output,
                    writer=fail_after_two,
                    validator=self._accept,
                )

            self.assertFalse(output.exists())
            self.assertEqual(
                tuple(Path(temp_dir).glob(".security.stage-*")),
                (),
            )

    def test_contract_mutation_before_publish_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract = root / "contract.json"
            contract.write_bytes(CONTRACT.read_bytes())
            output = root / "security"

            def mutate_contract() -> None:
                contract.write_bytes(contract.read_bytes() + b" ")

            with self.assertRaises(SecuritySnapshotError):
                generate_security_snapshot(
                    contract,
                    output,
                    validator=self._accept,
                    before_publish=mutate_contract,
                )

            self.assertFalse(output.exists())

    def test_manifest_uses_canonical_json_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "security"
            generate_security_snapshot(CONTRACT, output, validator=self._accept)
            manifest = output / "security-sources.json"
            values = json.loads(manifest.read_text(encoding="utf-8"))

            self.assertEqual(
                manifest.read_bytes(),
                (
                    json.dumps(
                        values,
                        ensure_ascii=True,
                        indent=2,
                        sort_keys=True,
                    )
                    + "\n"
                ).encode(),
            )

    def _accept(self, contract: Path, manifest: Path) -> None:
        self.assertTrue(contract.is_file())
        self.assertEqual(manifest.name, "security-sources.json")
        self.assertTrue(manifest.is_file())

    def _tree(self, root: Path) -> dict[str, bytes]:
        return {
            path.relative_to(root).as_posix(): path.read_bytes()
            for path in sorted(root.rglob("*"))
            if path.is_file()
        }


if __name__ == "__main__":
    unittest.main()
