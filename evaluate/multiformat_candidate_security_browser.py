from __future__ import annotations

import hashlib
import os
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from playwright.sync_api import Error as PlaywrightError
from playwright.sync_api import Request, Route, sync_playwright

from evaluate.multiformat_candidate_browser_checks import browser_version_matches
from evaluate.multiformat_candidate_browser_network import route_request
from evaluate.multiformat_candidate_types import CandidateCaptureError


@dataclass(frozen=True, slots=True)
class SecurityBrowserFacts:
    external_requests: tuple[str, ...]
    active_content_executed: bool


def inspect_security_html(
    html: str,
    *,
    chromium: Path,
    browser_version: str,
    font_config: Path,
) -> SecurityBrowserFacts:
    external: list[str] = []
    active = False
    url = "http://127.0.0.1/security/" + hashlib.sha256(html.encode()).hexdigest()
    environment: dict[str, str | float | bool] = {
        "HOME": font_config.parent.as_posix(),
        "TMPDIR": font_config.parent.as_posix(),
        "PATH": os.defpath,
        "TZ": "UTC",
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
        "FONTCONFIG_FILE": font_config.as_posix(),
        "FONTCONFIG_PATH": font_config.parent.as_posix(),
    }
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(
                headless=True,
                executable_path=chromium.as_posix(),
                args=[
                    "--disable-background-networking",
                    "--host-resolver-rules=MAP * 0.0.0.0",
                    "--no-first-run",
                ],
                env=environment,
            )
            if not browser_version_matches(browser_version, browser.version):
                raise CandidateCaptureError("security Chromium version mismatch")
            context = browser.new_context(
                locale="en-US",
                timezone_id="UTC",
                service_workers="block",
                accept_downloads=False,
            )
            context.route(
                "**/*",
                lambda route, request: _route_security(
                    route, request, url, html, external
                ),
            )
            page = context.new_page()
            signals: list[str] = []
            page.on("popup", lambda _popup: signals.append("popup"))
            page.on("download", lambda _download: signals.append("download"))
            page.goto(url, wait_until="domcontentloaded")
            active = bool(
                cast(
                    bool,
                    page.evaluate(
                        """() => {
                        if (document.querySelector('object,embed,iframe,applet')) {
                          return true;
                        }
                        return [...document.scripts].some(script => {
                          const type = script.type.trim().toLowerCase();
                          return !type || type === 'module'
                            || /^(?:text|application)\\/(?:java|ecma)script$/.test(type);
                        });
                        }"""
                    ),
                )
                or signals
            )
            context.close()
            browser.close()
    except CandidateCaptureError:
        raise
    except (OSError, PlaywrightError, TypeError, ValueError) as error:
        raise CandidateCaptureError("security browser inspection failed") from error
    return SecurityBrowserFacts(tuple(external), active)


def _route_security(
    route: Route,
    request: Request,
    document_url: str,
    html: str,
    external: list[str],
) -> None:
    if request.url == document_url and request.resource_type == "document":
        route.fulfill(status=200, content_type="text/html; charset=utf-8", body=html)
        return
    route_request(route, request, document_url, html, external)


__all__ = ["SecurityBrowserFacts", "inspect_security_html"]
