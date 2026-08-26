from __future__ import annotations

import os
from pathlib import Path
from typing import NamedTuple

from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class RustToolchainError(ValueError):
    """Raised when evaluator-locked Rust toolchain identity is invalid."""


class RustToolchainIdentity(NamedTuple):
    cargo: Path
    cargo_sha256: str
    rustc: Path
    rustc_sha256: str


def rust_toolchain_identity(cargo: Path, rustc: Path) -> RustToolchainIdentity:
    cargo_path = _executable(cargo, "cargo")
    rustc_path = _executable(rustc, "rustc")
    if cargo_path.parent != rustc_path.parent:
        raise RustToolchainError("cargo and rustc must belong to one toolchain")
    return RustToolchainIdentity(
        cargo_path,
        sha256_file(cargo_path),
        rustc_path,
        sha256_file(rustc_path),
    )


def load_locked_rust_toolchain(path: Path) -> RustToolchainIdentity:
    lock = read_strict_object(path)
    values = object_value(lock, "rust_toolchain")
    require_keys(values, {"cargo", "rustc"}, "rust_toolchain")
    cargo = object_value(values, "cargo")
    rustc = object_value(values, "rustc")
    require_keys(cargo, {"path", "sha256"}, "rust_toolchain.cargo")
    require_keys(rustc, {"path", "sha256"}, "rust_toolchain.rustc")
    identity = rust_toolchain_identity(
        Path(string_value(cargo, "path")),
        Path(string_value(rustc, "path")),
    )
    if (
        sha256_value(cargo, "sha256") != identity.cargo_sha256
        or sha256_value(rustc, "sha256") != identity.rustc_sha256
    ):
        raise RustToolchainError("Rust toolchain executable identity differs")
    return identity


def rust_toolchain_value(identity: RustToolchainIdentity) -> dict[str, JsonValue]:
    return {
        "cargo": {
            "path": identity.cargo.as_posix(),
            "sha256": identity.cargo_sha256,
        },
        "rustc": {
            "path": identity.rustc.as_posix(),
            "sha256": identity.rustc_sha256,
        },
    }


def resolve_rustc(path_argument: str) -> Path:
    directories = path_argument.removeprefix("PATH=").split(os.pathsep)
    if not path_argument.startswith("PATH=") or any(
        not directory or not Path(directory).is_absolute() for directory in directories
    ):
        raise RustToolchainError("Rust toolchain PATH is invalid")
    for directory in directories:
        candidate = Path(directory) / "rustc"
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate.resolve(strict=True)
    raise RustToolchainError("Rust toolchain PATH does not resolve rustc")


def _executable(path: Path, name: str) -> Path:
    if not path.is_absolute():
        raise RustToolchainError(f"{name} executable is not absolute")
    try:
        resolved = path.resolve(strict=True)
    except OSError as error:
        raise RustToolchainError(f"{name} executable is not real") from error
    if (
        resolved.name != name
        or not resolved.is_file()
        or not os.access(resolved, os.X_OK)
    ):
        raise RustToolchainError(f"{name} executable is not allowed")
    return resolved
