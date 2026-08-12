#!/usr/bin/env python3
"""Generate and verify Tasks 12-15 browser acceptance evidence.

Requires the already-installed Python Playwright package and browser. No network
access or dependency installation is performed.
"""
from __future__ import annotations

import argparse
from collections import Counter
from contextlib import contextmanager
import hashlib
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import shutil
import struct
import subprocess
import threading
from typing import Any, Iterator

from playwright.sync_api import Page, sync_playwright

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_EVIDENCE = ROOT / ".omo" / "evidence" / "task-12-15-browser"
VIEWPORTS = {"desktop": {"width": 960, "height": 720}, "mobile": {"width": 390, "height": 844}}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def run(command: list[str], *, cwd: Path = ROOT) -> None:
    subprocess.run(command, cwd=cwd, check=True)


def prepare(evidence: Path) -> Path:
    require(evidence == DEFAULT_EVIDENCE or ROOT in evidence.resolve().parents, "evidence path must be inside the repository")
    if evidence.exists():
        shutil.rmtree(evidence)
    evidence.mkdir(parents=True)
    work = ROOT / "target" / "task-12-15-browser"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    decks = work / "decks"
    html = work / "html"
    run(["python3", "evaluate/create_completion_decks.py", "--output-dir", str(decks)])
    run(["cargo", "build", "-p", "pptx2html-cli"])
    html.mkdir()
    cli = ROOT / "target" / "debug" / "pptx2html"
    for name in ("patterns", "picture-bullets", "table-styles", "actions"):
        run([str(cli), str(decks / f"{name}.pptx"), "--output", str(html / f"{name}.html")])
    return html


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, _format: str, *_args: object) -> None:
        pass


@contextmanager
def serve(directory: Path) -> Iterator[str]:
    handler = lambda *args, **kwargs: QuietHandler(*args, directory=str(directory), **kwargs)
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}"
    finally:
        server.shutdown()
        thread.join()
        server.server_close()


def load(page: Page, url: str) -> None:
    response = page.goto(url, wait_until="load")
    require(response is not None and response.ok, f"load failed: {url}")
    require(page.url == url, f"unexpected URL after load: {page.url}")


def diagnostics(page: Page) -> list[dict[str, Any]]:
    return page.locator("#pptx2html-diagnostics").evaluate("node => JSON.parse(node.textContent)")


def code_counts(items: list[dict[str, Any]]) -> dict[str, int]:
    return dict(sorted(Counter(item["code"] for item in items).items()))


def png_dimensions(path: Path) -> list[int]:
    data = path.read_bytes()
    require(data[:8] == b"\x89PNG\r\n\x1a\n", f"not a PNG: {path}")
    return list(struct.unpack(">II", data[16:24]))


def capture(page: Page, evidence: Path, qa: dict[str, Any], name: str, *, full_page: bool) -> None:
    path = evidence / f"{name}.png"
    page.screenshot(path=str(path), full_page=full_page)
    payload = path.read_bytes()
    qa["screenshots"][name] = {
        "file": path.name,
        "viewport": page.viewport_size,
        "dimensions": png_dimensions(path),
        "sha256": hashlib.sha256(payload).hexdigest(),
        "full_page": full_page,
    }


def visible_boxes(page: Page, selector: str) -> list[dict[str, float]]:
    return page.locator(selector).evaluate_all(
        """nodes => nodes.map(node => { const r = node.getBoundingClientRect();
        const s = getComputedStyle(node); return {x:r.x,y:r.y,width:r.width,height:r.height,
        visible: s.visibility !== 'hidden' && s.display !== 'none'}; })"""
    )


