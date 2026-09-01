from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_font_snapshot import FontSnapshotSummary
from evaluate.multiformat_native_unit_capture import (
    NativeUnitCaptureInputs,
    capture_native_unit_inventory,
)
from evaluate.multiformat_native_unit_tool_validation import LockedTool
from evaluate.multiformat_native_unit_stable_validation import StableFile
from evaluate.multiformat_native_unit_tree_validation import (
    validate_inventory_tree,
)
from evaluate.multiformat_native_unit_types import NativeUnitError
from evaluate.multiformat_native_unit_validation import (
    NativeUnitValidationInputs,
    validate_native_unit_inventory,
)
from evaluate.multiformat_schema import JsonValue
from evaluate.tests.multiformat_native_unit_fixture import (
    NativeInventoryFixture,
    RecordingNativeRunner,
    make_native_inventory_fixture,
)


class MultiFormatNativeUnitValidationFailureTests(unittest.TestCase):
    def test_same_byte_inode_replacement_before_final_tree_is_rejected(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = self._capture(root)
            evidence = (
                fixture.output / "observations/doc/blind-doc-001/run-1/execution.json"
            )
            replacement = root / "replacement.json"
            _ = replacement.write_bytes(evidence.read_bytes())
            original_validate_tree = validate_inventory_tree

            def replace_then_validate(
                root_descriptor: int,
                expected: tuple[tuple[str, StableFile], ...],
            ) -> None:
                os.replace(replacement, evidence)
                original_validate_tree(root_descriptor, expected)

            with (
                patch(
                    "evaluate.multiformat_native_unit_validation.validate_inventory_tree",
                    side_effect=replace_then_validate,
                ),
                patch(
                    "evaluate.multiformat_native_unit_validation._validate_pdf_count"
                ),
                self.assertRaises(NativeUnitError),
            ):
                _ = validate_native_unit_inventory(self._inputs(fixture))

    def test_restored_intermediate_symlink_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = self._capture(root)
            observations = fixture.output / "observations"
            owned_observations = root / "owned-observations"
            _ = observations.rename(owned_observations)
            observations.symlink_to(owned_observations, target_is_directory=True)
            restored = False

            def restore_tree(
                _root_descriptor: int,
                _expected: tuple[tuple[str, StableFile], ...],
            ) -> None:
                nonlocal restored
                observations.unlink()
                _ = owned_observations.rename(observations)
                restored = True

            try:
                with (
                    patch(
                        "evaluate.multiformat_native_unit_validation.validate_inventory_tree",
                        side_effect=restore_tree,
                    ),
                    patch(
                        "evaluate.multiformat_native_unit_validation._validate_pdf_count"
                    ),
                    self.assertRaises(NativeUnitError),
                ):
                    _ = validate_native_unit_inventory(self._inputs(fixture))
            finally:
                if not restored:
                    observations.unlink()
                    _ = owned_observations.rename(observations)

    def test_inventory_root_replacement_after_lstat_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = self._capture(root)
            backup = root / "owned-native-units"
            replaced = False

            def replace_root(
                _inputs: NativeUnitValidationInputs,
                _values: dict[str, JsonValue],
                _routing_sha256: str,
                _font: FontSnapshotSummary,
            ) -> None:
                nonlocal replaced
                if not replaced:
                    replaced = True
                    _ = fixture.output.rename(backup)
                    fixture.output.symlink_to(backup, target_is_directory=True)

            with (
                patch(
                    "evaluate.multiformat_native_unit_validation._validate_root",
                    side_effect=replace_root,
                ),
                patch(
                    "evaluate.multiformat_native_unit_validation._validate_pdf_count"
                ),
                self.assertRaises(NativeUnitError) as raised,
            ):
                _ = validate_native_unit_inventory(self._inputs(fixture))

            self.assertTrue(replaced, repr(raised.exception))

    def test_retained_pdf_replacement_after_pdfinfo_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = self._capture(root)
            retained_pdf = (
                fixture.output / "observations/doc/blind-doc-001/run-1/reference.pdf"
            )
            replaced = False

            def replace_pdf(
                _pdfinfo: Path,
                _pdfinfo_tool: LockedTool,
                _reference_pdf: Path,
                _count: int,
            ) -> None:
                nonlocal replaced
                if replaced:
                    return
                replaced = True
                content = retained_pdf.read_bytes()
                _ = retained_pdf.rename(root / "owned-reference.pdf")
                _ = retained_pdf.write_bytes(content)

            with (
                patch(
                    "evaluate.multiformat_native_unit_validation._validate_pdf_count",
                    side_effect=replace_pdf,
                ),
                self.assertRaises(NativeUnitError),
            ):
                _ = validate_native_unit_inventory(self._inputs(fixture))

            self.assertTrue(replaced)

    @staticmethod
    def _capture(root: Path) -> NativeInventoryFixture:
        fixture = make_native_inventory_fixture(root)
        nonce_index = 0

        def nonce_factory() -> str:
            nonlocal nonce_index
            nonce_index += 1
            return f"{nonce_index:064x}"

        with patch("evaluate.multiformat_native_unit_validation._validate_pdf_count"):
            _ = capture_native_unit_inventory(
                NativeUnitCaptureInputs(
                    fixture.contract,
                    fixture.public_config,
                    fixture.public_pool_manifest,
                    fixture.routing,
                    fixture.font_manifest,
                    fixture.soffice,
                    fixture.pdfinfo,
                    fixture.output,
                    8,
                ),
                runner=RecordingNativeRunner(),
                nonce_factory=nonce_factory,
            )
        return fixture

    @staticmethod
    def _inputs(fixture: NativeInventoryFixture) -> NativeUnitValidationInputs:
        return NativeUnitValidationInputs(
            fixture.contract,
            fixture.public_config,
            fixture.public_pool_manifest,
            fixture.routing,
            fixture.font_manifest,
            fixture.soffice,
            fixture.pdfinfo,
            fixture.output,
        )


if __name__ == "__main__":
    _ = unittest.main()
