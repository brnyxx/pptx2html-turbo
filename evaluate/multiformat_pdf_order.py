"""Deterministic indirect-object numbering for the reference canonical form.

LibreOffice assigns PDF object ids per run: the same document exports with its
font dictionaries, descriptors, and subset streams holding different object
numbers. Nothing about the document changes, only the labels, so two reference
exports of one source differ in raw bytes while rendering identically.

Canonical numbering removes that freedom. Objects are renumbered by a
structural walk from the trailer's `Root` (then `Info`), following each
object's references in the order they appear in its dictionary. That order is a
property of the document graph, not of the exporting run, so every numbering of
one document collapses to the same labels.

Only the structural part of an object is rewritten. Stream payloads are copied
byte for byte: a reference cannot occur inside stream data, and glyph subsets
must survive untouched.

A reference with no object is a corrupt graph and fails closed. Objects that no
reference reaches are legitimate - an exporter may emit a font the page
resources never name - so they are kept, never dropped, and ordered after the
reachable graph by a numbering-independent shape digest: their structure with
every reference blanked, plus their payload. Ties on that digest would make the
order ambiguous, so an ambiguous set fails closed rather than picking a
run-dependent winner.
"""

from __future__ import annotations

import hashlib
import re
from dataclasses import replace

from evaluate.multiformat_pdf_types import (
    ParsedPdfObjects,
    PdfConformanceError,
)

_HEADER = re.compile(rb"\s*([0-9]+)\s+0\s+obj")
_STREAM = re.compile(rb"stream\r?\n")
_REFERENCE = re.compile(rb"([0-9]+)(\s+0\s+R)")


def renumber_pdf_objects(
    parsed: ParsedPdfObjects,
    objects: dict[int, bytes],
) -> tuple[ParsedPdfObjects, dict[int, bytes]]:
    """Return `parsed` and `objects` renumbered into canonical order."""
    structures = {
        identifier: _structure(identifier, body) for identifier, body in objects.items()
    }
    order = _walk(parsed, structures)
    order.extend(_detached(structures, set(order)))
    assigned = {identifier: index for index, identifier in enumerate(order, start=1)}
    renumbered = {
        assigned[identifier]: _rewrite(
            assigned[identifier], structures[identifier], assigned
        )
        for identifier in order
    }
    info_id = None if parsed.info_id is None else assigned[parsed.info_id]
    return replace(
        parsed, root_id=assigned[parsed.root_id], info_id=info_id
    ), renumbered


class _Structure:
    """An object split into its rewritable structure and opaque payload."""

    __slots__ = ("payload", "references", "structure")

    def __init__(self, structure: bytes, payload: bytes) -> None:
        self.structure = structure
        self.payload = payload
        self.references = tuple(
            int(match.group(1)) for match in _REFERENCE.finditer(structure)
        )


def _structure(identifier: int, body: bytes) -> _Structure:
    header = _HEADER.match(body)
    if header is None or int(header.group(1)) != identifier:
        raise PdfConformanceError("PDF indirect object header differs")
    rest = body[header.end() :]
    stream = _STREAM.search(rest)
    if stream is None:
        return _Structure(rest, b"")
    return _Structure(rest[: stream.start()], rest[stream.start() :])


def _walk(parsed: ParsedPdfObjects, structures: dict[int, _Structure]) -> list[int]:
    roots = [parsed.root_id]
    if parsed.info_id is not None and parsed.info_id != parsed.root_id:
        roots.append(parsed.info_id)
    for root in roots:
        if root not in structures:
            raise PdfConformanceError("PDF trailer reference is unresolved")
    order = list(roots)
    seen = set(order)
    cursor = 0
    while cursor < len(order):
        current = structures[order[cursor]]
        cursor += 1
        for reference in current.references:
            if reference in seen:
                continue
            if reference not in structures:
                raise PdfConformanceError("PDF object reference is unresolved")
            seen.add(reference)
            order.append(reference)
    return order


def _detached(structures: dict[int, _Structure], reached: set[int]) -> list[int]:
    """Order unreachable objects by a digest that ignores object numbering."""
    detached = [identifier for identifier in structures if identifier not in reached]
    if not detached:
        return []
    digests = {identifier: _shape(structures[identifier]) for identifier in detached}
    if len(set(digests.values())) != len(detached):
        raise PdfConformanceError("PDF detached objects are indistinguishable")
    return sorted(detached, key=lambda identifier: digests[identifier])


def _shape(structure: _Structure) -> str:
    blanked = _REFERENCE.sub(b"\x00\\2", structure.structure)
    digest = hashlib.sha256()
    digest.update(len(blanked).to_bytes(8, "big"))
    digest.update(blanked)
    digest.update(structure.payload)
    return digest.hexdigest()


def _rewrite(identifier: int, structure: _Structure, assigned: dict[int, int]) -> bytes:
    def substitute(match: re.Match[bytes]) -> bytes:
        target = assigned.get(int(match.group(1)))
        if target is None:
            raise PdfConformanceError("PDF object reference is unresolved")
        return str(target).encode() + match.group(2)

    body = _REFERENCE.sub(substitute, structure.structure)
    return f"{identifier} 0 obj".encode() + body + structure.payload
