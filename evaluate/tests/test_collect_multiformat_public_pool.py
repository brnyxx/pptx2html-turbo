from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from evaluate.collect_multiformat_public_pool import (
    PublicPoolError,
    _raw_url,
    collect_public_pool,
)
from evaluate.multiformat_public_pool import validate_public_pool
from evaluate.multiformat_public_pool_config import load_public_pool_plans
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.multiformat_source_fixture import write_positive_source


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
            config = self._config(root)
            blobs = self._blobs(root)

            manifest = collect_public_pool(
                config,
                root / "output",
                tree_fetcher=self._tree,
                blob_fetcher=lambda repository, commit, path: blobs[(repository, path)],
            )

            values = json.loads(manifest.read_text(encoding="utf-8"))
            sources = values["formats"]["docx"]["sources"]
            self.assertEqual(len(sources), 5)
            self.assertEqual(len({item["producer"] for item in sources}), 5)
            self.assertEqual(len({item["sha256"] for item in sources}), 5)
            self.assertTrue(
                all(
                    sha256_file(manifest.parent / item["path"]) == item["sha256"]
                    for item in sources
                )
            )
            self.assertNotIn(
                "invalid.docx", {item["repository_path"] for item in sources}
            )
            self.assertTrue(any("%20" in item["source_uri"] for item in sources))

    def test_truncated_tree_requires_static_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            config = self._config(root)

            with self.assertRaisesRegex(PublicPoolError, "truncated"):
                collect_public_pool(
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
            config = self._config(root)
            blobs = self._blobs(root)
            manifest = collect_public_pool(
                config,
                root / "output",
                tree_fetcher=self._tree,
                blob_fetcher=lambda repository, commit, path: blobs[(repository, path)],
            )
            values = json.loads(manifest.read_text(encoding="utf-8"))
            source = manifest.parent / values["formats"]["docx"]["sources"][0]["path"]
            source.write_bytes(b"tampered")

            with self.assertRaises(PublicPoolError):
                validate_public_pool(config, manifest)

    def _config(self, root: Path) -> Path:
        config = root / "config.json"
        config.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "formats": {
                        "docx": {
                            "expected_count": 5,
                            "groups": [
                                {
                                    "producer": f"producer-{index}",
                                    "repository": f"owner/repo-{index}",
                                    "commit": str(index + 1) * 40,
                                    "license_spdx": "MIT",
                                    "quota": 1,
                                    "path_prefixes": ["fixtures/"],
                                    "static_paths": [],
                                }
                                for index in range(5)
                            ],
                        }
                    },
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        return config

    def _blobs(self, root: Path) -> dict[tuple[str, str], bytes]:
        result: dict[tuple[str, str], bytes] = {}
        first: bytes | None = None
        for index in range(5):
            path = root / f"valid-{index}.docx"
            write_positive_source(path, "docx", f"public-{index}")
            value = path.read_bytes()
            result[(f"owner/repo-{index}", "fixtures/invalid.docx")] = b"invalid"
            valid_name = (
                "fixtures/valid file.docx" if index == 4 else "fixtures/valid.docx"
            )
            result[(f"owner/repo-{index}", valid_name)] = value
            if index == 0:
                first = value
            elif index == 1 and first is not None:
                result[(f"owner/repo-{index}", "fixtures/duplicate.docx")] = first
        return result

    def _tree(self, repository: str, commit: str) -> dict[str, JsonValue]:
        paths = [
            "fixtures/invalid.docx",
            (
                "fixtures/valid file.docx"
                if repository == "owner/repo-4"
                else "fixtures/valid.docx"
            ),
        ]
        if repository == "owner/repo-1":
            paths.insert(1, "fixtures/duplicate.docx")
        return {
            "truncated": False,
            "tree": [
                {
                    "path": path,
                    "type": "blob",
                    "size": 1024,
                }
                for path in paths
            ],
        }
