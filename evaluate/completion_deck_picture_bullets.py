from __future__ import annotations

from typing import Final

if __package__:
    from .completion_deck_package import REL, ContentType, Part, Relationship
else:
    from completion_deck_package import REL, ContentType, Part, Relationship


def _paragraph(reference: str, size: str, text: str) -> str:
    blip = f"<a:blip {reference}/>" if reference else "<a:blip/>"
    return (
        f"<a:p><a:pPr>{size}<a:buBlip>{blip}</a:buBlip></a:pPr>"
        f'<a:r><a:rPr sz="2000"/><a:t>{text}</a:t></a:r></a:p>'
    )


PICTURE_BULLETS: Final = (
    '<p:sp><p:nvSpPr><p:cNvPr id="2" name="picture bullets"/>'
    "<p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/>"
    + "".join(
        (
            _paragraph('r:embed="rIdImage"', "<a:buSzTx/>", "Size text"),
            _paragraph(
                'r:embed="rIdImage"',
                '<a:buSzPct val="25000"/>',
                "Size 25",
            ),
            _paragraph(
                'r:embed="rIdImage"',
                '<a:buSzPct val="400000"/>',
                "Size 400",
            ),
            _paragraph(
                'r:embed="rIdImage"',
                '<a:buSzPts val="1250"/>',
                "Size points",
            ),
            _paragraph("", "<a:buSzTx/>", "Missing reference"),
            _paragraph('r:embed="rIdWrongKind"', "<a:buSzTx/>", "Wrong kind"),
            _paragraph('r:embed="rIdSvg"', "<a:buSzTx/>", "Unsupported SVG"),
            _paragraph('r:link="rIdLinked"', "<a:buSzTx/>", "Linked external"),
        )
    )
    + "</p:txBody></p:sp>"
)


def relationships() -> tuple[Relationship, ...]:
    return (
        ("rIdImage", REL + "image", "../media/bullet.png", None),
        ("rIdWrongKind", REL + "chart", "../media/bullet.png", None),
        ("rIdSvg", REL + "image", "../media/bullet.svg", None),
        (
            "rIdLinked",
            REL + "image",
            "https://user:password@example.invalid/bullet.png?token=secret",
            "External",
        ),
    )


def parts(image: bytes) -> tuple[Part, ...]:
    return (
        ("ppt/media/bullet.png", image),
        (
            "ppt/media/bullet.svg",
            b'<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>',
        ),
    )


def content_types() -> tuple[ContentType, ...]:
    return (
        ("/ppt/media/bullet.png", "image/png"),
        ("/ppt/media/bullet.svg", "image/svg+xml"),
    )
