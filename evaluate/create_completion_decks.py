#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
# How to run: python3 evaluate/create_completion_decks.py --output-dir <dir>

from __future__ import annotations

import argparse
import json
import struct
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Final


FIXED_ZIP_TIME: Final = (1980, 1, 1, 0, 0, 0)
NS: Final = (
    'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" '
    'xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" '
    'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" '
    'xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" '
    'xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"'
)


@dataclass(frozen=True, slots=True)
class DeckSpec:
    name: str
    slide_body: str
    slide_relationships: tuple[tuple[str, str, str, str | None], ...] = ()
    extra_parts: tuple[tuple[str, bytes], ...] = ()
    extra_content_types: tuple[tuple[str, str], ...] = ()


def _text_shape(shape_id: int, text: str, x: int, y: int) -> str:
    return (
        f'<p:sp><p:nvSpPr><p:cNvPr id="{shape_id}" name="{text}"/><p:cNvSpPr/>'
        f'<p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/>'
        '<a:ext cx="3000000" cy="500000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/>'
        f'</a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{text}</a:t>'
        '</a:r></a:p></p:txBody></p:sp>'
    )


def _pattern_shapes() -> str:
    shapes = []
    for index, preset in enumerate(("pct5", "diagCross", "ltDnDiag", "unknownFuturePattern"), 1):
        shapes.append(
            f'<p:sp><p:nvSpPr><p:cNvPr id="{index + 1}" name="pattern-{preset}"/>'
            f'<p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{400000 + index * 1700000}" y="1000000"/>'
            '<a:ext cx="1400000" cy="1400000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom>'
            f'<a:pattFill prst="{preset}"><a:fgClr><a:srgbClr val="336699"/></a:fgClr>'
            '<a:bgClr><a:srgbClr val="F2F2F2"/></a:bgClr></a:pattFill></p:spPr>'
            f'<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{preset}</a:t></a:r></a:p></p:txBody></p:sp>'
        )
    return "".join(shapes)


def _picture_bullets() -> str:
    return (
        '<p:sp><p:nvSpPr><p:cNvPr id="2" name="picture bullets"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>'
        '<p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/>'
        '<a:p><a:pPr><a:buSzPct val="100000"/><a:buBlip><a:blip r:embed="rIdImage"/></a:buBlip></a:pPr><a:r><a:t>Embedded picture bullet</a:t></a:r></a:p>'
        '<a:p><a:pPr><a:buBlip><a:blip r:embed="rIdMissing"/></a:buBlip></a:pPr><a:r><a:t>Missing picture bullet stays visible</a:t></a:r></a:p>'
        '</p:txBody></p:sp>'
    )


def _table() -> str:
    cells = [
        f'<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{label}</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>'
        for label in ("Header", "Explicit", "Band A", "Band B")
    ]
    return (
        '<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="styled table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>'
        '<p:xfrm><a:off x="500000" y="800000"/><a:ext cx="6000000" cy="2500000"/></p:xfrm><a:graphic>'
        '<a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr firstRow="1" bandRow="1" firstCol="1">'
        '<a:tableStyleId>{11111111-1111-1111-1111-111111111111}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w="3000000"/><a:gridCol w="3000000"/></a:tblGrid>'
        f'<a:tr h="1000000">{cells[0]}{cells[1]}</a:tr><a:tr h="1000000">{cells[2]}{cells[3]}</a:tr>'
        '</a:tbl></a:graphicData></a:graphic></p:graphicFrame>'
    )


def _actions() -> str:
    actions = (
        ("next", 'action="ppaction://hlinkshowjump?jump=nextslide"'),
        ("previous", 'action="ppaction://hlinkshowjump?jump=previousslide"'),
        ("external", 'r:id="rIdExternal"'),
        ("unsafe", 'r:id="rIdUnsafe"'),
    )
    return "".join(
        f'<p:sp><p:nvSpPr><p:cNvPr id="{index + 2}" name="{name}"><a:hlinkClick {attributes}/></p:cNvPr>'
        f'<p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="600000" y="{400000 + index * 800000}"/>'
        '<a:ext cx="3000000" cy="500000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>'
        f'<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{name}</a:t></a:r></a:p></p:txBody></p:sp>'
        for index, (name, attributes) in enumerate(actions)
    )


