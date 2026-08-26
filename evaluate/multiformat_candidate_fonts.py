from __future__ import annotations

import hashlib
import html
import json
import shutil
import xml.etree.ElementTree as ET
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_types import CandidateCaptureError
from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_schema import (
    integer_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class CandidateFontError(CandidateCaptureError):
    pass


@dataclass(frozen=True, slots=True)
class CandidateFontEnvironment:
    manifest_sha256: str
    environment_sha256: str
    config_path: Path


def prepare_font_environment(
    font_bundle: Path,
    runtime_root: Path,
) -> CandidateFontEnvironment:
    environment_hash, fonts = _validated_fonts(font_bundle)
    resolved = font_bundle.resolve(strict=True)
    if runtime_root.exists():
        shutil.rmtree(runtime_root)
    runtime_root.mkdir(parents=True, exist_ok=True)
    font_directory = runtime_root / "fonts"
    font_directory.mkdir()
    for index, font_path in enumerate(fonts, start=1):
        digest = sha256_file(font_path)
        destination = font_directory / (
            f"{index:04}-{digest}{font_path.suffix.lower()}"
        )
        shutil.copyfile(font_path, destination)
    cache = runtime_root / "cache"
    cache.mkdir(exist_ok=True)
    config = runtime_root / "fonts.conf"
    directory_xml = f"<dir>{html.escape(font_directory.as_posix())}</dir>"
    config.write_text(
        '<?xml version="1.0"?>'
        '<!DOCTYPE fontconfig SYSTEM "fonts.dtd">'
        f"<fontconfig>{directory_xml}<cachedir>{html.escape(cache.as_posix())}</cachedir>"
        "<config><rescan><int>0</int></rescan></config></fontconfig>",
        encoding="utf-8",
    )
    return CandidateFontEnvironment(
        sha256_file(resolved),
        environment_hash,
        config,
    )


def snapshot_font_environment(
    font_bundle: Path,
    snapshot_root: Path,
    copy_file: Callable[[Path, Path], str],
) -> tuple[Path, CandidateFontEnvironment]:
    _environment_hash, fonts = _validated_fonts(font_bundle)
    bundle_root = snapshot_root / "bundle"
    copied_manifest = bundle_root / font_bundle.name
    copy_file(font_bundle, copied_manifest)
    for font in fonts:
        copy_file(font, bundle_root / font.relative_to(font_bundle.parent))
    return copied_manifest, prepare_font_environment(
        copied_manifest,
        snapshot_root / "runtime",
    )


def validate_font_bundle(font_bundle: Path) -> str:
    environment_hash, _fonts = _validated_fonts(font_bundle)
    return environment_hash


def validate_font_config(
    font_bundle: Path,
    config_path: Path,
    evidence_root: Path,
) -> None:
    _environment_hash, originals = _validated_fonts(font_bundle)
    try:
        root = ET.parse(config_path).getroot()
    except (OSError, ET.ParseError) as error:
        raise CandidateFontError("font config is invalid") from error
    directories = root.findall("dir")
    if len(directories) != 1 or not directories[0].text:
        raise CandidateFontError("font config must contain one font directory")
    font_directory = Path(directories[0].text).resolve(strict=True)
    if not font_directory.is_relative_to(evidence_root.resolve(strict=True)):
        raise CandidateFontError("font config directory escapes evidence root")
    copied = sorted(
        path
        for path in font_directory.iterdir()
        if path.is_file() and not path.is_symlink()
    )
    if len(copied) != len(list(font_directory.iterdir())):
        raise CandidateFontError("font config directory contains invalid entries")
    if sorted(sha256_file(path) for path in copied) != sorted(
        sha256_file(path) for path in originals
    ):
        raise CandidateFontError("font config does not match the locked bundle")


def _validated_fonts(font_bundle: Path) -> tuple[str, list[Path]]:
    resolved = font_bundle.resolve(strict=True)
    values = read_strict_object(resolved)
    require_keys(values, {"schema_version", "fonts"}, "font_bundle")
    if integer_value(values, "schema_version") != 1:
        raise CandidateFontError("font bundle schema mismatch")
    fonts = object_list(values, "fonts", "font_bundle.fonts")
    if not fonts:
        raise CandidateFontError("font bundle is empty")
    entries: list[dict[str, str]] = []
    font_paths: list[Path] = []
    for font in fonts:
        require_keys(font, {"path", "sha256"}, "font_bundle.font")
        relative = string_value(font, "path")
        font_path = resolve_evidence_path(resolved.parent, relative)
        digest = sha256_file(font_path)
        if digest != sha256_value(font, "sha256"):
            raise CandidateFontError("font bundle file hash mismatch")
        entries.append({"path": relative, "sha256": digest})
        font_paths.append(font_path)
    environment_hash = hashlib.sha256(
        json.dumps(
            entries,
            ensure_ascii=True,
            sort_keys=True,
            separators=(",", ":"),
        ).encode()
    ).hexdigest()
    return environment_hash, font_paths
