from __future__ import annotations

import hashlib
import io
import zipfile
from enum import StrEnum
from typing import Final, assert_never
from xml.sax.saxutils import escape, quoteattr

from evaluate.completion_deck_charts import parts as chart_parts
from evaluate.completion_deck_package import REL, Deck, deck_bytes, png_bytes
from evaluate.completion_deck_specs import CHARTS
from evaluate.completion_deck_tables import TABLES
from evaluate.multiformat_package_validation import valid_ooxml_bytes
from evaluate.multiformat_schema import JsonValue, integer_value, string_value

PRESENTATION_CONTENT_TYPE: Final = (
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"
)
PRESENTATION_ROOT: Final = (
    "{http://schemas.openxmlformats.org/presentationml/2006/main}presentation"
)


class PptxConformanceError(Exception):
    pass


class PptxStratum(StrEnum):
    TEXT = "text"
    SHAPES_CONNECTORS = "shapes-connectors"
    IMAGES_EFFECTS = "images-effects"
    TABLES_CHARTS = "tables-charts"
    MASTERS_LAYOUTS_GROUPS = "masters-layouts-groups"
    INTERNATIONAL = "international"
    FALLBACK_EDGE = "fallback-edge"


def pptx_case_bytes(case: dict[str, JsonValue]) -> bytes:
    case_id = string_value(case, "id")
    ordinal = integer_value(case, "ordinal")
    seed = string_value(case, "feature_seed")
    try:
        stratum = PptxStratum(string_value(case, "primary_stratum"))
    except ValueError as error:
        raise PptxConformanceError("unsupported PPTX stratum") from error
    body, relationships, parts, types = _feature(stratum, seed)
    identity = _text_shape(
        2,
        "case identity",
        f"{case_id} | ordinal={ordinal} | feature_seed={seed}",
        y=5_400_000,
        height=1_200_000,
    )
    deck = Deck(
        case_id,
        ((identity + body, ""),),
        relationships,
        parts=parts,
        types=types,
    )
    value = deck_bytes(deck)
    admit_pptx_case(value, case)
    return value


def admit_pptx_case(value: bytes, case: dict[str, JsonValue]) -> None:
    if not valid_ooxml_bytes(
        value,
        "ppt/presentation.xml",
        PRESENTATION_CONTENT_TYPE,
        PRESENTATION_ROOT,
    ):
        raise PptxConformanceError("PPTX package admission failed")
    case_id = string_value(case, "id")
    ordinal = integer_value(case, "ordinal")
    seed = string_value(case, "feature_seed")
    try:
        stratum = PptxStratum(string_value(case, "primary_stratum"))
    except ValueError as error:
        raise PptxConformanceError("unsupported PPTX stratum") from error
    with zipfile.ZipFile(io.BytesIO(value)) as archive:
        infos = archive.infolist()
        if [info.filename for info in infos] != sorted(info.filename for info in infos):
            raise PptxConformanceError("PPTX ZIP ordering is not canonical")
        if any(info.date_time != (1980, 1, 1, 0, 0, 0) for info in infos):
            raise PptxConformanceError("PPTX ZIP timestamp is not canonical")
        slide = archive.read("ppt/slides/slide1.xml")
        visible = f"{case_id} | ordinal={ordinal} | feature_seed={seed}".encode()
        if visible not in slide or any(
            token not in slide for token in _admission_tokens(stratum)
        ):
            raise PptxConformanceError("PPTX visible feature admission failed")
        match stratum:
            case PptxStratum.IMAGES_EFFECTS:
                if "ppt/media/feature.png" not in archive.namelist():
                    raise PptxConformanceError("PPTX image admission failed")
            case PptxStratum.TABLES_CHARTS:
                if "ppt/charts/chart1.xml" not in archive.namelist():
                    raise PptxConformanceError("PPTX chart admission failed")
            case (
                PptxStratum.TEXT
                | PptxStratum.SHAPES_CONNECTORS
                | PptxStratum.MASTERS_LAYOUTS_GROUPS
                | PptxStratum.INTERNATIONAL
                | PptxStratum.FALLBACK_EDGE
            ):
                pass
            case unreachable:
                assert_never(unreachable)


def package_inventory(value: bytes) -> list[dict[str, JsonValue]]:
    with zipfile.ZipFile(io.BytesIO(value)) as archive:
        return [
            {
                "path": info.filename,
                "sha256": hashlib.sha256(archive.read(info)).hexdigest(),
                "size": info.file_size,
            }
            for info in archive.infolist()
        ]


