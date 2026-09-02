#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import logging
import os
import re
import stat
import subprocess
from collections.abc import Iterator
from contextlib import ExitStack, contextmanager
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Final, Protocol, TypeAlias
from urllib.parse import urlsplit, urlunsplit

JsonValue: TypeAlias = (
    None | bool | int | float | str | list["JsonValue"] | dict[str, "JsonValue"]
)
JsonObject: TypeAlias = dict[str, JsonValue]


class CatalogResponse(Protocol):
    def body(self) -> bytes: ...


logger = logging.getLogger(__name__)
REPO_ROOT: Final = Path(__file__).resolve().parents[1]
GIT_SHA_RE: Final = re.compile(r"^[0-9a-f]{40}$")
HASH_RE: Final = re.compile(r"^[0-9a-f]{64}$")
CAPTURED_AT_RE: Final = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$")
VIEWPORT_WIDTHS: Final = (375, 768, 1280)
VIEWPORT_HEIGHT: Final = 900
CANONICAL_URL: Final = "https://brnyxx.github.io/pptx2html-turbo/capabilities/"
DIMENSIONS: Final = ("semantic", "visual", "behavioral")
TIERS: Final = ("exact", "approximate", "fallback", "unparsed")
OWNED_EVIDENCE_NAMES: Final = tuple(
    [f"landing-{w}.png" for w in VIEWPORT_WIDTHS]
    + [f"catalog-top-{w}.png" for w in VIEWPORT_WIDTHS]
    + [f"catalog-records-{w}.png" for w in VIEWPORT_WIDTHS]
    + ["browser-qa.json", "integrity-review.md", "visual-fidelity-review.md"]
)


class QaError(Exception):
    pass


@dataclass(frozen=True, slots=True)
class CatalogNavigation:
    resolved_no_query: str
    capture_url: str


@dataclass(frozen=True, slots=True)
class ManifestStats:
    manifest_sha256: str
    feature_count: int
    family_count: int
    current_disposition_count: int
    target_disposition_count: int
    exact_current_count: int
    tier_counts: dict[str, int]
    unavailable_source_count: int


@dataclass(frozen=True, slots=True)
class CliArgs:
    base_url: str
    manifest: Path
    catalog_html: Path
    evidence_dir: Path
    git_sha: str


@dataclass(frozen=True, slots=True)
class RuntimeContext:
    base_url: str
    evidence: EvidenceDirectory
    stats: ManifestStats
    catalog_html_sha256: str


@dataclass(frozen=True, slots=True)
class EvidenceDirectory:
    path: Path
    descriptor: int
    device: int
    inode: int

    def write_bytes(self, name: str, content: bytes) -> None:
        _owned_evidence_name(name)
        try:
            file_descriptor = os.open(
                name,
                os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                0o600,
                dir_fd=self.descriptor,
            )
            try:
                remaining = memoryview(content)
                while remaining:
                    written = os.write(file_descriptor, remaining)
                    if written == 0:
                        raise OSError("evidence write made no progress")
                    remaining = remaining[written:]
            finally:
                os.close(file_descriptor)
        except OSError as error:
            raise QaError(f"failed to write evidence file: {name}") from error

    def sha256(self, name: str) -> str:
        _owned_evidence_name(name)
        try:
            file_descriptor = os.open(
                name,
                os.O_RDONLY | os.O_NOFOLLOW,
                dir_fd=self.descriptor,
            )
            with os.fdopen(file_descriptor, "rb") as evidence_file:
                file_status = os.fstat(evidence_file.fileno())
                if not stat.S_ISREG(file_status.st_mode):
                    raise OSError("evidence input is not a regular file")
                return hashlib.sha256(evidence_file.read()).hexdigest()
        except OSError as error:
            raise QaError(f"failed to hash evidence file: {name}") from error

    def validate_public_path(self) -> None:
        try:
            public_status = os.stat(self.path, follow_symlinks=False)
        except OSError as error:
            raise QaError(
                f"failed to inspect evidence directory: {self.path}"
            ) from error
        if (
            not stat.S_ISDIR(public_status.st_mode)
            or public_status.st_dev != self.device
            or public_status.st_ino != self.inode
        ):
            raise QaError(f"evidence directory changed during browser QA: {self.path}")


