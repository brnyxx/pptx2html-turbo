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
import io
import json
import re
import sys
import zipfile
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Final, TypeAlias
from xml.etree import ElementTree


UNKNOWN_ADJUSTMENT_KEY: Final = "UNKNOWN_ADJUSTMENT_KEY"
OFFICIAL_PRESET_INVENTORY_MISMATCH: Final = "OFFICIAL_PRESET_INVENTORY_MISMATCH"
OFFICIAL_ARTIFACT_CHECKSUM_MISMATCH: Final = "OFFICIAL_ARTIFACT_CHECKSUM_MISMATCH"
OFFICIAL_NAMES_SHA256: Final = (
    "f2c3bdcda8569b358ce3196cfeb183849e33bfc7955fac961dc85fceb6b3b587"
)
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


def _inventory_digest(names: Sequence[str]) -> str:
    return hashlib.sha256(("\n".join(names) + "\n").encode()).hexdigest()


def _read_text(path: Path, *, label: str) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError as error:
        raise ContractError(f"{label}_NOT_FOUND", f"path={path}") from error
    except (IsADirectoryError, PermissionError, OSError) as error:
        raise ContractError(f"{label}_UNREADABLE", f"path={path}") from error


def _load_manifest(path: Path) -> dict[str, JsonValue]:
    try:
        payload = json.loads(_read_text(path, label="MANIFEST"))
    except json.JSONDecodeError as error:
        raise ContractError(
            "MALFORMED_MANIFEST", f"path={path} line={error.lineno}"
        ) from error
    if not isinstance(payload, dict):
        raise ContractError("MALFORMED_MANIFEST", "root must be a JSON object")
    return payload


def _manifest_inventory(
    manifest: dict[str, JsonValue],
) -> tuple[list[str], dict[str, dict[str, JsonValue]]]:
    raw_names = manifest.get("official_preset_names")
    rows = manifest.get("presets")
    if not isinstance(raw_names, list) or not all(
        isinstance(name, str) for name in raw_names
    ):
        raise ContractError(
            OFFICIAL_PRESET_INVENTORY_MISMATCH, "official_preset_names is invalid"
        )
    names = [name for name in raw_names if isinstance(name, str)]
    recorded_digest = manifest.get("official_preset_names_sha256")
    if (
        recorded_digest != OFFICIAL_NAMES_SHA256
        or len(names) != len(set(names))
        or _inventory_digest(names) != OFFICIAL_NAMES_SHA256
    ):
        raise ContractError(
            OFFICIAL_PRESET_INVENTORY_MISMATCH, "official_preset_names digest mismatch"
        )
    if not isinstance(rows, list):
        raise ContractError(
            OFFICIAL_PRESET_INVENTORY_MISMATCH, "presets must be an array"
        )
    typed_rows = [
        row
        for row in rows
        if isinstance(row, dict) and isinstance(row.get("name"), str)
    ]
    by_name = {str(row["name"]): row for row in typed_rows}
    if (
        len(typed_rows) != len(rows)
        or len(by_name) != len(typed_rows)
        or set(by_name) != set(names)
    ):
        missing = sorted(set(names) - set(by_name))
        extra = sorted(set(by_name) - set(names))
        raise ContractError(
            OFFICIAL_PRESET_INVENTORY_MISMATCH, f"rows missing={missing} extra={extra}"
        )
    return names, by_name


def _starts_get_argument(output: list[str], quote_index: int) -> bool:
    cursor = quote_index - 1
    while cursor >= 0 and output[cursor].isspace():
        cursor -= 1
    if cursor < 0 or output[cursor] != "(":
        return False
    cursor -= 1
    while cursor >= 0 and output[cursor].isspace():
        cursor -= 1
    end = cursor + 1
    while cursor >= 0 and (output[cursor].isalnum() or output[cursor] == "_"):
        cursor -= 1
    return (
        cursor >= 0
        and output[cursor] == "."
        and "".join(output[cursor + 1 : end]) == "get"
    )


