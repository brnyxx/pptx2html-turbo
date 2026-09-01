from __future__ import annotations

import json
import os
import shlex
import shutil
import tempfile
from pathlib import Path

from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    run_bounded_process,
)
from evaluate.multiformat_portable_package_inventory import PortableLockIoError
from evaluate.multiformat_schema import JsonValue, sha256_file, string_value
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.multiformat_subprocess import clean_subprocess_environment

MAX_VERSION_BYTES = 1024 * 1024


class PortableLockMaterializeError(ValueError):
    pass


class PortableLockIncompleteError(PortableLockMaterializeError):
    pass


def validate_candidate_locks(browser_path: Path, runtime_path: Path) -> None:
    browser = read_strict_object(browser_path)
    expected = {
        "chromium",
        "executable_sha256",
        "playwright",
        "os",
        "architecture",
        "font_environment_sha256",
        "viewport_width",
        "viewport_height",
        "device_scale_factor",
        "locale",
        "timezone",
        "color_profile",
        "reduced_motion",
        "animations",
    }
    runtime = read_strict_object(runtime_path)
    if set(browser) != expected:
        raise PortableLockIoError("portable browser lock is incomplete")
    schema = runtime.get("schema_version")
    if (
        set(runtime)
        != {
            "schema_version",
            "status",
            "browser",
            "candidate_runtime",
            "sandbox_verifier",
            "font_bundle_sha256",
        }
        or schema not in {1, 2}
        or runtime.get("status") != "locked"
    ):
        raise PortableLockIoError("portable candidate runtime lock is incomplete")
    candidate = runtime.get("candidate_runtime")
    verifier = runtime.get("sandbox_verifier")
    candidate_fields = {
        "build_revision",
        "converter_sha256",
        "converter_version",
        "soffice_sha256",
        "soffice_version",
        "pdftohtml_sha256",
        "pdftohtml_version",
        "pdfinfo_sha256",
        "pdfinfo_version",
        "receipt_signer_sha256",
        "receipt_signer_version",
    }
    verifier_fields = {
        "algorithm",
        "verifier_id",
        "public_key_sha256",
        "openssl_sha256",
    }
    if schema == 2:
        candidate_fields.add("poppler_package_inventory_sha256")
        verifier_fields.add("openssl_package_inventory_sha256")
    if not isinstance(candidate, dict) or set(candidate) != candidate_fields:
        raise PortableLockIoError("portable candidate runtime lock is incomplete")
    if not isinstance(verifier, dict) or set(verifier) != verifier_fields:
        raise PortableLockIoError("portable candidate runtime lock is incomplete")


def validate_candidate_artifacts(
    runtime_path: Path, paths: dict[str, Path], versions: dict[str, str], revision: str
) -> None:
    candidate = read_strict_object(runtime_path)
    runtime = candidate.get("candidate_runtime")
    verifier = candidate.get("sandbox_verifier")
    browser = read_strict_object(paths["browser-lock"])
    if (
        candidate.get("browser") != browser
        or browser.get("chromium") != versions["chromium"]
        or browser.get("executable_sha256") != sha256_file(paths["chromium"])
    ):
        raise PortableLockIoError("portable browser runtime lock differs")
    if (
        not isinstance(runtime, dict)
        or not isinstance(verifier, dict)
        or runtime.get("build_revision") != revision
    ):
        raise PortableLockIoError("portable candidate runtime lock is incomplete")
    tools = {
        "converter": "converter",
        "soffice": "libreoffice",
        "pdftohtml": "pdftohtml",
        "pdfinfo": "poppler-metadata",
        "receipt_signer": "receipt-signer",
    }
    for name, key in tools.items():
        if (
            runtime.get(f"{name}_sha256") != sha256_file(paths[key])
            or runtime.get(f"{name}_version") != versions[key]
        ):
            raise PortableLockIoError("portable candidate tool lock differs")
    if candidate.get("font_bundle_sha256") != sha256_file(paths["font-bundle"]):
        raise PortableLockIoError("portable candidate font lock differs")
    if verifier.get("public_key_sha256") != sha256_file(
        paths["candidate-sandbox-public-key"]
    ) or verifier.get("openssl_sha256") != sha256_file(paths["openssl"]):
        raise PortableLockIoError("portable candidate sandbox lock differs")
    if candidate.get("schema_version") == 2:
        if (
            "poppler-package-inventory" not in paths
            or "openssl-package-inventory" not in paths
        ):
            raise PortableLockIoError("portable candidate package inventory is missing")
        if runtime.get("poppler_package_inventory_sha256") != sha256_file(
            paths["poppler-package-inventory"]
        ):
            raise PortableLockIoError("portable Poppler package lock differs")
        if verifier.get("openssl_package_inventory_sha256") != sha256_file(
            paths["openssl-package-inventory"]
        ):
            raise PortableLockIoError("portable OpenSSL package lock differs")


