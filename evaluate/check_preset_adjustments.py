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
UNPARSEABLE_ADJUSTMENT_LOOKUP: Final = "UNPARSEABLE_ADJUSTMENT_LOOKUP"
OFFICIAL_PRESET_INVENTORY_MISMATCH: Final = "OFFICIAL_PRESET_INVENTORY_MISMATCH"
OFFICIAL_ADJUSTMENT_SEMANTICS_MISMATCH: Final = "OFFICIAL_ADJUSTMENT_SEMANTICS_MISMATCH"
OFFICIAL_ARTIFACT_CHECKSUM_MISMATCH: Final = "OFFICIAL_ARTIFACT_CHECKSUM_MISMATCH"
OFFICIAL_SUPPLEMENT_NOT_FOUND: Final = "OFFICIAL_SUPPLEMENT_NOT_FOUND"
OFFICIAL_SUPPLEMENT_INVALID: Final = "OFFICIAL_SUPPLEMENT_INVALID"
OFFICIAL_SUPPLEMENT_CHECKSUM_MISMATCH: Final = "OFFICIAL_SUPPLEMENT_CHECKSUM_MISMATCH"
OFFICIAL_NAMES_SHA256: Final = (
    "f2c3bdcda8569b358ce3196cfeb183849e33bfc7955fac961dc85fceb6b3b587"
)
DEFAULT_MANIFEST: Final = Path("evaluate/preset_adjustments.json")
DEFAULT_DISPATCHER: Final = Path("crates/pptx2html-core/src/renderer/geometry.rs")
DEFAULT_SOURCE_ROOT: Final = Path("crates/pptx2html-core/src/renderer/geometry")
DEFAULT_SUPPLEMENT: Final = Path("evaluate/official_supplements/upArrow.xml")
DRAWINGML_NAMESPACE: Final = "http://schemas.openxmlformats.org/drawingml/2006/main"
OFFICIAL_ADJUSTMENT_COUNT: Final = 298
OFFICIAL_CONSTRAINT_COUNT: Final = 285
OFFICIAL_TOTAL_ADJUSTMENT_COUNT: Final = 300
OFFICIAL_TOTAL_CONSTRAINT_COUNT: Final = 287
OFFICIAL_SUPPLEMENT_SHA256: Final = (
    "23dc5bfc28c65bbac832c40452b29af40b535679f47c35d22e6db6f1292a48ad"
)
OFFICIAL_SUPPLEMENT_URL: Final = (
    "https://learn.microsoft.com/en-ca/answers/questions/2275994/"
    "uparrow-is-missing-in-presetshapedefinitions-xml"
)
BUNDLE_MODULES: Final = {
    "basic": frozenset(
        {"basic_shapes", "rects", "brackets_braces", "scrolls_tabs", "flowchart"}
    ),
    "arrows": frozenset(
        {
            "arrows",
            "bent_u_arrows",
            "curved_arrows",
            "circular_arrows",
            "arrow_callouts",
            "callouts",
            "connectors",
        }
    ),
}
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


def _starts_get_argument(output: list[str], literal_index: int) -> bool:
    cursor = literal_index - 1
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
    if (
        cursor < 0
        or output[cursor] != "."
        or "".join(output[cursor + 1 : end]) != "get"
    ):
        return False
    cursor -= 1
    receiver_end = cursor + 1
    while cursor >= 0 and (output[cursor].isalnum() or output[cursor] == "_"):
        cursor -= 1
    return "".join(output[cursor + 1 : receiver_end]) in {"adj", "adjust_values"}


def _normal_string_end(source: str, start: int) -> int:
    cursor = start + 1
    while cursor < len(source):
        if source[cursor] == "\\":
            cursor += 2
        elif source[cursor] == '"':
            return cursor + 1
        else:
            cursor += 1
    raise ContractError("UNPARSEABLE_RUST_SOURCE", "unterminated string literal")


def _raw_string_end(source: str, start: int) -> int | None:
    if source[start] != "r":
        return None
    cursor = start + 1
    while cursor < len(source) and source[cursor] == "#":
        cursor += 1
    if cursor >= len(source) or source[cursor] != '"':
        return None
    delimiter = '"' + source[start + 1 : cursor]
    closing = source.find(delimiter, cursor + 1)
    if closing < 0:
        raise ContractError(
            "UNPARSEABLE_RUST_SOURCE", "unterminated raw string literal"
        )
    return closing + len(delimiter)


