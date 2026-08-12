#!/usr/bin/env python3
"""Rebuild the exact Task 20 plan-named browser evidence artifacts."""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
import tempfile
import threading
import zipfile
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from playwright.sync_api import sync_playwright


ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / ".omo/evidence/task-20-charts"
HTML_NAME = "task-20-chart-completeness.html"
PNG_NAME = "task-20-chart-completeness.png"
STATE_NAME = "task-20-chart-completeness-browser.json"
ERROR_NAME = "task-20-chart-completeness-error.json"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_structure_error_evidence(source: Path, directory: Path) -> None:
    hostile = directory / "task-20-structure-error.pptx"
    with zipfile.ZipFile(source) as source_zip, zipfile.ZipFile(hostile, "w") as target_zip:
        for info in source_zip.infolist():
            payload = source_zip.read(info.filename)
            if info.filename == "ppt/charts/chart1.xml":
                xml = payload.decode("utf-8")
                family = re.search(r"(<c:barChart>.*?</c:barChart>)", xml)
                if family is None:
                    raise RuntimeError("direct chart fixture has no c:barChart")
                xml = xml.replace(family.group(1), family.group(1) * 2, 1)
                payload = xml.encode("utf-8")
            target_zip.writestr(info, payload)
    html = directory / "task-20-structure-error.html"
    subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "pptx2html-cli",
            "--",
            str(hostile),
            "-o",
            str(html),
        ],
        cwd=ROOT,
        check=True,
    )
    match = re.search(
        r'<script type="application/json" id="pptx2html-diagnostics">(.*?)</script>',
        html.read_text(encoding="utf-8"),
        re.DOTALL,
    )
    if match is None:
        raise RuntimeError("converted HTML has no diagnostics payload")
    diagnostics = [
        item
        for item in json.loads(match.group(1))
        if item["code"] == "CHART_STRUCTURE_UNSUPPORTED"
    ]
    if len(diagnostics) != 1:
        raise RuntimeError(f"expected one structural chart diagnostic: {diagnostics}")
    (EVIDENCE / ERROR_NAME).write_text(
        json.dumps(diagnostics, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def main() -> int:
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="task-20-chart-completeness-") as temporary:
        completion_decks = Path(temporary) / "completion-decks"
        subprocess.run(
            [
                sys.executable,
                "evaluate/create_completion_decks.py",
                "--output-dir",
                str(completion_decks),
            ],
            cwd=ROOT,
            check=True,
        )
        html_path = EVIDENCE / HTML_NAME
        subprocess.run(
            [
                "cargo",
                "run",
                "-q",
                "-p",
                "pptx2html-cli",
                "--",
                str(completion_decks / "charts.pptx"),
                "-o",
                str(html_path),
            ],
            cwd=ROOT,
            check=True,
        )
        write_structure_error_evidence(completion_decks / "charts.pptx", Path(temporary))

    handler = lambda *args, **kwargs: SimpleHTTPRequestHandler(  # noqa: E731
        *args, directory=str(EVIDENCE), **kwargs
    )
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(headless=True)
            page = browser.new_page(viewport={"width": 1440, "height": 1000})
            page_errors: list[str] = []
            console_errors: list[str] = []
            page.on("pageerror", lambda error: page_errors.append(str(error)))
            page.on(
                "console",
                lambda message: console_errors.append(message.text)
                if message.type == "error"
                else None,
            )
            response = page.goto(
                f"http://127.0.0.1:{server.server_port}/{HTML_NAME}",
                wait_until="load",
            )
            counts = {
                "direct": page.locator(".chart-direct").count(),
                "preview": page.locator("img.shape-image[alt='Chart']").count(),
                "placeholder": page.locator(".chart-placeholder").count(),
            }
            if response is None or response.status != 200:
                raise RuntimeError("browser did not load the generated Task 20 HTML")
            if counts != {"direct": 1, "preview": 1, "placeholder": 1}:
                raise RuntimeError(f"unexpected chart dispositions: {counts}")
            if page_errors or console_errors:
                raise RuntimeError(
                    f"browser errors: page={page_errors}, console={console_errors}"
                )
            screenshot_path = EVIDENCE / PNG_NAME
            page.screenshot(path=str(screenshot_path), full_page=True)
            state = {
                "command": "python3 scripts/task-20-chart-completeness-browser.py",
                "plan_scenario": "Mixed chart deck uses truthful direct/fallback paths",
                "url_path": f"/{HTML_NAME}",
                "http_status": response.status,
                "browser": {
                    "name": "Chromium",
                    "version": browser.version,
                    "viewport": {"width": 1440, "height": 1000},
                },
                "dispositions": counts,
                "page_errors": page_errors,
                "console_errors": console_errors,
                "artifacts": {
                    HTML_NAME: {"sha256": sha256(html_path)},
                    PNG_NAME: {"sha256": sha256(screenshot_path)},
                    ERROR_NAME: {"sha256": sha256(EVIDENCE / ERROR_NAME)},
                },
                "native_powerpoint_fidelity": "[cross-validation required]",
            }
            browser.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join()

    (EVIDENCE / STATE_NAME).write_text(
        json.dumps(state, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps(state, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
