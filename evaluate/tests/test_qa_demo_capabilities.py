from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import types
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
    return subprocess.CompletedProcess(
        args=["git"], returncode=0, stdout=stdout, stderr=""
    )


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
            "headingVisible": True,
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
            "recordStartVisible": True,
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

    def bounding_box(self) -> dict[str, int] | None:
        default = (
            {"x": 0, "y": 0, "width": 375, "height": 56}
            if self._selector == ".topbar"
            else {"x": 24, "y": 96, "width": 120, "height": 32}
        )
        return self._page.bounding_boxes.get(self._selector, default)


class FakePage:
    def __init__(
        self,
        tmpdir: Path,
        redirect_status: int | None = None,
        raise_on_goto: bool = False,
        bounding_boxes: dict[str, dict[str, int] | None] | None = None,
    ) -> None:
        self.url = "http://127.0.0.1:4173/"
        self.scrolled: list[str] = []
        self.locator_selectors: list[str] = []
        self.screenshots: list[dict[str, object]] = []
        self.alignment_targets: list[str] = []
        self._tmpdir = tmpdir
        self._listeners: dict[str, object] = {}
        self._evaluations = [
            {"scrollWidth": 375, "clientWidth": 375},
            {"scrollWidth": 375, "clientWidth": 375},
            valid_viewport(375)["catalog"],
        ]
        self._redirect_status = redirect_status
        self._raise_on_goto = raise_on_goto
        self.bounding_boxes = bounding_boxes or {}

    def on(self, event: str, callback: object) -> None:
        self._listeners[event] = callback

    def goto(self, url: str, wait_until: str) -> FakeResponse:
        if self._raise_on_goto:
            raise RuntimeError("browser navigation failed")
        self.url = url
        if self._redirect_status is not None and "capabilities" in url:
            self._listeners["response"](FakeResponse(self._redirect_status, url))
        return FakeResponse(200, url)

    def locator(self, selector: str) -> FakeLocator:
        self.locator_selectors.append(selector)
        return FakeLocator(self, selector)

    def screenshot(self, **kwargs: object) -> None:
        path = Path(str(kwargs["path"]))
        path.write_bytes(b"png")
        snapshot = dict(kwargs)
        snapshot["scrolled"] = list(self.scrolled)
        self.screenshots.append(snapshot)

    def wait_for_load_state(self, state: str) -> None:
        return None

    def evaluate(self, script: str, arg: object = None) -> object:
        if "window.scrollBy" in script:
            if isinstance(arg, str):
                self.alignment_targets.append(arg)
            return True
        return self._evaluations.pop(0)

    def close(self) -> None:
        return None


class FakeBrowser:
    def __init__(
        self,
        tmpdir: Path,
        redirect_status: int | None = None,
        raise_on_goto: bool = False,
        bounding_boxes: dict[str, dict[str, int] | None] | None = None,
    ) -> None:
        self._tmpdir = tmpdir
        self._redirect_status = redirect_status
        self._raise_on_goto = raise_on_goto
        self._bounding_boxes = bounding_boxes
        self.new_page_options: list[dict[str, object]] = []
        self.page = FakePage(
            tmpdir,
            redirect_status=redirect_status,
            raise_on_goto=raise_on_goto,
            bounding_boxes=bounding_boxes,
        )

    def new_page(self, **kwargs: object) -> FakePage:
        self.new_page_options.append(kwargs)
        self.page = FakePage(
            self._tmpdir,
            redirect_status=self._redirect_status,
            raise_on_goto=self._raise_on_goto,
            bounding_boxes=self._bounding_boxes,
        )
        return self.page

    def close(self) -> None:
        return None


class FakeChromium:
    def __init__(self, tmpdir: Path) -> None:
        self._tmpdir = tmpdir

    def launch(self, channel: str, headless: bool) -> FakeBrowser:
        return FakeBrowser(self._tmpdir)


