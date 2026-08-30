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
    AggregateGeometry,
    TARGET_PRESENTATION_SIZE,
    aggregate_geometry,
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
    ISOLATE_DISCOVERED_UNIT_SCRIPT,
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

MAX_AGGREGATE_PAGES = 128
MAX_AGGREGATE_WIDTH = 8_192
MAX_AGGREGATE_HEIGHT = 262_143
MAX_AGGREGATE_PIXELS = 256_000_000
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
            discovered_selectors = (
                [record_string(unit, "selector") for unit in units]
                if not aggregate_paged_units
                else []
            )
            for index, (unit_id, unit) in enumerate(
                zip(unit_ids, units, strict=True)
            ):
                canonical_geometry: AggregateGeometry | None = None
                if aggregate_paged_units:
                    canonical_geometry = _aggregate_unit_geometry(unit)
                aggregate_scale = (
                    canonical_geometry.scale if canonical_geometry is not None else 1.0
                )
                selector = (
                    discovered_selectors[index]
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
                if not aggregate_paged_units:
                    page.evaluate(
                        ISOLATE_DISCOVERED_UNIT_SCRIPT,
                        {
                            "selector": selector,
                            "selectors": discovered_selectors,
                        },
                    )
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
                    inventory_value(
                        unit_id,
                        raw,
                        document_format,
                        device_scale * aggregate_scale,
                    ),
                )
                parse_inventory(inventory, unit_id)
                if aggregate_paged_units:
                    if canonical_geometry is None:
                        raise CandidateCaptureError("aggregate geometry is unavailable")
                    _capture_aggregate_png(
                        page,
                        selectors,
                        png,
                        canonical_geometry,
                    )
                else:
                    page.locator(selector).screenshot(
                        path=png,
                        animations="disabled",
                        caret="hide",
                        scale="css" if presentation else "device",
                    )
                if aggregate_paged_units:
                    if canonical_geometry is None:
                        raise CandidateCaptureError("aggregate geometry is unavailable")
                    _validate_aggregate_png(png, canonical_geometry)
                else:
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
    _ = _aggregate_unit_geometry(unit)


def _aggregate_unit_geometry(unit: dict[str, JsonValue]) -> AggregateGeometry:
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

    dimensions: list[tuple[int, int]] = []
    expected_y = y
    for page in pages:
        if not isinstance(page, dict):
            raise CandidateCaptureError("aggregate page geometry is invalid")
        page_x = _record_number(page, "x")
        page_y = _record_number(page, "y")
        page_width = _record_number(page, "width")
        page_height = _record_number(page, "height")
        canonical_width = round(page_width)
        canonical_height = round(page_height)
        if (
            page_width <= 0
            or page_height <= 0
            or abs(page_width - canonical_width) > 1e-6
            or abs(page_height - canonical_height) > 1e-6
            or page_width > MAX_AGGREGATE_WIDTH
            or page_height > MAX_AGGREGATE_HEIGHT
            or page_width * page_height > MAX_AGGREGATE_PIXELS
            or abs(page_x - x) > 1e-6
            or abs(page_y - expected_y) > 1e-6
        ):
            raise CandidateCaptureError("aggregate page geometry is invalid")
        dimensions.append((canonical_width, canonical_height))
        expected_y += canonical_height
    if (
        abs(width - max(page_width for page_width, _ in dimensions)) > 1e-6
        or abs(height - sum(page_height for _, page_height in dimensions)) > 1e-6
        or abs(expected_y - (y + height)) > 1e-6
    ):
        raise CandidateCaptureError("aggregate page geometry is invalid")
    try:
        return aggregate_geometry(dimensions)
    except ValueError as error:
        raise CandidateCaptureError(str(error)) from error


def _capture_aggregate_png(
    page: object,
    selectors: list[str] | None,
    output: Path,
    geometry: AggregateGeometry,
) -> None:
    if selectors is None or len(geometry.pages) != len(selectors):
        raise CandidateCaptureError("aggregate page inventory is invalid")
    canvas = Image.new(
        "RGB",
        (geometry.scaled_width, geometry.scaled_height),
        (255, 255, 255),
    )
    with tempfile.TemporaryDirectory(prefix=".aggregate-pages-") as temp_dir:
        temporary = Path(temp_dir)
        for ordinal, (selector, page_geometry) in enumerate(
            zip(selectors, geometry.pages, strict=True), start=1
        ):
            page_png = temporary / f"page-{ordinal}.png"
            page.locator(selector).screenshot(
                path=page_png,
                animations="disabled",
                caret="hide",
                scale="device",
            )
            expected = (
                page_geometry.width,
                page_geometry.height,
            )
            with Image.open(page_png) as image:
                if image.format != "PNG" or image.size != expected:
                    raise CandidateCaptureError(
                        f"aggregate page dimension mismatch: expected {expected}, got {image.size}"
                    )
                resized = image.convert("RGB").resize(
                    (
                        page_geometry.scaled_width,
                        page_geometry.scaled_height,
                    ),
                    Image.Resampling.LANCZOS,
                )
                canvas.paste(resized, (0, page_geometry.scaled_top))
    canvas.save(output, format="PNG")


def _validate_aggregate_png(path: Path, geometry: AggregateGeometry) -> None:
    expected = (geometry.scaled_width, geometry.scaled_height)
    with Image.open(path) as image:
        if image.format != "PNG" or image.size != expected:
            raise CandidateCaptureError(
                f"aggregate PNG dimension mismatch: expected {expected}, got {image.size}"
            )


__all__ = [
    "CandidateCaptureError",
    "capture_html_units",
]
