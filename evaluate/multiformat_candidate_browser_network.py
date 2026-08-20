from __future__ import annotations

from playwright.sync_api import ConsoleMessage, Request, Route

CSP = (
    "default-src 'none'; img-src data:; style-src 'unsafe-inline'; "
    "font-src data:; script-src 'none'; media-src data:; connect-src 'none'; "
    "object-src 'none'; frame-src 'none'"
)


def route_request(
    route: Route,
    request: Request,
    document_url: str,
    html: str,
    external_requests: list[str],
) -> None:
    if request.url == document_url and request.resource_type == "document":
        route.fulfill(
            status=200,
            content_type="text/html; charset=utf-8",
            headers={"Content-Security-Policy": CSP},
            body=html,
        )
        return
    if request.url.startswith(("http:", "https:", "file:", "ws:", "wss:")):
        external_requests.append(request.url)
        route.abort("blockedbyclient")
    else:
        route.continue_()


def record_console_error(
    message: ConsoleMessage,
    browser_failures: list[str],
) -> None:
    if message.type != "error":
        return
    if "violates the following Content Security Policy directive" in message.text:
        return
    browser_failures.append(f"console: {message.text}")