def _feature(
    stratum: PptxStratum,
    seed: str,
) -> tuple[
    str,
    tuple[tuple[str, str, str, str | None], ...],
    tuple[tuple[str, bytes], ...],
    tuple[tuple[str, str], ...],
]:
    color = seed[:6].upper()
    match stratum:
        case PptxStratum.TEXT:
            body = _text_shape(3, "text feature", "Bold italic text")
            return body.replace("<a:rPr", '<a:rPr b="1" i="1"'), (), (), ()
        case PptxStratum.SHAPES_CONNECTORS:
            return _shape_connector(color), (), (), ()
        case PptxStratum.IMAGES_EFFECTS:
            return (
                _image_effect(color),
                (("rIdFeatureImage", REL + "image", "../media/feature.png", None),),
                (("ppt/media/feature.png", png_bytes()),),
                (),
            )
        case PptxStratum.TABLES_CHARTS:
            relationships = tuple(
                (f"rIdChart{name}", REL + "chart", f"../charts/chart{index}.xml", None)
                for index, name in enumerate(("Direct", "Preview", "Placeholder"), 1)
            )
            types = tuple(
                (
                    f"/ppt/charts/chart{index}.xml",
                    "application/vnd.openxmlformats-officedocument.drawingml.chart+xml",
                )
                for index in range(1, 4)
            )
            return TABLES + CHARTS, relationships, chart_parts(png_bytes()), types
        case PptxStratum.MASTERS_LAYOUTS_GROUPS:
            return _group_shape(color), (), (), ()
        case PptxStratum.INTERNATIONAL:
            return _international_shape(), (), (), ()
        case PptxStratum.FALLBACK_EDGE:
            return _fallback_shape(), (), (), ()
        case unreachable:
            assert_never(unreachable)


def _admission_tokens(stratum: PptxStratum) -> tuple[bytes, ...]:
    match stratum:
        case PptxStratum.TEXT:
            return (b'<a:rPr b="1" i="1"',)
        case PptxStratum.SHAPES_CONNECTORS:
            return (b'prst="roundRect"', b"<p:cxnSp>")
        case PptxStratum.IMAGES_EFFECTS:
            return (
                b'<a:blip r:embed="rIdFeatureImage"',
                b"<a:effectLst>",
            )
        case PptxStratum.TABLES_CHARTS:
            return (b"<a:tbl>", b"<c:chart ")
        case PptxStratum.MASTERS_LAYOUTS_GROUPS:
            return (b"<p:grpSp>",)
        case PptxStratum.INTERNATIONAL:
            return ("한글 العربية 日本語".encode(),)
        case PptxStratum.FALLBACK_EDGE:
            return (
                b"<mc:AlternateContent>",
                b"<mc:Fallback>",
                b"Visible fallback content",
            )
        case unreachable:
            assert_never(unreachable)


def _text_shape(
    shape_id: int,
    name: str,
    text: str,
    *,
    y: int = 500_000,
    height: int = 900_000,
) -> str:
    return (
        f'<p:sp><p:nvSpPr><p:cNvPr id="{shape_id}" name={quoteattr(name)}/>'
        '<p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm>'
        f'<a:off x="500000" y="{y}"/><a:ext cx="8200000" cy="{height}"/>'
        '</a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>'
        '<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr sz="1800"/>'
        f"<a:t>{escape(text)}</a:t></a:r></a:p></p:txBody></p:sp>"
    )


def _shape_connector(color: str) -> str:
    return (
        '<p:sp><p:nvSpPr><p:cNvPr id="3" name="seed shape"/><p:cNvSpPr/>'
        '<p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="700000" y="1800000"/>'
        '<a:ext cx="2500000" cy="1800000"/></a:xfrm><a:prstGeom prst="roundRect">'
        f'<a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="{color}"/>'
        '</a:solidFill></p:spPr></p:sp><p:cxnSp><p:nvCxnSpPr><p:cNvPr id="4" '
        'name="seed connector"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr>'
        '<a:xfrm><a:off x="3200000" y="2600000"/><a:ext cx="3000000" cy="0"/>'
        '</a:xfrm><a:prstGeom prst="line"><a:avLst/></a:prstGeom></p:spPr></p:cxnSp>'
    )


def _image_effect(color: str) -> str:
    return (
        '<p:pic><p:nvPicPr><p:cNvPr id="3" name="seed image"/><p:cNvPicPr/>'
        '<p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rIdFeatureImage"/>'
        "<a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm>"
        '<a:off x="1000000" y="1700000"/><a:ext cx="3600000" cy="2600000"/>'
        '</a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:effectLst>'
        f'<a:glow rad="50000"><a:srgbClr val="{color}"/></a:glow>'
        "</a:effectLst></p:spPr></p:pic>"
    )


def _group_shape(color: str) -> str:
    child = _text_shape(4, "group child", "Grouped layout content")
    return (
        '<p:grpSp><p:nvGrpSpPr><p:cNvPr id="3" name="layout group"/>'
        "<p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm>"
        '<a:off x="900000" y="1600000"/><a:ext cx="7000000" cy="3000000"/>'
        '<a:chOff x="0" y="0"/><a:chExt cx="7000000" cy="3000000"/>'
        f'</a:xfrm><a:solidFill><a:srgbClr val="{color}"/></a:solidFill></p:grpSpPr>'
        f"{child}</p:grpSp>"
    )


def _international_shape() -> str:
    return _text_shape(
        3,
        "international text",
        "한글 العربية 日本語 Español naïve façade",
    ).replace("<a:p>", '<a:p><a:pPr rtl="1"/>')


def _fallback_shape() -> str:
    preferred = _text_shape(3, "preferred edge", "Preferred future content")
    fallback = _text_shape(4, "visible fallback", "Visible fallback content")
    return (
        '<mc:AlternateContent><mc:Choice Requires="p188">'
        f"{preferred}</mc:Choice><mc:Fallback>{fallback}</mc:Fallback></mc:AlternateContent>"
    )
