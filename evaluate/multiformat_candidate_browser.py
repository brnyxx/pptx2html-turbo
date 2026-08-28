from __future__ import annotations

import hashlib
import json
import os
import shutil
import tempfile
from pathlib import Path
from typing import cast

from playwright.sync_api import Error as PlaywrightError
from playwright.sync_api import sync_playwright

from evaluate.multiformat_candidate_browser_network import (
    record_console_error,
    route_request,
)
from evaluate.multiformat_candidate_dom import inventory_value
from evaluate.multiformat_candidate_browser_checks import (
    TARGET_PRESENTATION_SIZE,
    record_string,
    require_presentation_dimensions,
    unit_records,
    validate_png,
)
from evaluate.multiformat_candidate_scripts import (
    DISCOVER_UNITS_SCRIPT,
    EXTERNAL_RESOURCES_SCRIPT,
    EXTRACT_DOM_SCRIPT,
    NORMALIZE_PRESENTATION_SCRIPT,
    READINESS_SCRIPT,
    STATIC_STYLE,
)
from evaluate.multiformat_candidate_types import (
    BrowserCaptureResult,
    CandidateCaptureError,
    CapturedUnit,
)
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_inventory import parse_inventory
from evaluate.multiformat_schema import JsonValue


def capture_html_units(
    html: str,
    document_format: DocumentFormat,
    unit_ids: tuple[str, ...],
    output_dir: Path,
    *,
    expected_browser_version: str | None = None,
    executable_path: Path | None = None,
    font_config: Path | None = None,
) -> BrowserCaptureResult:
    output_dir.mkdir(parents=True, exist_ok=True)
    presentation = document_format in {DocumentFormat.PPT, DocumentFormat.PPTX}
    device_scale = 1.0
    external_requests: list[str] = []
    browser_failures: list[str] = []
    captured: list[CapturedUnit] = []
    browser_home = output_dir / ".browser-home"
    browser_home.mkdir(exist_ok=True)
    # Chromium binds its singleton socket under TMPDIR and AF_UNIX socket
    # paths are limited to roughly 100 bytes, so the browser temp directory
    # must stay short even when output_dir is deeply nested.
    browser_temp = Path(tempfile.mkdtemp(prefix=".browser-tmp-"))
    browser_environment = {
        "HOME": browser_home.as_posix(),
        "TMPDIR": browser_temp.as_posix(),
        "PATH": os.defpath,
        "TZ": "UTC",
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
    }
    if font_config is not None:
        browser_environment["FONTCONFIG_FILE"] = font_config.as_posix()
        browser_environment["FONTCONFIG_PATH"] = font_config.parent.as_posix()
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(
                headless=True,
                executable_path=(
                    executable_path.as_posix() if executable_path is not None else None
                ),
                args=[
                    "--disable-background-networking",
                    "--disable-component-update",
                    "--disable-default-apps",
                    "--disable-sync",
                    "--force-color-profile=srgb",
                    "--host-resolver-rules=MAP * 0.0.0.0",
                    "--metrics-recording-only",
                    "--no-first-run",
                ],
                env=browser_environment,
            )
            if (
                expected_browser_version is not None
                and browser.version != expected_browser_version
            ):
                raise CandidateCaptureError(
                    f"Chromium version mismatch: {browser.version}"
                )
            context = browser.new_context(
                viewport={"width": 1920, "height": 2400},
                device_scale_factor=device_scale,
                locale="en-US",
                timezone_id="UTC",
                color_scheme="light",
                reduced_motion="reduce",
                service_workers="block",
                accept_downloads=False,
            )
            document_url = (
                "http://127.0.0.1/candidate/"
                + hashlib.sha256(html.encode()).hexdigest()
                + "/document.html"
            )
            context.route(
                "**/*",
                lambda route, request: route_request(
                    route,
                    request,
                    document_url,
                    html,
                    external_requests,
                ),
            )
            page = context.new_page()
            page.on("pageerror", lambda error: browser_failures.append(str(error)))
            page.on(
                "console",
                lambda message: record_console_error(
                    message,
                    browser_failures,
                ),
            )
            page.on("crash", lambda: browser_failures.append("page crashed"))
            page.on(
                "popup", lambda popup: browser_failures.append(f"popup: {popup.url}")
            )
            page.on(
                "download",
                lambda download: browser_failures.append(
                    f"download: {download.suggested_filename}"
                ),
            )
            page.goto(document_url, wait_until="domcontentloaded")
            external_requests.extend(
                cast(list[str], page.evaluate(EXTERNAL_RESOURCES_SCRIPT))
            )
            if external_requests:
                raise CandidateCaptureError(
                    f"network request attempted: {external_requests[0]}"
                )
            page.add_style_tag(content=STATIC_STYLE)
            if presentation:
                page.evaluate(
                    NORMALIZE_PRESENTATION_SCRIPT,
                    {
                        "format": document_format.value,
                        "width": TARGET_PRESENTATION_SIZE[0],
                        "height": TARGET_PRESENTATION_SIZE[1],
                    },
                )
            page.evaluate(READINESS_SCRIPT)
            if external_requests:
                raise CandidateCaptureError(
                    f"network request attempted: {external_requests[0]}"
                )
            if browser_failures:
                raise CandidateCaptureError(browser_failures[0])
            units = unit_records(
                page.evaluate(DISCOVER_UNITS_SCRIPT, document_format.value)
            )
            if len(units) != len(unit_ids):
                raise CandidateCaptureError(
                    f"unit count mismatch: expected {len(unit_ids)}, got {len(units)}"
                )
            for unit_id, unit in zip(unit_ids, units, strict=True):
                selector = record_string(unit, "selector")
                if presentation:
                    require_presentation_dimensions(unit)
                raw = cast(
                    dict[str, JsonValue],
                    page.evaluate(
                        EXTRACT_DOM_SCRIPT,
                        {
                            "selector": selector,
                            "spreadsheet": document_format
                            in {DocumentFormat.XLS, DocumentFormat.XLSX},
                        },
                    ),
                )
                inventory = output_dir / f"{unit_id}.json"
                png = output_dir / f"{unit_id}.png"
                _write_json(
                    inventory,
                    inventory_value(unit_id, raw, document_format, device_scale),
                )
                parse_inventory(inventory, unit_id)
                page.locator(selector).screenshot(
                    path=png,
                    animations="disabled",
                    caret="hide",
                    scale="css" if presentation else "device",
                )
                validate_png(
                    png,
                    presentation,
                    unit,
                    device_scale,
                )
                captured.append(CapturedUnit(unit_id, png, inventory))
            if external_requests:
                raise CandidateCaptureError(
                    f"network request attempted: {external_requests[0]}"
                )
            if browser_failures:
                raise CandidateCaptureError(browser_failures[0])
            context.close()
            browser.close()
            return BrowserCaptureResult(
                browser.version,
                tuple(captured),
                tuple(external_requests),
            )
    except CandidateCaptureError:
        raise
    except (OSError, PlaywrightError, TypeError, ValueError) as error:
        raise CandidateCaptureError(str(error)) from error
    finally:
        shutil.rmtree(browser_temp, ignore_errors=True)


def _write_json(path: Path, value: JsonValue) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


__all__ = [
    "CandidateCaptureError",
    "capture_html_units",
]
