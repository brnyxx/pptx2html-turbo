from __future__ import annotations

import os
import re
import shutil
import stat
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_candidate_fonts import (
    CandidateFontError,
    validate_font_bundle,
)
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_snapshot_publish import SnapshotPublishError, publish_snapshot
from evaluate.multiformat_strict_json import read_strict_object

_FONT_NAME = re.compile(r"^(\d{4})-([0-9a-f]{64})(\.otf|\.ttf)$")


@dataclass(frozen=True, slots=True)
class FontSnapshotError(Exception):
    reason: str
    path: Path | None = None

    def __str__(self) -> str:
        return f"{self.reason}: {self.path}" if self.path is not None else self.reason


@dataclass(frozen=True, slots=True)
class FontSource:
    source: Path
    digest: str
    suffix: str


@dataclass(frozen=True, slots=True)
class FontSnapshotSummary:
    files: int
    fonts: int
    manifest_sha256: str
    environment_sha256: str


def generate_font_snapshot(
    font_dirs: Sequence[Path],
    output_dir: Path,
) -> FontSnapshotSummary:
    try:
        sources = _discover_fonts(font_dirs)
        summary: FontSnapshotSummary | None = None

        def write_snapshot(staging: Path) -> None:
            nonlocal summary
            _write_snapshot(staging, sources)
            summary = validate_font_snapshot(staging / "font-bundle.json", staging)

        publish_snapshot(output_dir, write_snapshot)
        if summary is None:
            raise FontSnapshotError("font snapshot summary is missing")
        return summary
    except (
        OSError,
        SnapshotPublishError,
        TypeError,
        ValueError,
    ) as error:
        raise FontSnapshotError("font snapshot generation failed") from error


def validate_font_snapshot(
    manifest_path: Path,
    snapshot_root: Path,
) -> FontSnapshotSummary:
    try:
        root = _strict_path(snapshot_root, directory=True)
        manifest = _strict_path(manifest_path, directory=False)
        if manifest != (root / "font-bundle.json").resolve(strict=True):
            raise FontSnapshotError(
                "manifest is not the canonical snapshot manifest", manifest
            )
        if {entry.name for entry in root.iterdir()} != {"font-bundle.json", "fonts"}:
            raise FontSnapshotError("font snapshot has an unexpected root entry", root)
        if not stat.S_ISDIR((root / "fonts").lstat().st_mode):
            raise FontSnapshotError(
                "font snapshot fonts entry is not a directory", root / "fonts"
            )
        values = read_strict_object(manifest)
        require_keys(values, {"schema_version", "fonts"}, "font_bundle")
        if manifest.read_bytes() != canonicalize(values) + b"\n":
            raise FontSnapshotError("font bundle is not canonical JCS", manifest)
        if integer_value(values, "schema_version") != 1:
            raise FontSnapshotError("font bundle schema mismatch", manifest)
        fonts = object_list(values, "fonts", "font_bundle.fonts")
        if not fonts:
            raise FontSnapshotError("font bundle is empty", manifest)
        _validate_manifest_entries(root, manifest, fonts)
        environment_sha256 = validate_font_bundle(manifest)
        return FontSnapshotSummary(
            files=1 + len(fonts),
            fonts=len(fonts),
            manifest_sha256=sha256_file(manifest),
            environment_sha256=environment_sha256,
        )
    except (
        CandidateFontError,
        CorpusError,
        OSError,
        TypeError,
        ValueError,
    ) as error:
        raise FontSnapshotError(
            "font snapshot validation failed", manifest_path
        ) from error


def _discover_fonts(font_dirs: Sequence[Path]) -> tuple[FontSource, ...]:
    sources: list[FontSource] = []
    identities: set[tuple[int, int]] = set()
    digests: set[str] = set()
    for font_dir in font_dirs:
        root = _strict_path(font_dir, directory=True)
        state = (sources, identities, digests)
        _walk_font_root(root, root, state)
    if not sources:
        raise FontSnapshotError("font roots contain no fonts")
    return tuple(sorted(sources, key=lambda item: (item.digest, item.suffix)))


