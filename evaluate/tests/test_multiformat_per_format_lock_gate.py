from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_gate import GateStatus, evaluate_reports
from evaluate.multiformat_gate_types import FormatOracleLock, OracleLockInput
from evaluate.tests.multiformat_gate_fixture import (
    CONTRACT_PATH,
)
from evaluate.tests.multiformat_per_format_gate_fixture import PerFormatGateFixture


class MultiFormatPerFormatLockGateTests(PerFormatGateFixture, unittest.TestCase):
    def test_seven_distinct_corpus_scoped_locks_pass(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, locks = self._fixture(Path(temp_dir))

            with patch(
                "evaluate.multiformat_contract.validate_generated_report",
                return_value=[],
            ):
                summary = evaluate_reports(
                    CONTRACT_PATH,
                    reports,
                    OracleLockInput.per_format(locks),
                    root,
                )

            self.assertEqual(summary.status, GateStatus.PASS)
            self.assertEqual(len({path.read_bytes() for path in locks.values()}), 7)

    def test_lock_directory_resolves_all_required_formats(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, locks = self._fixture(Path(temp_dir))

            with patch(
                "evaluate.multiformat_contract.validate_generated_report",
                return_value=[],
            ):
                summary = evaluate_reports(
                    CONTRACT_PATH,
                    reports,
                    OracleLockInput.lock_directory(next(iter(locks.values())).parent),
                    root,
                )

            self.assertEqual(summary.status, GateStatus.PASS)

    def test_singular_schema_2_lock_cannot_substitute_for_lock_set(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, locks = self._fixture(Path(temp_dir))

            summary = evaluate_reports(CONTRACT_PATH, reports, locks["pdf"], root)

            self.assertEqual(summary.status, GateStatus.FAIL)
            self.assertTrue(
                all(item.reasons == ("oracle_lock",) for item in summary.formats)
            )

    def test_swapped_doc_and_docx_locks_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, locks = self._fixture(Path(temp_dir))
            locks["doc"], locks["docx"] = locks["docx"], locks["doc"]

            summary = evaluate_reports(
                CONTRACT_PATH, reports, OracleLockInput.per_format(locks), root
            )

            self.assertEqual(summary.status, GateStatus.FAIL)
            self.assertTrue(
                all("oracle_lock_scope" in item.reasons for item in summary.formats)
            )

    def test_lock_directory_rejects_extra_json_lock(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, locks = self._fixture(Path(temp_dir))
            lock_dir = next(iter(locks.values())).parent
            (lock_dir / "odt.json").write_bytes(locks["pdf"].read_bytes())

            summary = evaluate_reports(
                CONTRACT_PATH,
                reports,
                OracleLockInput.lock_directory(lock_dir),
                root,
            )

            self.assertEqual(summary.status, GateStatus.FAIL)
            self.assertTrue(
                all(item.reasons == ("oracle_lock_extra",) for item in summary.formats)
            )

    def test_missing_lock_is_incomplete(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, locks = self._fixture(Path(temp_dir))
            del locks["pdf"]

            summary = evaluate_reports(
                CONTRACT_PATH, reports, OracleLockInput.per_format(locks), root
            )

            self.assertEqual(summary.status, GateStatus.INCOMPLETE)

    def test_duplicate_format_lock_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, locks = self._fixture(Path(temp_dir))
            entries = [FormatOracleLock(name, path) for name, path in locks.items()]
            entries.append(FormatOracleLock("pdf", locks["pdf"]))

            summary = evaluate_reports(
                CONTRACT_PATH,
                reports,
                OracleLockInput(format_locks=tuple(entries)),
                root,
            )

            self.assertEqual(summary.status, GateStatus.FAIL)
            self.assertTrue(
                all(
                    item.reasons == ("oracle_lock_duplicate",)
                    for item in summary.formats
                )
            )

    def test_extra_and_shared_lock_substitution_fail(self) -> None:
        for mutation in ("extra", "shared"):
            with (
                self.subTest(mutation=mutation),
                tempfile.TemporaryDirectory() as temp_dir,
            ):
                root, reports, locks = self._fixture(Path(temp_dir))
                if mutation == "extra":
                    locks["odt"] = locks["pdf"]
                else:
                    locks["docx"] = locks["doc"]

                summary = evaluate_reports(
                    CONTRACT_PATH, reports, OracleLockInput.per_format(locks), root
                )

                self.assertEqual(summary.status, GateStatus.FAIL)

    def test_wrong_corpus_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, reports, locks = self._fixture(Path(temp_dir))
            path = locks["pdf"]
            lock = json.loads(path.read_text(encoding="utf-8"))
            lock["scope"]["corpus"] = json.loads(
                locks["pptx"].read_text(encoding="utf-8")
            )["scope"]["corpus"]
            path.write_text(json.dumps(lock, sort_keys=True), encoding="utf-8")

            summary = evaluate_reports(
                CONTRACT_PATH, reports, OracleLockInput.per_format(locks), root
            )

            self.assertEqual(summary.status, GateStatus.FAIL)
            pdf = next(item for item in summary.formats if item.format == "pdf")
            self.assertIn("oracle_lock_corpus", pdf.reasons)

    def test_mixed_schema_or_profile_fails(self) -> None:
        for field, value in (
            ("schema_version", 1),
            ("reference_profile", "microsoft-office"),
        ):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temp_dir:
                root, reports, locks = self._fixture(Path(temp_dir))
                path = locks["xlsx"]
                lock = json.loads(path.read_text(encoding="utf-8"))
                lock[field] = value
                path.write_text(json.dumps(lock, sort_keys=True), encoding="utf-8")

                summary = evaluate_reports(
                    CONTRACT_PATH, reports, OracleLockInput.per_format(locks), root
                )

                self.assertEqual(summary.status, GateStatus.FAIL)

    def _fixture(self, root: Path) -> tuple[Path, Path, dict[str, Path]]:
        reports = root / "reports"
        reports.mkdir()
        legacy = self._write_oracle_lock(root)
        self._write_reports(reports, legacy)
        return root, reports, self._write_per_format_locks(reports)


if __name__ == "__main__":
    unittest.main()
