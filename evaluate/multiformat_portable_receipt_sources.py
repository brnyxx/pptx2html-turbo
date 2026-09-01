from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_portable_receipt_context import PortableReceiptTrustError
from evaluate.multiformat_portable_receipt_validation import (
    StableFileIdentity,
    reject_identity_aliases,
    verify_stable_file,
)
from evaluate.multiformat_schema import object_value, sha256_value, string_value
from evaluate.multiformat_strict_json import read_strict_object


def load_receipt_sources(
    corpus_path: Path, root: Path
) -> tuple[StableFileIdentity, ...]:
    manifest = read_strict_object(corpus_path)
    raw = manifest.get("sources")
    if raw is None:
        tracks = object_value(manifest, "tracks")
        raw = []
        for name in ("conformance", "blind", "security"):
            items = object_value(tracks, name).get("items")
            if not isinstance(items, list):
                raise PortableReceiptTrustError(
                    "portable receipt corpus track is invalid"
                )
            raw.extend(items)
    if not isinstance(raw, list) or not raw:
        raise PortableReceiptTrustError("portable receipt corpus sources are missing")
    sources = []
    for value in raw:
        if not isinstance(value, dict):
            raise PortableReceiptTrustError("portable receipt source is invalid")
        source = resolve_evidence_path(corpus_path.parent, string_value(value, "path"))
        relative = source.relative_to(root.resolve(strict=True)).as_posix()
        sources.append(
            verify_stable_file(
                root, relative, sha256_value(value, "sha256"), None, "source"
            )
        )
    sources.sort(key=lambda item: item.path)
    result = tuple(sources)
    reject_identity_aliases((result,))
    return result
