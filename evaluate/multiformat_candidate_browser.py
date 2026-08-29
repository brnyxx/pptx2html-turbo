from __future__ import annotations

import hashlib
import json
import math
import os
import shutil
import tempfile
from pathlib import Path
from typing import cast

from PIL import Image
from playwright.sync_api import Error as PlaywrightError
from playwright.sync_api import sync_playwright

from evaluate.multiformat_candidate_browser_network import (
    record_console_error,
    route_request,
)
from evaluate.multiformat_candidate_dom import inventory_value
from evaluate.multiformat_candidate_browser_checks import (
    TARGET_PRESENTATION_SIZE,
    browser_version_matches,
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

MAX_AGGREGATE_PAGES = 256
MAX_AGGREGATE_WIDTH = 8_192
MAX_AGGREGATE_HEIGHT = 131_071
MAX_AGGREGATE_PIXELS = 128_000_000
MAX_AGGREGATE_TEXT_CODE_UNITS = 2_000_000
MAX_AGGREGATE_ELEMENTS = 100_000


def capture_html_units(
    html: str,
    document_format: DocumentFormat,
    unit_ids: tuple[str, ...],
    output_dir: Path,
    *,
    source_track: str,
    aggregate_paged_units: bool,
    expected_browser_version: str | None = None,
    executable_path: Path | None = None,
    font_config: Path | None = None,
) -> BrowserCaptureResult:
    output_dir.mkdir(parents=True, exist_ok=True)
    presentation = document_format in {DocumentFormat.PPT, DocumentFormat.PPTX}
    if source_track not in {"conformance", "blind"}:
        raise CandidateCaptureError(f"invalid source track: {source_track}")
    if aggregate_paged_units and (
        source_track != "conformance" or presentation or len(unit_ids) != 1
    ):
        raise CandidateCaptureError("invalid paged-unit aggregation policy")
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
            if expected_browser_version is not None and not browser_version_matches(
                expected_browser_version,
                browser.version,
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
                page.evaluate(
                    DISCOVER_UNITS_SCRIPT,
                    {
                        "format": document_format.value,
                        "aggregatePages": aggregate_paged_units,
                    },
                )
            )
            if len(units) != len(unit_ids):
                raise CandidateCaptureError(
                    f"unit count mismatch: expected {len(unit_ids)}, got {len(units)}"
                )
            for unit_id, unit in zip(unit_ids, units, strict=True):
                if aggregate_paged_units:
                    _validate_aggregate_unit(unit)
                selector = (
                    record_string(unit, "selector")
                    if not aggregate_paged_units
                    else None
                )
                selectors = (
                    _record_strings(unit, "selectors")
                    if aggregate_paged_units
                    else None
                )
                if presentation:
                    require_presentation_dimensions(unit)
                raw = cast(
                    dict[str, JsonValue],
                    page.evaluate(
                        EXTRACT_DOM_SCRIPT,
                        {
                            "selector": selector,
                            "selectors": selectors,
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
                if aggregate_paged_units:
                    _capture_aggregate_png(page, unit, selectors, png)
                else:
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


def _record_strings(values: dict[str, JsonValue], field: str) -> list[str]:
    value = values.get(field)
    if (
        not isinstance(value, list)
        or not value
        or any(not isinstance(item, str) or not item for item in value)
    ):
        raise CandidateCaptureError(f"invalid unit {field}")
    return value


def _record_number(values: dict[str, JsonValue], field: str) -> float:
    value = values.get(field)
    if (
        not isinstance(value, int | float)
        or isinstance(value, bool)
        or not math.isfinite(value)
        or value < 0
    ):
        raise CandidateCaptureError(f"invalid unit {field}")
    return float(value)


def _record_count(values: dict[str, JsonValue], field: str) -> int:
    value = values.get(field)
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        raise CandidateCaptureError(f"invalid unit {field}")
    return value


def _validate_aggregate_unit(unit: dict[str, JsonValue]) -> None:
    page_count = _record_count(unit, "pageCount")
    text_code_units = _record_count(unit, "textCodeUnits")
    element_count = _record_count(unit, "elementCount")
    pages = unit.get("pages")
    if (
        page_count == 0
        or page_count > MAX_AGGREGATE_PAGES
        or not isinstance(pages, list)
        or len(pages) != page_count
    ):
        raise CandidateCaptureError("aggregate page count exceeds limit")
    if text_code_units > MAX_AGGREGATE_TEXT_CODE_UNITS:
        raise CandidateCaptureError("aggregate text work exceeds limit")
    if element_count > MAX_AGGREGATE_ELEMENTS:
        raise CandidateCaptureError("aggregate element work exceeds limit")

    x = _record_number(unit, "x")
    y = _record_number(unit, "y")
    width = _record_number(unit, "width")
    height = _record_number(unit, "height")
    if width <= 0 or height <= 0:
        raise CandidateCaptureError("aggregate dimensions are invalid")
    if (
        width > MAX_AGGREGATE_WIDTH
        or height > MAX_AGGREGATE_HEIGHT
        or x + width > MAX_AGGREGATE_WIDTH
        or y + height > MAX_AGGREGATE_HEIGHT
    ):
        raise CandidateCaptureError("aggregate dimensions exceed limit")
    if width * height > MAX_AGGREGATE_PIXELS:
        raise CandidateCaptureError("aggregate pixel area exceeds limit")

    right = x + width
    bottom = y + height
    for page in pages:
        if not isinstance(page, dict):
            raise CandidateCaptureError("aggregate page geometry is invalid")
        page_x = _record_number(page, "x")
        page_y = _record_number(page, "y")
        page_width = _record_number(page, "width")
        page_height = _record_number(page, "height")
        if (
            page_width <= 0
            or page_height <= 0
            or page_width > MAX_AGGREGATE_WIDTH
            or page_height > MAX_AGGREGATE_HEIGHT
            or page_width * page_height > MAX_AGGREGATE_PIXELS
            or page_x < x
            or page_y < y
            or page_x + page_width > right
            or page_y + page_height > bottom
        ):
            raise CandidateCaptureError("aggregate page geometry is invalid")


def _aggregate_viewport(unit: dict[str, JsonValue]) -> dict[str, int]:
    width = math.ceil(_record_number(unit, "x") + _record_number(unit, "width"))
    height = math.ceil(_record_number(unit, "y") + _record_number(unit, "height"))
    if width <= 0 or height <= 0 or width * height > MAX_AGGREGATE_PIXELS:
        raise CandidateCaptureError("aggregate viewport exceeds limit")
    return {"width": width, "height": height}


def _capture_aggregate_png(
    page: object,
    unit: dict[str, JsonValue],
    selectors: list[str] | None,
    output: Path,
) -> None:
    pages = unit.get("pages")
    if not isinstance(pages, list) or selectors is None or len(pages) != len(selectors):
        raise CandidateCaptureError("aggregate page inventory is invalid")
    width = round(_record_number(unit, "width"))
    height = round(_record_number(unit, "height"))
    origin_x = _record_number(unit, "x")
    origin_y = _record_number(unit, "y")
    canvas = Image.new("RGB", (width, height), (255, 255, 255))
    with tempfile.TemporaryDirectory(prefix=".aggregate-pages-") as temp_dir:
        temporary = Path(temp_dir)
        for ordinal, (selector, geometry) in enumerate(
            zip(selectors, pages, strict=True), start=1
        ):
            if not isinstance(geometry, dict):
                raise CandidateCaptureError("aggregate page geometry is invalid")
            page_png = temporary / f"page-{ordinal}.png"
            page.locator(selector).screenshot(
                path=page_png,
                animations="disabled",
                caret="hide",
                scale="device",
            )
            expected = (
                round(_record_number(geometry, "width")),
                round(_record_number(geometry, "height")),
            )
            with Image.open(page_png) as image:
                if image.format != "PNG" or image.size != expected:
                    raise CandidateCaptureError(
                        f"aggregate page dimension mismatch: expected {expected}, got {image.size}"
                    )
                offset = (
                    round(_record_number(geometry, "x") - origin_x),
                    round(_record_number(geometry, "y") - origin_y),
                )
                canvas.paste(image.convert("RGB"), offset)
    canvas.save(output, format="PNG")


__all__ = [
    "CandidateCaptureError",
    "capture_html_units",
]
