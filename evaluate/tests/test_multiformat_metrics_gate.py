import json
import tempfile
import unittest
from collections.abc import Callable
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_gate import GateStatus, evaluate_reports
from evaluate.multiformat_schema import JsonValue
from evaluate.tests.multiformat_candidate_receipt_fixture import (
    refresh_candidate_receipt,
)
from evaluate.tests.multiformat_gate_fixture import (
    CONTRACT_PATH,
    MultiFormatGateFixture,
)
from evaluate.tests.multiformat_metric_artifact_fixture import write_checkerboard_png
from evaluate.tests.multiformat_metrics_fixture import (
    reviewer_key,
    sign_decision_value,
)


def _obj(value: JsonValue) -> dict[str, JsonValue]:
    if not isinstance(value, dict):
        raise TypeError("expected an object")
    return value


def _arr(value: JsonValue) -> list[JsonValue]:
    if not isinstance(value, list):
        raise TypeError("expected an array")
    return value


def _objs(value: JsonValue) -> list[dict[str, JsonValue]]:
    return [_obj(item) for item in _arr(value)]


def _txt(value: JsonValue) -> str:
    if not isinstance(value, str):
        raise TypeError("expected a string")
    return value


def _read(path: Path) -> dict[str, JsonValue]:
    return _obj(json.loads(path.read_text(encoding="utf-8")))


