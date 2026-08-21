from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from evaluate.build_multiformat_public_pool_input import (
    PublicPoolInputError,
    build_public_pool_input,
)
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.multiformat_public_pool_fixture import (
    write_public_pool_fixture,
)


class BuildMultiFormatPublicPoolInputTests(unittest.TestCase):
    def test_builds_exact_office_capture_input(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = write_public_pool_fixture(root)

            manifest = build_public_pool_input(
                fixture.config,
                fixture.manifest,
                root / "input",
            )

            values = json.loads(manifest.read_text(encoding="utf-8"))
            self.assertEqual(len(values["files"]), 5)
            self.assertTrue(all(item["track"] == "blind" for item in values["files"]))
            self.assertEqual(
                values["public_pool_manifest_sha256"],
                sha256_file(manifest.parent / "public-pool.json"),
            )
            self.assertEqual(
                values["public_pool_config_sha256"],
                sha256_file(manifest.parent / "public-pool-config.json"),
            )
            self.assertTrue(
                all(
                    sha256_file(manifest.parent / item["path"]) == item["sha256"]
                    for item in values["files"]
                )
            )

    def test_tampered_pool_is_rejected_without_partial_output(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = write_public_pool_fixture(root)
            pool = json.loads(fixture.manifest.read_text(encoding="utf-8"))
            source = (
                fixture.manifest.parent / pool["formats"]["docx"]["sources"][0]["path"]
            )
            source.write_bytes(b"tampered")
            output = root / "input"

            with self.assertRaises(PublicPoolInputError):
                build_public_pool_input(fixture.config, fixture.manifest, output)

            self.assertFalse(output.exists())
