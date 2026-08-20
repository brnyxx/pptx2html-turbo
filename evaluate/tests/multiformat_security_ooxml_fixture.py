from __future__ import annotations

import io
import tempfile
import zipfile
from enum import StrEnum
from functools import lru_cache
from pathlib import Path
from typing import assert_never

from evaluate.multiformat_package_validation import MAX_XML_BYTES
from evaluate.tests.multiformat_security_cfb_fixture import (
    write_cfb_security_fixture,
)
from evaluate.tests.multiformat_source_fixture import (
    OOXML_PARTS,
    SourceFixtureError,
    write_positive_source,
)

REL_NS = "http://schemas.openxmlformats.org/package/2006/relationships"
REL_BASE = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"


class OoxmlSecurityFamily(StrEnum):
    MALFORMED_ZIP = "malformed-zip"
    ZIP_BOMB = "zip-bomb"
    PATH_TRAVERSAL = "path-traversal"
    EXTERNAL_RELATIONSHIP = "external-relationship"
    MACRO_CONTENT = "macro-content"
    EMBEDDED_OBJECT = "embedded-object"
    OVERSIZED_XML = "oversized-xml"
    ENTITY_EXPANSION = "entity-expansion"
    RELATIONSHIP_CYCLE = "relationship-cycle"
    CORRUPT_MEDIA = "corrupt-media"


def write_ooxml_security_fixture(
    path: Path,
    document_format: str,
    family: str,
) -> None:
    path.write_bytes(_security_package(document_format, family))


@lru_cache(maxsize=30)
def _security_package(document_format: str, family: str) -> bytes:
    try:
        parsed_family = OoxmlSecurityFamily(family)
    except ValueError as error:
        raise SourceFixtureError(
            f"unsupported OOXML security family: {family}"
        ) from error
    with tempfile.TemporaryDirectory() as temp_dir:
        source = Path(temp_dir) / f"base.{document_format}"
        write_positive_source(source, document_format, family)
        base = source.read_bytes()
    if parsed_family is OoxmlSecurityFamily.MALFORMED_ZIP:
        return base[:-22]
    output = io.BytesIO(base)
    main_part = OOXML_PARTS[document_format][0]
    relationship_part = _relationship_part(main_part)
    with zipfile.ZipFile(
        output,
        "a",
        compression=zipfile.ZIP_DEFLATED,
        compresslevel=9,
    ) as archive:
        match parsed_family:
            case OoxmlSecurityFamily.ZIP_BOMB:
                archive.writestr("security/bomb.bin", b"\x00" * (MAX_XML_BYTES + 1))
            case OoxmlSecurityFamily.PATH_TRAVERSAL:
                archive.writestr("../escape.bin", b"escape")
            case OoxmlSecurityFamily.EXTERNAL_RELATIONSHIP:
                archive.writestr(
                    relationship_part,
                    _relationships(
                        f"{_relationship('external', 'hyperlink', 'https://example.invalid/', external=True)}"
                    ),
                )
            case OoxmlSecurityFamily.MACRO_CONTENT:
                archive.writestr("security/vbaProject.bin", _macro_payload())
                archive.writestr(
                    relationship_part,
                    _relationships(
                        _relationship(
                            "macro",
                            "vbaProject",
                            "../security/vbaProject.bin",
                        )
                    ),
                )
            case OoxmlSecurityFamily.EMBEDDED_OBJECT:
                archive.writestr(
                    "security/embeddings/object.bin",
                    _embedded_payload(),
                )
                archive.writestr(
                    relationship_part,
                    _relationships(
                        _relationship(
                            "embedded",
                            "oleObject",
                            "../security/embeddings/object.bin",
                        )
                    ),
                )
            case OoxmlSecurityFamily.OVERSIZED_XML:
                archive.writestr(
                    "security/oversized.xml",
                    b"<root>" + b"x" * MAX_XML_BYTES + b"</root>",
                )
                archive.writestr(
                    relationship_part,
                    _relationships(
                        _relationship(
                            "oversized",
                            "customXml",
                            "../security/oversized.xml",
                        )
                    ),
                )
            case OoxmlSecurityFamily.ENTITY_EXPANSION:
                archive.writestr(
                    "security/entity.xml",
                    b'<!DOCTYPE root [<!ENTITY x "expanded">]><root>&x;</root>',
                )
                archive.writestr(
                    relationship_part,
                    _relationships(
                        _relationship(
                            "entity",
                            "customXml",
                            "../security/entity.xml",
                        )
                    ),
                )
            case OoxmlSecurityFamily.RELATIONSHIP_CYCLE:
                archive.writestr("security/a.xml", b"<a/>")
                archive.writestr("security/b.xml", b"<b/>")
                archive.writestr(
                    relationship_part,
                    _relationships(
                        _relationship("cycle-root", "customXml", "../security/a.xml")
                    ),
                )
                archive.writestr(
                    "security/_rels/a.xml.rels",
                    _relationships(_relationship("cycle-a", "customXml", "b.xml")),
                )
                archive.writestr(
                    "security/_rels/b.xml.rels",
                    _relationships(_relationship("cycle-b", "customXml", "a.xml")),
                )
            case OoxmlSecurityFamily.CORRUPT_MEDIA:
                archive.writestr("security/media/image.png", b"not-a-png")
                archive.writestr(
                    relationship_part,
                    _relationships(
                        _relationship(
                            "media",
                            "image",
                            "../security/media/image.png",
                        )
                    ),
                )
            case OoxmlSecurityFamily.MALFORMED_ZIP:
                raise AssertionError("malformed ZIP returned before archive mutation")
            case unreachable:
                assert_never(unreachable)
    return output.getvalue()


def _relationship_part(main_part: str) -> str:
    parent, name = main_part.rsplit("/", 1)
    return f"{parent}/_rels/{name}.rels"


def _relationships(value: str) -> str:
    return f'<Relationships xmlns="{REL_NS}">{value}</Relationships>'


def _relationship(
    relationship_id: str,
    kind: str,
    target: str,
    *,
    external: bool = False,
) -> str:
    target_mode = ' TargetMode="External"' if external else ""
    return (
        f'<Relationship Id="{relationship_id}" Type="{REL_BASE}/{kind}" '
        f'Target="{target}"{target_mode}/>'
    )


@lru_cache(maxsize=1)
def _macro_payload() -> bytes:
    with tempfile.TemporaryDirectory() as temp_dir:
        path = Path(temp_dir) / "macro.doc"
        write_cfb_security_fixture(path, "doc", "macro-storage")
        return path.read_bytes()


@lru_cache(maxsize=1)
def _embedded_payload() -> bytes:
    with tempfile.TemporaryDirectory() as temp_dir:
        path = Path(temp_dir) / "embedded.doc"
        write_positive_source(path, "doc", "embedded")
        return path.read_bytes()
