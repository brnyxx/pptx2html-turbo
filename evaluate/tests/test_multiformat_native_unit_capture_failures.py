from __future__ import annotations

import tempfile
import unittest
from collections.abc import Callable
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_native_unit_capture import (
    NativeUnitCaptureInputs,
    capture_native_unit_inventory,
)
from evaluate.multiformat_native_unit_manifest import (
    NativeManifestInputs,
    build_native_unit_manifest,
)
from evaluate.multiformat_native_unit_types import (
    NativeObservation,
    NativeUnitError,
)
from evaluate.multiformat_native_unit_validation import (
    NativeUnitInventorySummary,
    NativeUnitValidationInputs,
    validate_native_unit_inventory,
)
from evaluate.multiformat_public_pool import load_validated_public_pool_sources
from evaluate.multiformat_schema import JsonValue
from evaluate.multiformat_snapshot_publish import publish_snapshot
from evaluate.tests.multiformat_native_unit_fixture import (
    NativeInventoryFixture,
    RecordingNativeRunner,
    make_native_inventory_fixture,
)


class CaptureCallbackError(RuntimeError):
    pass


class MultiFormatNativeUnitCaptureFailureTests(unittest.TestCase):
    def test_fixture_trusted_files_are_isolated(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_inventory_fixture(root)

            self.assertTrue(fixture.contract.is_relative_to(root))
            self.assertTrue(fixture.routing.is_relative_to(root))

    def test_contract_drift_before_manifest_is_rejected_typed(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_inventory_fixture(root)
            contract_before = fixture.contract.read_bytes()

            def mutate_contract(
                inputs: NativeManifestInputs,
                observations: tuple[NativeObservation, ...],
            ) -> dict[str, JsonValue]:
                _ = fixture.contract.write_bytes(contract_before + b"\n")
                return build_native_unit_manifest(inputs, observations)

            try:
                with (
                    patch(
                        "evaluate.multiformat_native_unit_capture.build_native_unit_manifest",
                        side_effect=mutate_contract,
                    ),
                    patch(
                        "evaluate.multiformat_native_unit_validation._validate_pdf_count"
                    ),
                    self.assertRaises(NativeUnitError),
                ):
                    _ = self._capture(fixture)
            finally:
                _ = fixture.contract.write_bytes(contract_before)

            self._assert_no_publication(root, fixture.output)

    def test_source_drift_after_staged_validation_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_inventory_fixture(root)
            source = load_validated_public_pool_sources(
                fixture.public_config,
                fixture.public_pool_manifest,
            )[0]
            source_path = fixture.public_pool_manifest.parent / source.relative_path
            source_before = source_path.read_bytes()

            def validate_then_mutate(
                inputs: NativeUnitValidationInputs,
            ) -> NativeUnitInventorySummary:
                summary = validate_native_unit_inventory(inputs)
                _ = source_path.write_bytes(b"attacker-source")
                return summary

            try:
                with (
                    patch(
                        "evaluate.multiformat_native_unit_capture.validate_native_unit_inventory",
                        side_effect=validate_then_mutate,
                    ),
                    patch(
                        "evaluate.multiformat_native_unit_validation._validate_pdf_count"
                    ),
                    self.assertRaises(NativeUnitError),
                ):
                    _ = self._capture(fixture)
            finally:
                _ = source_path.write_bytes(source_before)

            self._assert_no_publication(root, fixture.output)

    def test_trusted_drift_at_publication_boundary_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_inventory_fixture(root)
            contract_before = fixture.contract.read_bytes()
            mutated = False

            def publish_with_drift(
                destination: Path,
                writer: Callable[[Path], None],
                *,
                lock_namespace: str,
                before_publish: Callable[[], None],
            ) -> None:
                def mutate_then_check() -> None:
                    nonlocal mutated
                    mutated = True
                    _ = fixture.contract.write_bytes(contract_before + b"\n")
                    before_publish()

                publish_snapshot(
                    destination,
                    writer,
                    lock_namespace=lock_namespace,
                    before_publish=mutate_then_check,
                )

            try:
                with (
                    patch(
                        "evaluate.multiformat_native_unit_capture.publish_snapshot",
                        side_effect=publish_with_drift,
                    ),
                    patch(
                        "evaluate.multiformat_native_unit_validation._validate_pdf_count"
                    ),
                    self.assertRaises(NativeUnitError),
                ):
                    _ = self._capture(fixture)
            finally:
                _ = fixture.contract.write_bytes(contract_before)

            self.assertTrue(mutated)
            self._assert_no_publication(root, fixture.output)

    def test_nonce_factory_runtime_error_is_typed(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_inventory_fixture(root)

            def fail_nonce() -> str:
                raise CaptureCallbackError("nonce failure")

            with self.assertRaises(NativeUnitError):
                _ = capture_native_unit_inventory(
                    self._inputs(fixture),
                    runner=RecordingNativeRunner(),
                    nonce_factory=fail_nonce,
                )

            self._assert_no_publication(root, fixture.output)

    def test_writer_runtime_error_is_typed_without_residue(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_inventory_fixture(root)

            with (
                patch(
                    "evaluate.multiformat_native_unit_capture._capture_all",
                    side_effect=CaptureCallbackError("writer failure"),
                ),
                self.assertRaises(NativeUnitError),
            ):
                _ = self._capture(fixture)

            self._assert_no_publication(root, fixture.output)

    def _capture(
        self,
        fixture: NativeInventoryFixture,
    ) -> NativeUnitInventorySummary:
        return capture_native_unit_inventory(
            self._inputs(fixture),
            runner=RecordingNativeRunner(),
            nonce_factory=self._nonce_factory(),
        )

    @staticmethod
    def _inputs(fixture: NativeInventoryFixture) -> NativeUnitCaptureInputs:
        return NativeUnitCaptureInputs(
            fixture.contract,
            fixture.public_config,
            fixture.public_pool_manifest,
            fixture.routing,
            fixture.font_manifest,
            fixture.soffice,
            fixture.pdfinfo,
            fixture.output,
            8,
        )

    @staticmethod
    def _nonce_factory() -> Callable[[], str]:
        index = 0

        def next_nonce() -> str:
            nonlocal index
            index += 1
            return f"{index:064x}"

        return next_nonce

    def _assert_no_publication(self, root: Path, output: Path) -> None:
        self.assertFalse(output.exists())
        self.assertEqual(tuple(root.glob(".native-units.stage-*")), ())
        self.assertFalse((root / ".native-units.snapshot.lock").exists())


if __name__ == "__main__":
    _ = unittest.main()