class MultiFormatMetricsGateTests(MultiFormatGateFixture, unittest.TestCase):
    def test_report_cannot_hide_a_low_raw_unit_score(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, lock = self._fixture(Path(temp_dir))
            report_path = reports / "docx.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            metrics_path = root / report["metrics_evidence"]["path"]
            metrics = json.loads(metrics_path.read_text(encoding="utf-8"))
            candidate = metrics["conformance"]["units"][0]["artifacts"]["candidate_png"]
            candidate_path = root / candidate["path"]
            write_checkerboard_png(candidate_path, 192, 192)
            candidate["sha256"] = self._sha256(candidate_path)
            capture_binding = metrics["bindings"]["candidate_capture"]
            capture_path = root / capture_binding["path"]
            capture = json.loads(capture_path.read_text(encoding="utf-8"))
            capture_unit = next(
                unit
                for unit in capture["units"]
                if unit["unit_id"] == metrics["conformance"]["units"][0]["unit_id"]
            )
            capture_unit["png"]["sha256"] = candidate["sha256"]
            upstream_binding = capture["upstream_manifest"]
            upstream_path = root / upstream_binding["path"]
            upstream = json.loads(upstream_path.read_text(encoding="utf-8"))
            upstream_unit = next(
                unit
                for unit in upstream["units"]
                if unit["unit_id"] == capture_unit["unit_id"]
            )
            upstream_unit["png"]["sha256"] = candidate["sha256"]
            upstream_path.write_text(
                json.dumps(upstream, sort_keys=True),
                encoding="utf-8",
            )
            upstream_binding["sha256"] = self._sha256(upstream_path)
            capture_path.write_text(
                json.dumps(capture, sort_keys=True),
                encoding="utf-8",
            )
            capture_binding["sha256"] = self._sha256(capture_path)
            source_id = metrics["conformance"]["units"][0]["source_id"]
            run_file = next(
                item
                for item in metrics["determinism"]["runs"][0]["files"]
                if item["source_id"] == source_id
            )
            run_file["png"][0]["sha256"] = candidate["sha256"]
            self._rewrite_metrics(metrics_path, metrics, report_path, report)

            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            self.assertEqual(summary.status, GateStatus.FAIL)
            docx = next(result for result in summary.formats if result.format == "docx")
            self.assertIn("report.aggregate_mismatch", docx.reasons)
            self.assertIn("conformance.minimum_unit_score", docx.reasons)

    def test_determinism_is_recomputed_from_per_file_hashes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, lock = self._fixture(Path(temp_dir))
            report_path = reports / "pdf.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            metrics_path = root / report["metrics_evidence"]["path"]
            metrics = json.loads(metrics_path.read_text(encoding="utf-8"))
            html = metrics["determinism"]["runs"][1]["files"][0]["html"]
            html_path = root / html["path"]
            html_path.write_text("<html>changed</html>", encoding="utf-8")
            html["sha256"] = self._sha256(html_path)
            self._rewrite_metrics(metrics_path, metrics, report_path, report)

            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            pdf = next(result for result in summary.formats if result.format == "pdf")
            self.assertIn("determinism", pdf.reasons)
            self.assertIn("report.aggregate_mismatch", pdf.reasons)

    def test_each_reviewer_must_cover_every_pair(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, lock = self._fixture(Path(temp_dir))
            report_path = reports / "xlsx.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            metrics_path = root / report["metrics_evidence"]["path"]
            metrics = json.loads(metrics_path.read_text(encoding="utf-8"))
            decision_binding = _objs(_obj(metrics["review"])["decisions"])[0]
            self._resign_decision(
                root,
                decision_binding,
                lambda decision: _arr(decision["pairs"]).pop(),
            )
            self._rewrite_metrics(metrics_path, metrics, report_path, report)
            # The short decision is genuinely signed, so only missing coverage
            # can fail it; an unsigned edit would prove nothing here.
            self._assert_decision_signature_valid(root, decision_binding)

            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            xlsx = next(result for result in summary.formats if result.format == "xlsx")
            self.assertIn("review.signature", xlsx.reasons)

    def test_oracle_capture_requires_signed_execution_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, lock = self._fixture(Path(temp_dir))
            report_path = reports / "docx.json"
            report = json.loads(report_path.read_text(encoding="utf-8"))
            metrics_path = root / report["metrics_evidence"]["path"]
            metrics = json.loads(metrics_path.read_text(encoding="utf-8"))
            capture_binding = metrics["bindings"]["oracle_capture"]
            capture_path = root / capture_binding["path"]
            capture = json.loads(capture_path.read_text(encoding="utf-8"))
            upstream_binding = capture["upstream_manifest"]
            upstream_path = root / upstream_binding["path"]
            upstream = json.loads(upstream_path.read_text(encoding="utf-8"))
            del capture["execution_receipt"]
            del upstream["execution_receipt"]
            upstream_path.write_text(
                json.dumps(upstream, sort_keys=True),
                encoding="utf-8",
            )
            upstream_binding["sha256"] = self._sha256(upstream_path)
            capture_path.write_text(
                json.dumps(capture, sort_keys=True),
                encoding="utf-8",
            )
            capture_binding["sha256"] = self._sha256(capture_path)
            self._rewrite_metrics(metrics_path, metrics, report_path, report)

            summary = evaluate_reports(CONTRACT_PATH, reports, lock)

            docx = next(result for result in summary.formats if result.format == "docx")
            self.assertIs(docx.status, GateStatus.FAIL)

    def _fixture(self, root: Path) -> tuple[Path, Path, Path]:
        reports = root / "reports"
        reports.mkdir()
        lock = self._write_oracle_lock(root)
        self._write_reports(reports, lock)
        return root, reports, lock

    def _assert_decision_signature_valid(
        self,
        root: Path,
        decision_binding: dict[str, JsonValue],
    ) -> None:
        """Confirms the edited decision is still correctly signed."""
        path = root / _txt(decision_binding["path"])
        decision = _read(path)
        signature = bytes.fromhex(_txt(decision.pop("signature")))
        key = reviewer_key(_txt(decision["reviewer_id"])).public_key()
        key.verify(signature, canonicalize(decision))
        self.assertEqual(decision_binding["sha256"], self._sha256(path))

    def _resign_decision(
        self,
        root: Path,
        decision_binding: dict[str, JsonValue],
        edit: Callable[[dict[str, JsonValue]], object],
    ) -> None:
        """Edits one signed decision, re-signs it, and refreshes its binding."""
        path = root / _txt(decision_binding["path"])
        decision = _read(path)
        edit(decision)
        path.write_bytes(sign_decision_value(decision))
        decision_binding["sha256"] = self._sha256(path)

    def _rebind_review(
        self,
        root: Path,
        metrics: dict[str, JsonValue],
    ) -> None:
        """Realigns the review packet with the current metrics and re-signs.

        The packet re-states the metrics bindings and every pair's artifact
        digests, so any mutation that moves a capture binding or a candidate PNG
        invalidates the packet and both signatures unless they move with it.
        """
        review = _obj(metrics["review"])
        bindings = _obj(metrics["bindings"])
        packet_binding = _obj(review["packet"])
        packet_path = root / _txt(packet_binding["path"])
        packet = _read(packet_path)
        packet["bindings"] = {
            key: value
            for key, value in bindings.items()
            if key not in {"command_plan", "command_plan_sha256"}
        }
        capture = _read(root / _txt(_obj(bindings["candidate_capture"])["path"]))
        captured = {_txt(unit["unit_id"]): unit for unit in _objs(capture["units"])}
        for pair in _objs(packet["pairs"]):
            unit = captured[_txt(pair["pair_id"])]
            pair["candidate_png_sha256"] = _obj(unit["png"])["sha256"]
            pair["candidate_inventory_sha256"] = _obj(unit["inventory"])["sha256"]
        packet_path.write_bytes(canonicalize(packet))
        packet_hash = self._sha256(packet_path)
        packet_binding["sha256"] = packet_hash
        for decision_binding in _objs(review["decisions"]):
            self._resign_decision(
                root,
                decision_binding,
                lambda decision, digest=packet_hash: decision.__setitem__(
                    "packet_sha256", digest
                ),
            )

    def _rewrite_metrics(
        self,
        metrics_path: Path,
        metrics: dict[str, JsonValue],
        report_path: Path,
        report: dict[str, JsonValue],
    ) -> None:
        refresh_candidate_receipt(metrics_path.parent, metrics)
        self._rebind_review(metrics_path.parent, metrics)
        metrics_path.write_text(
            json.dumps(metrics, sort_keys=True),
            encoding="utf-8",
        )
        _obj(report["metrics_evidence"])["sha256"] = self._sha256(metrics_path)
        report_path.write_text(
            json.dumps(report, sort_keys=True),
            encoding="utf-8",
        )
