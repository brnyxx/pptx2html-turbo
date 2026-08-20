from __future__ import annotations

import json
import os
import shutil
from pathlib import Path

from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.multiformat_candidate_types import CandidateCaptureError


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
) -> dict[str, Path]:
    if output_dir.exists() and any(output_dir.iterdir()):
        raise CandidateArtifactError(
            f"runtime artifact output is not empty: {output_dir}"
        )
    output_dir.mkdir(parents=True, exist_ok=True)
    result: dict[str, Path] = {}
    package_entries: list[dict[str, JsonValue]] = []
    for name, source_value in sorted(artifacts.items()):
        source = source_value.resolve(strict=True)
        if source.is_relative_to(evidence_root):
            result[name] = source
            continue
        package_root = _package_root(name, source)
        if package_root is not None:
            destination_root = output_dir / f"{name}-package" / package_root.name
            shutil.copytree(
                package_root,
                destination_root,
                symlinks=True,
                copy_function=_link_or_copy,
            )
            result[name] = destination_root / source.relative_to(package_root)
            package_entries.extend(_package_entries(destination_root, evidence_root))
            continue
        destination = output_dir / name
        _link_or_copy(source, destination)
        result[name] = destination
    package_manifest = output_dir / "runtime-package-manifest.json"
    write_canonical_json(
        package_manifest,
        {"schema_version": 1, "entries": package_entries},
    )
    result["runtime_package_manifest"] = package_manifest
    return result


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


def _link_or_copy(source: str | Path, destination: str | Path) -> str:
    try:
        os.link(source, destination)
    except OSError:
        shutil.copy2(source, destination)
    return Path(destination).as_posix()


def _package_entries(root: Path, evidence_root: Path) -> list[dict[str, JsonValue]]:
    entries: list[dict[str, JsonValue]] = []
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(evidence_root).as_posix()
        if path.is_symlink():
            target = path.resolve(strict=True)
            if not target.is_relative_to(root):
                raise CandidateArtifactError(
                    f"package symlink escapes root: {relative}"
                )
            entries.append(
                {
                    "path": relative,
                    "symlink": path.readlink().as_posix(),
                }
            )
        elif path.is_file():
            entries.append(
                {
                    "path": relative,
                    "sha256": sha256_file(path),
                }
            )
    return entries
