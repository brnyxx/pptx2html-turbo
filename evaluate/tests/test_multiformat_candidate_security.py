from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate.multiformat_candidate_conversion import CandidateConversionError
from evaluate.multiformat_candidate_security import (
    CandidateSecurityError,
    CandidateSecuritySource,
    capture_candidate_security,
)
from evaluate.multiformat_candidate_types import CandidateRuntimePaths
from evaluate.multiformat_corpus_types import DocumentFormat, SecurityOutcome
from evaluate.multiformat_schema import sha256_file


class CandidateSecurityTests(unittest.TestCase):
    def test_exact_ten_execution_derived_rejections_are_published(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus, evaluator, runtime, sources = self._fixture(root)
            with (
                mock.patch(
                    "evaluate.multiformat_candidate_security.load_candidate_security_sources",
                    return_value=(DocumentFormat.DOCX, sources),
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_security.run_conversion",
                    side_effect=CandidateConversionError("converter exit code 1"),
                ),
            ):
                results = capture_candidate_security(
                    contract, corpus, evaluator, root / "security", runtime, "a" * 40
                )
            self.assertEqual(len(results), 10)
            for source, result in zip(sources, results, strict=True):
                value = json.loads(result.read_text(encoding="utf-8"))
                self.assertEqual(value["status"], "PASS")
                self.assertEqual(value["source_id"], source.source_id)
                self.assertEqual(value["observed_outcome"], "reject")
                self.assertEqual(
                    value["typed_error"], "document2html.conversion-rejected"
                )

    def test_timeout_cannot_be_relabelled_as_a_security_rejection(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            contract, corpus, evaluator, runtime, sources = self._fixture(root)
            with (
                mock.patch(
                    "evaluate.multiformat_candidate_security.load_candidate_security_sources",
                    return_value=(DocumentFormat.DOCX, sources),
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_security.run_conversion",
                    side_effect=CandidateConversionError("converter timeout"),
                ),
                self.assertRaisesRegex(CandidateSecurityError, "infrastructure"),
            ):
                capture_candidate_security(
                    contract, corpus, evaluator, root / "security", runtime, "a" * 40
                )

    @staticmethod
    def _fixture(root: Path):
        contract = root / "contract.json"
        corpus = root / "corpus.json"
        evaluator = root / "evaluator.json"
        for path in (contract, corpus, evaluator):
            path.write_text("{}", encoding="utf-8")
        tool = root / "tool"
        tool.write_bytes(b"tool")
        sources = []
        for index in range(10):
            source = root / f"source-{index}.docx"
            source.write_bytes(f"source-{index}".encode())
            sources.append(
                CandidateSecuritySource(
                    f"security-{index}",
                    source,
                    sha256_file(source),
                    f"family-{index}",
                    SecurityOutcome.REJECT,
                )
            )
        runtime = CandidateRuntimePaths(
            tool, tool, tool, tool, tool, tool, tool, "test", 30
        )
        return contract, corpus, evaluator, runtime, tuple(sources)


if __name__ == "__main__":
    unittest.main()
