from __future__ import annotations

import hashlib
import tempfile
import unittest
from collections.abc import Callable
from dataclasses import replace
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    CandidateProcessFailure,
)
from evaluate.multiformat_native_unit_capture import (
    NativeUnitCaptureInputs,
    capture_native_unit_inventory,
)
from evaluate.multiformat_native_unit_types import (
    NativeProcessRequest,
    NativeUnitError,
)
from evaluate.tests.multiformat_native_unit_fixture import (
    NativeInventoryFixture,
    RecordingNativeRunner,
    make_native_inventory_fixture,
)


class MultiFormatNativeUnitCacheInventoryTests(unittest.TestCase):
    def test_retry_reuses_all_cached_observations(self) -> None:
        class VersionOnlyRunner(RecordingNativeRunner):
            def __init__(self) -> None:
                super().__init__()
                self.stress_processes = 0

            def __call__(self, request: NativeProcessRequest) -> int:
                if request.command[-1:] in {("--version",), ("-v",)}:
                    return super().__call__(request)
                self.stress_processes += 1
                if self.stress_processes <= 4:
                    return super().__call__(request)
                raise CandidateProcessError(CandidateProcessFailure.PIPES_UNAVAILABLE)

        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_inventory_fixture(root)
            cache = root / "cache"
            first = replace(fixture, output=root / "first")
            second = replace(fixture, output=root / "second")

            with patch(
                "evaluate.multiformat_native_unit_validation._validate_pdf_count"
            ):
                _ = capture_native_unit_inventory(
                    replace(_inputs(first), cache_dir=cache),
                    runner=RecordingNativeRunner(),
                    nonce_factory=_nonce_factory(),
                )
                retry = VersionOnlyRunner()
                _ = capture_native_unit_inventory(
                    replace(_inputs(second), cache_dir=cache),
                    runner=retry,
                    nonce_factory=_nonce_factory(),
                )

            self.assertEqual(len(retry.requests), 6)
            self.assertEqual(_tree_digest(first.output), _tree_digest(second.output))

    def test_failed_inventory_reuses_completed_observation_cache(self) -> None:
        class FailOneRunner(RecordingNativeRunner):
            def __init__(self) -> None:
                super().__init__()
                self.conversions = 0

            def __call__(self, request: NativeProcessRequest) -> int:
                if "--convert-to" in request.command:
                    self.conversions += 1
                    if self.conversions == 3:
                        raise CandidateProcessError(
                            CandidateProcessFailure.PIPES_UNAVAILABLE
                        )
                return super().__call__(request)

        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_inventory_fixture(root)
            cache = root / "cache"
            failed = replace(fixture, output=root / "failed")
            recovered = replace(fixture, output=root / "recovered")
            failed_inputs = replace(_inputs(failed), workers=1, cache_dir=cache)
            recovered_inputs = replace(_inputs(recovered), workers=1, cache_dir=cache)

            with (
                patch(
                    "evaluate.multiformat_native_unit_validation._validate_pdf_count"
                ),
                self.assertRaises(NativeUnitError),
            ):
                _ = capture_native_unit_inventory(
                    failed_inputs,
                    runner=FailOneRunner(),
                    nonce_factory=_nonce_factory(),
                )

            self.assertFalse(failed.output.exists())
            self.assertGreater(len(tuple((cache / "v1").glob("*/*"))), 0)
            retry = RecordingNativeRunner()
            with patch(
                "evaluate.multiformat_native_unit_validation._validate_pdf_count"
            ):
                _ = capture_native_unit_inventory(
                    recovered_inputs,
                    runner=retry,
                    nonce_factory=_nonce_factory(),
                )

            self.assertEqual(len(retry.requests), 979)
            self.assertTrue(recovered.output.is_dir())


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


def _tree_digest(root: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(item for item in root.rglob("*") if item.is_file()):
        digest.update(path.relative_to(root).as_posix().encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
    return digest.hexdigest()


if __name__ == "__main__":
    _ = unittest.main()
