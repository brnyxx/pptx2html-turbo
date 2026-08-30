from __future__ import annotations

import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from playwright.sync_api import sync_playwright

from evaluate.multiformat_candidate_conversion import CandidateConversionError
from evaluate.multiformat_candidate_security import (
    CandidateSecurityError,
    CandidateSecurityResult,
    CandidateSecuritySource,
    capture_candidate_security,
    execute_candidate_security_case,
)
from evaluate.multiformat_candidate_security_browser import (
    SecurityBrowserFacts,
    inspect_security_html,
)
from evaluate.multiformat_candidate_types import CandidateRuntimePaths
from evaluate.multiformat_corpus_types import DocumentFormat, SecurityOutcome
from evaluate.multiformat_schema import sha256_file
from evaluate.run_multiformat_security_case import (
    load_exact_security_source,
)
from evaluate.run_multiformat_security_case import (
    main as security_case_main,
)


class CandidateSecurityTests(unittest.TestCase):
    def test_public_single_case_seam_handles_reject_and_safe_convert(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            _contract, _corpus, _evaluator, runtime, sources = self._fixture(root)
            reject = sources[0]
            safe = CandidateSecuritySource(
                sources[1].source_id,
                sources[1].path,
                sources[1].sha256,
                sources[1].case_family,
                SecurityOutcome.SAFE_CONVERT,
            )
            with mock.patch(
                "evaluate.multiformat_candidate_security.run_conversion",
                side_effect=CandidateConversionError("converter exit code 1"),
            ):
                rejected = execute_candidate_security_case(
                    reject, DocumentFormat.DOCX, root / "reject", runtime
                )
            with (
                mock.patch(
                    "evaluate.multiformat_candidate_security.run_conversion",
                    return_value=SimpleNamespace(html="<html></html>"),
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_security.inspect_security_html",
                    return_value=SecurityBrowserFacts((), False),
                ),
            ):
                converted = execute_candidate_security_case(
                    safe, DocumentFormat.DOCX, root / "safe", runtime
                )

            self.assertEqual(rejected.observed_outcome, SecurityOutcome.REJECT)
            self.assertEqual(rejected.typed_error, "document2html.conversion-rejected")
            self.assertEqual(converted.observed_outcome, SecurityOutcome.SAFE_CONVERT)
            self.assertIsNone(converted.typed_error)

    def test_single_case_cli_emits_only_command_evidence_fields(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            locked = root / "locked.json"
            locked.write_text("{}", encoding="utf-8")
            output = root / "output"
            tool = Path(sys.executable).resolve()
            runtime = CandidateRuntimePaths(
                tool, tool, tool, tool, tool, tool, tool, "test", 30
            )
            result = CandidateSecurityResult(
                SecurityOutcome.REJECT,
                "document2html.conversion-rejected",
                (),
                False,
            )
            argv = [
                "--project-root",
                root.as_posix(),
                "--contract",
                locked.as_posix(),
                "--corpus-manifest",
                locked.as_posix(),
                "--evaluator-manifest",
                locked.as_posix(),
                "--oracle-lock",
                locked.as_posix(),
                "--evidence-root",
                root.as_posix(),
                "--output-dir",
                output.as_posix(),
                "--source-id",
                "security-1",
                "--source",
                locked.as_posix(),
            ]
            for name in (
                "converter",
                "soffice",
                "pdftohtml",
                "pdfinfo",
                "chromium",
                "font-bundle",
                "sandbox-attestation",
                "sandbox-public-key",
                "openssl",
                "receipt-signer",
            ):
                argv.extend((f"--{name}", tool.as_posix()))
            with (
                mock.patch(
                    "evaluate.run_multiformat_security_case.preflight_candidate_capture",
                    return_value=SimpleNamespace(
                        runtime_profile=SimpleNamespace(portable=True),
                        sandbox=object(),
                    ),
                ),
                mock.patch(
                    "evaluate.run_multiformat_security_case.require_active_sandbox"
                ),
                mock.patch(
                    "evaluate.run_multiformat_security_case.load_exact_security_source",
                    return_value=(DocumentFormat.DOCX, object()),
                ),
                mock.patch(
                    "evaluate.run_multiformat_security_case.materialize_candidate_runtime",
                    return_value=(runtime, {}),
                ),
                mock.patch(
                    "evaluate.run_multiformat_security_case.execute_candidate_security_case",
                    return_value=result,
                ),
                redirect_stdout(io.StringIO()) as stdout,
            ):
                self.assertEqual(security_case_main(argv), 0)
            self.assertEqual(
                set(json.loads(stdout.getvalue())),
                {
                    "observed_outcome",
                    "typed_error",
                    "network_isolation",
                    "external_fetches",
                    "active_content_executed",
                    "within_limits",
                },
            )

    def test_single_case_loader_rejects_wrong_source_id(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            _contract, _corpus, _evaluator, _runtime, sources = self._fixture(root)
            with (
                mock.patch(
                    "evaluate.run_multiformat_security_case.load_candidate_security_sources",
                    return_value=(DocumentFormat.DOCX, sources),
                ),
                self.assertRaisesRegex(
                    CandidateSecurityError, "unknown security source ID"
                ),
            ):
                load_exact_security_source(
                    root / "contract.json",
                    root / "corpus.json",
                    "wrong-id",
                    sources[0].path,
                )

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

    def test_inline_script_fetch_and_source_mutation_are_detected(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            font_config = root / "fonts.conf"
            font_config.write_text("<fontconfig/>", encoding="utf-8")
            with sync_playwright() as playwright:
                executable = Path(playwright.chromium.executable_path)
                browser = playwright.chromium.launch(headless=True)
                version = browser.version
                browser.close()
            facts = inspect_security_html(
                "<script>fetch('https://blocked.example/data')</script>",
                chromium=executable,
                browser_version=version,
                font_config=font_config,
            )
            self.assertTrue(facts.active_content_executed)
            self.assertTrue(facts.external_requests)
            inert = inspect_security_html(
                '<script type="application/json">{"diagnostics":[]}</script>',
                chromium=executable,
                browser_version=version,
                font_config=font_config,
            )
            self.assertFalse(inert.active_content_executed)
            self.assertEqual(inert.external_requests, ())
            self_removing = inspect_security_html(
                "<script>document.currentScript.remove()</script>",
                chromium=executable,
                browser_version=version,
                font_config=font_config,
            )
            self.assertTrue(self_removing.active_content_executed)
            for script_type in (
                "application/x-javascript",
                "text/javascript1.5",
                "text/jscript",
            ):
                legacy_script = inspect_security_html(
                    f'<script type="{script_type}">document.currentScript.remove()</script>',
                    chromium=executable,
                    browser_version=version,
                    font_config=font_config,
                )
                self.assertTrue(legacy_script.active_content_executed)
            shadow_event = inspect_security_html(
                '<div><template shadowrootmode="closed">'
                '<svg onload="this.remove()"></svg>'
                "</template></div>",
                chromium=executable,
                browser_version=version,
                font_config=font_config,
            )
            self.assertTrue(shadow_event.active_content_executed)
            normalized_javascript_url = inspect_security_html(
                '<a href="java&#10;script:void(0)">run</a>',
                chromium=executable,
                browser_version=version,
                font_config=font_config,
            )
            self.assertTrue(normalized_javascript_url.active_content_executed)
            animated_javascript_url = inspect_security_html(
                '<svg xmlns="http://www.w3.org/2000/svg">'
                '<a href="#"><text>run</text>'
                '<animate attributeName="href" values="javascript:void(0)" '
                'dur="10s" fill="freeze"/>'
                "</a></svg>",
                chromium=executable,
                browser_version=version,
                font_config=font_config,
            )
            self.assertTrue(animated_javascript_url.active_content_executed)
            legacy_svg_animation = inspect_security_html(
                '<svg xmlns="http://www.w3.org/2000/svg">'
                '<animateColor attributeName="fill" values="red;blue" dur="1s"/>'
                "</svg>",
                chromium=executable,
                browser_version=version,
                font_config=font_config,
            )
            self.assertTrue(legacy_svg_animation.active_content_executed)
            inert_custom_attribute = inspect_security_html(
                '<div once="value">safe</div>',
                chromium=executable,
                browser_version=version,
                font_config=font_config,
            )
            self.assertFalse(inert_custom_attribute.active_content_executed)

            contract, corpus, evaluator, runtime, sources = self._fixture(root)

            def mutate(_html: str, **_kwargs):
                sources[0].path.write_bytes(b"mutated")
                return SecurityBrowserFacts((), False)

            with (
                mock.patch(
                    "evaluate.multiformat_candidate_security.load_candidate_security_sources",
                    return_value=(DocumentFormat.DOCX, sources),
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_security.run_conversion",
                    return_value=SimpleNamespace(html="<html></html>"),
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_security.inspect_security_html",
                    side_effect=mutate,
                ),
                self.assertRaisesRegex(CandidateSecurityError, "changed during"),
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
