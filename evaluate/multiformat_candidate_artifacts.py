from __future__ import annotations

import json
import os
import shutil
import stat
from functools import partial
from pathlib import Path

from evaluate.multiformat_candidate_fonts import snapshot_font_environment
from evaluate.multiformat_candidate_types import (
    CandidateCaptureError,
    RuntimeArtifactSnapshots,
)
from evaluate.multiformat_schema import JsonValue, sha256_file


class CandidateArtifactError(CandidateCaptureError):
    pass


def evidence_binding(root: Path, path: Path) -> dict[str, JsonValue]:
    resolved = path.resolve(strict=True)
    return {
        "path": resolved.relative_to(root).as_posix(),
        "sha256": sha256_file(resolved),
    }


def write_canonical_json(path: Path, value: JsonValue) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def materialize_runtime_artifacts(
    artifacts: dict[str, Path],
    evidence_root: Path,
    output_dir: Path,
) -> RuntimeArtifactSnapshots:
    evidence_root = evidence_root.resolve(strict=True)
    if output_dir.exists() and any(output_dir.iterdir()):
        raise CandidateArtifactError(
            f"runtime artifact output is not empty: {output_dir}"
        )
    if output_dir.is_symlink():
        raise CandidateArtifactError("runtime artifact output is a symlink")
    output_dir.mkdir(parents=True, exist_ok=True)
    result: dict[str, Path] = {}
    package_entries: list[JsonValue] = []
    copied_packages: dict[Path, Path] = {}
    font_config: Path | None = None
    for name, source_value in sorted(artifacts.items()):
        source = _regular_source(source_value)
        if name == "font_bundle":
            font_root = output_dir / "font_bundle-package" / "font-environment"
            font_bundle, environment = snapshot_font_environment(
                source,
                font_root,
                _copy_regular,
            )
            result[name] = font_bundle
            font_config = environment.config_path
            package_entries.extend(_package_entries(font_root, evidence_root))
            continue
        if name == "font_config" and font_config is not None:
            result[name] = font_config
            continue
        package_root = _package_root(name, source)
        if package_root is not None and output_dir.resolve().is_relative_to(
            package_root
        ):
            package_root = None
        if package_root is not None:
            destination_root = copied_packages.get(package_root)
            if destination_root is None:
                destination_root = output_dir / f"{name}-package" / package_root.name
                _copy_package(package_root, destination_root)
                copied_packages[package_root] = destination_root
                package_entries.extend(
                    _package_entries(destination_root, evidence_root)
                )
            result[name] = destination_root / source.relative_to(package_root)
            continue
        destination = output_dir / name
        _copy_regular(source, destination)
        result[name] = destination
    package_manifest = output_dir / "runtime-package-manifest.json"
    package_manifest_value: dict[str, JsonValue] = {
        "schema_version": 1,
        "entries": package_entries,
    }
    write_canonical_json(package_manifest, package_manifest_value)
    result["runtime_package_manifest"] = package_manifest
    _seal_snapshot_tree(output_dir)
    return RuntimeArtifactSnapshots.capture(output_dir, result)


def _seal_snapshot_tree(root: Path) -> None:
    for path in sorted(root.rglob("*"), reverse=True):
        value = path.lstat()
        path.chmod(stat.S_IMODE(value.st_mode) & ~0o222)
    value = root.lstat()
    root.chmod(stat.S_IMODE(value.st_mode) & ~0o222)


def _regular_source(source_value: Path) -> Path:
    supplied = source_value.lstat()
    if stat.S_ISLNK(supplied.st_mode):
        raise CandidateArtifactError(
            f"runtime artifact path is symlinked: {source_value}"
        )
    resolved = source_value.resolve(strict=True)
    value = resolved.lstat()
    if not stat.S_ISREG(value.st_mode):
        raise CandidateArtifactError(
            f"runtime artifact is not a regular file: {resolved}"
        )
    return resolved


