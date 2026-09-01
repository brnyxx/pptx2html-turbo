from __future__ import annotations

import os
import platform
import subprocess
from pathlib import Path
from typing import Final, NamedTuple

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.multiformat_subprocess import clean_subprocess_environment


class RustToolchainError(ValueError):
    """Raised when evaluator-locked Rust toolchain identity is invalid."""


class RustToolchainIdentity(NamedTuple):
    cargo: Path
    cargo_sha256: str
    rustc: Path
    rustc_sha256: str


class RustToolchainTrust(NamedTuple):
    cargo_version: str
    rustc_version_verbose: str


_TRUST_LOCK: Final = (
    Path(__file__).parent / "multiformat" / "rust-toolchain-lock.v1.json"
)


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


def evaluator_rust_toolchain_identity(
    cargo: Path, rustc: Path
) -> RustToolchainIdentity:
    identity = rust_toolchain_identity(cargo, rustc)
    trust = _evaluator_trust(identity)
    if (
        _version(identity.cargo, ("--version",)) != trust.cargo_version
        or _version(identity.rustc, ("--version", "--verbose"))
        != trust.rustc_version_verbose
    ):
        raise RustToolchainError("Rust toolchain version differs from evaluator trust")
    return identity


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
    _evaluator_trust(identity)
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


def _evaluator_trust(identity: RustToolchainIdentity) -> RustToolchainTrust:
    values = read_strict_object(_TRUST_LOCK)
    require_keys(values, {"schema_version", "identities"}, "rust_toolchain_trust")
    if integer_value(values, "schema_version") != 1:
        raise RustToolchainError("Rust toolchain trust schema is unsupported")
    for candidate in object_list(values, "identities", "rust_toolchain_trust"):
        require_keys(candidate, {"platform", "cargo", "rustc"}, "rust_toolchain")
        platform_value = object_value(candidate, "platform")
        cargo = object_value(candidate, "cargo")
        rustc = object_value(candidate, "rustc")
        require_keys(platform_value, {"os", "architecture"}, "rust_toolchain.platform")
        require_keys(cargo, {"sha256", "version"}, "rust_toolchain.cargo")
        require_keys(rustc, {"sha256", "version_verbose"}, "rust_toolchain.rustc")
        if (
            string_value(platform_value, "os") == platform.system()
            and string_value(platform_value, "architecture") == platform.machine()
            and sha256_value(cargo, "sha256") == identity.cargo_sha256
            and sha256_value(rustc, "sha256") == identity.rustc_sha256
        ):
            return RustToolchainTrust(
                string_value(cargo, "version"),
                string_value(rustc, "version_verbose"),
            )
    raise RustToolchainError("Rust toolchain is not allowed by evaluator trust")


def _version(path: Path, arguments: tuple[str, ...]) -> str:
    environment = clean_subprocess_environment()
    environment.update({"LANG": "C", "LC_ALL": "C"})
    try:
        result = subprocess.run(
            (path.as_posix(), *arguments),
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
            env=environment,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise RustToolchainError("Rust toolchain version probe failed") from error
    return result.stdout.strip()


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
