from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_legacy_binary_pool import (
    LegacyBinaryPoolError,
    collect_legacy_binary_pool,
    load_legacy_binary_plans,
    validate_legacy_binary_pool,
)
from evaluate.multiformat_public_pool import collect_public_pool
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.multiformat_source_fixture import write_positive_source

PROJECT_ROOT = Path(__file__).resolve().parents[2]
PUBLIC_CONFIG = (
    PROJECT_ROOT / "evaluate" / "multiformat" / "public-pool-sources.v1.json"
)
LEGACY_CONFIG = (
    PROJECT_ROOT / "evaluate" / "multiformat" / "legacy-binary-sources.v1.json"
)


class CollectMultiFormatLegacyBinaryPoolTests(unittest.TestCase):
    def test_catalog_selects_exact_forty_per_legacy_format(self) -> None:
        plans = load_legacy_binary_plans(LEGACY_CONFIG, PUBLIC_CONFIG)

        self.assertEqual(
            {plan.document_format.value: plan.expected_count for plan in plans},
            {"doc": 40, "xls": 40, "ppt": 40},
        )
        self.assertTrue(
            all(sum(group.quota for group in plan.groups) == 40 for plan in plans)
        )
        self.assertGreaterEqual(min(len(plan.groups) for plan in plans), 3)

    def test_ppt_catalog_matches_verified_unique_source_allocation(self) -> None:
        plans = load_legacy_binary_plans(LEGACY_CONFIG, PUBLIC_CONFIG)
        ppt = next(plan for plan in plans if plan.document_format.value == "ppt")

        self.assertEqual(
            {group.producer: group.quota for group in ppt.groups},
            {
                "apache-poi": 26,
                "aspose-slides": 2,
                "npoi": 12,
            },
        )

    def test_collects_nonblind_sources_with_independent_provenance(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            public_config = self._public_config(root)
            selection_config = self._selection_config(root, public_config)
            blobs = self._blobs(root)
            blind = collect_public_pool(
                public_config,
                root / "blind",
                tree_fetcher=self._tree,
                blob_fetcher=lambda repository, commit, path: blobs[(repository, path)],
            )

            # When
            manifest = collect_legacy_binary_pool(
                selection_config,
                public_config,
                blind,
                root / "binary",
                tree_fetcher=self._tree,
                blob_fetcher=lambda repository, commit, path: blobs[(repository, path)],
            )

            # Then
            validation = validate_legacy_binary_pool(
                selection_config,
                public_config,
                blind,
                manifest,
            )
            self.assertEqual(validation, {"doc": 2})
            values = json.loads(manifest.read_text(encoding="utf-8"))
            sources = values["formats"]["doc"]["sources"]
            blind_values = json.loads(blind.read_text(encoding="utf-8"))
            blind_sources = blind_values["formats"]["doc"]["sources"]
            self.assertEqual(len(sources), 2)
            self.assertTrue(
                all(item["independently_authored"] is True for item in sources)
            )
            self.assertTrue(
                all(item["id"].startswith("binary-doc-") for item in sources)
            )
            self.assertTrue(
                {item["sha256"] for item in sources}.isdisjoint(
                    {item["sha256"] for item in blind_sources}
                )
            )
            self.assertTrue(
                {item["source_uri"] for item in sources}.isdisjoint(
                    {item["source_uri"] for item in blind_sources}
                )
            )
            self.assertTrue(
                all(
                    sha256_file(manifest.parent / item["path"]) == item["sha256"]
                    for item in sources
                )
            )
            self.assertEqual(
                len([path for path in manifest.parent.rglob("*") if path.is_file()]),
                3,
            )

    def test_blind_digest_substitution_fails_without_publication(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            public_config = self._public_config(root)
            selection_config = self._selection_config(root, public_config)
            blobs = self._blobs(root)
            blind = collect_public_pool(
                public_config,
                root / "blind",
                tree_fetcher=self._tree,
                blob_fetcher=lambda repository, commit, path: blobs[(repository, path)],
            )
            values = json.loads(blind.read_text(encoding="utf-8"))
            values["formats"]["doc"]["sources"][0]["sha256"] = "0" * 64
            blind.write_text(json.dumps(values, sort_keys=True), encoding="utf-8")

            # When / Then
            with self.assertRaises(LegacyBinaryPoolError):
                collect_legacy_binary_pool(
                    selection_config,
                    public_config,
                    blind,
                    root / "binary",
                    tree_fetcher=self._tree,
                    blob_fetcher=lambda repository, commit, path: blobs[
                        (repository, path)
                    ],
                )

            self.assertFalse((root / "binary").exists())

    def test_duplicate_blind_bytes_are_skipped_then_shortage_fails_atomically(
        self,
    ) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            public_config = self._public_config(root)
            selection_config = self._selection_config(root, public_config)
            blobs = self._blobs(root)
            for index in range(2):
                blobs[(f"owner/repo-{index}", "fixtures/01-extra.doc")] = blobs[
                    (f"owner/repo-{index}", "fixtures/00-blind.doc")
                ]
            blind = collect_public_pool(
                public_config,
                root / "blind",
                tree_fetcher=self._tree,
                blob_fetcher=lambda repository, commit, path: blobs[(repository, path)],
            )

            # When / Then
            with self.assertRaisesRegex(LegacyBinaryPoolError, "source shortage"):
                collect_legacy_binary_pool(
                    selection_config,
                    public_config,
                    blind,
                    root / "binary",
                    tree_fetcher=self._tree,
                    blob_fetcher=lambda repository, commit, path: blobs[
                        (repository, path)
                    ],
                )

            self.assertFalse((root / "binary").exists())

    def _public_config(self, root: Path) -> Path:
        config = root / "public.json"
        config.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "formats": {
                        "doc": {
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

    def _selection_config(self, root: Path, public_config: Path) -> Path:
        config = root / "legacy.json"
        config.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "source_catalog_sha256": sha256_file(public_config),
                    "formats": {
                        "doc": {
                            "expected_count": 2,
                            "groups": [
                                {"producer": "producer-0", "quota": 1},
                                {"producer": "producer-1", "quota": 1},
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
        result = {}
        for index in range(5):
            for ordinal, label in ((0, "blind"), (1, "extra")):
                source = root / f"{index}-{label}.doc"
                write_positive_source(source, "doc", f"{label}-{index}")
                result[
                    (f"owner/repo-{index}", f"fixtures/{ordinal:02d}-{label}.doc")
                ] = source.read_bytes()
        return result

    def _tree(self, repository: str, commit: str) -> dict[str, JsonValue]:
        return {
            "truncated": False,
            "tree": [
                {
                    "path": f"fixtures/{ordinal:02d}-{label}.doc",
                    "type": "blob",
                    "size": 1024,
                }
                for ordinal, label in ((0, "blind"), (1, "extra"))
            ],
        }


if __name__ == "__main__":
    unittest.main()