def _owned_evidence_name(name: str) -> None:
    if name not in OWNED_EVIDENCE_NAMES or Path(name).name != name:
        raise QaError(f"unexpected evidence filename: {name}")


def sha256_file(path: Path) -> str:
    _regular_file(path, "sha256 input")
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError as error:
        raise QaError(f"failed to read sha256 input: {path}") from error


def _regular_file(path: Path, label: str) -> None:
    try:
        path.lstat()
    except OSError as error:
        raise QaError(f"failed to inspect {label}: {path}") from error
    if path.is_symlink():
        raise QaError(f"{label} must be a real regular file: {path}") from OSError(
            "symlink rejected"
        )
    if not path.is_file():
        raise QaError(
            f"{label} must be a real regular file: {path}"
        ) from IsADirectoryError(str(path))


@contextmanager
def _prepared_evidence_directory(
    evidence_dir: Path, repo_root: Path = REPO_ROOT
) -> Iterator[EvidenceDirectory]:
    try:
        root = repo_root.resolve(strict=True)
    except OSError as error:
        raise QaError(f"failed to resolve repository root: {repo_root}") from error
    lexical_root = repo_root if repo_root.is_absolute() else Path.cwd() / repo_root
    if evidence_dir.is_absolute():
        try:
            relative = evidence_dir.relative_to(lexical_root)
        except ValueError:
            try:
                relative = evidence_dir.relative_to(root)
            except ValueError as error:
                raise QaError(
                    "evidence directory must be inside the repository"
                ) from error
    else:
        relative = evidence_dir
    candidate = root / relative
    if (
        len(relative.parts) < 3
        or relative.parts[:2] != (".omo", "evidence")
        or ".." in relative.parts
    ):
        raise QaError("evidence directory must be below .omo/evidence") from ValueError(
            str(evidence_dir)
        )
    try:
        with ExitStack() as descriptors:
            directory_descriptor = os.open(
                root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
            )
            descriptors.callback(os.close, directory_descriptor)
            for part in relative.parts:
                try:
                    os.mkdir(part, dir_fd=directory_descriptor)
                except FileExistsError:
                    pass
                directory_descriptor = os.open(
                    part,
                    os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                    dir_fd=directory_descriptor,
                )
                descriptors.callback(os.close, directory_descriptor)
            for name in OWNED_EVIDENCE_NAMES:
                try:
                    os.unlink(name, dir_fd=directory_descriptor)
                except FileNotFoundError:
                    pass
            directory_status = os.fstat(directory_descriptor)
            yield EvidenceDirectory(
                candidate,
                directory_descriptor,
                directory_status.st_dev,
                directory_status.st_ino,
            )
    except OSError as error:
        raise QaError(f"failed to clean evidence directory: {candidate}") from error


def cleanup_owned_evidence(evidence_dir: Path, repo_root: Path = REPO_ROOT) -> Path:
    with _prepared_evidence_directory(evidence_dir, repo_root) as prepared:
        return prepared.path


def _git_text(args: list[str], repo_root: Path) -> str:
    try:
        return subprocess.run(
            args, cwd=repo_root, check=True, capture_output=True, text=True
        ).stdout.strip()
    except subprocess.CalledProcessError as error:
        raise QaError(f"git command failed: {' '.join(args)}") from error
    except OSError as error:
        raise QaError(f"git command could not start: {' '.join(args)}") from error


def assert_clean_git_binding(git_sha: str, repo_root: Path) -> None:
    if GIT_SHA_RE.fullmatch(git_sha) is None:
        raise QaError("--git-sha must be 40 lowercase hexadecimal characters")
    if _git_text(["git", "rev-parse", "HEAD"], repo_root) != git_sha:
        raise QaError("--git-sha does not match HEAD")
    if _git_text(["git", "status", "--porcelain", "--untracked-files=no"], repo_root):
        raise QaError("tracked worktree changes must be committed before browser QA")