class FakePlaywright:
    def __init__(self, tmpdir: Path) -> None:
        self.chromium = FakeChromium(tmpdir)


class FakePlaywrightContext:
    def __init__(self, tmpdir: Path) -> None:
        self._tmpdir = tmpdir

    def __enter__(self) -> FakePlaywright:
        return FakePlaywright(self._tmpdir)

    def __exit__(self, exc_type: object, exc_value: object, traceback: object) -> None:
        return None


def manifest_fixture() -> dict[str, object]:
    disposition = {
        "semantic": {"tier": "approximate"},
        "visual": {"tier": "fallback"},
        "behavioral": {"tier": "fallback"},
    }
    return {
        "features": [
            {
                "id": "presentation",
                "family": "presentationml-package",
                "source_status": "unavailable",
                "current": disposition,
                "target": disposition,
            }
        ]
    }


def write_valid_inputs(root: Path) -> tuple[Path, Path]:
    manifest = root / "manifest.json"
    catalog = root / "catalog.html"
    manifest.write_text(json.dumps(manifest_fixture()), encoding="utf-8")
    catalog.write_text("<main></main>", encoding="utf-8")
    return manifest, catalog


def evidence_bytes(evidence_dir: Path) -> dict[str, bytes]:
    return {
        path.name: path.read_bytes()
        for path in sorted(evidence_dir.iterdir())
        if path.is_file()
    }


def write_evidence_sentinel(evidence_dir: Path, module) -> dict[str, bytes]:
    evidence_dir.mkdir()
    for name in module.OWNED_EVIDENCE_NAMES:
        (evidence_dir / name).write_bytes(f"existing:{name}".encode())
    (evidence_dir / "unrelated.txt").write_bytes(b"keep")
    return evidence_bytes(evidence_dir)


