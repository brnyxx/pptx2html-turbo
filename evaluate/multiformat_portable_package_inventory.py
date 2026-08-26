from __future__ import annotations

import json
import os
import shutil
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import TypeAlias

from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

JsonObject: TypeAlias = dict[str, JsonValue]


class PortableLockIoError(ValueError):
    pass


@dataclass(frozen=True, slots=True)
class PackageEntry:
    path: str
    kind: str
    sha256: str | None = None
    size: int | None = None
    target: str | None = None


def bind_package_executable(source: Path, root: Path, destination: Path) -> Path:
    executable, _inventory = bind_package_executable_with_inventory(
        source, root, destination
    )
    return executable


def bind_package_executable_with_inventory(
    source: Path, root: Path, destination: Path
) -> tuple[Path, Path | None]:
    resolved = source.resolve(strict=True)
    if resolved.is_relative_to(root):
        return resolved, None
    package = next(
        (item for item in (resolved, *resolved.parents) if item.suffix == ".app"), None
    )
    if package is None:
        shutil.copyfile(resolved, destination)
        destination.chmod(resolved.stat().st_mode & 0o777)
        return destination, None
    copied = destination / package.name
    shutil.copytree(package, copied, symlinks=True)
    inventory = destination / "inventory.json"
    entries = _scan_package(copied)
    inventory.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "package_root": copied.relative_to(root).as_posix(),
                "entries": [_entry_json(entry) for entry in entries],
            },
            sort_keys=True,
            separators=(",", ":"),
        )
        + "\n",
        encoding="utf-8",
    )
    return copied / resolved.relative_to(package), inventory


def package_binding(
    root: Path, executable: Path, version: str, inventory: Path | None
) -> JsonObject:
    resolved_root = root.resolve(strict=True)
    result: JsonObject = {
        "version": version,
        "path": executable.resolve(strict=True).relative_to(resolved_root).as_posix(),
        "sha256": sha256_file(executable),
    }
    if inventory is not None:
        result["package_inventory"] = {
            "path": inventory.resolve(strict=True)
            .relative_to(resolved_root)
            .as_posix(),
            "sha256": sha256_file(inventory),
        }
    return result


def validate_package_inventory(
    inventory: Path, evidence_root: Path
) -> tuple[PackageEntry, ...]:
    values = read_strict_object(inventory)
    if (
        set(values) != {"schema_version", "package_root", "entries"}
        or integer_value(values, "schema_version") != 1
    ):
        raise PortableLockIoError("portable package inventory schema differs")
    package_relative = string_value(values, "package_root")
    package = evidence_root / package_relative
    try:
        package = package.resolve(strict=True)
        package.relative_to(evidence_root.resolve(strict=True))
    except (OSError, ValueError) as error:
        raise PortableLockIoError("portable package root escapes evidence") from error
    raw_entries = values.get("entries")
    if not isinstance(raw_entries, list) or not raw_entries:
        raise PortableLockIoError("portable package inventory is empty")
    expected = tuple(_parse_package_entry(entry) for entry in raw_entries)
    if expected != tuple(sorted(expected, key=lambda entry: entry.path)):
        raise PortableLockIoError("portable package inventory is not ordered")
    if len({entry.path for entry in expected}) != len(expected):
        raise PortableLockIoError("portable package inventory path is duplicated")
    if _scan_package(package) != expected:
        raise PortableLockIoError("portable package inventory differs")
    return expected


def validate_package_binding(
    binding: JsonObject,
    executable: Path,
    evidence_root: Path,
    resolve_binding: Callable[[JsonObject, Path], Path],
) -> None:
    package = next(
        (
            parent
            for parent in (executable, *executable.parents)
            if parent.suffix == ".app"
        ),
        None,
    )
    inventory_value = binding.get("package_inventory")
    if package is None:
        if inventory_value is not None:
            raise PortableLockIoError(
                "portable non-package tool has a package inventory"
            )
        return
    if not isinstance(inventory_value, dict):
        raise PortableLockIoError("portable app package inventory is missing")
    inventory = resolve_binding(inventory_value, evidence_root)
    entries = validate_package_inventory(inventory, evidence_root)
    values = read_strict_object(inventory)
    if string_value(values, "package_root") != package.relative_to(
        evidence_root.resolve(strict=True)
    ).as_posix() or not any(
        entry.path == executable.relative_to(package).as_posix() for entry in entries
    ):
        raise PortableLockIoError("portable app executable is outside its inventory")


def _scan_package(package: Path) -> tuple[PackageEntry, ...]:
    root = package.resolve(strict=True)
    entries: list[PackageEntry] = []
    ordered = sorted(
        package.rglob("*"), key=lambda path: path.relative_to(package).as_posix()
    )
    for item in ordered:
        relative = item.relative_to(package).as_posix()
        if item.is_symlink():
            try:
                item.resolve(strict=True).relative_to(root)
            except (OSError, ValueError) as error:
                raise PortableLockIoError(
                    "portable package symlink escapes package"
                ) from error
            entries.append(PackageEntry(relative, "symlink", target=os.readlink(item)))
        elif item.is_file():
            metadata = item.stat(follow_symlinks=False)
            if metadata.st_nlink != 1:
                raise PortableLockIoError("portable package file has an external alias")
            entries.append(
                PackageEntry(relative, "file", sha256_file(item), metadata.st_size)
            )
        elif not item.is_dir():
            raise PortableLockIoError("portable package contains a special file")
    if not entries:
        raise PortableLockIoError("portable package is empty")
    return tuple(entries)


def _entry_json(entry: PackageEntry) -> JsonObject:
    if entry.kind == "file":
        return {
            "path": entry.path,
            "kind": entry.kind,
            "sha256": entry.sha256 or "",
            "size": entry.size if entry.size is not None else -1,
        }
    return {"path": entry.path, "kind": entry.kind, "target": entry.target or ""}


def _parse_package_entry(value: object) -> PackageEntry:
    if not isinstance(value, dict):
        raise PortableLockIoError("portable package inventory entry is invalid")
    entry: JsonObject = value
    kind, path = string_value(entry, "kind"), string_value(entry, "path")
    if not path or Path(path).is_absolute() or ".." in Path(path).parts:
        raise PortableLockIoError("portable package inventory path is invalid")
    if kind == "file":
        if set(entry) != {"path", "kind", "sha256", "size"}:
            raise PortableLockIoError("portable package file entry fields differ")
        size = integer_value(entry, "size")
        if size < 0:
            raise PortableLockIoError("portable package file size is invalid")
        return PackageEntry(path, kind, sha256_value(entry, "sha256"), size)
    if kind == "symlink":
        if set(entry) != {"path", "kind", "target"}:
            raise PortableLockIoError("portable package symlink entry fields differ")
        return PackageEntry(path, kind, target=string_value(entry, "target"))
    raise PortableLockIoError("portable package inventory kind is invalid")
