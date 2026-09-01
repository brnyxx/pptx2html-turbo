from __future__ import annotations

import os
import shutil
import tempfile
import unittest
from functools import partial
from pathlib import Path
from unittest.mock import patch

from evaluate import multiformat_corpus_sources, multiformat_public_pool_fs
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_public_pool import load_validated_public_pool_sources
from evaluate.multiformat_public_pool_bindings import ExpectedFileBinding
from evaluate.multiformat_public_pool_types import PublicPoolError
from evaluate.multiformat_schema import object_value, string_value
from evaluate.multiformat_source_fixture import write_positive_source
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_public_pool_fixture import (
    write_multiformat_public_pool_fixture,
)


class MultiFormatPublicPoolFilesystemTests(unittest.TestCase):
    def test_symlink_source_replacement_at_exact_tree_boundary_is_rejected(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))
            source = self._first_source_path(fixture.manifest)
            relative_path = source.relative_to(fixture.manifest.parent).as_posix()
            attacker = Path(temp_dir) / "attacker-source.docx"
            shutil.copyfile(source, attacker)
            attacker_bytes = attacker.read_bytes()
            attacker_identity = _entry_identity(attacker)

            with (
                patch.object(
                    multiformat_public_pool_fs,
                    "_before_exact_tree_validation",
                    side_effect=partial(
                        _replace_source_with_symlink,
                        relative_path=relative_path,
                        attacker=attacker,
                    ),
                    create=True,
                ),
                self.assertRaisesRegex(PublicPoolError, "public pool symlink"),
            ):
                _ = load_validated_public_pool_sources(
                    fixture.config,
                    fixture.manifest,
                )

            self.assertTrue(source.is_symlink())
            self.assertEqual(attacker.read_bytes(), attacker_bytes)
            self.assertEqual(_entry_identity(attacker), attacker_identity)

    def test_hard_link_source_replacement_at_exact_tree_boundary_is_rejected(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))
            source = self._first_source_path(fixture.manifest)
            relative_path = source.relative_to(fixture.manifest.parent).as_posix()
            attacker = Path(temp_dir) / "attacker-source.docx"
            shutil.copyfile(source, attacker)
            attacker_bytes = attacker.read_bytes()
            attacker_inode = attacker.stat().st_ino

            with (
                patch.object(
                    multiformat_public_pool_fs,
                    "_before_exact_tree_validation",
                    side_effect=partial(
                        _replace_source_with_hard_link,
                        relative_path=relative_path,
                        attacker=attacker,
                    ),
                    create=True,
                ),
                self.assertRaisesRegex(PublicPoolError, "hard link"),
            ):
                _ = load_validated_public_pool_sources(
                    fixture.config,
                    fixture.manifest,
                )

            self.assertEqual(source.stat().st_ino, attacker_inode)
            self.assertEqual(source.stat().st_nlink, 2)
            self.assertEqual(attacker.read_bytes(), attacker_bytes)
            self.assertEqual(attacker.stat().st_ino, attacker_inode)

    def test_source_symlink_is_rejected_as_typed_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))
            source = self._first_source_path(fixture.manifest)
            external = Path(temp_dir) / "external.docx"
            write_positive_source(external, "docx", "external")
            external_bytes = external.read_bytes()
            source.unlink()
            source.symlink_to(external)

            with self.assertRaises(PublicPoolError):
                _ = load_validated_public_pool_sources(
                    fixture.config,
                    fixture.manifest,
                )

            self.assertTrue(source.is_symlink())
            self.assertEqual(external.read_bytes(), external_bytes)

    def test_source_hard_link_is_rejected_as_typed_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))
            source = self._first_source_path(fixture.manifest)
            hard_link = Path(temp_dir) / "attacker-hard-link.docx"
            hard_link.hardlink_to(source)

            with self.assertRaises(PublicPoolError) as raised:
                _ = load_validated_public_pool_sources(
                    fixture.config,
                    fixture.manifest,
                )

            cause = raised.exception.__cause__
            if not isinstance(cause, CorpusError):
                self.fail("source link failure was not chained")
            self.assertEqual(cause.reason, "source.link")
            self.assertEqual(source.stat().st_nlink, 2)
            self.assertEqual(hard_link.stat().st_ino, source.stat().st_ino)

    def test_same_inode_mutation_after_validation_before_tuple_is_rejected(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))

            def mutate_after_validation(path: Path) -> None:
                before = path.stat()
                value = bytearray(path.read_bytes())
                marker = value.find(b"public-0")
                self.assertGreaterEqual(marker, 0)
                value[marker : marker + len(b"public-0")] = b"public-1"
                path.write_bytes(value)
                os.utime(
                    path,
                    ns=(before.st_atime_ns, before.st_mtime_ns),
                    follow_symlinks=False,
                )
                after = path.stat()
                self.assertEqual(after.st_size, before.st_size)
                self.assertEqual(after.st_mtime_ns, before.st_mtime_ns)

            with (
                patch.object(
                    multiformat_corpus_sources,
                    "_after_source_validation",
                    side_effect=mutate_after_validation,
                ),
                self.assertRaises(PublicPoolError),
            ):
                _ = load_validated_public_pool_sources(
                    fixture.config,
                    fixture.manifest,
                )

    def test_source_substitution_at_final_boundary_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))
            source = self._first_source_path(fixture.manifest)
            external = Path(temp_dir) / "external.docx"
            write_positive_source(external, "docx", "external")
            external_bytes = external.read_bytes()
            original = source.read_bytes()

            def substitute_at_final_boundary(path: Path) -> None:
                _ = source.rename(source.with_name("validated-original.docx"))
                shutil.copyfile(external, path)

            with (
                patch.object(
                    multiformat_corpus_sources,
                    "_before_source_final_verification",
                    side_effect=substitute_at_final_boundary,
                ),
                self.assertRaises(PublicPoolError),
            ):
                _ = load_validated_public_pool_sources(
                    fixture.config,
                    fixture.manifest,
                )

            self.assertEqual(external.read_bytes(), external_bytes)
            self.assertEqual(
                (source.with_name("validated-original.docx")).read_bytes(),
                original,
            )

    @staticmethod
    def _first_source_path(manifest: Path) -> Path:
        values = read_strict_object(manifest)
        formats = object_value(values, "formats")
        source_values = object_list(object_value(formats, "docx"), "sources", "test")
        return manifest.parent / string_value(source_values[0], "path")


def _replace_source_with_symlink(
    root: Path,
    _expected: tuple[ExpectedFileBinding, ...],
    *,
    relative_path: str,
    attacker: Path,
) -> None:
    source = root / relative_path
    source.unlink()
    source.symlink_to(attacker)


def _replace_source_with_hard_link(
    root: Path,
    _expected: tuple[ExpectedFileBinding, ...],
    *,
    relative_path: str,
    attacker: Path,
) -> None:
    source = root / relative_path
    source.unlink()
    os.link(attacker, source)


def _entry_identity(path: Path) -> tuple[int, int, int, int]:
    value = path.stat()
    return value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns


if __name__ == "__main__":
    _ = unittest.main()