def bind_corpus(source: Path, root: Path, destination_root: Path) -> Path:
    resolved = source.resolve(strict=True)
    if resolved.is_relative_to(root):
        return resolved
    document_format = string_value(read_strict_object(resolved), "format")
    destination = destination_root / document_format
    shutil.copytree(resolved.parent, destination)
    return destination / resolved.name


def bind_font_bundle(source: Path, root: Path, destination: Path) -> Path:
    resolved = source.resolve(strict=True)
    if resolved.is_file():
        return bind_file(resolved, root, destination)
    if not resolved.is_dir():
        raise PortableLockIoError("portable font bundle is unavailable")
    copied = destination / "sources"
    shutil.copytree(resolved, copied, symlinks=False)
    manifest = destination / "manifest.json"
    entries = [
        {"path": item.relative_to(copied).as_posix(), "sha256": sha256_file(item)}
        for item in sorted(copied.rglob("*"))
        if item.is_file()
    ]
    if not entries:
        raise PortableLockIoError("portable font bundle is empty")
    manifest.write_text(
        json.dumps({"entries": entries}, sort_keys=True) + "\n", encoding="utf-8"
    )
    return manifest


def bind_file(source: Path, root: Path, destination: Path) -> Path:
    resolved = source.resolve(strict=True)
    if resolved.is_relative_to(root):
        return resolved
    shutil.copyfile(resolved, destination)
    destination.chmod(resolved.stat().st_mode & 0o777)
    return destination


def tool_version(path: Path, arguments: tuple[str, ...]) -> str:
    try:
        with tempfile.TemporaryDirectory(prefix="portable-version-") as temporary:
            root = Path(temporary)
            stdout, stderr = root / "stdout", root / "stderr"
            code = run_bounded_process(
                (path.as_posix(), *arguments),
                root,
                clean_subprocess_environment(),
                stdout,
                stderr,
                timeout_seconds=15,
                max_log_bytes=MAX_VERSION_BYTES,
            )
            output = stdout.read_bytes() + stderr.read_bytes()
    except (CandidateProcessError, OSError) as error:
        raise PortableLockIoError("portable tool version probe failed") from error
    if code != 0 or len(output) > MAX_VERSION_BYTES:
        raise PortableLockIoError("portable tool version probe failed")
    lines = [
        line.strip()
        for line in output.decode("utf-8", errors="strict").splitlines()
        if line.strip()
    ]
    if not lines:
        raise PortableLockIoError("portable tool version is empty")
    return lines[0]


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


def write_sandbox_profile(path: Path) -> None:
    path.write_text(sandbox_profile_text(), encoding="utf-8")


def sandbox_profile_text() -> str:
    return (
        "\n".join(
            (
                "(version 1)",
                "(allow default)",
                "(deny network*)",
                '(if (param "LIBREOFFICE")',
                '  (with-filter (process-path (param "LIBREOFFICE"))',
                "    (allow network-bind",
                '      (local unix-socket (regex #"^/private/tmp/OSL_PIPE_[0-9]+_SingleOfficeIPC_[0-9a-f]+$")))))',
                '(if (param "CHROMIUM")',
                '  (with-filter (process-path (param "CHROMIUM"))',
                "    (allow network-bind",
                '      (local unix-socket (regex #"^/private/var/folders/[A-Za-z0-9_]+/[A-Za-z0-9_]+/T/com[.]google[.]chrome[.]for[.]testing[.][A-Za-z0-9]+/SingletonSocket$")))))',
                '(deny file-read* (subpath (param "ORACLE_ROOT")))',
            )
        )
        + "\n"
    )


def write_sandbox_wrapper(path: Path, executable: Path) -> None:
    value = f'#!/bin/sh\nexec {shlex.quote(executable.resolve(strict=True).as_posix())} "$@"\n'
    exclusive_write(path, value.encode(), 0o755)


def exclusive_write(path: Path, value: bytes, mode: int) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
    try:
        os.write(descriptor, value)
    finally:
        os.close(descriptor)
