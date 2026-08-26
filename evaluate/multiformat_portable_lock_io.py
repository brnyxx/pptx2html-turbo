from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

from evaluate.multiformat_schema import JsonValue, sha256_file, string_value
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.multiformat_subprocess import clean_subprocess_environment


class PortableLockIoError(ValueError):
    pass


def bind_corpus(source: Path, root: Path, destination_root: Path) -> Path:
    resolved = source.resolve(strict=True)
    if resolved.is_relative_to(root):
        return resolved
    document_format = string_value(read_strict_object(resolved), "format")
    destination = destination_root / document_format
    shutil.copytree(resolved.parent, destination)
    return destination / resolved.name


def bind_file(source: Path, root: Path, destination: Path) -> Path:
    resolved = source.resolve(strict=True)
    if resolved.is_relative_to(root):
        return resolved
    shutil.copyfile(resolved, destination)
    destination.chmod(resolved.stat().st_mode & 0o777)
    return destination


def tool_version(path: Path) -> str:
    result = subprocess.run(
        [path.as_posix(), "--version"],
        check=False,
        capture_output=True,
        env=clean_subprocess_environment(),
        timeout=15,
    )
    if result.returncode != 0 or len(result.stdout) > 1024 * 1024:
        raise PortableLockIoError("portable tool version probe failed")
    value = result.stdout.decode("utf-8", errors="strict").strip()
    if not value:
        raise PortableLockIoError("portable tool version is empty")
    return value


def binding(root: Path, path: Path) -> dict[str, JsonValue]:
    resolved = path.resolve(strict=True)
    if not resolved.is_relative_to(root):
        raise PortableLockIoError("portable artifact is outside evidence root")
    return {
        "path": resolved.relative_to(root).as_posix(),
        "sha256": sha256_file(resolved),
    }


def versioned(root: Path, path: Path, version: str) -> dict[str, JsonValue]:
    return {"version": version, **binding(root, path)}


def exclusive_write(path: Path, value: bytes, mode: int) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
    try:
        os.write(descriptor, value)
    finally:
        os.close(descriptor)
