from __future__ import annotations

import json
import logging
import os
import tempfile
from collections.abc import Iterator
from contextlib import contextmanager
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from shutil import copy2
from threading import Thread

LOGGER = logging.getLogger(__name__)


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, message_format: str, *args: object) -> None:
        LOGGER.debug(message_format, *args)


@contextmanager
def serve(root: Path) -> Iterator[str]:
    handler = partial(QuietHandler, directory=str(root))
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        host, port = server.server_address
        yield f"http://{host}:{port}"
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


def assemble_site(root: Path, site: Path) -> None:
    package = root / "crates" / "pptx2html-wasm" / "pkg"
    package_site = site / "pkg"
    package_site.mkdir()
    (site / "index.html").write_text(
        """<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><title>pptx-to-html Demo · v2.0.1</title></head>
<body><iframe id="output" title="Converted presentation"></iframe></body>
</html>
""",
        encoding="utf-8",
    )
    copy2(
        root / "crates" / "pptx2html-cli" / "tests" / "fixtures" / "two-slides.pptx",
        site,
    )
    for file_name in (
        "index.js",
        "pptx2html_wasm.js",
        "pptx2html_wasm_bg.wasm",
    ):
        copy2(package / file_name, package_site)


def main() -> None:
    from playwright.sync_api import (
        ConsoleMessage,
        Error,
        Request,
        Response,
        sync_playwright,
    )

    root = Path(__file__).resolve().parents[3]
    browser_errors: list[str] = []
    wasm_requests: list[str] = []

    def record_console(message: ConsoleMessage) -> None:
        if message.type == "error":
            browser_errors.append(message.text)

    def record_page_error(error: Error) -> None:
        browser_errors.append(str(error))

    def record_response(response: Response) -> None:
        if response.status >= 400:
            browser_errors.append(f"HTTP {response.status}: {response.url}")

    def record_request(request: Request) -> None:
        if request.url.endswith("pptx2html_wasm_bg.wasm"):
            wasm_requests.append(request.url)

    with tempfile.TemporaryDirectory(prefix="pptx-package-browser-") as temporary:
        site = Path(temporary)
        assemble_site(root, site)
        with serve(site) as base_url, sync_playwright() as playwright:
            browser = playwright.chromium.launch(headless=True)
            page = browser.new_page(viewport={"width": 1440, "height": 1000})
            page.on("console", record_console)
            page.on("pageerror", record_page_error)
            page.on("request", record_request)
            page.on("response", record_response)
            page.goto(f"{base_url}/index.html", wait_until="load")
            wasm_requests_before_facade = len(wasm_requests)
            result = page.evaluate(
                """async () => {
                  const module = await import('/pkg/index.js');
                  const response = await fetch('/two-slides.pptx');
                  const output = document.querySelector('#output');
                  const loaded = new Promise(resolve => {
                    output.addEventListener('load', resolve, {once: true});
                  });
                  const html = await module.pptxToHtml(await response.blob());
                  output.srcdoc = html;
                  await loaded;
                  return {
                    title: document.title,
                    startsWithDoctype: html.startsWith('<!DOCTYPE html>'),
                    hasDiagnostics: html.includes('id="pptx2html-diagnostics"'),
                    htmlBytes: new TextEncoder().encode(html).byteLength,
                  };
                }"""
            )
            screenshot_value = os.environ.get("PPTX_BROWSER_SMOKE_SCREENSHOT")
            if screenshot_value is not None:
                page.screenshot(path=screenshot_value, full_page=True)
            browser.close()

    if wasm_requests_before_facade != 0:
        raise AssertionError(
            "page initialized WASM before the package facade call"
        )
    if len(wasm_requests) != 1:
        raise AssertionError(
            f"expected one lazy WASM request, observed {len(wasm_requests)}"
        )
    if browser_errors:
        raise AssertionError(f"browser errors: {browser_errors}")
    if result["title"] != "pptx-to-html Demo · v2.0.1":
        raise AssertionError(f"unexpected demo title: {result['title']}")
    if result["startsWithDoctype"] is not True:
        raise AssertionError("facade output does not start with HTML doctype")
    if result["hasDiagnostics"] is not True:
        raise AssertionError("facade output is missing diagnostics")
    if result["htmlBytes"] < 1_000:
        raise AssertionError(
            f"facade output is unexpectedly small: {result['htmlBytes']}"
        )

    LOGGER.info("browser package smoke: %s", json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    main()