def _package_root(name: str, source: Path) -> Path | None:
    if name not in {"chromium_binary", "soffice_binary"}:
        return None
    for parent in [source, *source.parents]:
        if parent.suffix == ".app":
            return parent
    if name == "chromium_binary":
        return source.parent
    for parent in source.parents:
        if parent.name.lower() == "libreoffice":
            return parent
    return None


def _copy_package(source: Path, destination: Path) -> None:
    before = _source_tree_identity(source)
    try:
        shutil.copytree(
            source,
            destination,
            copy_function=partial(_copy_package_regular, package_root=source),
        )
    except (OSError, shutil.Error) as error:
        raise CandidateArtifactError(
            f"cannot snapshot runtime package: {source}"
        ) from error
    if _source_tree_identity(source) != before:
        raise CandidateArtifactError(f"runtime package changed while copying: {source}")


def _copy_package_regular(
    source: str | Path,
    destination: str | Path,
    *,
    package_root: Path,
) -> str:
    resolved = Path(source).resolve(strict=True)
    if not resolved.is_relative_to(package_root):
        raise CandidateArtifactError(f"runtime package symlink escapes root: {source}")
    return _copy_regular(resolved, destination)


def _copy_regular(source_value: str | Path, destination_value: str | Path) -> str:
    source = Path(source_value)
    destination = Path(destination_value)
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    source_descriptor = os.open(source, flags)
    try:
        before = os.fstat(source_descriptor)
        if not stat.S_ISREG(before.st_mode):
            raise CandidateArtifactError(f"runtime artifact is not regular: {source}")
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination_descriptor = os.open(
            destination,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL,
            stat.S_IMODE(before.st_mode),
        )
        try:
            while chunk := os.read(source_descriptor, 1024 * 1024):
                _write_all(destination_descriptor, chunk)
            os.fsync(destination_descriptor)
        finally:
            os.close(destination_descriptor)
        after = os.fstat(source_descriptor)
    finally:
        os.close(source_descriptor)
    if _stat_identity(before) != _stat_identity(after):
        raise CandidateArtifactError(
            f"runtime artifact changed while copying: {source}"
        )
    copied = destination.lstat()
    if not stat.S_ISREG(copied.st_mode) or copied.st_nlink != 1:
        raise CandidateArtifactError(f"runtime snapshot is not private: {destination}")
    destination.chmod(stat.S_IMODE(before.st_mode) & ~0o222)
    return destination.as_posix()


def _write_all(descriptor: int, value: bytes) -> None:
    offset = 0
    while offset < len(value):
        offset += os.write(descriptor, value[offset:])


def _source_tree_identity(root: Path) -> tuple[tuple[str, tuple[int, ...]], ...]:
    values: list[tuple[str, tuple[int, ...]]] = []
    for path in [root, *sorted(root.rglob("*"), key=lambda item: item.as_posix())]:
        information = path.lstat()
        relative = "." if path == root else path.relative_to(root).as_posix()
        if stat.S_ISLNK(information.st_mode):
            target = path.resolve(strict=True)
            if not target.is_relative_to(root):
                raise CandidateArtifactError(
                    f"runtime package symlink escapes root: {relative}"
                )
        if not (
            stat.S_ISREG(information.st_mode)
            or stat.S_ISDIR(information.st_mode)
            or stat.S_ISLNK(information.st_mode)
        ):
            raise CandidateArtifactError(
                f"runtime package contains special file: {relative}"
            )
        values.append((relative, _stat_identity(information)))
    return tuple(values)


def _stat_identity(value: os.stat_result) -> tuple[int, ...]:
    return (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_nlink,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def _package_entries(root: Path, evidence_root: Path) -> list[dict[str, JsonValue]]:
    entries: list[dict[str, JsonValue]] = []
    for path in sorted(root.rglob("*")):
        if path.is_file():
            entries.append(
                {
                    "path": path.relative_to(evidence_root).as_posix(),
                    "sha256": sha256_file(path),
                }
            )
    return entries
