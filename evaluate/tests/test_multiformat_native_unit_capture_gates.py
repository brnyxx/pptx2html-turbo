from __future__ import annotations

import tempfile
import threading
import unittest
from dataclasses import replace
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_capture_gates import (
    run_two_worker_gate,
    select_two_worker_requests,
)
from evaluate.multiformat_native_unit_gate_validation import (
    validate_convertibility_preflight,
    validate_two_worker_gate,
)
from evaluate.multiformat_native_unit_types import NativeUnitError
from evaluate.multiformat_schema import JsonValue
from evaluate.tests.multiformat_native_unit_fixture import (
    RecordingNativeRunner,
    make_native_unit_fixture,
)


class MultiFormatNativeUnitCaptureGateTests(unittest.TestCase):
    def test_gate_selects_and_coordinates_two_office_run_two_requests(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            first = fixture.request(root / "first", DocumentFormat.DOCX)
            first = replace(first, run=2, nonce="1" * 64)
            second = replace(
                fixture.request(root / "second", DocumentFormat.PPTX),
                source=replace(
                    first.source,
                    source_id="blind-pptx-002",
                    document_format=DocumentFormat.PPTX,
                ),
                run=2,
                nonce="2" * 64,
            )
            pdf = replace(
                fixture.request(root / "pdf", DocumentFormat.PDF),
                run=2,
                nonce="3" * 64,
            )
            run_one = replace(
                first,
                observation_dir=root / "run-one",
                run=1,
                nonce="4" * 64,
            )

            selected = select_two_worker_requests((pdf, second, run_one, first))
            barrier = threading.Barrier(2)
            with patch.object(barrier, "wait", wraps=barrier.wait) as wait:
                gate = run_two_worker_gate(
                    selected,
                    RecordingNativeRunner(),
                    barrier=barrier,
                )

            self.assertEqual(selected, (first, second))
            self.assertEqual(wait.call_count, 2)
            self.assertEqual(
                tuple(
                    (item.source.document_format, item.source.source_id)
                    for item in gate.observations
                ),
                (
                    (DocumentFormat.DOCX, first.source.source_id),
                    (DocumentFormat.PPTX, second.source.source_id),
                ),
            )

    def test_gate_validation_rejects_rebound_execution_hash(self) -> None:
        first = _source_value("docx", "blind-docx-001", "1" * 64)
        second = _source_value("pptx", "blind-pptx-001", "2" * 64)
        gate: dict[str, JsonValue] = {
            "status": "PASSED",
            "worker_count": 2,
            "coordinator": "barrier-before-libreoffice-v1",
            "observations": [
                {
                    "format": "docx",
                    "id": "blind-docx-001",
                    "run": 2,
                    "execution_sha256": "1" * 64,
                },
                {
                    "format": "pptx",
                    "id": "blind-pptx-001",
                    "run": 2,
                    "execution_sha256": "2" * 64,
                },
            ],
        }
        manifest: dict[str, JsonValue] = {"two_worker_gate": gate}

        validate_two_worker_gate(manifest, [first, second])
        observations = object_list(
            gate,
            "observations",
            "native.inventory.two_worker_gate.observations",
        )
        observations[0]["execution_sha256"] = "f" * 64

        with self.assertRaises(NativeUnitError):
            validate_two_worker_gate(manifest, [first, second])

    def test_preflight_validation_rejects_source_count_drift(self) -> None:
        sources = [
            _source_value("docx", "blind-docx-001", "1" * 64),
            _source_value("pptx", "blind-pptx-001", "2" * 64),
        ]
        preflight: dict[str, JsonValue] = {
            "status": "PASSED",
            "worker_count": 1,
            "observation_run": 1,
            "source_count": 2,
        }
        manifest: dict[str, JsonValue] = {"convertibility_preflight": preflight}

        validate_convertibility_preflight(manifest, sources)
        preflight["source_count"] = 3

        with self.assertRaises(NativeUnitError):
            validate_convertibility_preflight(manifest, sources)


def _source_value(
    document_format: str,
    source_id: str,
    execution_sha256: str,
) -> dict[str, JsonValue]:
    return {
        "id": source_id,
        "format": document_format,
        "observations": [
            {
                "run": 1,
                "execution": {"sha256": "0" * 64},
            },
            {
                "run": 2,
                "execution": {"sha256": execution_sha256},
            },
        ],
    }


if __name__ == "__main__":
    _ = unittest.main()