def _walk_font_root(
    root: Path,
    current: Path,
    state: tuple[list[FontSource], set[tuple[int, int]], set[str]],
) -> None:
    sources, identities, digests = state
    with os.scandir(current) as entries:
        children = sorted(entries, key=lambda entry: entry.name)
    for entry in children:
        path = Path(entry.path)
        information = entry.stat(follow_symlinks=False)
        mode = information.st_mode
        if stat.S_ISLNK(mode):
            raise FontSnapshotError("font roots may not contain links", path)
        if stat.S_ISDIR(mode):
            _walk_font_root(root, path, state)
            continue
        if not stat.S_ISREG(mode) or information.st_nlink != 1:
            raise FontSnapshotError(
                "font root contains a non-regular or linked file", path
            )
        suffix = path.suffix.lower()
        if suffix not in {".ttf", ".otf"}:
            raise FontSnapshotError("unsupported font suffix", path)
        resolved = path.resolve(strict=True)
        if not resolved.is_relative_to(root):
            raise FontSnapshotError("font escapes its root", path)
        identity = (information.st_dev, information.st_ino)
        if identity in identities:
            raise FontSnapshotError("font inode is repeated", path)
        identities.add(identity)
        digest = sha256_file(resolved)
        if digest in digests:
            raise FontSnapshotError("font digest is repeated", path)
        digests.add(digest)
        sources.append(FontSource(resolved, digest, suffix))


def _write_snapshot(staging: Path, sources: Sequence[FontSource]) -> None:
    font_root = staging / "fonts"
    font_root.mkdir()
    entries: list[JsonValue] = []
    for ordinal, source in enumerate(sources, start=1):
        destination = font_root / f"{ordinal:04d}-{source.digest}{source.suffix}"
        information = source.source.stat(follow_symlinks=False)
        if (
            not stat.S_ISREG(information.st_mode)
            or information.st_nlink != 1
            or sha256_file(source.source) != source.digest
        ):
            raise FontSnapshotError("font source changed", source.source)
        shutil.copyfile(source.source, destination)
        if sha256_file(destination) != source.digest:
            raise FontSnapshotError("copied font digest differs", destination)
        entries.append(
            {
                "path": destination.relative_to(staging).as_posix(),
                "sha256": source.digest,
            }
        )
    manifest = staging / "font-bundle.json"
    values: dict[str, JsonValue] = {"fonts": entries, "schema_version": 1}
    manifest.write_bytes(canonicalize(values) + b"\n")


def _strict_path(path: Path, *, directory: bool) -> Path:
    information = path.lstat()
    valid = (
        stat.S_ISDIR(information.st_mode)
        if directory
        else (stat.S_ISREG(information.st_mode) and information.st_nlink == 1)
    )
    if not valid:
        raise FontSnapshotError("expected a valid snapshot path", path)
    return path.resolve(strict=True)


def _validate_manifest_entries(
    root: Path,
    manifest: Path,
    fonts: list[dict[str, JsonValue]],
) -> None:
    manifest_information = manifest.lstat()
    identities = {(manifest_information.st_dev, manifest_information.st_ino)}
    seen_digests: set[str] = set()
    expected_names: set[str] = set()
    previous: tuple[str, str] | None = None
    for ordinal, font in enumerate(fonts, start=1):
        require_keys(font, {"path", "sha256"}, "font_bundle.font")
        relative = string_value(font, "path")
        parts = relative.split("/")
        if (
            not relative
            or relative.startswith("/")
            or "\\" in relative
            or "\x00" in relative
            or any(not part or part in {".", ".."} for part in parts)
        ):
            raise FontSnapshotError(
                "font bundle path is not a safe POSIX-relative path"
            )
        digest = sha256_value(font, "sha256")
        match = _FONT_NAME.fullmatch(parts[-1]) if parts[:-1] == ["fonts"] else None
        if match is None or int(match.group(1)) != ordinal or match.group(2) != digest:
            raise FontSnapshotError(
                "font bundle path does not match its ordinal and digest", manifest
            )
        suffix = match.group(3)
        ordering = (digest, suffix)
        if previous is not None and ordering < previous:
            raise FontSnapshotError(
                "font bundle fonts are not canonically ordered", manifest
            )
        previous = ordering
        if digest in seen_digests:
            raise FontSnapshotError("font bundle digest is repeated", manifest)
        seen_digests.add(digest)
        expected_names.add(parts[-1])
        candidate = root.joinpath(*parts)
        resolved = _strict_path(candidate, directory=False)
        if not resolved.is_relative_to(root) or resolved != candidate:
            raise FontSnapshotError(
                "font bundle path escapes its snapshot root", candidate
            )
        information = resolved.lstat()
        identity = (information.st_dev, information.st_ino)
        if identity in identities:
            raise FontSnapshotError("font snapshot reuses a file inode", candidate)
        identities.add(identity)
        if sha256_file(candidate) != digest:
            raise FontSnapshotError("font bundle file hash mismatch", candidate)
    actual_names = {entry.name for entry in (root / "fonts").iterdir()}
    if actual_names != expected_names:
        raise FontSnapshotError(
            "font bundle file set differs from its manifest", root / "fonts"
        )