class QaDemoCapabilitiesTests(unittest.TestCase):
    def test_git_binding_accepts_exact_lowercase_sha_and_matching_clean_head(
        self,
    ) -> None:
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

    def test_git_text_wraps_os_error_with_cause(self) -> None:
        module = load_module()

        with mock.patch.object(
            module.subprocess, "run", side_effect=FileNotFoundError("git")
        ):
            with self.assertRaises(module.QaError) as caught:
                module._git_text(["git", "rev-parse", "HEAD"], ROOT)

        self.assertIsInstance(caught.exception.__cause__, FileNotFoundError)

    def test_cleanup_removes_only_owned_evidence_files(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            evidence_dir = root / ".omo" / "evidence" / "cleanup"
            evidence_dir.mkdir(parents=True)
            for name in module.OWNED_EVIDENCE_NAMES:
                (evidence_dir / name).write_text("owned", encoding="utf-8")
            sentinel = evidence_dir / "sentinel.txt"
            nested = evidence_dir / "nested"
            nested.mkdir()
            sentinel.write_text("keep", encoding="utf-8")
            (nested / "landing-375.png").write_text("keep nested", encoding="utf-8")

            module.cleanup_owned_evidence(evidence_dir, root)

            for name in module.OWNED_EVIDENCE_NAMES:
                self.assertFalse((evidence_dir / name).exists(), name)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")
            self.assertTrue((nested / "landing-375.png").is_file())

    def test_cleanup_rejects_symlinked_evidence_paths_without_deleting_targets(
        self,
    ) -> None:
        module = load_module()

        for nested in (False, True):
            with self.subTest(nested=nested), tempfile.TemporaryDirectory() as tmpdir:
                root = Path(tmpdir)
                outside = root / "outside"
                target = outside / "nested" if nested else outside
                target.mkdir(parents=True)
                owned = target / module.OWNED_EVIDENCE_NAMES[0]
                owned.write_bytes(b"keep")
                evidence_root = root / ".omo" / "evidence"
                evidence_root.mkdir(parents=True)
                linked = evidence_root / "linked"
                linked.symlink_to(outside, target_is_directory=True)
                evidence_dir = linked / "nested" if nested else linked

                with self.assertRaises(module.QaError):
                    module.cleanup_owned_evidence(evidence_dir, root)

                self.assertEqual(owned.read_bytes(), b"keep")

    def test_cleanup_rejects_paths_outside_repository_evidence_root(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            outside = root / "outside"
            outside.mkdir()
            owned = outside / module.OWNED_EVIDENCE_NAMES[0]
            owned.write_bytes(b"keep")

            with self.assertRaises(module.QaError):
                module.cleanup_owned_evidence(outside, root)

            self.assertEqual(owned.read_bytes(), b"keep")

    def test_cleanup_wraps_mkdir_os_error_with_cause(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            cause = PermissionError("mkdir blocked")
            with mock.patch.object(module.Path, "mkdir", side_effect=cause):
                with self.assertRaises(module.QaError) as caught:
                    module.cleanup_owned_evidence(
                        root / ".omo" / "evidence" / "blocked", root
                    )

        self.assertIs(caught.exception.__cause__, cause)

    def test_report_schema_accepts_exact_required_shape(self) -> None:
        module = load_module()

        module.validate_report_schema(valid_report())

    def test_report_schema_rejects_missing_or_unknown_keys_at_each_object_level(
        self,
    ) -> None:
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

        self.assertEqual(
            navigation.resolved_no_query, "http://127.0.0.1:4173/capabilities/"
        )
        self.assertEqual(navigation.capture_url, "http://127.0.0.1:4173/capabilities/")

    def test_catalog_capture_url_reapplies_public_query_after_recording_resolved_link(
        self,
    ) -> None:
        module = load_module()

        navigation = module.catalog_navigation_urls(
            "https://brnyxx.github.io/pptx2html-turbo/?v=01234567",
            "https://brnyxx.github.io/pptx2html-turbo/capabilities/",
        )

        self.assertEqual(
            navigation.resolved_no_query,
            "https://brnyxx.github.io/pptx2html-turbo/capabilities/",
        )
        self.assertEqual(
            navigation.capture_url,
            "https://brnyxx.github.io/pptx2html-turbo/capabilities/?v=01234567",
        )

    def test_catalog_capture_url_rejects_external_or_wrong_path_destinations(
        self,
    ) -> None:
        module = load_module()

        for resolved in (
            "https://example.test/pptx2html-turbo/capabilities/",
            "https://brnyxx.github.io/pptx2html-turbo/not-capabilities/",
            "https://user@brnyxx.github.io/pptx2html-turbo/capabilities/",
            "https://brnyxx.github.io/pptx2html-turbo/capabilities/?stale=true",
            "https://brnyxx.github.io/pptx2html-turbo/capabilities/#top",
        ):
            with self.subTest(resolved=resolved):
                with self.assertRaises(module.QaError):
                    module.catalog_navigation_urls(
                        "https://brnyxx.github.io/pptx2html-turbo/?v=01234567",
                        resolved,
                    )

    def test_new_qa_page_sets_viewport_and_reduced_motion(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            browser = FakeBrowser(Path(tmpdir))

            module._new_qa_page(browser, 375)

        self.assertEqual(
            browser.new_page_options,
            [{"viewport": {"width": 375, "height": 900}, "reduced_motion": "reduce"}],
        )

    def test_full_visibility_rejects_clipped_and_offscreen_boxes(
        self,
    ) -> None:
        module = load_module()

        visible = {"x": 300, "y": 80, "width": 40, "height": 20}
        right = {"x": 340, "y": 80, "width": 40, "height": 20}
        left = {"x": -50, "y": 80, "width": 40, "height": 20}
        behind_header = {"x": 20, "y": 20, "width": 40, "height": 20}
        below = {"x": 20, "y": 901, "width": 40, "height": 20}
        zero = {"x": 20, "y": 80, "width": 0, "height": 20}

        self.assertTrue(module._fully_visible_in_viewport(visible, 375, 900, 56))
        self.assertFalse(module._fully_visible_in_viewport(right, 375, 900, 56))
        self.assertFalse(module._fully_visible_in_viewport(left, 375, 900, 56))
        self.assertFalse(module._fully_visible_in_viewport(behind_header, 375, 900, 56))
        self.assertFalse(module._fully_visible_in_viewport(below, 375, 900, 56))
        self.assertFalse(module._fully_visible_in_viewport(zero, 375, 900, 56))
        self.assertFalse(module._fully_visible_in_viewport(None, 375, 900, 56))

    def test_preflight_failures_leave_existing_evidence_byte_identical(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            manifest, catalog = write_valid_inputs(root)
            bad_json = root / "bad.json"
            bad_json.write_text("{", encoding="utf-8")
            invalid_manifest = root / "invalid.json"
            invalid_manifest.write_text(json.dumps({"features": {}}), encoding="utf-8")
            cases = (
                module.CliArgs(
                    "http://127.0.0.1:4173/",
                    root / "missing.json",
                    catalog,
                    root / "evidence-a",
                    VALID_SHA,
                ),
                module.CliArgs(
                    "http://127.0.0.1:4173/",
                    root,
                    catalog,
                    root / "evidence-b",
                    VALID_SHA,
                ),
                module.CliArgs(
                    "http://127.0.0.1:4173/",
                    bad_json,
                    catalog,
                    root / "evidence-c",
                    VALID_SHA,
                ),
                module.CliArgs(
                    "http://127.0.0.1:4173/",
                    manifest,
                    root / "missing.html",
                    root / "evidence-d",
                    VALID_SHA,
                ),
                module.CliArgs(
                    "http://127.0.0.1:4173/",
                    manifest,
                    root,
                    root / "evidence-e",
                    VALID_SHA,
                ),
                module.CliArgs(
                    "http://127.0.0.1:4173/",
                    invalid_manifest,
                    catalog,
                    root / "evidence-f",
                    VALID_SHA,
                ),
            )
            for args in cases:
                with self.subTest(args=args):
                    before = write_evidence_sentinel(args.evidence_dir, module)
                    with self.assertRaises(module.QaError):
                        module.run_browser_qa(args)
                    self.assertEqual(evidence_bytes(args.evidence_dir), before)

    def test_dirty_or_mismatched_git_preflight_leaves_existing_evidence_byte_identical(
        self,
    ) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            manifest, catalog = write_valid_inputs(root)
            cases = (
                ([completed(f"{OTHER_SHA}\n"), completed("")], "mismatch"),
                ([completed(f"{VALID_SHA}\n"), completed(" M README.md\n")], "dirty"),
            )
            for side_effect, name in cases:
                with self.subTest(name=name):
                    evidence_dir = root / f"evidence-{name}"
                    before = write_evidence_sentinel(evidence_dir, module)
                    args = module.CliArgs(
                        "http://127.0.0.1:4173/",
                        manifest,
                        catalog,
                        evidence_dir,
                        VALID_SHA,
                    )
                    with mock.patch.object(
                        module.subprocess, "run", side_effect=side_effect
                    ):
                        with self.assertRaises(module.QaError):
                            module.run_browser_qa(args)
                    self.assertEqual(evidence_bytes(evidence_dir), before)

    def test_capture_uses_viewport_screenshots_for_scrolled_states(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            browser = FakeBrowser(Path(tmpdir))
            context = module.RuntimeContext(
                "http://127.0.0.1:4173/",
                Path(tmpdir),
                module.ManifestStats(
                    "b" * 64,
                    56,
                    19,
                    168,
                    168,
                    0,
                    {
                        "exact": 0,
                        "approximate": 54,
                        "fallback": 114,
                        "unparsed": 0,
                    },
                    2,
                ),
            )

            module._capture_viewport(browser, context, 375)

        self.assertEqual(
            [shot.get("full_page") for shot in browser.page.screenshots],
            [False, False, False],
        )
        landing_scrolls = browser.page.screenshots[0]["scrolled"]
        records_scrolls = browser.page.screenshots[2]["scrolled"]
        self.assertIn("#coverage", landing_scrolls)
        self.assertIn("#coverageHeading", landing_scrolls)
        self.assertIn("#capabilityCatalogLink", landing_scrolls)
        self.assertNotIn("#capability-presentation", landing_scrolls)
        self.assertIn("#capability-presentation", records_scrolls)
        self.assertEqual(
            browser.page.alignment_targets,
            ["#coverageHeading", "#capability-presentation"],
        )

    def test_capture_uses_coverage_owned_section_note_scope_locator(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            browser = FakeBrowser(Path(tmpdir))
            context = module.RuntimeContext(
                "http://127.0.0.1:4173/",
                Path(tmpdir),
                module.ManifestStats(
                    "b" * 64,
                    56,
                    19,
                    168,
                    168,
                    0,
                    {
                        "exact": 0,
                        "approximate": 54,
                        "fallback": 114,
                        "unparsed": 0,
                    },
                    2,
                ),
            )

            module._capture_viewport(browser, context, 375)

        self.assertIn("#coverage .section-note", browser.page.locator_selectors)
        self.assertNotIn(".section-note", browser.page.locator_selectors)

    def test_capture_requires_link_and_section_note_to_intersect_viewport(self) -> None:
        module = load_module()

        for selector in ("#capabilityCatalogLink", "#coverage .section-note"):
            with self.subTest(selector=selector):
                with tempfile.TemporaryDirectory() as tmpdir:
                    browser = FakeBrowser(
                        Path(tmpdir),
                        bounding_boxes={
                            selector: {"x": 0, "y": 901, "width": 120, "height": 32}
                        },
                    )
                    context = module.RuntimeContext(
                        "http://127.0.0.1:4173/",
                        Path(tmpdir),
                        module.ManifestStats(
                            "b" * 64,
                            56,
                            19,
                            168,
                            168,
                            0,
                            {
                                "exact": 0,
                                "approximate": 54,
                                "fallback": 114,
                                "unparsed": 0,
                            },
                            2,
                        ),
                    )

                    with self.assertRaises(module.QaError):
                        module._capture_viewport(browser, context, 375)

    def test_capture_rejects_heading_hidden_by_sticky_header(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            browser = FakeBrowser(
                Path(tmpdir),
                bounding_boxes={
                    ".topbar": {"x": 0, "y": 0, "width": 375, "height": 56},
                    "#coverageHeading": {
                        "x": 24,
                        "y": 24,
                        "width": 240,
                        "height": 40,
                    },
                    "#capabilityCatalogLink": {
                        "x": 24,
                        "y": 128,
                        "width": 220,
                        "height": 32,
                    },
                    "#coverage .section-note": {
                        "x": 24,
                        "y": 96,
                        "width": 327,
                        "height": 160,
                    },
                },
            )
            context = module.RuntimeContext(
                "http://127.0.0.1:4173/",
                Path(tmpdir),
                module.ManifestStats(
                    "b" * 64,
                    56,
                    19,
                    168,
                    168,
                    0,
                    {
                        "exact": 0,
                        "approximate": 54,
                        "fallback": 114,
                        "unparsed": 0,
                    },
                    2,
                ),
            )

            with self.assertRaises(module.QaError):
                module._capture_viewport(browser, context, 375)

    def test_capture_rejects_catalog_record_hidden_by_sticky_header(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            browser = FakeBrowser(
                Path(tmpdir),
                bounding_boxes={
                    ".topbar": {"x": 0, "y": 0, "width": 375, "height": 161},
                    "#coverageHeading": {
                        "x": 20,
                        "y": 177,
                        "width": 320,
                        "height": 48,
                    },
                    "#capabilityCatalogLink": {
                        "x": 20,
                        "y": 300,
                        "width": 220,
                        "height": 32,
                    },
                    "#coverage .section-note": {
                        "x": 20,
                        "y": 240,
                        "width": 335,
                        "height": 220,
                    },
                    "#capability-presentation": {
                        "x": 16,
                        "y": 120,
                        "width": 343,
                        "height": 780,
                    },
                },
            )
            context = module.RuntimeContext(
                "http://127.0.0.1:4173/",
                Path(tmpdir),
                module.ManifestStats(
                    "b" * 64,
                    56,
                    19,
                    168,
                    168,
                    0,
                    {
                        "exact": 0,
                        "approximate": 54,
                        "fallback": 114,
                        "unparsed": 0,
                    },
                    2,
                ),
            )

            with self.assertRaises(module.QaError):
                module._capture_viewport(browser, context, 375)

    def test_capture_records_redirect_responses_as_non2xx_errors(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            browser = FakeBrowser(Path(tmpdir), redirect_status=302)
            context = module.RuntimeContext(
                "http://127.0.0.1:4173/",
                Path(tmpdir),
                module.ManifestStats(
                    "b" * 64,
                    56,
                    19,
                    168,
                    168,
                    0,
                    {
                        "exact": 0,
                        "approximate": 54,
                        "fallback": 114,
                        "unparsed": 0,
                    },
                    2,
                ),
            )

            with self.assertRaises(module.QaError):
                module._capture_viewport(browser, context, 375)

    def test_browser_runtime_errors_are_wrapped_as_qa_errors(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            browser = FakeBrowser(Path(tmpdir), raise_on_goto=True)
            context = module.RuntimeContext(
                "http://127.0.0.1:4173/",
                Path(tmpdir),
                module.ManifestStats(
                    "b" * 64,
                    56,
                    19,
                    168,
                    168,
                    0,
                    {
                        "exact": 0,
                        "approximate": 54,
                        "fallback": 114,
                        "unparsed": 0,
                    },
                    2,
                ),
            )

            with self.assertRaises(module.QaError) as caught:
                module._capture_viewport(browser, context, 375)

        self.assertIsInstance(caught.exception.__cause__, RuntimeError)

    def test_browser_report_write_os_error_is_wrapped_with_cause(self) -> None:
        module = load_module()

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            evidence_dir = root / ".omo" / "evidence" / "write-error"
            evidence_dir.mkdir(parents=True)
            stats = module.ManifestStats(
                "b" * 64,
                56,
                19,
                168,
                168,
                0,
                {
                    "exact": 0,
                    "approximate": 54,
                    "fallback": 114,
                    "unparsed": 0,
                },
                2,
            )
            source = {
                "gitSha": VALID_SHA,
                "manifestSha256": "b" * 64,
                "catalogHtmlSha256": "c" * 64,
            }
            sync_api = types.ModuleType("playwright.sync_api")
            sync_api.Error = RuntimeError
            sync_api.sync_playwright = lambda: FakePlaywrightContext(evidence_dir)
            cause = PermissionError("write blocked")

            with mock.patch.dict(
                sys.modules,
                {
                    "playwright": types.ModuleType("playwright"),
                    "playwright.sync_api": sync_api,
                },
            ):
                with mock.patch.object(
                    module, "preflight_inputs", return_value=(stats, source)
                ):
                    with mock.patch.object(
                        module.Path, "write_text", side_effect=cause
                    ):
                        with self.assertRaises(module.QaError) as caught:
                            module.run_browser_qa(
                                module.CliArgs(
                                    "http://127.0.0.1:4173/",
                                    root / "manifest.json",
                                    root / "catalog.html",
                                    evidence_dir,
                                    VALID_SHA,
                                ),
                                root,
                            )

        self.assertIs(caught.exception.__cause__, cause)


if __name__ == "__main__":
    unittest.main()