def _lexical_source(source: str, *, dispatcher: bool = False) -> str:
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
            if depth:
                raise ContractError("UNPARSEABLE_RUST_SOURCE", "unterminated comment")
            output[index:end] = " " * (end - index)
            index = end
        elif (raw_end := _raw_string_end(source, index)) is not None:
            if dispatcher or not _starts_get_argument(output, index):
                output[index:raw_end] = " " * (raw_end - index)
            index = raw_end
        elif source[index] == '"':
            end = _normal_string_end(source, index)
            if not dispatcher and not _starts_get_argument(output, index):
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
        raw_source = dispatcher.read_text(encoding="utf-8")
    except (FileNotFoundError, IsADirectoryError, PermissionError, OSError) as error:
        raise ContractError("INVALID_DISPATCHER", f"path={dispatcher}") from error
    source = _lexical_source(raw_source, dispatcher=True)
    structure = _lexical_source(raw_source)
    try:
        block = source[
            structure.index("pub fn preset_shape_svg(") : structure.index(
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
    keys: set[str] = set()
    for match in re.finditer(r"\b(?:adj|adjust_values)\.get\s*\(\s*", body):
        argument = body[match.end() :]
        normal = re.match(r'"(?P<key>[A-Za-z0-9_]+)"\s*\)', argument)
        raw = re.match(
            r'r(?P<hashes>#{0,255})"(?P<key>[A-Za-z0-9_]+)"(?P=hashes)\s*\)',
            argument,
        )
        literal = normal or raw
        if literal is None:
            snippet = " ".join(argument[:40].split())
            raise ContractError(UNPARSEABLE_ADJUSTMENT_LOOKUP, f"argument={snippet}")
        keys.add(literal.group("key"))
    return keys


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
    supplement_path: Path | None = None,
    bundle: str | None = None,
) -> CheckReport:
    manifest = _load_manifest(manifest_path or repo_root / DEFAULT_MANIFEST)
    names, rows = _manifest_inventory(manifest)
    supplement = _official_supplement(
        supplement_path or repo_root / DEFAULT_SUPPLEMENT, manifest
    )
    contract = manifest.get("contract")
    artifact = manifest.get("official_artifact")
    if (
        not isinstance(contract, dict)
        or not isinstance(artifact, dict)
        or contract.get("official_adjustment_count") != OFFICIAL_TOTAL_ADJUSTMENT_COUNT
        or contract.get("official_handle_constraint_count")
        != OFFICIAL_TOTAL_CONSTRAINT_COUNT
        or artifact.get("adjustment_count") != OFFICIAL_ADJUSTMENT_COUNT
        or artifact.get("handle_constraint_count") != OFFICIAL_CONSTRAINT_COUNT
        or rows["upArrow"].get("source_status") != "available"
        or rows["upArrow"].get("adjustments") != supplement["upArrow"]
    ):
        raise ContractError(
            OFFICIAL_ADJUSTMENT_SEMANTICS_MISMATCH,
            "official base plus supplement contract mismatch",
        )
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
    selected_modules: frozenset[str] | None = None
    if bundle in BUNDLE_MODULES:
        selected_modules = BUNDLE_MODULES[bundle]
    elif bundle == "remaining":
        selected_modules = frozenset(
            module
            for _, module, _ in routes
            if module not in BUNDLE_MODULES["basic"]
            and module not in BUNDLE_MODULES["arrows"]
        )
    selected_names = [
        name
        for name in names
        if selected_modules is None or route_by_name[name][1] in selected_modules
    ]
    functions = _function_bodies(source_root or repo_root / DEFAULT_SOURCE_ROOT)
    consumed: set[tuple[str, str]] = set()
    unknown: list[dict[str, str]] = []
    preserved_count = 0
    for preset in selected_names:
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
        if preset in selected_names
        for key in _row_keys(row, "adjustments")
    }
    known = {key for _, key in official_pairs}
    known.update(key for row in rows.values() for key in _row_keys(row, "preservation"))
    reported = {(item["family"], item["key"]) for item in unknown}
    for (module, _), body in functions.items():
        if selected_modules is not None and module not in selected_modules:
            continue
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
        "bundle": bundle or "all",
        "presets": len(selected_names),
        "unclassified_presets": 0,
        "unclassified_preset_names": [],
        "unknown_consumed_keys": len(unknown),
        "unknown_consumed_key_details": unknown,
        "non_official_consumed_keys_preserved": preserved_count,
        "manifest_keys_never_consumed": len(official_pairs - consumed),
        "official_adjustment_pairs": len(official_pairs),
    }