def _reflection_3d() -> str:
    return (
        '<p:sp><p:nvSpPr><p:cNvPr id="2" name="reflection and 3d"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr>'
        '<a:xfrm><a:off x="1000000" y="1000000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm><a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom>'
        '<a:effectLst><a:reflection blurRad="40000" stA="50000" endA="0" dist="100000" dir="5400000"/></a:effectLst>'
        '<a:scene3d><a:camera prst="perspectiveFront"><a:rot lat="0" lon="0" rev="0"/></a:camera><a:lightRig rig="threePt" dir="t"/></a:scene3d>'
        '<a:sp3d extrusionH="120000" prstMaterial="warmMatte"><a:bevelT w="60000" h="60000"/></a:sp3d></p:spPr>'
        '<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Reflection plus preserved 3D</a:t></a:r></a:p></p:txBody></p:sp>'
    )


def _media() -> str:
    return (
        _text_shape(2, "audio controls", 500000, 500000)
        + '<p:pic><p:nvPicPr><p:cNvPr id="3" name="Embedded audio"><a:hlinkClick action="ppaction://media" r:id="rIdAudio"/></p:cNvPr><p:cNvPicPr/><p:nvPr><a:audioFile r:link="rIdAudio"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="500000" y="1300000"/><a:ext cx="2500000" cy="1800000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>'
        + _text_shape(4, "linked video must not be fetched", 3500000, 1300000)
        + _text_shape(5, "unsupported codec fallback", 3500000, 2100000)
    )


def _timing() -> str:
    return (
        _text_shape(2, "click fade", 500000, 700000)
        + _text_shape(3, "unsupported motion path stays static", 500000, 1600000)
        + '<p:transition spd="slow"><p:fade/></p:transition><p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" nodeType="tmRoot"><p:childTnLst>'
        '<p:seq concurrent="1" nextAc="seek"><p:cTn id="2" dur="indefinite" nodeType="mainSeq"><p:childTnLst>'
        '<p:par><p:cTn id="3" fill="hold"><p:childTnLst><p:animEffect transition="in" filter="fade"><p:cBhvr><p:cTn id="4" dur="500"><p:stCondLst><p:cond evt="onClick" delay="0"/></p:stCondLst></p:cTn><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:animEffect>'
        '<p:animMotion origin="layout" path="M 0 0 L 1 1"><p:cBhvr><p:cTn id="5" dur="1000"/><p:tgtEl><p:spTgt spid="3"/></p:tgtEl></p:cBhvr></p:animMotion>'
        '</p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:seq></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>'
    )


def _chart() -> str:
    return (
        '<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="mixed charts"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>'
        '<p:xfrm><a:off x="500000" y="700000"/><a:ext cx="6000000" cy="3500000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">'
        '<c:chart r:id="rIdChart"/></a:graphicData></a:graphic></p:graphicFrame>'
    )


def _fallback() -> str:
    return (
        _text_shape(2, "SmartArt preview/fallback", 400000, 400000)
        + '<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="3" name="SmartArt"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram"><a:relIds xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/diagram" r:dm="rIdDiagram" r:lo="" r:qs="" r:cs=""/></a:graphicData></a:graphic></p:graphicFrame>'
        + '<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="4" name="Inert OLE"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/presentationml/2006/ole"><p:oleObj r:id="rIdOle" progId="Package" name="inert"/></a:graphicData></a:graphic></p:graphicFrame>'
        + '<mc:AlternateContent><mc:Choice Requires="x14"><p:sp><p:nvSpPr><p:cNvPr id="5" name="Unsupported choice"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp></mc:Choice><mc:Fallback>'
        + _text_shape(6, "AlternateContent fallback", 400000, 1400000)
        + '</mc:Fallback></mc:AlternateContent><p:extLst><p:ext uri="urn:pptx2html:test:unknown"><unknown:payload xmlns:unknown="urn:pptx2html:test:unknown" value="preserve"/></p:ext></p:extLst>'
    )


def _png() -> bytes:
    return bytes.fromhex(
        "89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c489"
        "0000000d49444154789c6360f8cfc000000301010018dd8db10000000049454e44ae426082"
    )


def _wav() -> bytes:
    samples = b"\x00\x00" * 80
    return b"RIFF" + struct.pack("<I", 36 + len(samples)) + b"WAVEfmt " + struct.pack(
        "<IHHIIHH", 16, 1, 1, 8000, 16000, 2, 16
    ) + b"data" + struct.pack("<I", len(samples)) + samples


