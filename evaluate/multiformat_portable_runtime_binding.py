from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

from evaluate import multiformat_portable_lock_io as lock_io
from evaluate import multiformat_portable_package_inventory as package_io
from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_portable_native_package import bind_homebrew_package_closure
from evaluate.multiformat_schema import object_value, sha256_file
from evaluate.multiformat_strict_json import read_strict_object


class PortableRuntimeInputs(Protocol):
    @property
    def pdftoppm(self) -> Path: ...

    @property
    def pdftotext(self) -> Path: ...

    @property
    def pdfinfo(self) -> Path: ...

    @property
    def pdftohtml(self) -> Path: ...

    @property
    def openssl(self) -> Path: ...

    @property
    def canonicalizer(self) -> Path: ...

    @property
    def configuration(self) -> Path: ...

    @property
    def browser_lock(self) -> Path: ...

    @property
    def candidate_runtime_lock(self) -> Path: ...

    @property
    def converter(self) -> Path: ...

    @property
    def receipt_signer(self) -> Path: ...

    @property
    def candidate_sandbox_public_key(self) -> Path: ...

    @property
    def executor(self) -> Path: ...

    @property
    def sandbox_exec(self) -> Path: ...

    @property
    def contract(self) -> Path: ...

    @property
    def evaluator(self) -> Path: ...


@dataclass(frozen=True, slots=True)
class BoundPortableRuntime:
    paths: dict[str, Path]
    versions: dict[str, str]
    inventories: dict[str, Path]


def bind_portable_runtime(
    inputs: PortableRuntimeInputs, root: Path, artifacts: Path
) -> BoundPortableRuntime:
    """Bind flat artifacts and relocatable Homebrew package closures."""
    flat = {
        "canonicalizer": inputs.canonicalizer,
        "configuration": inputs.configuration,
        "browser-lock": inputs.browser_lock,
        "converter": inputs.converter,
        "receipt-signer": inputs.receipt_signer,
        "candidate-sandbox-public-key": inputs.candidate_sandbox_public_key,
        "executor": inputs.executor,
        "sandbox-exec": inputs.sandbox_exec,
        "contract": inputs.contract,
        "evaluator": inputs.evaluator,
    }
    paths = {
        name: lock_io.bind_file(path, root, artifacts / name)
        for name, path in flat.items()
    }
    poppler = _bind_native_or_flat(
        (
            inputs.pdftoppm,
            inputs.pdftotext,
            inputs.pdfinfo,
            inputs.pdftohtml,
        ),
        root,
        artifacts / "poppler-package",
    )
    for name, path in zip(
        ("poppler-render", "poppler-text", "poppler-metadata", "pdftohtml"),
        poppler.executables,
        strict=True,
    ):
        paths[name] = path
    openssl = _bind_native_or_flat(
        (inputs.openssl,), root, artifacts / "openssl-package"
    )
    paths["openssl"] = openssl.executables[0]
    versions = {
        "poppler-render": lock_io.tool_version(paths["poppler-render"], ("-v",)),
        "poppler-text": lock_io.tool_version(paths["poppler-text"], ("-v",)),
        "poppler-metadata": lock_io.tool_version(
            paths["poppler-metadata"], ("-v",)
        ),
        "converter": lock_io.tool_version(paths["converter"], ("--version",)),
        "pdftohtml": lock_io.tool_version(paths["pdftohtml"], ("-v",)),
        "openssl": lock_io.tool_version(paths["openssl"], ("version",)),
        "receipt-signer": lock_io.tool_version(
            paths["receipt-signer"], ("--version",)
        ),
    }
    inventories = {}
    if poppler.inventory is not None:
        inventories["poppler"] = poppler.inventory
        paths["poppler-package-inventory"] = poppler.inventory
    if openssl.inventory is not None:
        inventories["openssl"] = openssl.inventory
        paths["openssl-package-inventory"] = openssl.inventory
    candidate_destination = artifacts / "candidate-runtime-lock"
    if not inventories:
        paths["candidate-runtime-lock"] = lock_io.bind_file(
            inputs.candidate_runtime_lock, root, candidate_destination
        )
        return BoundPortableRuntime(paths, versions, inventories)
    if set(inventories) != {"poppler", "openssl"}:
        raise package_io.PortableLockIoError(
            "portable candidate native package closure is incomplete"
        )
    value = read_strict_object(inputs.candidate_runtime_lock)
    value["schema_version"] = 2
    candidate = object_value(value, "candidate_runtime")
    candidate["pdftohtml_sha256"] = sha256_file(paths["pdftohtml"])
    candidate["pdfinfo_sha256"] = sha256_file(paths["poppler-metadata"])
    candidate["poppler_package_inventory_sha256"] = sha256_file(
        inventories["poppler"]
    )
    verifier = object_value(value, "sandbox_verifier")
    verifier["openssl_sha256"] = sha256_file(paths["openssl"])
    verifier["openssl_package_inventory_sha256"] = sha256_file(
        inventories["openssl"]
    )
    write_canonical_json(candidate_destination, value)
    paths["candidate-runtime-lock"] = candidate_destination
    return BoundPortableRuntime(paths, versions, inventories)


@dataclass(frozen=True, slots=True)
class _OptionalClosure:
    executables: tuple[Path, ...]
    inventory: Path | None


def _bind_native_or_flat(
    sources: tuple[Path, ...], root: Path, destination: Path
) -> _OptionalClosure:
    closure = bind_homebrew_package_closure(sources, root, destination)
    if closure is not None:
        return _OptionalClosure(closure.executables, closure.inventory)
    paths = tuple(
        lock_io.bind_file(source, root, destination.with_name(f"{destination.name}-{i}"))
        for i, source in enumerate(sources)
    )
    return _OptionalClosure(paths, None)
