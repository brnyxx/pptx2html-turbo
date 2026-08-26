from __future__ import annotations

import tempfile
import unittest
from collections.abc import Generator
from contextlib import contextmanager
from pathlib import Path
from typing import ClassVar
from unittest.mock import patch

from evaluate.multiformat_ready_assembly import assemble_ready_corpora
from evaluate.multiformat_ready_assembly_types import (
    ReadyAssemblyError,
    ReadyAssemblyInputs,
    ReadyValidationInputs,
)
from evaluate.multiformat_ready_types import ReadyInputPaths, ReadySourceSet
from evaluate.multiformat_ready_validation import validate_ready_corpora
from evaluate.multiformat_schema import object_value
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_ready_manifest_fixture import (
    CONTRACT,
    make_manifest_sources,
)


def _fixture() -> tuple[
    tempfile.TemporaryDirectory[str], Path, ReadyInputPaths, ReadySourceSet
]:
    temporary = tempfile.TemporaryDirectory()
    root = Path(temporary.name)
    sources = make_manifest_sources(root / "sources")
    upstream = root / "upstream.json"
    _ = upstream.write_text("{}\n", encoding="utf-8")
    plan = root / "plan.json"
    _ = plan.write_text("{}\n", encoding="utf-8")
    inventory = root / "inventory"
    inventory.mkdir()
    _ = (inventory / "native-unit-inventory.json").write_text("{}\n", encoding="utf-8")
    paths = ReadyInputPaths(
        CONTRACT,
        plan,
        upstream,
        upstream,
        upstream,
        upstream,
        upstream,
        upstream,
        upstream,
        upstream,
        upstream,
        upstream,
        upstream,
        upstream,
        upstream,
        upstream,
        inventory,
    )
    return temporary, root, paths, sources


_TEMPORARY, _ROOT, _PATHS, _SOURCES = _fixture()


class MultiFormatReadyAssemblyTests(unittest.TestCase):
    temporary: ClassVar[tempfile.TemporaryDirectory[str]] = _TEMPORARY
    root: ClassVar[Path] = _ROOT
    paths: ClassVar[ReadyInputPaths] = _PATHS
    sources: ClassVar[ReadySourceSet] = _SOURCES

    def test_assembles_and_independently_validates_exact_tree(self) -> None:
        output = self.root / "assembled"
        with self._loaded_sources():
            summary = assemble_ready_corpora(ReadyAssemblyInputs(self.paths, output))
            checked = validate_ready_corpora(ReadyValidationInputs(self.paths, output))
        self.assertEqual(summary, checked)
        self.assertEqual(
            (summary.status, summary.files, summary.sources, summary.supports),
            ("VALIDATED", 1485, 1295, 180),
        )
        self.assertFalse((output / "READY").exists())
        manifest = output / "assembly-manifest.json"
        root_values = read_strict_object(manifest)
        corpora = object_value(root_values, "corpora")
        self.assertTrue(
            all(
                set(object_value(corpora, name))
                == {
                    "path",
                    "sha256",
                    "conformance_units",
                    "blind_files",
                    "security_cases",
                    "support_files",
                }
                for name in corpora
            )
        )
        _ = manifest.write_bytes(
            manifest.read_bytes().replace(b'"VALIDATED"', b'"CAPTURED"')
        )
        with self._loaded_sources(), self.assertRaises(ReadyAssemblyError):
            _ = validate_ready_corpora(ReadyValidationInputs(self.paths, output))

    def test_two_assemblies_are_byte_identical(self) -> None:
        first, second = self.root / "deterministic-a", self.root / "deterministic-b"
        with self._loaded_sources():
            _ = assemble_ready_corpora(ReadyAssemblyInputs(self.paths, first))
            _ = assemble_ready_corpora(ReadyAssemblyInputs(self.paths, second))
        first_files = _files(first)
        second_files = _files(second)
        self.assertEqual(first_files, second_files)
        for relative in first_files:
            self.assertEqual(
                (first / relative).read_bytes(), (second / relative).read_bytes()
            )

    def test_source_mutation_fails_without_partial_publication(self) -> None:
        output = self.root / "failed"
        source = self.sources.sources[0]
        original = source.source_path.read_bytes()

        def mutate(selected: Path, _destination: Path) -> None:
            if selected == source.source_path:
                _ = selected.write_bytes(b"changed")

        try:
            with (
                self._loaded_sources(),
                patch(
                    "evaluate.multiformat_ready_copy._before_copy", side_effect=mutate
                ),
                self.assertRaises(ReadyAssemblyError),
            ):
                _ = assemble_ready_corpora(ReadyAssemblyInputs(self.paths, output))
        finally:
            _ = source.source_path.write_bytes(original)
        self.assertFalse(output.exists())

    def test_dangling_ready_marker_fails_at_schema_boundary(self) -> None:
        output = self.root / "dangling-ready"
        with self._loaded_sources():
            _ = assemble_ready_corpora(ReadyAssemblyInputs(self.paths, output))
        _ = (output / "READY").symlink_to(output / "missing")

        with (
            self._loaded_sources(),
            self.assertRaises(ReadyAssemblyError) as raised,
        ):
            _ = validate_ready_corpora(ReadyValidationInputs(self.paths, output))

        self.assertIn("aggregate READY marker is forbidden", raised.exception.detail)

    @contextmanager
    def _loaded_sources(self) -> Generator[None, None, None]:
        with (
            patch(
                "evaluate.multiformat_ready_assembly.load_ready_inputs",
                return_value=self.sources,
            ),
            patch(
                "evaluate.multiformat_ready_validation.load_ready_inputs",
                return_value=self.sources,
            ),
        ):
            yield


def _files(root: Path) -> set[str]:
    return {
        path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file()
    }


if __name__ == "__main__":
    _ = unittest.main()
