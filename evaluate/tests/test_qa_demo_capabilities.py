from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from dataclasses import dataclass
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts" / "qa_demo_capabilities.py"
VALID_SHA = "0123456789abcdef0123456789abcdef01234567"
OTHER_SHA = "89abcdef0123456789abcdef0123456789abcdef"
VIEWPORTS = (375, 768, 1280)


def load_module():
    spec = importlib.util.spec_from_file_location("qa_demo_capabilities", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise AssertionError("qa_demo_capabilities module is not loadable")
    module = importlib.util.module_from_spec(spec)
    sys.modules["qa_demo_capabilities"] = module
    spec.loader.exec_module(module)
    return module


def completed(stdout: str) -> subprocess.CompletedProcess[str]:
    return subprocess.CompletedProcess(args=["git"], returncode=0, stdout=stdout, stderr="")


def screenshot(path: str) -> dict[str, str]:
    return {
        "path": path,
        "sha256": "a" * 64,
    }


def valid_viewport(width: int) -> dict[str, object]:
    return {
        "width": width,
        "height": 900,
        "screenshots": {
            "landing": screenshot(f"landing-{width}.png"),
            "catalogTop": screenshot(f"catalog-top-{width}.png"),
            "catalogRecords": screenshot(f"catalog-records-{width}.png"),
        },
        "landing": {
            "status": 200,
            "resolvedCatalogHref": "http://127.0.0.1:4173/capabilities/",
            "linkVisible": True,
            "scopeVisible": True,
            "scrollWidth": width,
            "clientWidth": width,
        },
        "catalog": {
            "status": 200,
            "canonical": "https://brnyxx.github.io/pptx2html-turbo/capabilities/",
            "sourceSha256": "b" * 64,
            "featureCount": 56,
            "familyCount": 19,
            "uniqueFeatureCount": 56,
            "currentDispositionCount": 168,
            "targetDispositionCount": 168,
            "exactCurrentCount": 0,
            "tierCounts": {
                "exact": 0,
                "approximate": 54,
                "fallback": 114,
                "unparsed": 0,
            },
            "unavailableSourceCount": 2,
            "warningsBeforeFirstRecord": True,
            "scrollWidth": width,
            "clientWidth": width,
        },
        "errors": {
            "console": [],
            "page": [],
            "failedRequests": [],
            "non2xxResponses": [],
        },
    }


def valid_report() -> dict[str, object]:
    return {
        "schemaVersion": 1,
        "capturedAt": "2026-09-02T01:02:03.004Z",
        "source": {
            "gitSha": VALID_SHA,
            "manifestSha256": "b" * 64,
            "catalogHtmlSha256": "c" * 64,
        },
        "viewports": [valid_viewport(width) for width in VIEWPORTS],
    }


def with_extra_key(value: dict[str, object], key: str = "extra") -> dict[str, object]:
    copied = json.loads(json.dumps(value))
    copied[key] = True
    return copied


@dataclass(frozen=True, slots=True)
class FakeResponse:
    status: int
    url: str


@dataclass(frozen=True, slots=True)
class FakeRequest:
    method: str
    url: str


class FakeLocator:
    def __init__(self, page: "FakePage", selector: str) -> None:
        self._page = page
        self._selector = selector

    def scroll_into_view_if_needed(self) -> None:
        self._page.scrolled.append(self._selector)

    def click(self) -> None:
        self._page.url = "http://127.0.0.1:4173/capabilities/"

    def is_visible(self) -> bool:
        return True


class FakePage:
    def __init__(
        self,
        tmpdir: Path,
        redirect_status: int | None = None,
        require_warning_contract: bool = False,
    ) -> None:
        self.url = "http://127.0.0.1:4173/"
        self.scrolled: list[str] = []
        self.screenshots: list[dict[str, object]] = []
        self._tmpdir = tmpdir
        self._listeners: dict[str, object] = {}
        self._evaluations = [
            {"scrollWidth": 375, "clientWidth": 375},
            {"scrollWidth": 375, "clientWidth": 375},
            valid_viewport(375)["catalog"],
        ]
        self._redirect_status = redirect_status
        self._require_warning_contract = require_warning_contract

    def on(self, event: str, callback: object) -> None:
        self._listeners[event] = callback

    def goto(self, url: str, wait_until: str) -> FakeResponse:
        self.url = url
        if self._redirect_status is not None and "capabilities" in url:
            self._listeners["response"](FakeResponse(self._redirect_status, url))
        return FakeResponse(200, url)

    def locator(self, selector: str) -> FakeLocator:
        return FakeLocator(self, selector)

    def screenshot(self, **kwargs: object) -> None:
        path = Path(str(kwargs["path"]))
        path.write_bytes(b"png")
        self.screenshots.append(kwargs)

    def wait_for_load_state(self, state: str) -> None:
        return None

    def evaluate(self, script: str) -> object:
        if self._require_warning_contract and ".catalog-note" in script:
            if ".hero-copy" not in script or "exact" not in script or "fallback" not in script or "cross-validation" not in script:
                raise AssertionError("catalog warning check must include both pre-record warnings")
        return self._evaluations.pop(0)

    def close(self) -> None:
        return None


class FakeBrowser:
    def __init__(self, tmpdir: Path, redirect_status: int | None = None) -> None:
        self.page = FakePage(tmpdir, redirect_status=redirect_status)

    def new_page(self, viewport: dict[str, int]) -> FakePage:
        return self.page


class QaDemoCapabilitiesTests(unittest.TestCase):
    def test_git_binding_accepts_exact_lowercase_sha_and_matching_clean_head(self) -> None:
        module = load_module()

        with mock.patch.object(
            module.subprocess,
            "run",
            side_effect=[completed(f"{VALID_SHA}\n"), completed("")],
        ) as run:
            module.assert_clean_git_binding(VALID_SHA, ROOT)

        self.assertEqual(run.call_count, 2)

    def test_git_binding_rejects_invalid_or_mismatched_sha(self) -> None:
        module = load_module()

        for git_sha in (VALID_SHA.upper(), VALID_SHA[:-1]):
            with self.subTest(git_sha=git_sha):
                with self.assertRaises(module.QaError):
                    module.assert_clean_git_binding(git_sha, ROOT)

        with mock.patch.object(
            module.subprocess,
            "run",
            side_effect=[completed(f"{OTHER_SHA}\n"), completed("")],
        ):
            with self.assertRaises(module.QaError):
                module.assert_clean_git_binding(VALID_SHA, ROOT)

    def test_git_binding_rejects_dirty_tracked_worktree(self) -> None:
        module = load_module()

        with mock.patch.object(
            module.subprocess,
            "run",
            side_effect=[completed(f"{VALID_SHA}\n"), completed(" M README.md\n")],
        ):
            with self.assertRaises(module.QaError):
                module.assert_clean_git_binding(VALID_SHA, ROOT)

    def test_cleanup_removes_only_owned_evidence_files(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            evidence_dir = Path(tmpdir)
            for name in module.OWNED_EVIDENCE_NAMES:
                (evidence_dir / name).write_text("owned", encoding="utf-8")
            sentinel = evidence_dir / "sentinel.txt"
            nested = evidence_dir / "nested"
            nested.mkdir()
            sentinel.write_text("keep", encoding="utf-8")
            (nested / "landing-375.png").write_text("keep nested", encoding="utf-8")

            module.cleanup_owned_evidence(evidence_dir)

            for name in module.OWNED_EVIDENCE_NAMES:
                self.assertFalse((evidence_dir / name).exists(), name)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")
            self.assertTrue((nested / "landing-375.png").is_file())

    def test_report_schema_accepts_exact_required_shape(self) -> None:
        module = load_module()

        module.validate_report_schema(valid_report())

    def test_report_schema_rejects_missing_or_unknown_keys_at_each_object_level(self) -> None:
        module = load_module()
        cases = []
        for path in (
            (),
            ("source",),
            ("viewports", 0),
            ("viewports", 0, "screenshots"),
            ("viewports", 0, "screenshots", "landing"),
            ("viewports", 0, "landing"),
            ("viewports", 0, "catalog"),
            ("viewports", 0, "catalog", "tierCounts"),
            ("viewports", 0, "errors"),
        ):
            cases.append((path, "missing"))
            cases.append((path, "unknown"))

        for path, mutation in cases:
            with self.subTest(path=path, mutation=mutation):
                report = valid_report()
                target = report
                for part in path:
                    target = target[part]
                if mutation == "missing":
                    del target[next(iter(target))]
                else:
                    target["extra"] = True
                with self.assertRaises(module.QaError):
                    module.validate_report_schema(report)

    def test_report_schema_rejects_missing_or_unknown_viewport_widths(self) -> None:
        module = load_module()

        report = valid_report()
        report["viewports"] = [valid_viewport(375), valid_viewport(1280)]
        with self.assertRaises(module.QaError):
            module.validate_report_schema(report)

        report = valid_report()
        report["viewports"] = [
            valid_viewport(375),
            valid_viewport(768),
            valid_viewport(1280),
            valid_viewport(1440),
        ]
        with self.assertRaises(module.QaError):
            module.validate_report_schema(report)

    def test_catalog_capture_url_keeps_local_url_without_query_unchanged(self) -> None:
        module = load_module()

        navigation = module.catalog_navigation_urls(
            "http://127.0.0.1:4173/",
            "http://127.0.0.1:4173/capabilities/",
        )

        self.assertEqual(navigation.resolved_no_query, "http://127.0.0.1:4173/capabilities/")
        self.assertEqual(navigation.capture_url, "http://127.0.0.1:4173/capabilities/")

    def test_catalog_capture_url_reapplies_public_query_after_recording_resolved_link(
        self,
    ) -> None:
        module = load_module()

        navigation = module.catalog_navigation_urls(
            "https://brnyxx.github.io/pptx2html-turbo/?v=01234567",
            "https://brnyxx.github.io/pptx2html-turbo/capabilities/?stale=true",
        )

        self.assertEqual(
            navigation.resolved_no_query,
            "https://brnyxx.github.io/pptx2html-turbo/capabilities/",
        )
        self.assertEqual(
            navigation.capture_url,
            "https://brnyxx.github.io/pptx2html-turbo/capabilities/?v=01234567",
        )

    def test_capture_uses_viewport_screenshots_for_scrolled_states(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            browser = FakeBrowser(Path(tmpdir))
            context = module.RuntimeContext(
                "http://127.0.0.1:4173/",
                Path(tmpdir),
                module.ManifestStats("b" * 64, 56, 19, 168, 168, 0, {
                    "exact": 0,
                    "approximate": 54,
                    "fallback": 114,
                    "unparsed": 0,
                }, 2),
            )

            module._capture_viewport(browser, context, 375)

        self.assertEqual([shot.get("full_page") for shot in browser.page.screenshots], [False, False, False])
        self.assertEqual(browser.page.scrolled, ["#coverage", "#capability-presentation"])

    def test_capture_records_redirect_responses_as_non2xx_errors(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            browser = FakeBrowser(Path(tmpdir), redirect_status=302)
            context = module.RuntimeContext(
                "http://127.0.0.1:4173/",
                Path(tmpdir),
                module.ManifestStats("b" * 64, 56, 19, 168, 168, 0, {
                    "exact": 0,
                    "approximate": 54,
                    "fallback": 114,
                    "unparsed": 0,
                }, 2),
            )

            with self.assertRaises(module.QaError):
                module._capture_viewport(browser, context, 375)

    def test_catalog_dom_requires_hero_and_boundary_warnings_before_records(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            page = FakePage(Path(tmpdir), require_warning_contract=True)

            module._catalog_dom(page)


if __name__ == "__main__":
    unittest.main()
