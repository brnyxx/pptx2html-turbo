from __future__ import annotations

import hashlib
import os
import platform
import shutil
import stat
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Final

from evaluate.multiformat_portable_package_inventory import (
    PortableLockIoError,
    package_inventory_for_executable,
    validate_package_inventory,
    write_package_inventory,
)

_SYSTEM_PREFIXES: Final = ("/usr/lib/", "/System/")
_OTOOL: Final = Path("/usr/bin/otool")
_INSTALL_NAME_TOOL: Final = Path("/usr/bin/install_name_tool")
_CODESIGN: Final = Path("/usr/bin/codesign")
_APPLE_TOOLS: Final = (_OTOOL, _INSTALL_NAME_TOOL, _CODESIGN)


@dataclass(frozen=True, slots=True)
class NativePackageClosure:
    executables: tuple[Path, ...]
    inventory: Path


@dataclass(frozen=True, slots=True)
class _MachO:
    source: Path
    install_name: str | None
    dependencies: tuple[tuple[str, Path], ...]
    rpaths: tuple[str, ...]


def bind_homebrew_package_closure(
    sources: tuple[Path, ...], root: Path, destination: Path
) -> NativePackageClosure | None:
    """Copy, relocate, sign, and inventory one Homebrew runtime closure."""
    evidence = root.resolve(strict=True)
    destination = destination.resolve(strict=False)
    resolved = tuple(source.resolve(strict=True) for source in sources)
    reused = tuple(
        package_inventory_for_executable(source, evidence) for source in resolved
    )
    if all(item is not None for item in reused):
        packages = {item[0] for item in reused if item is not None}
        inventories = {item[1] for item in reused if item is not None}
        if len(packages) != 1 or len(inventories) != 1:
            raise PortableLockIoError("portable native package closure differs")
        inventory = inventories.pop()
        validate_package_inventory(inventory, evidence)
        return NativePackageClosure(resolved, inventory)
    if platform.system() != "Darwin":
        return None
    prefixes = tuple(_cellar_prefix(source) for source in resolved)
    if all(prefix is None for prefix in prefixes):
        return None
    if any(prefix is None for prefix in prefixes) or len(set(prefixes)) != 1:
        raise PortableLockIoError("portable native tools do not share a package")
    prefix = prefixes[0]
    if prefix is None:
        return None
    if any(not tool.is_file() or not os.access(tool, os.X_OK) for tool in _APPLE_TOOLS):
        raise PortableLockIoError("required Apple native tool is unavailable")
    machos = _collect_closure(resolved)
    identities = {path: _identity(path) for path in machos}
    package = destination / "root"
    copied = {source: package / _relative_member(source, prefix) for source in machos}
    for source, target in copied.items():
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        target.chmod(stat.S_IMODE(source.stat().st_mode))
    for macho in machos.values():
        target = copied[macho.source]
        changes = tuple(
            (load, f"@loader_path/{os.path.relpath(copied[dependency], target.parent)}")
            for load, dependency in macho.dependencies
        )
        if changes or macho.install_name is not None or macho.rpaths:
            command = [_INSTALL_NAME_TOOL.as_posix()]
            if macho.install_name is not None:
                command.extend(("-id", f"@loader_path/{target.name}"))
            for old, new in changes:
                command.extend(("-change", old, new))
            for rpath in macho.rpaths:
                command.extend(("-delete_rpath", rpath))
            _run((*command, target.as_posix()), "cannot relocate native package")
            _run(
                (
                    _CODESIGN.as_posix(),
                    "--force",
                    "--sign",
                    "-",
                    target.as_posix(),
                ),
                "cannot sign native package",
            )
    if any(_identity(path) != identity for path, identity in identities.items()):
        raise PortableLockIoError("portable native package changed while copying")
    _validate_relocated_closure(tuple(copied.values()), package)
    inventory = destination / "inventory.json"
    write_package_inventory(inventory, package, evidence)
    executables = tuple(copied[source] for source in resolved)
    return NativePackageClosure(executables, inventory)


def _collect_closure(sources: tuple[Path, ...]) -> dict[Path, _MachO]:
    pending = list(sources)
    values: dict[Path, _MachO] = {}
    while pending:
        source = pending.pop()
        if source in values:
            continue
        install_name = _install_name(source)
        loads = _load_commands(source)
        rpaths = _rpaths(source)
        dependencies = tuple(
            (load, resolved)
            for load in loads
            if load != install_name
            if (resolved := _resolve_dependency(source, load, rpaths)) is not None
        )
        values[source] = _MachO(source, install_name, dependencies, rpaths)
        pending.extend(path for _load, path in dependencies)
    return values