def catalog_navigation_urls(
    base_url: str, resolved_catalog_url: str
) -> CatalogNavigation:
    base = urlsplit(base_url)
    catalog = urlsplit(resolved_catalog_url)
    if base.username or base.password or not base.scheme or not base.netloc:
        raise QaError("--base-url must be an absolute URL without credentials")
    base_path = (
        base.path if base.path.endswith("/") else base.path.rsplit("/", 1)[0] + "/"
    )
    expected = urlunsplit(
        (base.scheme, base.netloc, f"{base_path}capabilities/", "", "")
    )
    if catalog.username or catalog.password or catalog.query or catalog.fragment:
        raise QaError(
            "resolved catalog URL must not contain credentials, query, or fragment"
        )
    resolved = urlunsplit(
        (catalog.scheme, catalog.netloc, catalog.path, "", catalog.fragment)
    )
    if resolved != expected:
        raise QaError(f"resolved catalog URL must equal {expected}")
    capture = (
        resolved
        if not base.query
        else urlunsplit(
            (catalog.scheme, catalog.netloc, catalog.path, base.query, catalog.fragment)
        )
    )
    return CatalogNavigation(resolved_no_query=resolved, capture_url=capture)


def _obj(value: JsonValue, label: str) -> JsonObject:
    if not isinstance(value, dict):
        raise QaError(f"{label} must be an object") from TypeError(label)
    return value


def _arr(value: JsonValue, label: str) -> list[JsonValue]:
    if not isinstance(value, list):
        raise QaError(f"{label} must be an array") from TypeError(label)
    return value


def _keys(value: JsonObject, expected: set[str], label: str) -> JsonObject:
    if set(value) != expected:
        raise QaError(f"{label} keys must equal {sorted(expected)}")
    return value


def _string(value: JsonValue, label: str) -> str:
    if not isinstance(value, str):
        raise QaError(f"{label} must be a string") from TypeError(label)
    return value