def task12(browser: Any, base: str, evidence: Path, qa: dict[str, Any]) -> None:
    page = browser.new_page(viewport=VIEWPORTS["desktop"])
    load(page, f"{base}/patterns.html")
    expected = [
        "pct5", "pct10", "pct20", "pct25", "pct30", "pct40", "pct50", "pct60", "pct70", "pct75", "pct80", "pct90",
        "horz", "vert", "ltHorz", "ltVert", "dkHorz", "dkVert", "narHorz", "narVert", "dashHorz", "dashVert", "cross",
        "dnDiag", "upDiag", "ltDnDiag", "ltUpDiag", "dkDnDiag", "dkUpDiag", "wdDnDiag", "wdUpDiag", "dashDnDiag", "dashUpDiag",
        "diagCross", "smCheck", "lgCheck", "smGrid", "lgGrid", "dotGrid", "smConfetti", "lgConfetti", "horzBrick", "diagBrick",
        "solidDmnd", "openDmnd", "dotDmnd", "plaid", "sphere", "weave", "divot", "shingle", "wave", "trellis", "zigZag",
    ]
    pattern_shapes = page.locator(".shape").evaluate_all(
        """(nodes, labels) => nodes.filter(n => labels.includes(n.textContent.trim())).map(n => ({
        label:n.textContent.trim(), repeat:getComputedStyle(n).backgroundRepeat,
        image:getComputedStyle(n).backgroundImage, box:n.getBoundingClientRect().toJSON()}))""", expected
    )
    require(len(pattern_shapes) == 54, f"expected 54 pattern shapes, got {len(pattern_shapes)}")
    require(sorted(item["label"] for item in pattern_shapes) == sorted(expected), "pattern labels differ")
    require(all(item["repeat"] == "repeat" and item["image"].startswith('url("data:image/svg+xml') for item in pattern_shapes), "pattern SVG repeat missing")
    trellis = page.locator("#slide-2").evaluate("n => ({repeat:getComputedStyle(n).backgroundRepeat,image:getComputedStyle(n).backgroundImage})")
    require(trellis["repeat"] == "repeat" and trellis["image"].startswith('url("data:image/svg+xml'), "trellis background missing")
    table_pattern = page.get_by_text("table pattern", exact=True).locator("xpath=ancestor::td").evaluate("n => ({repeat:getComputedStyle(n).backgroundRepeat,image:getComputedStyle(n).backgroundImage})")
    require(table_pattern["repeat"] == "repeat" and table_pattern["image"].startswith('url("data:image/svg+xml'), "table pattern missing")
    items = diagnostics(page)
    unsupported = [item for item in items if item["code"] == "DRAWINGML_PATTERN_UNSUPPORTED"]
    require(len(unsupported) == 1 and unsupported[0]["location"]["relationship_id"] == "pattern-s0-e0", "pattern diagnostic mismatch")
    qa["tasks"]["12"] = {"assertions": {"pattern_labels": 54, "svg_repeat_shapes": 54, "trellis_background": True, "table_pattern": True, "diagnostic_identity": "pattern-s0-e0"}, "diagnostic_counts": code_counts(items)}
    capture(page, evidence, qa, "task-12-patterns-960x720-full", full_page=True)
    page.close()


def task13(browser: Any, base: str, evidence: Path, qa: dict[str, Any]) -> None:
    expected_points = ["20pt", "5pt", "80pt", "12.5pt"]
    records = []
    for label, viewport in VIEWPORTS.items():
        page = browser.new_page(viewport=viewport)
        load(page, f"{base}/picture-bullets.html")
        images = page.locator("img.picture-bullet").evaluate_all("nodes => nodes.map(n => ({width:n.style.width,height:n.style.height,src:n.src,box:n.getBoundingClientRect().toJSON()}))")
        fallbacks = visible_boxes(page, ".picture-bullet-missing")
        require(len(images) == 4 and len(fallbacks) == 4, f"picture bullet count mismatch at {label}")
        actual_points = [item["width"] for item in images]
        require(actual_points == expected_points, f"picture sizes differ at {label}: {actual_points}")
        require(all(item["box"]["width"] > 0 and item["box"]["height"] > 0 for item in images), f"hidden image at {label}")
        require(all(item["box"]["right"] > 0 and item["box"]["left"] < viewport["width"] and item["box"]["bottom"] > 0 and item["box"]["top"] < viewport["height"] for item in images), f"image outside viewport at {label}")
        require(all(item["visible"] and item["width"] > 0 and item["height"] > 0 for item in fallbacks), f"hidden fallback at {label}")
        require(all(item["x"] + item["width"] > 0 and item["x"] < viewport["width"] and item["y"] + item["height"] > 0 and item["y"] < viewport["height"] for item in fallbacks), f"fallback outside viewport at {label}")
        source_text = page.content().lower()
        require("password" not in source_text and "token=secret" not in source_text and "<script>alert(1)" not in source_text and "image/svg+xml" not in source_text, "secret/external/SVG content leaked")
        items = diagnostics(page)
        require(sum(item["code"] == "PICTURE_BULLET_IMAGE_MISSING" for item in items) == 4, "picture diagnostic count mismatch")
        records.append({"viewport": label, "image_sizes": actual_points, "visible_images": 4, "visible_fallbacks": 4})
        capture(page, evidence, qa, f"task-13-picture-bullets-{viewport['width']}x{viewport['height']}", full_page=False)
        page.close()
    qa["tasks"]["13"] = {"assertions": records, "diagnostic_counts": code_counts(items), "leakage": {"credentials": False, "external_uri": False, "svg_payload": False}}


