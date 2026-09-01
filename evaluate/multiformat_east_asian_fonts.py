"""Shared east-Asian font substitution policy for reference and candidate.

LibreOffice bundles no CJK-capable font and its macOS backend contains no
fontconfig, so a missing glyph resolves through CoreText
``CTFontCreateForString``. That fallback is not stable across processes: a
document naming an absent family such as ``Noto Sans CJK KR`` selects a
different host face per run, changing the embedded font name and the glyph
advances and therefore the produced bytes.

Both the signed reference producer and the shipped native converter pin those
families with LibreOffice's documented font replacement table
(``org.openoffice.Office.Common/Font/Substitution``), seeded into the private
per-conversion profile before first launch.

The family list and the substitute candidates live in
``multiformat/east-asian-font-policy.v1.json`` so the two implementations
cannot drift. ``crates/document2html-native/src/fonts.rs`` holds the Rust
side; both are pinned to this fixture and to the shared golden registry by
parity tests.

https://help.libreoffice.org/latest/en-US/text/shared/optionen/01010700.html
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from pathlib import Path
from typing import Final, TypeAlias

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    sha256_file,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

JsonBinding: TypeAlias = dict[str, JsonValue]
JsonObject: TypeAlias = dict[str, JsonValue]

POLICY_PATH: Final = (
    Path(__file__).parent / "multiformat/east-asian-font-policy.v1.json"
)

_SCHEMA_VERSION: Final = 1
_HEADER: Final = (
    '<?xml version="1.0" encoding="UTF-8"?>\n'
    '<oor:items xmlns:oor="http://openoffice.org/2001/registry"'
    ' xmlns:xs="http://www.w3.org/2001/XMLSchema"'
    ' xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">\n'
    '<item oor:path="/org.openoffice.Office.Common/Font/Substitution">'
    '<prop oor:name="Replacement" oor:op="fuse"><value>true</value></prop>'
    "</item>\n"
)
_FOOTER: Final = "</oor:items>\n"


class EastAsianFontError(ValueError):
    """The policy fixture, or the host font it selects, is unusable."""


@dataclass(frozen=True, slots=True)
class SubstituteCandidate:
    family: str
    evidence: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class EastAsianFontPolicy:
    substituted_families: tuple[str, ...]
    candidates: tuple[SubstituteCandidate, ...]
    policy_sha256: str


@dataclass(frozen=True, slots=True)
class EastAsianSubstitute:
    """A substitute family plus the identity of the host file behind it."""

    family: str
    path: Path
    sha256: str
    size_bytes: int


def load_policy(path: Path | None = None) -> EastAsianFontPolicy:
    """Load and validate the shared substitution policy."""
    resolved = (path or POLICY_PATH).resolve(strict=True)
    values = read_strict_object(resolved)
    require_keys(
        values,
        {
            "schema_version",
            "platform",
            "rationale",
            "substituted_families",
            "substitute_candidates",
        },
        "east_asian_font_policy",
    )
    if integer_value(values, "schema_version") != _SCHEMA_VERSION:
        raise EastAsianFontError("east-Asian font policy schema mismatch")
    families = values.get("substituted_families")
    if not isinstance(families, list) or not families:
        raise EastAsianFontError("east-Asian font policy lists no families")
    parsed: list[str] = []
    for family in families:
        if not isinstance(family, str) or not family:
            raise EastAsianFontError("east-Asian font family must be a string")
        parsed.append(family)
    if parsed != sorted(set(parsed)):
        raise EastAsianFontError("east-Asian font families must be sorted and unique")
    candidates = tuple(
        SubstituteCandidate(
            string_value(candidate, "family"),
            _evidence(candidate),
        )
        for candidate in object_list(
            values, "substitute_candidates", "east_asian_font_policy.candidates"
        )
    )
    if not candidates:
        raise EastAsianFontError("east-Asian font policy lists no candidates")
    return EastAsianFontPolicy(
        tuple(parsed),
        candidates,
        hashlib.sha256(resolved.read_bytes()).hexdigest(),
    )


def _evidence(candidate: JsonObject) -> tuple[str, ...]:
    require_keys(candidate, {"family", "evidence"}, "east_asian_font_candidate")
    evidence = candidate.get("evidence")
    if not isinstance(evidence, list) or not evidence:
        raise EastAsianFontError("east-Asian font candidate lists no evidence")
    paths: list[str] = []
    for item in evidence:
        if not isinstance(item, str) or not item.startswith("/"):
            raise EastAsianFontError("east-Asian font evidence must be absolute")
        paths.append(item)
    return tuple(paths)


def resolve_substitute(policy: EastAsianFontPolicy) -> EastAsianSubstitute | None:
    """Return the first candidate whose evidence file is present on this host."""
    for candidate in policy.candidates:
        for evidence in candidate.evidence:
            path = Path(evidence)
            try:
                information = path.stat()
            except OSError:
                continue
            if not path.is_file():
                continue
            return EastAsianSubstitute(
                candidate.family,
                path,
                sha256_file(path),
                information.st_size,
            )
    return None


def require_substitute(policy: EastAsianFontPolicy) -> EastAsianSubstitute:
    """Resolve the substitute or fail closed."""
    substitute = resolve_substitute(policy)
    if substitute is None:
        raise EastAsianFontError(
            "no CJK-capable substitute font is installed, so east-Asian text "
            "would resolve through the nondeterministic CoreText fallback"
        )
    return substitute


def lock_binding(
    substitute: EastAsianSubstitute,
    policy: EastAsianFontPolicy,
) -> JsonBinding:
    """Render the outer-lock binding for the selected host font.

    ``path`` is an absolute system path, not an evidence-root artifact: the
    substitute is an OS-provided font that cannot be copied into evidence under
    its license. Determinism is bound by re-reading that file and comparing the
    digest and size, plus the digest of the policy that selected it.
    """
    return {
        "family": substitute.family,
        "path": substitute.path.as_posix(),
        "sha256": substitute.sha256,
        "size_bytes": substitute.size_bytes,
        "policy_sha256": policy.policy_sha256,
    }


def validate_lock_binding(
    binding: JsonObject,
    policy: EastAsianFontPolicy,
) -> EastAsianSubstitute:
    """Re-read the locked host font and reject any drift.

    Both the reference producer and the candidate gate call this, so a host
    whose font bytes, size, family, or selection policy differ from the wave's
    lock is refused instead of silently producing divergent output.
    """
    require_keys(
        binding,
        {"family", "path", "sha256", "size_bytes", "policy_sha256"},
        "east_asian_font",
    )
    if string_value(binding, "policy_sha256") != policy.policy_sha256:
        raise EastAsianFontError("east-Asian font policy digest differs")
    family = string_value(binding, "family")
    known = {candidate.family for candidate in policy.candidates}
    if family not in known:
        raise EastAsianFontError("east-Asian font family is not a policy candidate")
    declared = string_value(binding, "path")
    allowed = {
        evidence
        for candidate in policy.candidates
        if candidate.family == family
        for evidence in candidate.evidence
    }
    if declared not in allowed:
        raise EastAsianFontError("east-Asian font path is not policy evidence")
    path = Path(declared)
    try:
        information = path.stat()
        if not path.is_file():
            raise EastAsianFontError("east-Asian font is not a regular file")
        digest = sha256_file(path)
    except OSError as error:
        raise EastAsianFontError(
            "east-Asian font is unavailable on this host"
        ) from error
    if information.st_size != integer_value(binding, "size_bytes"):
        raise EastAsianFontError("east-Asian font size differs from the lock")
    if digest != string_value(binding, "sha256"):
        raise EastAsianFontError("east-Asian font digest differs from the lock")
    return EastAsianSubstitute(family, path, digest, information.st_size)


def substitution_registry(substitute: str, policy: EastAsianFontPolicy) -> str:
    """Render the ``registrymodifications.xcu`` that pins every family.

    Byte-identical to the Rust renderer in ``document2html-native``; both are
    pinned to the shared golden registry by parity tests.
    """
    lines = [_HEADER]
    index = 0
    for family in policy.substituted_families:
        if family.lower() == substitute.lower():
            continue
        index += 1
        lines.append(
            '<item oor:path="/org.openoffice.Office.Common/Font/Substitution'
            f'/FontPairs"><node oor:name="d2h-cjk-{index:04}" oor:op="replace">'
            f'<prop oor:name="ReplaceFont" oor:op="fuse"><value>{_escape(family)}'
            '</value></prop><prop oor:name="SubstituteFont" oor:op="fuse">'
            f"<value>{_escape(substitute)}</value></prop>"
            '<prop oor:name="OnScreenOnly" oor:op="fuse"><value>false</value></prop>'
            '<prop oor:name="Always" oor:op="fuse"><value>true</value></prop>'
            "</node></item>\n"
        )
    lines.append(_FOOTER)
    return "".join(lines)


def seed_profile(profile: Path, substitute: str, policy: EastAsianFontPolicy) -> Path:
    """Write the replacement table into a LibreOffice profile before launch."""
    user = profile / "user"
    user.mkdir(parents=True, exist_ok=True)
    registry = user / "registrymodifications.xcu"
    registry.write_text(substitution_registry(substitute, policy), encoding="utf-8")
    return registry


def seed_host_profile(profile: Path) -> Path:
    """Seed a profile with the host-resolved substitute, failing closed.

    For producers that own no lock binding and must simply apply the shipped
    policy before LibreOffice reads a fresh profile.
    """
    policy = load_policy()
    return seed_profile(profile, require_substitute(policy).family, policy)


def _escape(value: str) -> str:
    return (
        value.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )
