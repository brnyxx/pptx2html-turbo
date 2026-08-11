#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///

# How to run:
#   python3 evaluate/check_preset_adjustments.py --repo-root .

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections.abc import Mapping
from pathlib import Path
from typing import Final, TypeAlias


UNKNOWN_ADJUSTMENT_KEY: Final = "UNKNOWN_ADJUSTMENT_KEY"
MANIFEST_PRESET_MISSING: Final = "MANIFEST_PRESET_MISSING"
OFFICIAL_ARTIFACT_CHECKSUM_MISMATCH: Final = "OFFICIAL_ARTIFACT_CHECKSUM_MISMATCH"
DEFAULT_MANIFEST: Final = Path("evaluate/preset_adjustments.json")
DEFAULT_DISPATCHER: Final = Path("crates/pptx2html-core/src/renderer/geometry.rs")
DEFAULT_SOURCE_ROOT: Final = Path("crates/pptx2html-core/src/renderer/geometry")
JsonValue: TypeAlias = (
    str | int | float | bool | None | list["JsonValue"] | dict[str, "JsonValue"]
)


CheckReport: TypeAlias = Mapping[str, JsonValue]


Route: TypeAlias = tuple[str, str, str]


class ContractError(Exception):
    def __init__(self, code: str, detail: str) -> None:
        super().__init__(f"{code}: {detail}")


def _balanced_body(source: str, opening_brace: int) -> str:
    depth = 0
    for index in range(opening_brace, len(source)):
        character = source[index]
        if character == "{":
            depth += 1
        elif character == "}":
            depth -= 1
            if depth == 0:
                return source[opening_brace + 1 : index]
    return source[opening_brace + 1 :]


def _dispatcher_routes(dispatcher: Path) -> list[Route]:
    source = dispatcher.read_text(encoding="utf-8")
    start = source.index("pub fn preset_shape_svg(")
    end = source.index("pub fn preset_shape_multi_svg(")
    block = source[start:end]
    route_pattern = re.compile(
        r'(?P<names>"[A-Za-z0-9]+"(?:\s*\|\s*"[A-Za-z0-9]+")*)'
        r"\s*=>\s*(?:\{\s*)?Some\(\s*"
        r"(?P<module>[a-z_][a-z0-9_]*)::(?P<function>[a-z_][a-z0-9_]*)\s*\(",
        re.MULTILINE,
    )
    routes: list[Route] = []
    for match in route_pattern.finditer(block):
        names = re.findall(r'"([A-Za-z0-9]+)"', match.group("names"))
        routes.extend(
            (name, match.group("module"), match.group("function")) for name in names
        )
    return routes


def _function_bodies(source_root: Path) -> dict[tuple[str, str], str]:
    functions: dict[tuple[str, str], str] = {}
    signature = re.compile(
        r"(?:^|\n)\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+"
        r"(?P<name>[a-z_][a-z0-9_]*)\s*\(",
    )
    for path in sorted(source_root.glob("*.rs")):
        module = path.stem
        source = path.read_text(encoding="utf-8")
        for match in signature.finditer(source):
            opening_brace = source.find("{", match.end())
            if opening_brace < 0:
                continue
            body = _balanced_body(source, opening_brace)
            functions[(module, match.group("name"))] = body
    return functions


def _consumed_keys(
    function_id: tuple[str, str],
    functions: dict[tuple[str, str], str],
    visited: set[tuple[str, str]] | None = None,
) -> set[str]:
    if visited is None:
        visited = set()
    if function_id in visited or function_id not in functions:
        return set()
    visited.add(function_id)
    module, _ = function_id
    body = functions[function_id]
    keys = set(re.findall(r'\.get\("([A-Za-z0-9_]+)"\)', body))
    local_calls = re.findall(r"(?<!::)\b([a-z_][a-z0-9_]*)\s*\(", body)
    qualified_calls = re.findall(
        r"\b([a-z_][a-z0-9_]*)::([a-z_][a-z0-9_]*)\s*\(",
        body,
    )
    for called in local_calls:
        keys.update(_consumed_keys((module, called), functions, visited))
    for module, called in qualified_calls:
        keys.update(_consumed_keys((module, called), functions, visited))
    return keys


def _load_manifest(path: Path) -> dict[str, JsonValue]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ContractError("INVALID_MANIFEST", "root must be a JSON object")
    return payload


def _official_keys(row: dict[str, JsonValue]) -> set[str]:
    adjustments = row.get("adjustments")
    if not isinstance(adjustments, list):
        raise ContractError("INVALID_MANIFEST", "adjustments must be a JSON array")
    return {
        str(item["name"])
        for item in adjustments
        if isinstance(item, dict) and isinstance(item.get("name"), str)
    }


def _preservation_keys(row: dict[str, JsonValue]) -> set[str]:
    preservation = row.get("preservation")
    if not isinstance(preservation, dict):
        return set()
    observed = preservation.get("observed_consumed_keys")
    return (
        {str(key) for key in observed if isinstance(key, str)}
        if isinstance(observed, list)
        else set()
    )


def check_repository(
    repo_root: Path,
    *,
    manifest_path: Path | None = None,
    source_root: Path | None = None,
) -> CheckReport:
    manifest_file = manifest_path or repo_root / DEFAULT_MANIFEST
    geometry_root = source_root or repo_root / DEFAULT_SOURCE_ROOT
    manifest = _load_manifest(manifest_file)
    raw_rows = manifest.get("presets")
    if not isinstance(raw_rows, list):
        raise ContractError("INVALID_MANIFEST", "presets must be a JSON array")
    rows = [row for row in raw_rows if isinstance(row, dict)]
    by_name = {
        str(row["name"]): row for row in rows if isinstance(row.get("name"), str)
    }
    routes = _dispatcher_routes(repo_root / DEFAULT_DISPATCHER)
    route_by_name = {route[0]: route for route in routes}
    functions = _function_bodies(geometry_root)
    official_names = set(by_name)
    dispatcher_names = set(route_by_name)
    unclassified = sorted(official_names - dispatcher_names)
    consumed_pairs: set[tuple[str, str]] = set()
    unknown: list[dict[str, str]] = []
    preserved_non_official = 0
    for preset in sorted(official_names & dispatcher_names):
        _, module, function = route_by_name[preset]
        keys = _consumed_keys((module, function), functions)
        official = _official_keys(by_name[preset])
        preserved = _preservation_keys(by_name[preset])
        for key in sorted(keys):
            consumed_pairs.add((preset, key))
            if key in preserved and key not in official:
                preserved_non_official += 1
            elif key not in official:
                unknown.append(
                    {
                        "code": UNKNOWN_ADJUSTMENT_KEY,
                        "preset": preset,
                        "family": module,
                        "key": key,
                    }
                )
    manifest_pairs = {
        (preset, key) for preset, row in by_name.items() for key in _official_keys(row)
    }
    known_keys = {key for _, key in manifest_pairs}
    known_keys.update(
        key for row in by_name.values() for key in _preservation_keys(row)
    )
    reported = {(item["family"], item["key"]) for item in unknown}
    for (module, _), body in functions.items():
        for key in set(re.findall(r'\.get\("([A-Za-z0-9_]+)"\)', body)) - known_keys:
            if (module, key) not in reported:
                unknown.append(
                    {
                        "code": UNKNOWN_ADJUSTMENT_KEY,
                        "preset": "<unrouted>",
                        "family": module,
                        "key": key,
                    }
                )
    never_consumed = len(manifest_pairs - consumed_pairs)
    return {
        "ok": not unclassified and not unknown and len(rows) == 187,
        "presets": len(rows),
        "unclassified_presets": len(unclassified),
        "unclassified_preset_names": unclassified,
        "unknown_consumed_keys": len(unknown),
        "unknown_consumed_key_details": unknown,
        "non_official_consumed_keys_preserved": preserved_non_official,
        "manifest_keys_never_consumed": never_consumed,
    }


def verify_official_artifact(path: Path, expected_sha256: str) -> str:
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != expected_sha256:
        detail = f"path={path} expected={expected_sha256} actual={digest}"
        raise ContractError(OFFICIAL_ARTIFACT_CHECKSUM_MISMATCH, detail)
    return digest


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--json", type=Path, dest="json_output")
    parser.add_argument("--official-artifact", type=Path)
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    repo_root = args.repo_root.resolve()
    manifest_path = args.manifest.resolve() if args.manifest else None
    source_root = args.source_root.resolve() if args.source_root else None
    report = check_repository(
        repo_root,
        manifest_path=manifest_path,
        source_root=source_root,
    )
    manifest = _load_manifest(manifest_path or repo_root / DEFAULT_MANIFEST)
    if args.official_artifact:
        artifact = manifest.get("official_artifact")
        if not isinstance(artifact, dict) or not isinstance(
            artifact.get("sha256"), str
        ):
            raise ContractError(
                "INVALID_MANIFEST", "official_artifact.sha256 is required"
            )
        verify_official_artifact(args.official_artifact, artifact["sha256"])
    if args.json_output:
        args.json_output.parent.mkdir(parents=True, exist_ok=True)
        args.json_output.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    summary = " ".join(
        f"{key}={report[key]}"
        for key in (
            "presets",
            "unclassified_presets",
            "unknown_consumed_keys",
            "manifest_keys_never_consumed",
        )
    )
    print(summary)
    for detail in report["unknown_consumed_key_details"]:
        print(
            f"{detail['code']}: preset={detail['preset']} "
            f"family={detail['family']} key={detail['key']}",
            file=sys.stderr,
        )
    for preset in report["unclassified_preset_names"]:
        print(f"{MANIFEST_PRESET_MISSING}: preset={preset}", file=sys.stderr)
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