def arrow_contract_tsv(
    manifest: dict[str, JsonValue], dispatcher_path: Path, manifest_sha256: str
) -> str:
    names, rows = _manifest_inventory(manifest)
    routes = {name: module for name, module, _ in _dispatcher_routes(dispatcher_path)}
    selected = [name for name in names if routes.get(name) in BUNDLE_MODULES["arrows"]]
    lines = [
        f"# manifest_sha256={manifest_sha256}",
        f"# official_names_sha256={OFFICIAL_NAMES_SHA256}",
        f"# supplement_sha256={OFFICIAL_SUPPLEMENT_SHA256}",
        "preset\tkey\tdefault\tlower\tupper",
    ]
    lines[3:3] = [f"# preset_name={name}" for name in selected]
    for preset in selected:
        adjustments = rows[preset].get("adjustments")
        if not isinstance(adjustments, list):
            raise ContractError("MALFORMED_MANIFEST", f"{preset}.adjustments")
        for adjustment in adjustments:
            if not isinstance(adjustment, dict):
                raise ContractError("MALFORMED_MANIFEST", f"{preset}.adjustment")
            key = adjustment.get("name")
            formula = adjustment.get("default_formula")
            if not isinstance(key, str) or not isinstance(formula, str):
                raise ContractError("MALFORMED_MANIFEST", f"{preset}.adjustment fields")
            try:
                default = float(formula.removeprefix("val "))
            except ValueError as error:
                raise ContractError(
                    "MALFORMED_MANIFEST", f"{preset}.{key} default"
                ) from error
            constraints = adjustment.get("constraints")
            first = (
                constraints[0]
                if isinstance(constraints, list) and constraints
                else None
            )
            try:
                lower = (
                    float(first["minimum_formula"]) if isinstance(first, dict) else None
                )
                upper = (
                    float(first["maximum_formula"]) if isinstance(first, dict) else None
                )
            except (KeyError, TypeError, ValueError):
                lower = upper = None
            if lower is None or upper is None:
                span = max(abs(default), 10_000.0) / 2.0
                lower, upper = default - span, default + span
            lines.append(f"{preset}\t{key}\t{default:g}\t{lower:g}\t{upper:g}")
    return "\n".join(lines) + "\n"


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


