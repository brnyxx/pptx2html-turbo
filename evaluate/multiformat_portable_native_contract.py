from __future__ import annotations

from collections.abc import Callable
from pathlib import Path
from typing import TypeAlias

from evaluate.multiformat_portable_package_inventory import PortableLockIoError
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

JsonObject: TypeAlias = dict[str, JsonValue]
ArtifactResolver: TypeAlias = Callable[[JsonObject, Path], Path]


def has_native_package_inventories(lock: JsonObject) -> bool:
    """Return whether the outer lock declares a complete native package profile."""
    return _outer_inventory_hashes(lock) is not None


def validate_native_package_runtime_binding(
    lock: JsonObject,
    evidence_root: Path,
    resolve_binding: ArtifactResolver,
) -> None:
    """Bind outer native package inventories to the inner candidate runtime lock."""
    runtime_path = resolve_binding(
        object_value(lock, "candidate_runtime_lock"), evidence_root
    )
    expected = _outer_inventory_hashes(lock)
    if expected is None:
        return
    runtime_lock = read_strict_object(runtime_path)
    if integer_value(runtime_lock, "schema_version") != 2:
        raise PortableLockIoError("portable native candidate lock schema differs")
    candidate = object_value(runtime_lock, "candidate_runtime")
    verifier = object_value(runtime_lock, "sandbox_verifier")
    if sha256_value(candidate, "poppler_package_inventory_sha256") != expected[0]:
        raise PortableLockIoError("portable Poppler package lock differs")
    if sha256_value(verifier, "openssl_package_inventory_sha256") != expected[1]:
        raise PortableLockIoError("portable OpenSSL package lock differs")


def _outer_inventory_hashes(lock: JsonObject) -> tuple[str, str] | None:
    tools = object_value(lock, "tools")
    poppler_bindings = tuple(
        object_value(tools, field)
        for field in ("poppler_render", "poppler_text", "poppler_metadata")
    )
    poppler_inventories = tuple(
        binding.get("package_inventory") for binding in poppler_bindings
    )
    openssl_inventory = object_value(
        object_value(lock, "candidate_sandbox"), "openssl"
    ).get("package_inventory")
    poppler_present = tuple(value is not None for value in poppler_inventories)
    if any(poppler_present) and not all(poppler_present):
        raise PortableLockIoError("portable Poppler package closure is incomplete")
    if all(poppler_present) != (openssl_inventory is not None):
        raise PortableLockIoError(
            "portable candidate native package closure is incomplete"
        )
    if not all(poppler_present):
        system = string_value(object_value(lock, "platform"), "os")
        if system == "Darwin":
            raise PortableLockIoError(
                "portable Darwin native package closure is missing"
            )
        return None
    poppler_hashes = tuple(
        sha256_value(object_value(binding, "package_inventory"), "sha256")
        for binding in poppler_bindings
    )
    if len(set(poppler_hashes)) != 1:
        raise PortableLockIoError("portable Poppler package closure differs")
    if not isinstance(openssl_inventory, dict):
        raise PortableLockIoError("portable OpenSSL package closure is incomplete")
    return poppler_hashes[0], sha256_value(openssl_inventory, "sha256")