def task14(browser: Any, base: str, evidence: Path, qa: dict[str, Any]) -> None:
    page = browser.new_page(viewport=VIEWPORTS["desktop"])
    load(page, f"{base}/table-styles.html")
    cells = page.locator("table").first.locator("td")
    require(cells.count() == 18, f"expected 18 logical styled-table cells, got {cells.count()}")
    require(page.locator("td").count() == 19, "fallback table cell missing")
    require(page.get_by_text("merge-continuation", exact=True).count() == 0, "merge continuation leaked")
    merged = page.get_by_text("merged", exact=True).locator("xpath=ancestor::td")
    require(merged.get_attribute("colspan") == "2", "gridSpan not rendered")
    override = page.get_by_text("r1c1", exact=True).locator("xpath=ancestor::td").evaluate("n => getComputedStyle(n).backgroundColor")
    no_fill = page.get_by_text("r1c2", exact=True).locator("xpath=ancestor::td").evaluate("n => getComputedStyle(n).backgroundColor")
    require(override == "rgb(171, 205, 239)", f"direct override missing: {override}")
    require(no_fill == "rgba(0, 0, 0, 0)", f"no-fill missing: {no_fill}")
    require(page.get_by_text("built-in unavailable", exact=True).is_visible(), "table fallback not visible")
    boxes = visible_boxes(page, ".shape")
    require(len(boxes) == 2 and all(box["visible"] and box["width"] > 0 and box["height"] > 0 for box in boxes), "table frame geometry invalid")
    region_colors = cells.evaluate_all("nodes => [...new Set(nodes.map(n => getComputedStyle(n).backgroundColor))]")
    require(len(region_colors) >= 4, f"table regions not visibly distinct: {region_colors}")
    items = diagnostics(page)
    unavailable = [item for item in items if item["code"] == "TABLE_STYLE_DEFINITION_UNAVAILABLE"]
    require(len(unavailable) == 1, "table fallback diagnostic mismatch")
    qa["tasks"]["14"] = {"assertions": {"logical_cells": 18, "grid_span": 2, "merge_continuation_hidden": True, "override_color": override, "no_fill": no_fill, "region_color_count": len(region_colors), "fallback_visible": True}, "diagnostic_counts": code_counts(items), "diagnostic": unavailable[0]}
    capture(page, evidence, qa, "task-14-table-styles-960x720", full_page=False)
    page.close()


def click_hash(page: Page, selector: str, target: str, *, position: dict[str, float] | None = None) -> None:
    with page.expect_event("framenavigated", predicate=lambda frame: frame == page.main_frame and frame.url.endswith(target)):
        page.locator(selector).click(position=position)
    require(page.url.endswith(target), f"hash target mismatch: expected {target}, got {page.url}")


