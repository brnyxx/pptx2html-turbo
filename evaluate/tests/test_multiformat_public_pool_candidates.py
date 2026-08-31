from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_public_pool_sources import collect_public_source_group
from evaluate.multiformat_public_pool_types import PublicSourceGroup
from evaluate.multiformat_schema import JsonValue
from evaluate.multiformat_source_fixture import write_positive_source


class MultiFormatPublicPoolCandidateTests(unittest.TestCase):
    def test_parenthesized_encrypted_marker_is_not_a_candidate(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.doc"
            write_positive_source(source, DocumentFormat.DOC, "candidate")
            content = source.read_bytes()
            group = PublicSourceGroup(
                "producer",
                "owner/repo",
                "1" * 40,
                "MIT",
                1,
                ("fixtures/",),
                (),
            )
            tree: dict[str, JsonValue] = {
                "truncated": False,
                "tree": [
                    {
                        "path": "fixtures/document (enc).doc",
                        "type": "blob",
                        "size": len(content),
                    },
                    {
                        "path": "fixtures/document.doc",
                        "type": "blob",
                        "size": len(content),
                    },
                ],
            }

            def fetch_blob(
                _repository: str,
                _commit: str,
                _path: str,
            ) -> bytes:
                return content

            values = collect_public_source_group(
                root / "pool",
                DocumentFormat.DOC,
                group,
                tree,
                fetch_blob,
                set(),
            )

        self.assertEqual(values[0]["repository_path"], "fixtures/document.doc")

    def test_dde_link_marker_is_not_a_candidate(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.xlsx"
            write_positive_source(source, DocumentFormat.XLSX, "candidate")
            content = source.read_bytes()
            group = PublicSourceGroup(
                "producer",
                "owner/repo",
                "1" * 40,
                "MIT",
                1,
                ("fixtures/",),
                (),
            )
            tree: dict[str, JsonValue] = {
                "truncated": False,
                "tree": [
                    {
                        "path": "fixtures/001-testDdeLink.xlsx",
                        "type": "blob",
                        "size": len(content),
                    },
                    {
                        "path": "fixtures/002-document.xlsx",
                        "type": "blob",
                        "size": len(content),
                    },
                ],
            }

            values = collect_public_source_group(
                root / "pool",
                DocumentFormat.XLSX,
                group,
                tree,
                lambda _repository, _commit, _path: content,
                set(),
            )

        self.assertEqual(
            values[0]["repository_path"],
            "fixtures/002-document.xlsx",
        )

    def test_pinned_unconvertible_paths_are_not_candidates(self) -> None:
        excluded = (
            "test-data/slideshow/2100a8d44da546f97ab7795c500a58bed6cb655d.ppt",
            (
                "test-data/slideshow/"
                "60f557c0a46bcb0068b1c3e15589dac383307bc8.ppt"
            ),
            (
                "tika-parsers/tika-parsers-standard/"
                "tika-parsers-standard-modules/tika-parser-microsoft-module/"
                "src/test/resources/test-documents/pictures.ppt"
            ),
            "Examples/Data/Presentations/Properties/open_pass1.ppt",
            "testcases/test-data/slideshow/backgrounds.ppt",
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.ppt"
            write_positive_source(source, DocumentFormat.PPT, "candidate")
            content = source.read_bytes()
            group = PublicSourceGroup(
                "producer",
                "owner/repo",
                "1" * 40,
                "MIT",
                1,
                (),
                (),
            )
            for repository_path in excluded:
                with self.subTest(repository_path=repository_path):
                    tree: dict[str, JsonValue] = {
                        "truncated": False,
                        "tree": [
                            {
                                "path": repository_path,
                                "type": "blob",
                                "size": len(content),
                            },
                            {
                                "path": "zzzz/safe.ppt",
                                "type": "blob",
                                "size": len(content),
                            },
                        ],
                    }
                    values = collect_public_source_group(
                        root / repository_path.split("/", 1)[0],
                        DocumentFormat.PPT,
                        group,
                        tree,
                        lambda _repository, _commit, _path: content,
                        set(),
                    )
                    self.assertEqual(
                        values[0]["repository_path"],
                        "zzzz/safe.ppt",
                    )

    def test_pinned_oversized_xlsx_path_is_not_a_candidate(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source.xlsx"
            write_positive_source(source, DocumentFormat.XLSX, "candidate")
            content = source.read_bytes()
            group = PublicSourceGroup(
                "producer",
                "owner/repo",
                "1" * 40,
                "MIT",
                1,
                (),
                (),
            )
            tree: dict[str, JsonValue] = {
                "truncated": False,
                "tree": [
                    {
                        "path": "spec/integration/data/huge.xlsx",
                        "type": "blob",
                        "size": len(content),
                    },
                    {
                        "path": "zzzz/safe.xlsx",
                        "type": "blob",
                        "size": len(content),
                    },
                ],
            }

            values = collect_public_source_group(
                root / "pool",
                DocumentFormat.XLSX,
                group,
                tree,
                lambda _repository, _commit, _path: content,
                set(),
            )

        self.assertEqual(values[0]["repository_path"], "zzzz/safe.xlsx")


if __name__ == "__main__":
    _ = unittest.main()
