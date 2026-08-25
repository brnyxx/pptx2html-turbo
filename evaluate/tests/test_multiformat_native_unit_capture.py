from __future__ import annotations

import hashlib
import tempfile
import unittest
from collections.abc import Callable, Iterable, Iterator
from concurrent.futures import Future
from dataclasses import replace
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_candidate_process import CandidateProcessFailure
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_native_unit_capture import (
    NativeUnitCaptureInputs,
    capture_native_unit_inventory,
)
from evaluate.multiformat_native_unit_tool_validation import validate_pdf_count
from evaluate.multiformat_native_unit_types import (
    NativeObservation,
    NativeUnitError,
    NativeUnitFailure,
)
from evaluate.multiformat_schema import string_value
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_native_unit_fixture import (
    NativeInventoryFixture,
    RecordingNativeRunner,
    make_native_inventory_fixture,
)


class MultiFormatNativeUnitCaptureTests(unittest.TestCase):
    def test_captures_exact_sorted_525_source_inventory(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = make_native_inventory_fixture(Path(temp_dir))
            runner = RecordingNativeRunner()
            nonce_index = 0

            def nonce_factory() -> str:
                nonlocal nonce_index
                nonce_index += 1
                return f"{nonce_index:064x}"

            with patch(
                "evaluate.multiformat_native_unit_validation._validate_pdf_count",
                wraps=validate_pdf_count,
            ) as pdf_checks:
                summary = capture_native_unit_inventory(
                    self._inputs(fixture),
                    runner=runner,
                    nonce_factory=nonce_factory,
                )

            values = read_strict_object(fixture.output / "native-unit-inventory.json")
            sources = object_list(values, "sources", "native.inventory.sources")
            ordering = [
                (string_value(source, "format"), string_value(source, "id"))
                for source in sources
            ]
            self.assertEqual(summary.files, 3_151)
            self.assertEqual(summary.sources, 525)
            self.assertEqual(summary.observations, 1_050)
            self.assertEqual(pdf_checks.call_count, 1_050)
            self.assertEqual(len(sources), 525)
            self.assertEqual(nonce_index, 1_050)
            self.assertEqual(ordering, sorted(ordering))

    def test_rejects_duplicate_nonces_before_publication(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_inventory_fixture(root)

            with self.assertRaises(NativeUnitError):
                _ = capture_native_unit_inventory(
                    self._inputs(fixture),
                    runner=RecordingNativeRunner(),
                    nonce_factory=lambda: "0" * 64,
                )

            self._assert_no_publication(root, fixture.output)

    def test_runner_failure_leaves_no_publication_residue(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_inventory_fixture(root)
            runner = RecordingNativeRunner()
            runner.failure = CandidateProcessFailure.PIPES_UNAVAILABLE

            with self.assertRaises(NativeUnitError):
                _ = capture_native_unit_inventory(
                    self._inputs(fixture),
                    runner=runner,
                    nonce_factory=self._nonce_factory(),
                )

            self._assert_no_publication(root, fixture.output)

    def test_unsupported_platform_fails_before_any_input_or_nonce(self) -> None:
        root = Path("/paths-must-not-be-resolved")
        nonce_called = False

        def nonce_factory() -> str:
            nonlocal nonce_called
            nonce_called = True
            return "0" * 64

        with (
            patch(
                "evaluate.multiformat_native_unit_capture.platform.system",
                return_value="Windows",
            ),
            self.assertRaises(NativeUnitError) as raised,
        ):
            _ = capture_native_unit_inventory(
                NativeUnitCaptureInputs(
                    root / "contract",
                    root / "config",
                    root / "manifest",
                    root / "routing",
                    root / "font",
                    root / "soffice",
                    root / "pdfinfo",
                    root / "output",
                    1,
                ),
                runner=RecordingNativeRunner(),
                nonce_factory=nonce_factory,
            )

        self.assertIs(
            raised.exception.failure,
            NativeUnitFailure.UNSUPPORTED_PLATFORM,
        )
        self.assertFalse(nonce_called)

    def test_reversed_future_collection_keeps_tree_byte_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_inventory_fixture(root)
            first = replace(fixture, output=root / "first")
            second = replace(fixture, output=root / "second")

            with patch(
                "evaluate.multiformat_native_unit_validation._validate_pdf_count"
            ):
                _ = capture_native_unit_inventory(
                    self._inputs(first),
                    runner=RecordingNativeRunner(),
                    nonce_factory=self._nonce_factory(),
                )
                with patch(
                    "evaluate.multiformat_native_unit_capture.as_completed",
                    side_effect=_reverse_futures,
                ):
                    _ = capture_native_unit_inventory(
                        self._inputs(second),
                        runner=RecordingNativeRunner(),
                        nonce_factory=self._nonce_factory(),
                    )

            self.assertEqual(
                self._tree_digest(first.output),
                self._tree_digest(second.output),
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

    @staticmethod
    def _tree_digest(root: Path) -> str:
        digest = hashlib.sha256()
        for path in sorted(item for item in root.rglob("*") if item.is_file()):
            digest.update(path.relative_to(root).as_posix().encode())
            digest.update(b"\0")
            digest.update(path.read_bytes())
        return digest.hexdigest()


def _reverse_futures(
    futures: Iterable[Future[NativeObservation]],
) -> Iterator[Future[NativeObservation]]:
    return reversed(tuple(futures))


if __name__ == "__main__":
    _ = unittest.main()
