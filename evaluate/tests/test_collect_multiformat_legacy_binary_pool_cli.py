from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate.collect_multiformat_legacy_binary_pool import main
from evaluate.multiformat_legacy_binary_pool import LegacyBinaryPoolError


class CollectMultiFormatLegacyBinaryPoolCliTests(unittest.TestCase):
    def test_cli_binds_selection_catalog_blind_and_output(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            with mock.patch(
                "evaluate.collect_multiformat_legacy_binary_pool."
                "collect_legacy_binary_pool",
            ) as collect:
                # When
                exit_code = main(self._arguments(root))

            # Then
            self.assertEqual(exit_code, 0)
            arguments = collect.call_args.args
            self.assertEqual(arguments[0], root / "selection.json")
            self.assertEqual(arguments[1], root / "public.json")
            self.assertEqual(arguments[2], root / "blind.json")
            self.assertEqual(arguments[3], root / "output")
            self.assertIn("tree_fetcher", collect.call_args.kwargs)
            self.assertIn("blob_fetcher", collect.call_args.kwargs)

    def test_cli_reports_typed_collection_failure(self) -> None:
        # Given
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            with (
                mock.patch(
                    "evaluate.collect_multiformat_legacy_binary_pool."
                    "collect_legacy_binary_pool",
                    side_effect=LegacyBinaryPoolError("source shortage"),
                ),
                self.assertRaises(SystemExit) as raised,
            ):
                # When
                main(self._arguments(root))

            # Then
            self.assertEqual(raised.exception.code, 2)

    def _arguments(self, root: Path) -> list[str]:
        return [
            "--config",
            str(root / "selection.json"),
            "--public-config",
            str(root / "public.json"),
            "--blind-manifest",
            str(root / "blind.json"),
            "--output-dir",
            str(root / "output"),
        ]


if __name__ == "__main__":
    unittest.main()
