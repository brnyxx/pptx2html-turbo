from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_candidate_sources import (
    CandidateSourceSet,
    load_candidate_sources,
)
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_schema import sha256_file


class PortableReferenceSourceError(ValueError):
    pass


def load_reference_sources(contract: Path, manifest: Path) -> CandidateSourceSet:
    """Load exact positive units from one validated READY format corpus."""
    try:
        result = load_candidate_sources(contract, manifest)
        for source in result.sources:
            if sha256_file(source.path) != source.source_sha256:
                raise PortableReferenceSourceError(
                    f"portable reference source digest differs: {source.source_id}"
                )
        return result
    except PortableReferenceSourceError:
        raise
    except (CorpusError, OSError, TypeError, ValueError) as error:
        raise PortableReferenceSourceError(
            f"portable reference source digest or schema differs: {error}"
        ) from error


def bind_trusted_sources(
    sources: CandidateSourceSet,
    trusted: tuple[tuple[str, Path], ...],
) -> CandidateSourceSet:
    """Replace corpus paths with the unique lock-trusted file of the same digest."""
    from dataclasses import replace

    by_digest: dict[str, list[Path]] = {}
    for digest, path in trusted:
        by_digest.setdefault(digest, []).append(path)
    bound = []
    for source in sources.sources:
        matches = by_digest.get(source.source_sha256, [])
        if len(matches) != 1:
            raise PortableReferenceSourceError(
                f"portable reference trusted source differs: {source.source_id}"
            )
        bound.append(replace(source, path=matches[0]))
    return replace(sources, sources=tuple(bound))