def _specs() -> tuple[DeckSpec, ...]:
    image = _png()
    return (
        DeckSpec("patterns", _pattern_shapes()),
        DeckSpec("picture-bullets", _picture_bullets(), (("rIdImage", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image", "../media/bullet.png", None), ("rIdMissing", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image", "../media/missing.png", None)), (("ppt/media/bullet.png", image),)),
        DeckSpec("table-styles", _table(), extra_parts=(("ppt/tableStyles.xml", b'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" def="{11111111-1111-1111-1111-111111111111}"><a:tblStyle styleId="{11111111-1111-1111-1111-111111111111}" styleName="Completion"><a:wholeTbl><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="DDEBF7"/></a:solidFill></a:fill></a:tcStyle></a:wholeTbl><a:firstRow><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="4472C4"/></a:solidFill></a:fill></a:tcStyle></a:firstRow></a:tblStyle></a:tblStyleLst>'),), extra_content_types=(("/ppt/tableStyles.xml", "application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml"),)),
        DeckSpec("actions", _actions(), (("rIdExternal", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink", "https://example.com/", "External"), ("rIdUnsafe", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink", "javascript:alert(1)", "External"))),
        DeckSpec("notes-comments", _text_shape(2, "Visible slide text; notes stay off canvas", 500000, 500000), (("rIdNotes", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide", "../notesSlides/notesSlide1.xml", None), ("rIdComments", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments", "../comments/comment1.xml", None)), (("ppt/notesSlides/notesSlide1.xml", b'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="notes"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>NOTES_SENTINEL_OFF_CANVAS</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>'), ("ppt/comments/comment1.xml", b'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:cmLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cm authorId="404" dt="2026-01-01T00:00:00Z" idx="1"><p:pos x="0" y="0"/><p:text>COMMENT_SENTINEL_MISSING_AUTHOR</p:text></p:cm></p:cmLst>')), (("/ppt/notesSlides/notesSlide1.xml", "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml"), ("/ppt/comments/comment1.xml", "application/vnd.openxmlformats-officedocument.presentationml.comments+xml"))),
        DeckSpec("reflection-3d", _reflection_3d()),
        DeckSpec("media", _media(), (("rIdAudio", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio", "../media/audio.wav", None), ("rIdPoster", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image", "../media/poster.png", None), ("rIdVideo", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/video", "https://example.invalid/video.mp4", "External"), ("rIdUnsupported", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio", "../media/unsupported.bin", None)), (("ppt/media/audio.wav", _wav()), ("ppt/media/poster.png", image), ("ppt/media/unsupported.bin", b"INERT_UNSUPPORTED_MEDIA"))),
        DeckSpec("timing-transitions", _timing()),
        DeckSpec("charts", _chart(), (("rIdChart", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart", "../charts/chart1.xml", None),), (("ppt/charts/chart1.xml", b'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea><c:layout/><c:barChart><c:barDir val="col"/><c:ser><c:idx val="0"/><c:order val="0"/><c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numLit></c:val></c:ser></c:barChart><c:surface3DChart/><c:pieChart><c:ser><c:idx val="1"/><c:order val="1"/></c:ser><c:ser><c:idx val="2"/><c:order val="2"/></c:ser></c:pieChart></c:plotArea></c:chart></c:chartSpace>'),), (("/ppt/charts/chart1.xml", "application/vnd.openxmlformats-officedocument.drawingml.chart+xml"),)),
        DeckSpec("fallback-domains", _fallback(), (("rIdDiagram", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData", "../diagrams/data1.xml", None), ("rIdOle", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject", "../embeddings/inert.bin", None)), (("ppt/diagrams/data1.xml", b'<?xml version="1.0" encoding="UTF-8"?><dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:ptLst/></dgm:dataModel>'), ("ppt/embeddings/inert.bin", b"INERT_OLE_TEST_PAYLOAD_DO_NOT_EXECUTE")), (("/ppt/diagrams/data1.xml", "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml"), ("/ppt/embeddings/inert.bin", "application/vnd.openxmlformats-officedocument.oleObject"))),
    )


def _slide_xml(body: str) -> bytes:
    return (
        f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld {NS}><p:cSld><p:spTree>'
        '<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr>'
        '<a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>'
        f'{body}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>'
    ).encode()


def _relationships(rows: tuple[tuple[str, str, str, str | None], ...]) -> bytes:
    content = "".join(
        f'<Relationship Id="{rel_id}" Type="{rel_type}" Target="{target}"{f" TargetMode={json.dumps(mode)}" if mode else ""}/>'
        for rel_id, rel_type, target, mode in rows
    )
    return f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{content}</Relationships>'.encode()


def _package_parts(spec: DeckSpec) -> dict[str, bytes]:
    overrides = (
        ('/ppt/presentation.xml', 'application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml'),
        ('/ppt/slides/slide1.xml', 'application/vnd.openxmlformats-officedocument.presentationml.slide+xml'),
        *spec.extra_content_types,
    )
    content_types = ''.join(f'<Override PartName="{part}" ContentType="{content_type}"/>' for part, content_type in overrides)
    parts = {
        '[Content_Types].xml': f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Default Extension="wav" ContentType="audio/wav"/><Default Extension="bin" ContentType="application/octet-stream"/>{content_types}</Types>'.encode(),
        '_rels/.rels': _relationships((('rId1', 'http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument', 'ppt/presentation.xml', None),)),
        'ppt/presentation.xml': f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentation {NS}><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>'.encode(),
        'ppt/_rels/presentation.xml.rels': _relationships((('rId1', 'http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide', 'slides/slide1.xml', None),)),
        'ppt/slides/slide1.xml': _slide_xml(spec.slide_body),
        'ppt/slides/_rels/slide1.xml.rels': _relationships(spec.slide_relationships),
    }
    parts.update(spec.extra_parts)
    return parts


def _write_deck(path: Path, spec: DeckSpec) -> None:
    with zipfile.ZipFile(path, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, payload in sorted(_package_parts(spec).items()):
            info = zipfile.ZipInfo(name, FIXED_ZIP_TIME)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 0
            info.external_attr = 0
            archive.writestr(info, payload)


def _manifest(specs: tuple[DeckSpec, ...]) -> dict[str, list[dict[str, str | bool | dict[str, list[str] | None]]] | str | int | dict[str, str]]:
    feature_map = {
        8: ('patterns', ('adjustment-basic',)), 9: ('patterns', ('adjustment-arrows',)),
        10: ('patterns', ('adjustment-remaining', 'custom-geometry-unknown-formula')),
        12: ('patterns', ('pattern-fill-known', 'pattern-fill-unknown')),
        13: ('picture-bullets', ('picture-bullet-embedded', 'picture-bullet-missing')),
        14: ('table-styles', ('table-style-regions', 'table-style-missing')),
        15: ('actions', ('action-external', 'action-internal', 'action-unsafe')),
        16: ('notes-comments', ('notes-slide', 'comments-legacy', 'comments-modern', 'comment-author-missing')),
        17: ('reflection-3d', ('reflection', 'drawingml-3d-fallback')),
        18: ('media', ('media-audio', 'media-video', 'media-unsupported')),
        19: ('timing-transitions', ('transition-cut', 'transition-fade', 'animation-bounded', 'animation-unsupported')),
        20: ('charts', ('chart-direct', 'chart-preview-fallback', 'chart-placeholder')),
        21: ('fallback-domains', ('fallback-smartart', 'fallback-ole', 'fallback-math', 'fallback-alternate-content', 'fallback-unknown-extension')),
    }
    features = [
        {'id': feature_id, 'task': task, 'deck': f'{deck}.pptx', 'powerpoint_capture_required': True, 'native_evidence': {'images': [], 'metadata': None}}
        for task, (deck, feature_ids) in feature_map.items()
        for feature_id in feature_ids
    ]
    return {
        'schema_version': 1,
        'powerpoint_capture_required': True,
        'native_evidence': {'images': [], 'metadata': None},
        'decks': [{'name': spec.name, 'file': f'{spec.name}.pptx'} for spec in specs],
        'features': features,
        'adjustment_case_source': {'path': '../preset_adjustments.json', 'status': 'awaiting-task-2-manifest'},
        'adjustment_case_scaffold': [{'kind': kind, 'expected_pixels': None} for kind in ('default', 'lower', 'upper', 'representative')],
    }


def generate(output_dir: Path) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    specs = _specs()
    for spec in specs:
        _write_deck(output_dir / f'{spec.name}.pptx', spec)
    (output_dir / 'manifest.json').write_text(json.dumps(_manifest(specs), indent=2, sort_keys=True) + '\n', encoding='utf-8', newline='\n')


def main() -> None:
    parser = argparse.ArgumentParser(description='Generate deterministic PPTX completion fixtures.')
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    generate(args.output_dir)


if __name__ == '__main__':
    main()