def _local_name(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def _shape_adjustments(shape: ElementTree.Element) -> list[JsonValue]:
    namespace = f"{{{DRAWINGML_NAMESPACE}}}"
    adjustment_list = shape.find(f"{namespace}avLst")
    handle_list = shape.find(f"{namespace}ahLst")
    guides = (
        [] if adjustment_list is None else adjustment_list.findall(f"{namespace}gd")
    )
    handles = [] if handle_list is None else list(handle_list)
    axes = (
        ("X", "x", "minX", "maxX"),
        ("Y", "y", "minY", "maxY"),
        ("R", "radius", "minR", "maxR"),
        ("Ang", "angle", "minAng", "maxAng"),
    )
    adjustments: list[JsonValue] = []
    for guide in guides:
        name = guide.get("name")
        formula = guide.get("fmla")
        if name is None or formula is None:
            raise ContractError(
                OFFICIAL_ADJUSTMENT_SEMANTICS_MISMATCH,
                f"preset={_local_name(shape.tag)} invalid adjustment guide",
            )
        constraints: list[JsonValue] = []
        for handle in handles:
            for suffix, axis, minimum, maximum in axes:
                if handle.get(f"gdRef{suffix}") == name:
                    constraints.append(
                        {
                            "handle": _local_name(handle.tag),
                            "axis": axis,
                            "minimum_formula": handle.get(minimum),
                            "maximum_formula": handle.get(maximum),
                        }
                    )
        adjustments.append(
            {
                "name": name,
                "default_formula": formula,
                "source_status": "available",
                "range_status": "available" if constraints else "unavailable",
                "constraints": constraints,
            }
        )
    return adjustments


def _official_supplement(
    path: Path, manifest: dict[str, JsonValue]
) -> dict[str, list[JsonValue]]:
    metadata = manifest.get("official_supplements")
    expected_metadata = {
        "preset": "upArrow",
        "path": str(DEFAULT_SUPPLEMENT),
        "url": OFFICIAL_SUPPLEMENT_URL,
        "accepted_answer_author": (
            "Tom Jebo, Microsoft Employee, Microsoft Open Specifications Support"
        ),
        "accepted_answer_timestamp": "2025-05-16T21:51:23.2866667+00:00",
        "retrieved": "2026-08-11",
        "sha256": OFFICIAL_SUPPLEMENT_SHA256,
        "adjustment_count": 2,
        "handle_constraint_count": 2,
    }
    if metadata != [expected_metadata]:
        raise ContractError(
            OFFICIAL_ADJUSTMENT_SEMANTICS_MISMATCH,
            "official supplement metadata mismatch",
        )
    try:
        payload = path.read_bytes()
    except FileNotFoundError as error:
        raise ContractError(OFFICIAL_SUPPLEMENT_NOT_FOUND, f"path={path}") from error
    except (IsADirectoryError, PermissionError, OSError) as error:
        raise ContractError(OFFICIAL_SUPPLEMENT_INVALID, f"path={path}") from error
    actual_sha = hashlib.sha256(payload).hexdigest()
    if actual_sha != OFFICIAL_SUPPLEMENT_SHA256:
        raise ContractError(
            OFFICIAL_SUPPLEMENT_CHECKSUM_MISMATCH,
            f"expected={OFFICIAL_SUPPLEMENT_SHA256} actual={actual_sha}",
        )
    try:
        root = ElementTree.fromstring(payload)
    except ElementTree.ParseError as error:
        raise ContractError(OFFICIAL_SUPPLEMENT_INVALID, f"path={path}") from error
    if _local_name(root.tag) != "upArrow":
        raise ContractError(OFFICIAL_SUPPLEMENT_INVALID, "root must be upArrow")
    adjustments = _shape_adjustments(root)
    constraint_count = sum(
        len(adjustment["constraints"])
        for adjustment in adjustments
        if isinstance(adjustment, dict)
        and isinstance(adjustment.get("constraints"), list)
    )
    if len(adjustments) != 2 or constraint_count != 2:
        raise ContractError(
            OFFICIAL_ADJUSTMENT_SEMANTICS_MISMATCH,
            f"supplement adjustments={len(adjustments)} constraints={constraint_count}",
        )
    return {"upArrow": adjustments}


def _official_adjustments(
    root: ElementTree.Element, names: list[str]
) -> dict[str, list[JsonValue]]:
    semantics: dict[str, list[JsonValue]] = {}
    duplicates: dict[str, int] = {}
    for shape in root:
        name = _local_name(shape.tag)
        adjustments = _shape_adjustments(shape)
        if name in semantics:
            duplicates[name] = duplicates.get(name, 1) + 1
            if name != "upDownArrow" or semantics[name] != adjustments:
                raise ContractError(
                    OFFICIAL_ADJUSTMENT_SEMANTICS_MISMATCH,
                    f"unexpected duplicate preset={name}",
                )
            continue
        semantics[name] = adjustments
    expected_names = set(names) - {"upArrow"}
    if set(semantics) != expected_names or duplicates != {"upDownArrow": 2}:
        raise ContractError(
            OFFICIAL_ADJUSTMENT_SEMANTICS_MISMATCH,
            "presetShapeDefinitions normalization mismatch",
        )
    adjustment_count = sum(len(adjustments) for adjustments in semantics.values())
    constraint_count = sum(
        len(adjustment["constraints"])
        for adjustments in semantics.values()
        for adjustment in adjustments
        if isinstance(adjustment, dict)
        and isinstance(adjustment.get("constraints"), list)
    )
    if (
        adjustment_count != OFFICIAL_ADJUSTMENT_COUNT
        or constraint_count != OFFICIAL_CONSTRAINT_COUNT
    ):
        raise ContractError(
            OFFICIAL_ADJUSTMENT_SEMANTICS_MISMATCH,
            f"adjustments={adjustment_count} constraints={constraint_count}",
        )
    return semantics


def _verify_manifest_adjustments(
    manifest: dict[str, JsonValue], semantics: dict[str, list[JsonValue]]
) -> None:
    rows = manifest.get("presets")
    artifact = manifest.get("official_artifact")
    if not isinstance(rows, list) or not isinstance(artifact, dict):
        raise ContractError("MALFORMED_MANIFEST", "preset semantics are required")
    contract = manifest.get("contract")
    if not isinstance(contract, dict):
        raise ContractError("MALFORMED_MANIFEST", "contract is required")
    if (
        artifact.get("adjustment_count") != OFFICIAL_ADJUSTMENT_COUNT
        or artifact.get("handle_constraint_count") != OFFICIAL_CONSTRAINT_COUNT
        or contract.get("official_adjustment_count") != OFFICIAL_TOTAL_ADJUSTMENT_COUNT
        or contract.get("official_handle_constraint_count")
        != OFFICIAL_TOTAL_CONSTRAINT_COUNT
    ):
        raise ContractError(
            OFFICIAL_ADJUSTMENT_SEMANTICS_MISMATCH,
            "manifest official semantic counts mismatch",
        )
    for row in rows:
        if not isinstance(row, dict) or not isinstance(row.get("name"), str):
            raise ContractError("MALFORMED_MANIFEST", "invalid preset row")
        name = row["name"]
        if row.get("source_status") != "available" or row.get(
            "adjustments"
        ) != semantics.get(name):
            raise ContractError(
                OFFICIAL_ADJUSTMENT_SEMANTICS_MISMATCH,
                f"preset={name}",
            )


def _verify_official_package(
    path: Path,
    supplement_path: Path,
    manifest: dict[str, JsonValue],
    names: list[str],
) -> None:
    artifact = manifest.get("official_artifact")
    if not isinstance(artifact, dict):
        raise ContractError("MALFORMED_MANIFEST", "official_artifact is required")
    outer_sha, member_sha, geometry_sha = (
        artifact.get("sha256"),
        artifact.get("inventory_member_sha256"),
        artifact.get("geometry_member_sha256"),
    )
    if (
        not isinstance(outer_sha, str)
        or not isinstance(member_sha, str)
        or not isinstance(geometry_sha, str)
    ):
        raise ContractError(
            "MALFORMED_MANIFEST", "official artifact checksums are required"
        )
    verify_official_artifact(path, outer_sha)
    try:
        with zipfile.ZipFile(path) as outer:
            nested_bytes = outer.read("OfficeOpenXML-XMLSchema-Strict.zip")
            geometry_bytes = outer.read("OfficeOpenXML-DrawingMLGeometries.zip")
        with zipfile.ZipFile(io.BytesIO(nested_bytes)) as nested:
            xsd = nested.read("dml-main.xsd")
        with zipfile.ZipFile(io.BytesIO(geometry_bytes)) as nested:
            geometry = nested.read("presetShapeDefinitions.xml")
        xsd_root = ElementTree.fromstring(xsd)
        geometry_root = ElementTree.fromstring(geometry)
    except (
        zipfile.BadZipFile,
        KeyError,
        OSError,
        RuntimeError,
        ElementTree.ParseError,
    ) as error:
        raise ContractError("OFFICIAL_ARTIFACT_INVALID", f"path={path}") from error
    actual_member_sha = hashlib.sha256(xsd).hexdigest()
    if actual_member_sha != member_sha:
        raise ContractError(
            OFFICIAL_ARTIFACT_CHECKSUM_MISMATCH,
            f"nested expected={member_sha} actual={actual_member_sha}",
        )
    actual_geometry_sha = hashlib.sha256(geometry).hexdigest()
    if actual_geometry_sha != geometry_sha:
        raise ContractError(
            OFFICIAL_ARTIFACT_CHECKSUM_MISMATCH,
            f"geometry expected={geometry_sha} actual={actual_geometry_sha}",
        )
    namespace = {"x": "http://www.w3.org/2001/XMLSchema"}
    shape_type = next(
        (
            item
            for item in xsd_root.findall("x:simpleType", namespace)
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
    semantics = _official_adjustments(geometry_root, names)
    semantics.update(_official_supplement(supplement_path, manifest))
    _verify_manifest_adjustments(manifest, semantics)


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--dispatcher", type=Path)
    parser.add_argument("--json", type=Path, dest="json_output")
    parser.add_argument("--export-arrow-contract", type=Path)
    parser.add_argument("--official-artifact", type=Path)
    parser.add_argument("--official-supplement", type=Path)
    parser.add_argument("--bundle", choices=("basic", "arrows", "remaining"))
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
    supplement_path = (
        args.official_supplement.resolve()
        if args.official_supplement
        else repo_root / DEFAULT_SUPPLEMENT
    )
    manifest = _load_manifest(manifest_path)
    names, _ = _manifest_inventory(manifest)
    report = check_repository(
        repo_root,
        manifest_path=manifest_path,
        source_root=source_root,
        dispatcher_path=dispatcher_path,
        supplement_path=supplement_path,
        bundle=args.bundle,
    )
    if args.official_artifact:
        _verify_official_package(
            args.official_artifact, supplement_path, manifest, names
        )
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
    if args.export_arrow_contract:
        try:
            args.export_arrow_contract.parent.mkdir(parents=True, exist_ok=True)
            args.export_arrow_contract.write_text(
                arrow_contract_tsv(
                    manifest,
                    dispatcher_path,
                    hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
                ),
                encoding="utf-8",
            )
        except OSError as error:
            raise ContractError(
                "CONTRACT_OUTPUT_UNWRITABLE", f"path={args.export_arrow_contract}"
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
