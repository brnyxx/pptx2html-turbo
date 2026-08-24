from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.collect_multiformat_public_pool import _raw_url
from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_public_pool import (
    collect_public_pool,
    load_validated_public_pool_sources,
    validate_public_pool,
)
from evaluate.multiformat_public_pool_config import load_public_pool_plans
from evaluate.multiformat_public_pool_types import (
    PublicPoolError,
    ValidatedPublicPoolSource,
)
from evaluate.multiformat_schema import object_value, sha256_file, string_value
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_public_pool_fixture import (
    public_pool_tree,
    write_multiformat_public_pool_fixture,
    write_public_pool_blobs,
    write_public_pool_config,
)


class CollectMultiFormatPublicPoolTests(unittest.TestCase):
    def test_raw_url_encodes_repository_path_segments(self) -> None:
        self.assertEqual(
            _raw_url(
                "owner/repo",
                "1" * 40,
                "fixtures/Embedded font.doc",
            ),
            (
                "https://raw.githubusercontent.com/owner/repo/"
                + "1" * 40
                + "/fixtures/Embedded%20font.doc"
            ),
        )

    def test_pinned_catalog_requires_75_sources_from_five_groups(self) -> None:
        project_root = Path(__file__).resolve().parents[2]

        plans = load_public_pool_plans(
            project_root / "evaluate" / "multiformat" / "public-pool-sources.v1.json"
        )

        self.assertEqual(len(plans), 7)
        self.assertTrue(all(plan.expected_count == 75 for plan in plans))
        self.assertTrue(all(len(plan.groups) == 5 for plan in plans))
        self.assertTrue(
            all(len(group.commit) == 40 for plan in plans for group in plan.groups)
        )

    def test_collects_exact_independent_valid_sources(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            config = write_public_pool_config(root)
            blobs = write_public_pool_blobs(root)

            manifest = collect_public_pool(
                config,
                root / "output",
                tree_fetcher=public_pool_tree,
                blob_fetcher=lambda repository, commit, path: blobs[(repository, path)],
            )

            values = read_strict_object(manifest)
            formats = object_value(values, "formats")
            sources = object_list(object_value(formats, "docx"), "sources", "test")
            self.assertEqual(len(sources), 5)
            self.assertEqual(
                len({string_value(item, "producer") for item in sources}), 5
            )
            self.assertEqual(len({string_value(item, "sha256") for item in sources}), 5)
            self.assertTrue(
                all(
                    sha256_file(manifest.parent / string_value(item, "path"))
                    == string_value(item, "sha256")
                    for item in sources
                )
            )
            self.assertNotIn(
                "invalid.docx",
                {string_value(item, "repository_path") for item in sources},
            )
            self.assertTrue(
                any("%20" in string_value(item, "source_uri") for item in sources)
            )

    def test_truncated_tree_requires_static_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            config = write_public_pool_config(root)

            with self.assertRaisesRegex(PublicPoolError, "truncated"):
                _ = collect_public_pool(
                    config,
                    root / "output",
                    tree_fetcher=lambda repository, commit: {
                        "truncated": True,
                        "tree": [],
                    },
                    blob_fetcher=lambda repository, commit, path: b"",
                )

            self.assertFalse((root / "output").exists())

    def test_validator_wraps_source_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            config = write_public_pool_config(root)
            blobs = write_public_pool_blobs(root)
            manifest = collect_public_pool(
                config,
                root / "output",
                tree_fetcher=public_pool_tree,
                blob_fetcher=lambda repository, commit, path: blobs[(repository, path)],
            )
            values = read_strict_object(manifest)
            formats = object_value(values, "formats")
            sources = object_list(object_value(formats, "docx"), "sources", "test")
            source = manifest.parent / string_value(sources[0], "path")
            _ = source.write_bytes(b"tampered")

            with self.assertRaises(PublicPoolError):
                validate_public_pool(config, manifest)

    def test_loader_returns_sorted_typed_records(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            config, manifest = self._multi_format_fixture(root)
            values = read_strict_object(manifest)
            formats = object_value(values, "formats")

            result = load_validated_public_pool_sources(config, manifest)

            self.assertEqual(type(result[0]), ValidatedPublicPoolSource)
            expected = sorted(
                (
                    format_name,
                    string_value(source, "id"),
                    string_value(source, "path"),
                    string_value(source, "sha256"),
                )
                for format_name in formats
                for source in object_list(
                    object_value(formats, format_name),
                    "sources",
                    "test",
                )
            )
            self.assertEqual(
                [
                    (
                        item.document_format.value,
                        item.source_id,
                        item.relative_path,
                        item.source_sha256,
                    )
                    for item in result
                ],
                expected,
            )

    def test_loader_rejects_duplicate_same_format_key(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            config, manifest = self._multi_format_fixture(root)
            values = read_strict_object(manifest)
            formats = object_value(values, "formats")
            sources = object_list(object_value(formats, "docx"), "sources", "test")
            sources[1]["id"] = string_value(sources[0], "id")
            write_canonical_json(manifest, values)

            with self.assertRaises(PublicPoolError):
                _ = load_validated_public_pool_sources(config, manifest)

    def test_loader_allows_duplicate_bare_ids_across_formats(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            config, manifest = self._multi_format_fixture(root)
            values = read_strict_object(manifest)
            formats = object_value(values, "formats")
            pdf_sources = object_list(
                object_value(formats, "pdf"),
                "sources",
                "test",
            )
            docx_sources = object_list(
                object_value(formats, "docx"),
                "sources",
                "test",
            )
            pdf_sources[0]["id"] = string_value(docx_sources[0], "id")
            write_canonical_json(manifest, values)

            result = load_validated_public_pool_sources(config, manifest)

            self.assertEqual(len(result), 10)
            self.assertEqual(
                sum(item.source_id == result[0].source_id for item in result),
                2,
            )

    def test_loader_rejects_duplicate_same_format_path(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            config, manifest = self._multi_format_fixture(root)
            values = read_strict_object(manifest)
            formats = object_value(values, "formats")
            docx_sources = object_list(
                object_value(formats, "docx"),
                "sources",
                "test",
            )
            first_source, second_source = docx_sources[:2]
            second_path = manifest.parent / string_value(second_source, "path")
            second_source["path"] = string_value(first_source, "path")
            second_source["sha256"] = string_value(first_source, "sha256")
            _ = second_path.unlink()
            write_canonical_json(manifest, values)

            with self.assertRaisesRegex(
                PublicPoolError,
                "^public pool source path is duplicated$",
            ):
                _ = load_validated_public_pool_sources(config, manifest)

    def test_loader_rejects_duplicate_same_format_digest(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            config, manifest = self._multi_format_fixture(root)
            values = read_strict_object(manifest)
            formats = object_value(values, "formats")
            docx_sources = object_list(
                object_value(formats, "docx"),
                "sources",
                "test",
            )
            first_source, second_source = docx_sources[:2]
            first_path = manifest.parent / string_value(first_source, "path")
            second_path = manifest.parent / string_value(second_source, "path")
            _ = second_path.write_bytes(first_path.read_bytes())
            second_source["sha256"] = string_value(first_source, "sha256")
            write_canonical_json(manifest, values)

            with self.assertRaisesRegex(
                PublicPoolError,
                "^public pool source bytes are duplicated$",
            ):
                _ = load_validated_public_pool_sources(config, manifest)

    def test_validator_delegates_and_preserves_none_api(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            config, manifest = self._multi_format_fixture(root)
            with patch(
                "evaluate.multiformat_public_pool.load_validated_public_pool_sources",
                wraps=load_validated_public_pool_sources,
            ) as loader:
                result = validate_public_pool(config, manifest)

            self.assertIsNone(result)
            loader.assert_called_once_with(config, manifest)

    def _multi_format_fixture(self, root: Path) -> tuple[Path, Path]:
        fixture = write_multiformat_public_pool_fixture(root)
        return fixture.config, fixture.manifest