def _integer(value: JsonValue, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise QaError(f"{label} must be an integer") from TypeError(label)
    return value


def _hash(value: JsonValue, label: str) -> str:
    text = _string(value, label)
    if HASH_RE.fullmatch(text) is None:
        raise QaError(f"{label} must be a lowercase sha256")
    return text


def validate_report_schema(report: JsonObject) -> None:
    _keys(report, {"schemaVersion", "capturedAt", "source", "viewports"}, "report")
    if (
        report["schemaVersion"] != 1
        or CAPTURED_AT_RE.fullmatch(_string(report["capturedAt"], "capturedAt")) is None
    ):
        raise QaError("report version or capturedAt is invalid")
    source = _keys(
        _obj(report["source"], "source"),
        {"gitSha", "manifestSha256", "catalogHtmlSha256"},
        "source",
    )
    if GIT_SHA_RE.fullmatch(_string(source["gitSha"], "source.gitSha")) is None:
        raise QaError("source.gitSha must be a lowercase git SHA")
    _hash(source["manifestSha256"], "source.manifestSha256")
    _hash(source["catalogHtmlSha256"], "source.catalogHtmlSha256")
    viewports = _arr(report["viewports"], "viewports")
    if len(viewports) != len(VIEWPORT_WIDTHS):
        raise QaError("viewports must contain exactly three entries")
    for index, width in enumerate(VIEWPORT_WIDTHS):
        _validate_viewport(viewports[index], index, width)


def _validate_viewport(value: JsonValue, index: int, width: int) -> None:
    label = f"viewports[{index}]"
    viewport = _keys(
        _obj(value, label),
        {"width", "height", "screenshots", "landing", "catalog", "errors"},
        label,
    )
    if (
        _integer(viewport["width"], f"{label}.width") != width
        or _integer(viewport["height"], f"{label}.height") != VIEWPORT_HEIGHT
    ):
        raise QaError("viewport dimensions are invalid")
    screenshots = _keys(
        _obj(viewport["screenshots"], f"{label}.screenshots"),
        {"landing", "catalogTop", "catalogRecords"},
        f"{label}.screenshots",
    )
    for key, path in {
        "landing": f"landing-{width}.png",
        "catalogTop": f"catalog-top-{width}.png",
        "catalogRecords": f"catalog-records-{width}.png",
    }.items():
        shot = _keys(
            _obj(screenshots[key], f"{label}.screenshots.{key}"),
            {"path", "sha256"},
            f"{label}.screenshots.{key}",
        )
        if _string(shot["path"], f"{label}.screenshots.{key}.path") != path:
            raise QaError(f"unexpected screenshot path: {path}")
        _hash(shot["sha256"], f"{label}.screenshots.{key}.sha256")
    landing = _keys(
        _obj(viewport["landing"], f"{label}.landing"),
        {
            "status",
            "resolvedCatalogHref",
            "headingVisible",
            "linkVisible",
            "scopeVisible",
            "scrollWidth",
            "clientWidth",
        },
        f"{label}.landing",
    )
    catalog_keys = {
        "status",
        "canonical",
        "sourceSha256",
        "featureCount",
        "familyCount",
        "uniqueFeatureCount",
        "currentDispositionCount",
        "targetDispositionCount",
        "exactCurrentCount",
        "tierCounts",
        "unavailableSourceCount",
        "warningsBeforeFirstRecord",
        "recordStartVisible",
        "scrollWidth",
        "clientWidth",
    }
    catalog = _keys(
        _obj(viewport["catalog"], f"{label}.catalog"), catalog_keys, f"{label}.catalog"
    )
    errors = _keys(
        _obj(viewport["errors"], f"{label}.errors"),
        {"console", "page", "failedRequests", "non2xxResponses"},
        f"{label}.errors",
    )
    _validate_leaf_types(landing, catalog, errors, label, catalog_keys)


def _validate_leaf_types(
    landing: JsonObject,
    catalog: JsonObject,
    errors: JsonObject,
    label: str,
    catalog_keys: set[str],
) -> None:
    for key in ("status", "scrollWidth", "clientWidth"):
        _integer(landing[key], f"{label}.landing.{key}")
    _string(landing["resolvedCatalogHref"], f"{label}.landing.resolvedCatalogHref")
    if not all(
        isinstance(landing[key], bool)
        for key in ("headingVisible", "linkVisible", "scopeVisible")
    ):
        raise QaError(f"{label}.landing visibility values must be booleans")
    _string(catalog["canonical"], f"{label}.catalog.canonical")
    _hash(catalog["sourceSha256"], f"{label}.catalog.sourceSha256")
    if not all(
        isinstance(catalog[key], bool)
        for key in ("warningsBeforeFirstRecord", "recordStartVisible")
    ):
        raise QaError(f"{label}.catalog visibility values must be booleans")
    for key in catalog_keys - {
        "canonical",
        "sourceSha256",
        "tierCounts",
        "warningsBeforeFirstRecord",
        "recordStartVisible",
    }:
        _integer(catalog[key], f"{label}.catalog.{key}")
    for tier, value in _keys(
        _obj(catalog["tierCounts"], f"{label}.catalog.tierCounts"),
        set(TIERS),
        f"{label}.catalog.tierCounts",
    ).items():
        _integer(value, f"{label}.catalog.tierCounts.{tier}")
    for key, value in errors.items():
        if not all(
            isinstance(item, str) for item in _arr(value, f"{label}.errors.{key}")
        ):
            raise QaError(f"{label}.errors.{key} must contain only strings")


def load_manifest_stats(path: Path) -> ManifestStats:
    _regular_file(path, "manifest")
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except UnicodeDecodeError as error:
        raise QaError(f"manifest must be valid UTF-8: {path}") from error
    except json.JSONDecodeError as error:
        raise QaError(f"manifest must be valid JSON: {path}") from error
    except OSError as error:
        raise QaError(f"failed to read manifest: {path}") from error
    if not isinstance(payload, dict) or not isinstance(payload.get("features"), list):
        raise QaError("manifest must contain a features array") from TypeError(
            "manifest features"
        )
    tier_counts = {tier: 0 for tier in TIERS}
    families: set[str] = set()
    unavailable = 0
    for feature in payload["features"]:
        if not isinstance(feature, dict) or not isinstance(feature.get("family"), str):
            raise QaError(
                "manifest features must be objects with family strings"
            ) from TypeError("manifest feature")
        families.add(feature["family"])
        unavailable += 1 if feature.get("source_status") == "unavailable" else 0
        current, target = feature.get("current"), feature.get("target")
        if not isinstance(current, dict) or not isinstance(target, dict):
            raise QaError("manifest dispositions must be objects") from TypeError(
                "manifest dispositions"
            )
        for dimension in DIMENSIONS:
            current_cell, target_cell = current.get(dimension), target.get(dimension)
            if not isinstance(current_cell, dict) or not isinstance(target_cell, dict):
                raise QaError(
                    "manifest disposition cells must be objects"
                ) from TypeError("manifest disposition cells")
            tier = current_cell.get("tier")
            if not isinstance(tier, str) or tier not in tier_counts:
                raise QaError("manifest current tier is invalid") from TypeError(
                    "manifest tier"
                )
            tier_counts[tier] += 1
    count = len(payload["features"])
    return ManifestStats(
        sha256_file(path),
        count,
        len(families),
        count * len(DIMENSIONS),
        count * len(DIMENSIONS),
        tier_counts["exact"],
        tier_counts,
        unavailable,
    )


def _dom_widths(page) -> JsonObject:
    return page.evaluate(
        "() => ({scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth})"
    )


def _catalog_dom(page) -> JsonObject:
    return page.evaluate(
        """
        () => {
          const main = document.querySelector('#capabilityCatalog')
          const first = document.querySelector('article[data-capability-id]')
          const hero = document.querySelector('.hero-copy')
          const note = document.querySelector('.catalog-note')
          const heroText = (hero?.textContent || '').toLowerCase()
          const noteText = (note?.textContent || '').toLowerCase()
          const ids = [...document.querySelectorAll('article[data-capability-id]')]
            .map(node => node.getAttribute('data-capability-id'))
          const tiers = {}
          for (const item of document.querySelectorAll('[data-tier-count]')) {
            tiers[item.getAttribute('data-tier')] = Number(item.getAttribute('data-tier-count'))
          }
          const before = node => Boolean(
            node && first && (node.compareDocumentPosition(first) & Node.DOCUMENT_POSITION_FOLLOWING)
          )
          return {
            canonical: document.querySelector('link[rel="canonical"]').getAttribute('href'),
            sourceSha256: main.getAttribute('data-source-sha256'),
            featureCount: Number(main.getAttribute('data-feature-count')),
            familyCount: document.querySelectorAll('section[data-capability-family]').length,
            uniqueFeatureCount: new Set(ids).size,
            currentDispositionCount: document.querySelectorAll('td[data-disposition="current"]').length,
            targetDispositionCount: document.querySelectorAll('td[data-disposition="target"]').length,
            exactCurrentCount: document.querySelectorAll('td[data-disposition="current"][data-tier="exact"]').length,
            tierCounts: tiers,
            unavailableSourceCount: document.querySelectorAll('article[data-source-status="unavailable"]').length,
            warningsBeforeFirstRecord: before(hero)
              && before(note)
              && heroText.includes('exact')
              && heroText.includes('fallback')
              && noteText.includes('cross-validation'),
          }
        }
        """
    )


def _new_qa_page(browser, width: int):
    return browser.new_page(
        viewport={"width": width, "height": VIEWPORT_HEIGHT}, reduced_motion="reduce"
    )


def _fully_visible_in_viewport(
    box: JsonValue, width: int, height: int, top_edge: int | float
) -> bool:
    if not isinstance(box, dict):
        return False
    x, y, box_width, box_height = (
        box.get("x"),
        box.get("y"),
        box.get("width"),
        box.get("height"),
    )
    if not all(
        isinstance(value, int | float) and not isinstance(value, bool)
        for value in (x, y, box_width, box_height)
    ):
        return False
    return (
        box_width > 0
        and box_height > 0
        and x >= 0
        and x + box_width <= width
        and y >= top_edge
        and y + box_height <= height
    )


def _box_bottom(box: JsonValue) -> int | float | None:
    if not isinstance(box, dict):
        return None
    y, height = box.get("y"), box.get("height")
    if not all(
        isinstance(value, int | float) and not isinstance(value, bool)
        for value in (y, height)
    ):
        return None
    if height <= 0:
        return None
    return y + height


def _align_capture_below_topbar(page, selector: str) -> None:
    aligned = page.evaluate(
        """
        (selector) => {
          const target = document.querySelector(selector);
          const topbar = document.querySelector('.topbar');
          if (!target || !topbar) return false;
          const gap = 16;
          const delta = target.getBoundingClientRect().top
            - topbar.getBoundingClientRect().bottom
            - gap;
          window.scrollBy(0, delta);
          return true;
        }
        """,
        selector,
    )
    if aligned is not True:
        raise QaError(f"capture landmark is missing: {selector}")


def _starts_in_viewport(
    box: JsonValue, width: int, height: int, top_edge: int | float
) -> bool:
    if not isinstance(box, dict):
        return False
    x, y, box_width, box_height = (
        box.get("x"),
        box.get("y"),
        box.get("width"),
        box.get("height"),
    )
    if not all(
        isinstance(value, int | float) and not isinstance(value, bool)
        for value in (x, y, box_width, box_height)
    ):
        return False
    return (
        box_width > 0
        and box_height > 0
        and x >= 0
        and x + box_width <= width
        and y >= top_edge
        and y < height
    )


def _check_viewport(report: JsonObject, stats: ManifestStats) -> None:
    catalog, landing = (
        _obj(report["catalog"], "catalog"),
        _obj(report["landing"], "landing"),
    )
    expected = {
        "sourceSha256": stats.manifest_sha256,
        "featureCount": stats.feature_count,
        "familyCount": stats.family_count,
        "uniqueFeatureCount": stats.feature_count,
        "currentDispositionCount": stats.current_disposition_count,
        "targetDispositionCount": stats.target_disposition_count,
        "exactCurrentCount": stats.exact_current_count,
        "tierCounts": stats.tier_counts,
        "unavailableSourceCount": stats.unavailable_source_count,
    }
    if (
        landing["status"] != 200
        or catalog["status"] != 200
        or catalog["canonical"] != CANONICAL_URL
    ):
        raise QaError("landing or catalog response contract failed")
    if (
        landing["scrollWidth"] != landing["clientWidth"]
        or catalog["scrollWidth"] != catalog["clientWidth"]
    ):
        raise QaError("horizontal overflow detected")
    if (
        not landing["headingVisible"]
        or not landing["linkVisible"]
        or not landing["scopeVisible"]
        or not catalog["warningsBeforeFirstRecord"]
        or not catalog["recordStartVisible"]
    ):
        raise QaError("required page landmark is not visible or ordered")
    if any(
        _arr(value, f"errors.{key}")
        for key, value in _obj(report["errors"], "errors").items()
    ):
        raise QaError("browser emitted console, page, request, or HTTP errors")
    for key, value in expected.items():
        if catalog[key] != value:
            raise QaError(f"catalog DOM mismatch for {key}: {catalog[key]} != {value}")


def _verify_catalog_response(
    response: CatalogResponse | None, expected_sha256: str
) -> None:
    if response is None:
        raise QaError("catalog navigation produced no response") from RuntimeError(
            "main resource response is unavailable"
        )
    try:
        body = response.body()
    except RuntimeError as error:
        raise QaError("failed to read catalog response body") from error
    if not isinstance(body, bytes):
        raise QaError("catalog response body must be bytes") from TypeError(
            type(body).__name__
        )
    actual_sha256 = hashlib.sha256(body).hexdigest()
    if actual_sha256 != expected_sha256:
        raise QaError(
            "catalog response body does not match --catalog-html"
        ) from ValueError(actual_sha256)


def _capture_viewport(browser, context: RuntimeContext, width: int) -> JsonObject:
    page = _new_qa_page(browser, width)
    try:
        errors: JsonObject = {
            "console": [],
            "page": [],
            "failedRequests": [],
            "non2xxResponses": [],
        }
        page.on(
            "console",
            lambda message: (
                errors["console"].append(message.text)
                if message.type == "error"
                else None
            ),
        )
        page.on("pageerror", lambda error: errors["page"].append(str(error)))
        page.on(
            "requestfailed",
            lambda request: errors["failedRequests"].append(
                f"{request.method} {request.url}"
            ),
        )
        page.on(
            "response",
            lambda response: (
                errors["non2xxResponses"].append(f"{response.status} {response.url}")
                if response.status < 200 or response.status >= 300
                else None
            ),
        )
        landing_response = page.goto(context.base_url, wait_until="load")
        if landing_response is None:
            raise QaError("landing navigation produced no response")
        coverage, heading, link, scope, topbar = (
            page.locator("#coverage"),
            page.locator("#coverageHeading"),
            page.locator("#capabilityCatalogLink"),
            page.locator("#coverage .section-note"),
            page.locator(".topbar"),
        )
        coverage.scroll_into_view_if_needed()
        heading.scroll_into_view_if_needed()
        link.scroll_into_view_if_needed()
        _align_capture_below_topbar(page, "#coverageHeading")
        landing_widths = _dom_widths(page)
        top_edge = _box_bottom(topbar.bounding_box())
        if top_edge is None:
            raise QaError("sticky header bounds are unavailable")
        heading_visible, link_visible, scope_visible = (
            _fully_visible_in_viewport(
                heading.bounding_box(), width, VIEWPORT_HEIGHT, top_edge
            ),
            _fully_visible_in_viewport(
                link.bounding_box(), width, VIEWPORT_HEIGHT, top_edge
            ),
            _fully_visible_in_viewport(
                scope.bounding_box(), width, VIEWPORT_HEIGHT, top_edge
            ),
        )
        landing_name = f"landing-{width}.png"
        landing_image = page.screenshot(full_page=False)
        if not isinstance(landing_image, bytes):
            raise QaError("browser screenshot did not return bytes")
        context.evidence.write_bytes(landing_name, landing_image)
        link.click()
        page.wait_for_load_state("load")
        navigation = catalog_navigation_urls(context.base_url, page.url)
        catalog_response = page.goto(navigation.capture_url, wait_until="load")
        _verify_catalog_response(catalog_response, context.catalog_html_sha256)
        catalog_widths = _dom_widths(page)
        catalog_name, records_name = (
            f"catalog-top-{width}.png",
            f"catalog-records-{width}.png",
        )
        catalog_image = page.screenshot(full_page=False)
        if not isinstance(catalog_image, bytes):
            raise QaError("browser screenshot did not return bytes")
        context.evidence.write_bytes(catalog_name, catalog_image)
        record = page.locator("#capability-presentation")
        record.scroll_into_view_if_needed()
        _align_capture_below_topbar(page, "#capability-presentation")
        catalog_top_edge = _box_bottom(page.locator(".topbar").bounding_box())
        if catalog_top_edge is None:
            raise QaError("catalog sticky header bounds are unavailable")
        record_start_visible = _starts_in_viewport(
            record.bounding_box(), width, VIEWPORT_HEIGHT, catalog_top_edge
        )
        records_image = page.screenshot(full_page=False)
        if not isinstance(records_image, bytes):
            raise QaError("browser screenshot did not return bytes")
        context.evidence.write_bytes(records_name, records_image)
        catalog = _catalog_dom(page) | {
            "status": catalog_response.status,
            "recordStartVisible": record_start_visible,
            "scrollWidth": catalog_widths["scrollWidth"],
            "clientWidth": catalog_widths["clientWidth"],
        }
        report = {
            "width": width,
            "height": VIEWPORT_HEIGHT,
            "screenshots": {
                "landing": _shot(context.evidence, landing_name),
                "catalogTop": _shot(context.evidence, catalog_name),
                "catalogRecords": _shot(context.evidence, records_name),
            },
            "landing": {
                "status": landing_response.status,
                "resolvedCatalogHref": navigation.resolved_no_query,
                "headingVisible": heading_visible,
                "linkVisible": link_visible,
                "scopeVisible": scope_visible,
                "scrollWidth": landing_widths["scrollWidth"],
                "clientWidth": landing_widths["clientWidth"],
            },
            "catalog": catalog,
            "errors": errors,
        }
        _check_viewport(report, context.stats)
        return report
    except RuntimeError as error:
        raise QaError("browser QA runtime failed") from error
    finally:
        page.close()


def _shot(evidence: EvidenceDirectory, name: str) -> JsonObject:
    return {"path": name, "sha256": evidence.sha256(name)}


def preflight_inputs(
    args: CliArgs, repo_root: Path = REPO_ROOT
) -> tuple[ManifestStats, JsonObject]:
    stats = load_manifest_stats(args.manifest)
    source = {
        "gitSha": args.git_sha,
        "manifestSha256": stats.manifest_sha256,
        "catalogHtmlSha256": sha256_file(args.catalog_html),
    }
    assert_clean_git_binding(args.git_sha, repo_root)
    return stats, source


def _expected_output_hashes(report: JsonObject, report_bytes: bytes) -> dict[str, str]:
    hashes: dict[str, str] = {}
    for index, viewport_value in enumerate(_arr(report["viewports"], "viewports")):
        screenshots = _obj(
            _obj(viewport_value, f"viewports[{index}]")["screenshots"],
            f"viewports[{index}].screenshots",
        )
        for label, shot_value in screenshots.items():
            shot = _obj(shot_value, f"viewports[{index}].screenshots.{label}")
            hashes[_string(shot["path"], f"{label}.path")] = _hash(
                shot["sha256"], f"{label}.sha256"
            )
    hashes["browser-qa.json"] = hashlib.sha256(report_bytes).hexdigest()
    return hashes


def _validate_output_integrity(
    evidence: EvidenceDirectory, expected_hashes: dict[str, str]
) -> None:
    expected_names = {
        name
        for name in OWNED_EVIDENCE_NAMES
        if name.endswith(".png") or name == "browser-qa.json"
    }
    if set(expected_hashes) != expected_names:
        raise QaError("browser QA output set is incomplete")
    evidence.validate_public_path()
    for name, expected_hash in expected_hashes.items():
        if evidence.sha256(name) != expected_hash:
            raise QaError(f"browser QA output hash mismatch: {name}")
    evidence.validate_public_path()


def run_browser_qa(args: CliArgs, repo_root: Path = REPO_ROOT) -> JsonObject:
    stats, source = preflight_inputs(args, repo_root)
    with _prepared_evidence_directory(args.evidence_dir, repo_root) as evidence:
        try:
            from playwright.sync_api import Error as PlaywrightError, sync_playwright
        except ImportError as error:
            raise QaError("Python Playwright is required for browser QA") from error
        try:
            with sync_playwright() as playwright:
                browser = playwright.chromium.launch(channel="chrome", headless=True)
                try:
                    context = RuntimeContext(
                        args.base_url,
                        evidence,
                        stats,
                        _hash(source["catalogHtmlSha256"], "source.catalogHtmlSha256"),
                    )
                    viewports = [
                        _capture_viewport(browser, context, width)
                        for width in VIEWPORT_WIDTHS
                    ]
                finally:
                    browser.close()
        except PlaywrightError as error:
            raise QaError("browser QA runtime failed") from error
        report = {
            "schemaVersion": 1,
            "capturedAt": datetime.now(UTC)
            .isoformat(timespec="milliseconds")
            .replace("+00:00", "Z"),
            "source": source,
            "viewports": viewports,
        }
        validate_report_schema(report)
        report_bytes = (json.dumps(report, indent=2, sort_keys=True) + "\n").encode(
            "utf-8"
        )
        evidence.write_bytes("browser-qa.json", report_bytes)
        _validate_output_integrity(
            evidence, _expected_output_hashes(report, report_bytes)
        )
        return report


def parse_args(argv: list[str] | None = None) -> CliArgs:
    parser = argparse.ArgumentParser(
        description="Capture GitHub Pages capability-catalog browser QA evidence."
    )
    for option in (
        "--base-url",
        "--manifest",
        "--catalog-html",
        "--evidence-dir",
        "--git-sha",
    ):
        parser.add_argument(option, required=True)
    parsed = parser.parse_args(argv)
    return CliArgs(
        parsed.base_url,
        Path(parsed.manifest),
        Path(parsed.catalog_html),
        Path(parsed.evidence_dir),
        parsed.git_sha,
    )


def main(argv: list[str] | None = None) -> int:
    logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
    try:
        run_browser_qa(parse_args(argv))
    except QaError as error:
        logger.error("%s", error)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
