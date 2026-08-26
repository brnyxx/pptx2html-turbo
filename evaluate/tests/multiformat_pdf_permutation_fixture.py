"""Synthetic PDFs that differ only by indirect-object numbering.

The reference producer emits the same document with run-dependent object ids,
so the canonical form must be invariant under renumbering. Building the same
object graph under several id assignments reproduces that permutation exactly,
with no dependence on how a real converter happens to number a given run.
"""

from __future__ import annotations

import zlib
from dataclasses import dataclass
from typing import Final

_LATIN_FACE: Final = b"BAAAAA+LiberationSans"
_CJK_FACE: Final = b"CAAAAA+ArialUnicodeMS"


@dataclass(frozen=True, slots=True)
class GraphObject:
    """One indirect object whose references are symbolic, not numeric.

    Exactly one of `body` (a dictionary) or `stream` (a payload) is present.
    """

    key: str
    body: bytes | None = None
    stream: bytes | None = None

    def __post_init__(self) -> None:
        if (self.body is None) == (self.stream is None):
            raise ValueError("a graph object needs either a body or a stream")


def font_graph(second_face: bytes) -> tuple[GraphObject, ...]:
    """A one-page document embedding two subset faces.

    `second_face` selects the CJK or the CJK-free pairing; both reproduce the
    same numbering permutation, so neither depends on a host font.
    """
    content = b"BT /F1 12 Tf 72 720 Td (Alpha) Tj /F2 12 Tf (Beta) Tj ET"
    return (
        GraphObject("catalog", b"<</Type /Catalog /Pages {pages} 0 R>>"),
        GraphObject(
            "pages",
            b"<</Type /Pages /Kids [{page} 0 R] /Count 1>>",
        ),
        GraphObject(
            "page",
            b"<</Type /Page /Parent {pages} 0 R /Resources {resources} 0 R"
            b" /Contents {contents} 0 R /MediaBox [0 0 612 792]>>",
        ),
        GraphObject(
            "resources",
            b"<</Font <</F1 {font_latin} 0 R /F2 {font_second} 0 R>>>>",
        ),
        GraphObject("contents", stream=content),
        GraphObject(
            "font_latin",
            b"<</Type /Font /Subtype /TrueType /BaseFont /"
            + _LATIN_FACE
            + b" /FirstChar 65 /LastChar 90 /FontDescriptor {descriptor_latin} 0 R>>",
        ),
        GraphObject(
            "descriptor_latin",
            b"<</Type /FontDescriptor /FontName /"
            + _LATIN_FACE
            + b" /Flags 32 /FontFile2 {file_latin} 0 R>>",
        ),
        GraphObject("file_latin", stream=b"latin-subset-bytes" * 40),
        GraphObject(
            "font_second",
            b"<</Type /Font /Subtype /TrueType /BaseFont /"
            + second_face
            + b" /FirstChar 65 /LastChar 90 /FontDescriptor {descriptor_second} 0 R>>",
        ),
        GraphObject(
            "descriptor_second",
            b"<</Type /FontDescriptor /FontName /"
            + second_face
            + b" /Flags 32 /FontFile2 {file_second} 0 R>>",
        ),
        GraphObject("file_second", stream=b"second-subset-bytes" * 90),
        GraphObject(
            "info",
            b"<</Producer (fixture) /CreationDate (D:20260101000000Z)>>",
        ),
    )


CJK_GRAPH: Final = font_graph(_CJK_FACE)
LATIN_GRAPH: Final = font_graph(b"DAAAAA+LiberationSerif")


def with_detached(
    graph: tuple[GraphObject, ...],
    bodies: tuple[bytes, ...],
) -> tuple[GraphObject, ...]:
    """Append objects that no reference reaches.

    An exporter may emit a font the page resources never name, so the canonical
    form must keep such objects rather than dropping them.
    """
    return graph + tuple(
        GraphObject(f"detached_{index}", body)
        for index, body in enumerate(bodies, start=1)
    )


def build_pdf(graph: tuple[GraphObject, ...], order: tuple[str, ...]) -> bytes:
    """Serialize `graph` assigning ids 1..N in `order`.

    Every ordering encodes the same document, so any two differ only by
    indirect-object numbering.
    """
    keys = {item.key for item in graph}
    if set(order) != keys or len(order) != len(graph):
        raise ValueError("order must be a permutation of the graph keys")
    identifiers = {key: index for index, key in enumerate(order, start=1)}
    by_key = {item.key: item for item in graph}
    payload = bytearray(b"%PDF-1.7\n")
    offsets: dict[int, int] = {}
    for key in order:
        item = by_key[key]
        identifier = identifiers[key]
        offsets[identifier] = len(payload)
        payload.extend(f"{identifier} 0 obj\n".encode())
        if item.stream is None:
            if item.body is None:
                raise ValueError("graph object has neither body nor stream")
            payload.extend(_resolve(item.body, identifiers))
        else:
            encoded = zlib.compress(item.stream, level=9)
            payload.extend(
                b"<</Length "
                + str(len(encoded)).encode()
                + b" /Length1 "
                + str(len(item.stream)).encode()
                + b" /Filter /FlateDecode>>\nstream\n"
            )
            payload.extend(encoded)
            payload.extend(b"\nendstream")
        payload.extend(b"\nendobj\n")
    xref_offset = len(payload)
    size = len(order) + 1
    payload.extend(f"xref\n0 {size}\n".encode())
    payload.extend(b"0000000000 65535 f \n")
    for identifier in range(1, size):
        payload.extend(f"{offsets[identifier]:010d} 00000 n \n".encode())
    payload.extend(
        f"trailer\n<< /Size {size} /Root {identifiers['catalog']} 0 R"
        f" /Info {identifiers['info']} 0 R >>\n"
        f"startxref\n{xref_offset}\n%%EOF\n".encode()
    )
    return bytes(payload)


def _resolve(body: bytes, identifiers: dict[str, int]) -> bytes:
    resolved = body
    for key, identifier in identifiers.items():
        resolved = resolved.replace(
            b"{" + key.encode() + b"}", str(identifier).encode()
        )
    if b"{" in resolved:
        raise ValueError("unresolved symbolic reference in fixture body")
    return resolved


def natural_order(graph: tuple[GraphObject, ...]) -> tuple[str, ...]:
    return tuple(item.key for item in graph)


def face_swapped_order(graph: tuple[GraphObject, ...]) -> tuple[str, ...]:
    """Swap the two face groups, reproducing the observed converter layout.

    Real captures alternate between exactly two layouts in which the two font
    dictionaries, descriptors, and subset streams trade object ids.
    """
    order = list(natural_order(graph))
    for left, right in (
        ("font_latin", "font_second"),
        ("descriptor_latin", "descriptor_second"),
        ("file_latin", "file_second"),
    ):
        first, second = order.index(left), order.index(right)
        order[first], order[second] = order[second], order[first]
    return tuple(order)


def reversed_body_order(graph: tuple[GraphObject, ...]) -> tuple[str, ...]:
    """Reverse every object except the catalog, a maximally different layout."""
    order = list(natural_order(graph))
    order.remove("catalog")
    return ("catalog", *reversed(order))
