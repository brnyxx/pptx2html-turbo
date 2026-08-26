from __future__ import annotations

import tempfile
import threading
import unittest
from collections.abc import Callable
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_native_unit_capture import (
    NativeUnitCaptureInputs,
    capture_native_unit_inventory,
)
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_native_unit_types import (
    NativeObservation,
    NativeProcessRunner,
    NativeUnitError,
    NativeUnitFailure,
    NativeUnitRequest,
)
from evaluate.tests.multiformat_native_unit_fixture import (
    NativeInventoryFixture,
    RecordingNativeRunner,
    make_native_inventory_fixture,
)


class MultiFormatNativeUnitPreflightTests(unittest.TestCase):
    def test_every_run_one_observation_finishes_before_run_two(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = make_native_inventory_fixture(Path(temp_dir))
            runs: list[int] = []
            lock = threading.Lock()

            def record(
                request: NativeUnitRequest,
                runner: NativeProcessRunner,
            ) -> NativeObservation:
                with lock:
                    runs.append(request.run)
                return capture_native_observation(request, runner)

            with (
                patch(
                    "evaluate.multiformat_native_unit_capture_worker.capture_native_observation",
                    side_effect=record,
                ),
                patch(
                    "evaluate.multiformat_native_unit_capture_gates.capture_native_observation",
                    side_effect=record,
                ),
                patch(
                    "evaluate.multiformat_native_unit_validation._validate_pdf_count"
                ),
            ):
                _ = capture_native_unit_inventory(
                    _inputs(fixture),
                    runner=RecordingNativeRunner(),
                    nonce_factory=_nonce_factory(),
                )

            self.assertEqual(runs[:525], [1] * 525)
            self.assertEqual(runs[525:], [2] * 525)

    def test_preflight_collects_all_failures_without_starting_run_two(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = make_native_inventory_fixture(Path(temp_dir))
            runs: list[int] = []
            failed = False

            def fail_one(
                request: NativeUnitRequest,
                _runner: NativeProcessRunner,
            ) -> NativeObservation:
                nonlocal failed
                runs.append(request.run)
                if request.run == 1 and not failed:
                    failed = True
                    raise NativeUnitError(
                        NativeUnitFailure.OUTPUT_MISSING,
                        request.source.document_format,
                        request.source.source_id,
                        "preflight fixture failure",
                    )
                return NativeObservation(
                    request.source,
                    request.run,
                    request.nonce,
                    1,
                    request.observation_dir,
                    request.observation_dir / "execution.json",
                    request.observation_dir / "reference.pdf",
                    request.observation_dir / "pdfinfo.txt",
                    "1" * 64,
                    "2" * 64,
                    "3" * 64,
                )

            with (
                patch(
                    "evaluate.multiformat_native_unit_capture_worker.capture_native_observation",
                    side_effect=fail_one,
                ),
                self.assertRaises(NativeUnitError),
            ):
                _ = capture_native_unit_inventory(
                    _inputs(fixture),
                    runner=RecordingNativeRunner(),
                    nonce_factory=_nonce_factory(),
                )

            self.assertEqual(runs, [1] * 525)
            self.assertFalse(fixture.output.exists())


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


def _nonce_factory() -> Callable[[], str]:
    index = 0

    def next_nonce() -> str:
        nonlocal index
        index += 1
        return f"{index:064x}"

    return next_nonce


if __name__ == "__main__":
    _ = unittest.main()