def task15(browser: Any, base: str, evidence: Path, qa: dict[str, Any]) -> None:
    page = browser.new_page(viewport=VIEWPORTS["desktop"])
    popups: list[str] = []
    dialogs: list[str] = []
    page.context.on("page", lambda popup: popups.append(popup.url))
    page.on("dialog", lambda dialog: (dialogs.append(dialog.message), dialog.dismiss()))
    url = f"{base}/actions.html"
    load(page, url)
    capture(page, evidence, qa, "task-15-initial", full_page=False)

    scenarios = (
        ("next", '[aria-label="shape 2"]', "#slide-2", None),
        ("previous", '[aria-label="shape 3"]', "#slide-1", None),
        ("first", '[aria-label="shape 4"]', "#slide-1", None),
        ("last", '[aria-label="shape 5"]', "#slide-3", None),
        ("specific", '[aria-label="shape 6"]', "#slide-3", None),
        ("connector", '[aria-label="shape 15"]', "#slide-2", None),
        ("inner-group", '[aria-label="shape 19"]', "#slide-2", {"x": 30, "y": 30}),
        ("leaf", '[aria-label="shape 20"]', "#slide-1", None),
    )
    states = []
    for name, selector, target, position in scenarios:
        load(page, url)
        click_hash(page, selector, target, position=position)
        capture(page, evidence, qa, f"task-15-{name}", full_page=False)
        states.append({"scenario": name, "hash": target})

    load(page, url)
    page.locator('[aria-label="shape 10"]').hover()
    require(page.url == url, "hover-only action navigated")
    page.locator('[aria-label="shape 18"]').hover(position={"x": 410, "y": 155})
    require(page.url == url, "outer group hover navigated")
    for shape_id in (9, 11, 12, 13, 14):
        page.locator(f'[aria-label="shape {shape_id}"]').click()
        require(page.url == url, f"inert shape {shape_id} navigated")
    require(len(popups) == 0 and len(dialogs) == 0, f"unexpected popup/dialog: {popups} {dialogs}")
    require(page.locator('a[href^="javascript:"]').count() == 0, "javascript href rendered")

    https_shape = page.locator('[aria-label="shape 7"]')
    mailto_shape = page.locator('[aria-label="shape 8"]')
    require(https_shape.get_attribute("href") == "https://example.com/", "HTTPS href missing")
    require(mailto_shape.get_attribute("href") == "mailto:fixture@example.com", "mailto href missing")
    for anchor in (https_shape, mailto_shape):
        require(anchor.get_attribute("target") == "_blank" and anchor.get_attribute("rel") == "noopener noreferrer", "safe-link attrs missing")
    run_https = page.get_by_text("RUN_HTTPS", exact=True).locator("xpath=ancestor::a")
    run_unsafe = page.get_by_text("RUN_UNSAFE_VISIBLE", exact=True).locator("xpath=ancestor::*[@class='run']")
    table_run = page.get_by_text("TABLE_RUN_MAILTO", exact=True).locator("xpath=ancestor::a")
    require(run_https.get_attribute("href") == "https://example.com/", "run owner precedence failed")
    require(run_unsafe.get_attribute("href") is None, "unsafe run got href")
    require(table_run.get_attribute("href") == "mailto:fixture@example.com", "table run precedence failed")
    require(page.locator('[aria-label="shape 17"]').get_attribute("href") == "https://example.com/", "table owner missing")
    require(page.locator('[aria-label="shape 18"]').get_attribute("href") == "https://example.com/", "group owner missing")
    require(page.locator('[aria-label="shape 19"]').get_attribute("data-action") == "next", "inner group precedence failed")
    require(page.locator('[aria-label="shape 20"]').get_attribute("data-action") == "previous", "leaf precedence failed")
    boxes = visible_boxes(page, "#slide-1 > .shape")
    require(all(box["width"] > 0 and box["height"] > 0 for box in boxes), "zero-sized top-level action shape")
    overlaps = []
    for left_index, left in enumerate(boxes):
        for right_index, right in enumerate(boxes[left_index + 1 :], left_index + 1):
            if left["x"] < right["x"] + right["width"] and left["x"] + left["width"] > right["x"] and left["y"] < right["y"] + right["height"] and left["y"] + left["height"] > right["y"]:
                overlaps.append([left_index, right_index])
    require(not overlaps, f"overlapping top-level action shapes: {overlaps}")
    items = diagnostics(page)
    counts = code_counts(items)
    expected_action = {"ACTION_UNSAFE_URI": 2, "ACTION_UNSUPPORTED": 3}
    actual_action = {key: value for key, value in counts.items() if key.startswith("ACTION_")}
    require(actual_action == expected_action, f"action diagnostic mismatch: {actual_action}")
    qa["tasks"]["15"] = {"assertions": {"states": states, "hover_only_inert": True, "outer_hover_inert": True, "unsafe_program_macro_noop_media_inert": True, "safe_link_attributes": True, "owner_run_table_group_precedence": True, "javascript_hrefs": 0, "popups": 0, "dialogs": 0, "nonzero_top_level_shapes": len(boxes), "top_level_overlaps": 0}, "diagnostic_counts": counts, "action_diagnostic_counts": actual_action}
    page.close()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-dir", type=Path, default=DEFAULT_EVIDENCE)
    args = parser.parse_args()
    evidence = args.evidence_dir.resolve()
    html = prepare(evidence)
    qa: dict[str, Any] = {"schema_version": 1, "native_fidelity_claimed": False, "screenshots": {}, "tasks": {}}
    with serve(html) as base, sync_playwright() as playwright:
        browser = playwright.chromium.launch(headless=True)
        try:
            task12(browser, base, evidence, qa)
            task13(browser, base, evidence, qa)
            task14(browser, base, evidence, qa)
            task15(browser, base, evidence, qa)
            qa["browser"] = {"name": "chromium", "version": browser.version}
        finally:
            browser.close()
    qa["assertion_count"] = sum(len(task["assertions"]) if isinstance(task["assertions"], dict) else len(task["assertions"]) for task in qa["tasks"].values())
    (evidence / "qa.json").write_text(json.dumps(qa, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Browser acceptance evidence written to {evidence}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