def _load_commands(path: Path) -> tuple[str, ...]:
    output = _run(
        (_OTOOL.as_posix(), "-L", path.as_posix()),
        "cannot inspect native package",
    )
    lines = output.splitlines()
    if not lines or not lines[0].endswith(":"):
        raise PortableLockIoError("portable native package is not Mach-O")
    return tuple(
        line.strip().split(" (compatibility version", maxsplit=1)[0]
        for line in lines[1:]
        if line.strip()
    )


def _install_name(path: Path) -> str | None:
    output = _run(
        (_OTOOL.as_posix(), "-D", path.as_posix()), "cannot inspect install name"
    )
    lines = tuple(line.strip() for line in output.splitlines()[1:] if line.strip())
    if len(lines) > 1:
        raise PortableLockIoError("portable native install name is ambiguous")
    return lines[0] if lines else None


def _rpaths(path: Path) -> tuple[str, ...]:
    output = _run(
        (_OTOOL.as_posix(), "-l", path.as_posix()), "cannot inspect native rpaths"
    )
    lines = output.splitlines()
    return tuple(
        lines[index + 2].strip().split(" (offset", maxsplit=1)[0].removeprefix("path ")
        for index, line in enumerate(lines[:-2])
        if line.strip() == "cmd LC_RPATH"
    )


def _resolve_dependency(
    loader: Path, load: str, rpaths: tuple[str, ...]
) -> Path | None:
    if load.startswith(_SYSTEM_PREFIXES):
        return None
    if load.startswith("/"):
        return Path(load).resolve(strict=True)
    if load.startswith("@loader_path/"):
        return (loader.parent / load.removeprefix("@loader_path/")).resolve(strict=True)
    if load.startswith("@rpath/"):
        suffix = load.removeprefix("@rpath/")
        for rpath in rpaths:
            if rpath.startswith("@loader_path/"):
                candidate = loader.parent / rpath.removeprefix("@loader_path/") / suffix
            elif rpath.startswith("/"):
                candidate = Path(rpath) / suffix
            else:
                continue
            if candidate.exists():
                return candidate.resolve(strict=True)
    raise PortableLockIoError(f"portable native dependency is unresolved: {load}")


def _cellar_prefix(path: Path) -> Path | None:
    parts = path.parts
    try:
        index = parts.index("Cellar")
    except ValueError:
        return None
    if len(parts) <= index + 2:
        return None
    return Path(*parts[: index + 3])


def _relative_member(path: Path, primary: Path) -> Path:
    if path.is_relative_to(primary):
        return path.relative_to(primary)
    identity = hashlib.sha256(path.as_posix().encode()).hexdigest()[:16]
    return Path("dependencies") / f"{identity}-{path.name}"


def _validate_relocated_closure(paths: tuple[Path, ...], package: Path) -> None:
    for path in paths:
        _run(
            (_CODESIGN.as_posix(), "--verify", "--strict", path.as_posix()),
            "portable native signature verification failed",
        )
        install_name = _install_name(path)
        for rpath in _rpaths(path):
            if not rpath.startswith("@loader_path/"):
                raise PortableLockIoError("portable native rpath remains external")
            resolved_rpath = (
                path.parent / rpath.removeprefix("@loader_path/")
            ).resolve(strict=True)
            if not resolved_rpath.is_relative_to(package.resolve(strict=True)):
                raise PortableLockIoError("portable native rpath escapes package")
        for load in _load_commands(path):
            if load == install_name:
                if not load.startswith("@loader_path/"):
                    raise PortableLockIoError(
                        "portable native install name remains external"
                    )
                continue
            if load.startswith(_SYSTEM_PREFIXES):
                continue
            if not load.startswith("@loader_path/"):
                raise PortableLockIoError("portable native dependency remains external")
            resolved = (path.parent / load.removeprefix("@loader_path/")).resolve(
                strict=True
            )
            if not resolved.is_relative_to(package.resolve(strict=True)):
                raise PortableLockIoError("portable native dependency escapes package")


def _identity(path: Path) -> tuple[int, ...]:
    value = path.stat()
    return (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_nlink,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def _run(command: tuple[str, ...], message: str) -> str:
    try:
        result = subprocess.run(
            command, check=True, capture_output=True, text=True, timeout=30
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise PortableLockIoError(message) from error
    return result.stdout