def _lexical_source(source: str) -> str:
    output = list(source)
    index = 0
    while index < len(source):
        if source.startswith("//", index):
            end = source.find("\n", index)
            end = len(source) if end < 0 else end
            output[index:end] = " " * (end - index)
            index = end
        elif source.startswith("/*", index):
            depth, end = 1, index + 2
            while end < len(source) and depth:
                if source.startswith("/*", end):
                    depth, end = depth + 1, end + 2
                elif source.startswith("*/", end):
                    depth, end = depth - 1, end + 2
                else:
                    end += 1
            output[index:end] = " " * (end - index)
            index = end
        elif source[index] == '"':
            end = index + 1
            while end < len(source):
                if source[end] == "\\":
                    end += 2
                elif source[end] == '"':
                    end += 1
                    break
                else:
                    end += 1
            if not _starts_get_argument(output, index):
                output[index:end] = " " * (end - index)
            index = end
        else:
            index += 1
    return "".join(output)


def _balanced_body(source: str, opening_brace: int) -> str:
    depth = 0
    for index in range(opening_brace, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[opening_brace + 1 : index]
    raise ContractError("INVALID_SOURCE_ROOT", "unbalanced Rust function body")


def _dispatcher_routes(dispatcher: Path) -> list[Route]:
    try:
        source = dispatcher.read_text(encoding="utf-8")
    except (FileNotFoundError, IsADirectoryError, PermissionError, OSError) as error:
        raise ContractError("INVALID_DISPATCHER", f"path={dispatcher}") from error
    try:
        block = source[
            source.index("pub fn preset_shape_svg(") : source.index(
                "pub fn preset_shape_multi_svg("
            )
        ]
    except ValueError as error:
        raise ContractError(
            "INVALID_DISPATCHER", f"preset dispatcher not found in {dispatcher}"
        ) from error
    pattern = re.compile(
        r'(?P<names>"[A-Za-z0-9]+"(?:\s*\|\s*"[A-Za-z0-9]+")*)\s*=>\s*'
        r"(?:\{\s*)?Some\(\s*(?P<module>[a-z_][a-z0-9_]*)::(?P<function>[a-z_][a-z0-9_]*)\s*\("
    )
    return [
        (name, match.group("module"), match.group("function"))
        for match in pattern.finditer(block)
        for name in re.findall(r'"([A-Za-z0-9]+)"', match.group("names"))
    ]


def _function_bodies(source_root: Path) -> dict[tuple[str, str], str]:
    if not source_root.is_dir():
        raise ContractError("INVALID_SOURCE_ROOT", f"path={source_root}")
    functions: dict[tuple[str, str], str] = {}
    signature = re.compile(
        r"(?:^|\n)\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+(?P<name>[a-z_][a-z0-9_]*)\s*\("
    )
    for path in sorted(source_root.glob("*.rs")):
        source = _lexical_source(_read_text(path, label="SOURCE"))
        for match in signature.finditer(source):
            opening_brace = source.find("{", match.end())
            if opening_brace >= 0:
                functions[(path.stem, match.group("name"))] = _balanced_body(
                    source, opening_brace
                )
    return functions


def _direct_keys(body: str) -> set[str]:
    return set(re.findall(r'\.get\s*\(\s*"([A-Za-z0-9_]+)"\s*\)', body))


def _consumed_keys(
    function_id: tuple[str, str],
    functions: dict[tuple[str, str], str],
    visited: set[tuple[str, str]] | None = None,
) -> set[str]:
    visited = set() if visited is None else visited
    if function_id in visited or function_id not in functions:
        return set()
    visited.add(function_id)
    module, _ = function_id
    body = functions[function_id]
    keys = _direct_keys(body)
    for called in re.findall(r"(?<!::)\b([a-z_][a-z0-9_]*)\s*\(", body):
        keys.update(_consumed_keys((module, called), functions, visited))
    for called_module, called in re.findall(
        r"\b([a-z_][a-z0-9_]*)::([a-z_][a-z0-9_]*)\s*\(", body
    ):
        keys.update(_consumed_keys((called_module, called), functions, visited))
    return keys


def _row_keys(row: dict[str, JsonValue], field: str) -> set[str]:
    container = row.get(field)
    if field == "preservation" and isinstance(container, dict):
        container = container.get("observed_consumed_keys")
        return (
            {str(key) for key in container if isinstance(key, str)}
            if isinstance(container, list)
            else set()
        )
    if not isinstance(container, list):
        raise ContractError(
            "MALFORMED_MANIFEST", f"{row.get('name')}.{field} must be an array"
        )
    return {
        str(item["name"])
        for item in container
        if isinstance(item, dict) and isinstance(item.get("name"), str)
    }


def check_repository(
    repo_root: Path,
    *,
    manifest_path: Path | None = None,
    source_root: Path | None = None,
    dispatcher_path: Path | None = None,
) -> CheckReport:
    manifest = _load_manifest(manifest_path or repo_root / DEFAULT_MANIFEST)
    names, rows = _manifest_inventory(manifest)
    routes = _dispatcher_routes(dispatcher_path or repo_root / DEFAULT_DISPATCHER)
    route_by_name = {route[0]: route for route in routes}
    aliases = manifest.get("dispatcher_aliases")
    if not isinstance(aliases, dict) or not all(
        isinstance(alias, str) and isinstance(target, str) and target in names
        for alias, target in aliases.items()
    ):
        raise ContractError(
            OFFICIAL_PRESET_INVENTORY_MISMATCH, "dispatcher aliases are invalid"
        )
    alias_names = set(aliases)
    dispatch_names = set(route_by_name)
    if set(names) - dispatch_names or dispatch_names - set(names) != alias_names:
        raise ContractError(
            OFFICIAL_PRESET_INVENTORY_MISMATCH, "dispatcher missing/extra names"
        )
    functions = _function_bodies(source_root or repo_root / DEFAULT_SOURCE_ROOT)
    consumed: set[tuple[str, str]] = set()
    unknown: list[dict[str, str]] = []
    preserved_count = 0
    for preset in names:
        _, module, function = route_by_name[preset]
        official = _row_keys(rows[preset], "adjustments")
        preserved = _row_keys(rows[preset], "preservation")
        for key in sorted(_consumed_keys((module, function), functions)):
            consumed.add((preset, key))
            if key in preserved and key not in official:
                preserved_count += 1
            elif key not in official:
                unknown.append(
                    {
                        "code": UNKNOWN_ADJUSTMENT_KEY,
                        "preset": preset,
                        "family": module,
                        "key": key,
                    }
                )
    official_pairs = {
        (preset, key)
        for preset, row in rows.items()
        for key in _row_keys(row, "adjustments")
    }
    known = {key for _, key in official_pairs}
    known.update(key for row in rows.values() for key in _row_keys(row, "preservation"))
    reported = {(item["family"], item["key"]) for item in unknown}
    for (module, _), body in functions.items():
        for key in _direct_keys(body) - known:
            if (module, key) not in reported:
                unknown.append(
                    {
                        "code": UNKNOWN_ADJUSTMENT_KEY,
                        "preset": "<unrouted>",
                        "family": module,
                        "key": key,
                    }
                )
    return {
        "ok": not unknown,
        "presets": len(names),
        "unclassified_presets": 0,
        "unclassified_preset_names": [],
        "unknown_consumed_keys": len(unknown),
        "unknown_consumed_key_details": unknown,
        "non_official_consumed_keys_preserved": preserved_count,
        "manifest_keys_never_consumed": len(official_pairs - consumed),
    }


def verify_official_artifact(path: Path, expected_sha256: str) -> str:
    try:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
    except FileNotFoundError as error:
        raise ContractError("OFFICIAL_ARTIFACT_NOT_FOUND", f"path={path}") from error
    except (IsADirectoryError, PermissionError, OSError) as error:
        raise ContractError("OFFICIAL_ARTIFACT_UNREADABLE", f"path={path}") from error
    if digest != expected_sha256:
        raise ContractError(
            OFFICIAL_ARTIFACT_CHECKSUM_MISMATCH,
            f"expected={expected_sha256} actual={digest}",
        )
    return digest


def _verify_official_package(
    path: Path, manifest: dict[str, JsonValue], names: list[str]
) -> None:
    artifact = manifest.get("official_artifact")
    if not isinstance(artifact, dict):
        raise ContractError("MALFORMED_MANIFEST", "official_artifact is required")
    outer_sha, member_sha = (
        artifact.get("sha256"),
        artifact.get("inventory_member_sha256"),
    )
    if not isinstance(outer_sha, str) or not isinstance(member_sha, str):
        raise ContractError(
            "MALFORMED_MANIFEST", "official artifact checksums are required"
        )
    verify_official_artifact(path, outer_sha)
    try:
        with zipfile.ZipFile(path) as outer:
            nested_bytes = outer.read("OfficeOpenXML-XMLSchema-Strict.zip")
        with zipfile.ZipFile(io.BytesIO(nested_bytes)) as nested:
            xsd = nested.read("dml-main.xsd")
        root = ElementTree.fromstring(xsd)
    except (zipfile.BadZipFile, KeyError, ElementTree.ParseError) as error:
        raise ContractError("OFFICIAL_ARTIFACT_INVALID", f"path={path}") from error
    actual_member_sha = hashlib.sha256(xsd).hexdigest()
    if actual_member_sha != member_sha:
        raise ContractError(
            OFFICIAL_ARTIFACT_CHECKSUM_MISMATCH,
            f"nested expected={member_sha} actual={actual_member_sha}",
        )
    namespace = {"x": "http://www.w3.org/2001/XMLSchema"}
    shape_type = next(
        (
            item
            for item in root.findall("x:simpleType", namespace)
            if item.get("name") == "ST_ShapeType"
        ),
        None,
    )
    artifact_names = (
        []
        if shape_type is None
        else [
            item.get("value", "")
            for item in shape_type.findall(".//x:enumeration", namespace)
        ]
    )
    if (
        artifact_names != names
        or _inventory_digest(artifact_names) != OFFICIAL_NAMES_SHA256
    ):
        raise ContractError(
            OFFICIAL_PRESET_INVENTORY_MISMATCH, "artifact ST_ShapeType mismatch"
        )


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--dispatcher", type=Path)
    parser.add_argument("--json", type=Path, dest="json_output")
    parser.add_argument("--official-artifact", type=Path)
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    repo_root = args.repo_root.resolve()
    manifest_path = (
        args.manifest.resolve() if args.manifest else repo_root / DEFAULT_MANIFEST
    )
    dispatcher_path = (
        args.dispatcher.resolve() if args.dispatcher else repo_root / DEFAULT_DISPATCHER
    )
    source_root = (
        args.source_root.resolve()
        if args.source_root
        else repo_root / DEFAULT_SOURCE_ROOT
    )
    manifest = _load_manifest(manifest_path)
    names, _ = _manifest_inventory(manifest)
    report = check_repository(
        repo_root,
        manifest_path=manifest_path,
        source_root=source_root,
        dispatcher_path=dispatcher_path,
    )
    if args.official_artifact:
        _verify_official_package(args.official_artifact, manifest, names)
    if args.json_output:
        try:
            args.json_output.parent.mkdir(parents=True, exist_ok=True)
            args.json_output.write_text(
                json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
            )
        except OSError as error:
            raise ContractError(
                "JSON_OUTPUT_UNWRITABLE", f"path={args.json_output}"
            ) from error
    summary_keys = (
        "presets",
        "unclassified_presets",
        "unknown_consumed_keys",
        "manifest_keys_never_consumed",
    )
    sys.stdout.write(" ".join(f"{key}={report[key]}" for key in summary_keys) + "\n")
    for detail in report["unknown_consumed_key_details"]:
        sys.stderr.write(
            f"{detail['code']}: preset={detail['preset']} "
            f"family={detail['family']} key={detail['key']}\n"
        )
    return 0 if report["ok"] else 1


def cli() -> int:
    try:
        return main()
    except ContractError as error:
        sys.stderr.write(f"{error}\n")
        return 1


if __name__ == "__main__":
    raise SystemExit(cli())
