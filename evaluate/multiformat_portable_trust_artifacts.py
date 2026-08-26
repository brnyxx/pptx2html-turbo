from __future__ import annotations

from pathlib import Path
from typing import TypeAlias

from evaluate.multiformat_portable_package_inventory import validate_package_inventory
from evaluate.multiformat_portable_receipt_validation import (
    StableFileIdentity,
    verify_stable_file,
)
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

JsonObject: TypeAlias = dict[str, JsonValue]


def load_lock_artifacts(
    lock_path: Path,
    lock: JsonObject,
    root: Path,
    lock_sha256: str,
) -> tuple[StableFileIdentity, ...]:
    tools = object_value(lock, "tools")
    signer = object_value(lock, "signer")
    scope = object_value(lock, "scope")
    runtime = object_value(lock, "runtime")
    browser = object_value(lock, "browser")
    candidate = object_value(lock, "candidate_sandbox")
    sandbox = object_value(lock, "sandbox")
    attestation = read_strict_object(
        root / string_value(object_value(runtime, "attestation"), "path")
    )
    bindings = (
        ("portable-lock", _file_binding(lock_path, root, lock_sha256)),
        ("tool:libreoffice", object_value(tools, "libreoffice")),
        ("tool:poppler-render", object_value(tools, "poppler_render")),
        ("tool:poppler-text", object_value(tools, "poppler_text")),
        ("tool:poppler-metadata", object_value(tools, "poppler_metadata")),
        ("canonicalizer", object_value(lock, "canonicalizer")),
        ("font-bundle", object_value(lock, "font_bundle")),
        ("configuration", object_value(lock, "configuration")),
        ("browser:chromium", object_value(browser, "chromium")),
        ("browser:lock", object_value(browser, "lock")),
        ("candidate-runtime-lock", object_value(lock, "candidate_runtime_lock")),
        ("candidate-sandbox:public-key", object_value(candidate, "public_key")),
        ("candidate-sandbox:openssl", object_value(candidate, "openssl")),
        ("candidate-sandbox:receipt-signer", object_value(candidate, "receipt_signer")),
        ("sandbox:executable", object_value(sandbox, "executable")),
        ("sandbox:profile", object_value(sandbox, "profile")),
        ("sandbox:host-artifact", object_value(attestation, "sandbox_host_artifact")),
        ("public-key", object_value(signer, "public_key")),
        ("executor", object_value(signer, "executor")),
        ("contract", object_value(scope, "contract")),
        ("evaluator", object_value(scope, "evaluator")),
        ("corpus-manifest", object_value(scope, "corpus")),
        ("attestation", object_value(runtime, "attestation")),
    )
    identities = [_bound_identity(binding, root, role) for role, binding in bindings]
    for package_role, executable_binding in (
        ("libreoffice", object_value(tools, "libreoffice")),
        ("chromium", object_value(browser, "chromium")),
    ):
        identities.extend(_package_artifacts(root, package_role, executable_binding))
    return tuple(identities)


def revalidate_package_inventories(
    root: Path, identities: tuple[StableFileIdentity, ...]
) -> None:
    for artifact in identities:
        if artifact.role.startswith("package-inventory:"):
            validate_package_inventory(root / artifact.path, root)


def _package_artifacts(
    root: Path, package_role: str, executable: JsonObject
) -> list[StableFileIdentity]:
    inventory = executable.get("package_inventory")
    if not isinstance(inventory, dict):
        return []
    inventory_identity = _bound_identity(
        inventory, root, f"package-inventory:{package_role}"
    )
    inventory_path = root / inventory_identity.path
    package_root = string_value(read_strict_object(inventory_path), "package_root")
    executable_path = string_value(executable, "path")
    result = [inventory_identity]
    for entry in validate_package_inventory(inventory_path, root):
        member_path = f"{package_root}/{entry.path}"
        if entry.kind == "file" and member_path != executable_path:
            result.append(
                verify_stable_file(
                    root,
                    member_path,
                    entry.sha256 or "",
                    entry.size,
                    f"package:{package_role}:{entry.path}",
                )
            )
    return result


def _bound_identity(binding: JsonObject, root: Path, role: str) -> StableFileIdentity:
    return verify_stable_file(
        root,
        string_value(binding, "path"),
        sha256_value(binding, "sha256"),
        None,
        role,
    )


def _file_binding(path: Path, root: Path, digest: str) -> JsonObject:
    relative = (
        path.resolve(strict=True).relative_to(root.resolve(strict=True)).as_posix()
    )
    return {"path": relative, "sha256": digest}
