from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import CorpusError, DocumentFormat
from evaluate.multiformat_schema import object_value
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture


class MultiFormatCorpusDescriptorTests(unittest.TestCase):
    def test_format_validation_does_not_reopen_owned_descriptor_via_dev_fd(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            _, manifest_path = ready_fixture(root)
            manifest = read_strict_object(manifest_path)
            tracks = object_value(manifest, "tracks")
            conformance = object_value(tracks, "conformance")
            item = object_list(conformance, "items", "conformance")[0]
            original_stat = Path.stat

            def reject_descriptor_path(
                path: Path,
                *,
                follow_symlinks: bool = True,
            ) -> os.stat_result:
                if path.as_posix().startswith("/dev/fd/"):
                    raise OSError(9, "Bad file descriptor", path)
                return original_stat(path, follow_symlinks=follow_symlinks)

            with patch.object(Path, "stat", new=reject_descriptor_path):
                try:
                    validate_source(
                        item,
                        manifest_path.parent,
                        DocumentFormat.DOCX,
                        require_valid_format=True,
                    )
                except CorpusError as error:
                    self.fail(f"format validator reopened /dev/fd: {error}")


if __name__ == "__main__":
    unittest.main()
