from __future__ import annotations

import os
import shutil
import socket
import stat
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate import multiformat_corpus_sources
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_public_pool import load_validated_public_pool_sources
from evaluate.multiformat_public_pool_types import PublicPoolError
from evaluate.multiformat_schema import object_value, string_value
from evaluate.multiformat_source_fixture import write_positive_source
from evaluate.tests.multiformat_public_pool_fixture import (
    write_multiformat_public_pool_fixture,
)
from evaluate.multiformat_strict_json import read_strict_object


class MultiFormatPublicPoolFilesystemTests(unittest.TestCase):
    @unittest.skipUnless(os.name == "posix", "requires POSIX filesystem entries")
    def test_exact_tree_rejects_fifo_socket_and_empty_directory_untouched(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = write_multiformat_public_pool_fixture(Path(temp_dir))
            root = fixture.manifest.parent
            fifo = root / "attacker-fifo"
            socket_path = root / "attacker.sock"
            empty_directory = root / "attacker-directory"
            os.mkfifo(fifo)
            attacker_socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            attacker_socket.bind(str(socket_path))
            empty_directory.mkdir()
            before = {
                path: (path.lstat().st_dev, path.lstat().st_ino)
                for path in (fifo, socket_path, empty_directory)
            }

            try:
                with self.assertRaises(PublicPoolError):
                    _ = load_validated_public_pool_sources(
                        fixture.config,
                        fixture.manifest,
                    )

                self.assertTrue(stat.S_ISFIFO(fifo.lstat().st_mode))
                self.assertTrue(stat.S_ISSOCK(socket_path.lstat().st_mode))
                self.assertTrue(stat.S_ISDIR(empty_directory.lstat().st_mode))
                self.assertEqual(
                    before,
                    {
                        path: (path.lstat().st_dev, path.lstat().st_ino)
                        for path in (fifo, socket_path, empty_directory)
                    },
                )
            finally:
                attacker_socket.close()

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
                raise AssertionError("source link failure was not chained")
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


if __name__ == "__main__":
    _ = unittest.main()
