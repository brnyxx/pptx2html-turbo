from __future__ import annotations

import json
import os
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate import multiformat_public_pool_fs
from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_public_pool import load_validated_public_pool_sources
from evaluate.multiformat_public_pool_types import PublicPoolError
from evaluate.multiformat_schema import JsonValue, object_value, string_value
from evaluate.tests.multiformat_public_pool_fixture import (
    write_multiformat_public_pool_fixture,
)
from evaluate.multiformat_strict_json import read_strict_object


class MultiFormatPublicPoolFinalBindingTests(unittest.TestCase):
    def test_source_mutation_at_exact_tree_boundary_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))
            source = _first_source_path(fixture.manifest)
            relative_path = string_value(_first_source(fixture.manifest), "path")
            original = source.read_bytes()

            def mutate_at_boundary(root: Path, _expected: set[Path]) -> None:
                candidate = root / relative_path
                before = candidate.stat()
                value = bytearray(candidate.read_bytes())
                marker = value.find(b"public-0")
                self.assertGreaterEqual(marker, 0)
                value[marker : marker + len(b"public-0")] = b"public-1"
                candidate.write_bytes(value)
                os.utime(
                    candidate,
                    ns=(before.st_atime_ns, before.st_mtime_ns),
                    follow_symlinks=False,
                )
                self.assertEqual(candidate.stat().st_size, len(original))
                self.assertEqual(candidate.stat().st_mtime_ns, before.st_mtime_ns)

            with (
                patch.object(
                    multiformat_public_pool_fs,
                    "_before_exact_tree_validation",
                    side_effect=mutate_at_boundary,
                    create=True,
                ),
                self.assertRaises(PublicPoolError),
            ):
                _ = load_validated_public_pool_sources(
                    fixture.config,
                    fixture.manifest,
                )

            self.assertNotEqual(source.read_bytes(), original)

    def test_final_manifest_bytes_and_identity_are_bound(self) -> None:
        mutations = (
            "status",
            "malformed",
            "noncanonical",
            "replaced",
        )
        for mutation in mutations:
            with (
                self.subTest(mutation=mutation),
                tempfile.TemporaryDirectory() as temp_dir,
            ):
                fixture = write_multiformat_public_pool_fixture(Path(temp_dir))

                def mutate_at_boundary(root: Path, _expected: set[Path]) -> None:
                    manifest = root / "public-pool.json"
                    if mutation == "status":
                        values = read_strict_object(manifest)
                        values["status"] = "MUTATED"
                        write_canonical_json(manifest, values)
                    elif mutation == "malformed":
                        manifest.write_bytes(b"{\n")
                    elif mutation == "noncanonical":
                        values = read_strict_object(manifest)
                        manifest.write_bytes(json.dumps(values).encode("utf-8"))
                    else:
                        replacement = root / "replacement.json"
                        shutil.copyfile(manifest, replacement)
                        os.replace(replacement, manifest)

                with (
                    patch.object(
                        multiformat_public_pool_fs,
                        "_before_exact_tree_validation",
                        side_effect=mutate_at_boundary,
                        create=True,
                    ),
                    self.assertRaises(PublicPoolError),
                ):
                    _ = load_validated_public_pool_sources(
                        fixture.config,
                        fixture.manifest,
                    )


def _first_source(manifest: Path) -> dict[str, JsonValue]:
    values = read_strict_object(manifest)
    formats = object_value(values, "formats")
    return object_list(object_value(formats, "docx"), "sources", "test")[0]


def _first_source_path(manifest: Path) -> Path:
    return manifest.parent / string_value(_first_source(manifest), "path")


if __name__ == "__main__":
    _ = unittest.main()
