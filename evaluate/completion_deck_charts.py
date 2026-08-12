from __future__ import annotations

from xml.sax.saxutils import quoteattr

if __package__:
    from .completion_deck_package import REL, Part, relationships_xml
else:
    from completion_deck_package import REL, Part, relationships_xml


C = "http://schemas.openxmlformats.org/drawingml/2006/chart"


def parts(image: bytes) -> tuple[Part, ...]:
    return (
        ("ppt/charts/chart1.xml", _direct_bar()),
        ("ppt/charts/chart2.xml", _preview_surface()),
        (
            "ppt/charts/_rels/chart2.xml.rels",
            relationships_xml(
                (
                    (
                        "rIdPreviewImage",
                        REL + "image",
                        "../media/chart-preview.png",
                        None,
                    ),
                )
            ),
        ),
        ("ppt/charts/chart3.xml", _placeholder_stock()),
        ("ppt/media/chart-preview.png", image),
    )


def _space(plot: str) -> bytes:
    return f'<?xml version="1.0"?><c:chartSpace xmlns:c={quoteattr(C)}><c:chart><c:plotArea><c:layout/>{plot}</c:plotArea><c:plotVisOnly val="1"/></c:chart></c:chartSpace>'.encode()


def _series(index: int) -> str:
    value = quoteattr(str(index))
    first = str(index + 1)
    second = str((index + 1) * 2)
    return f'<c:ser><c:idx val={value}/><c:order val={value}/><c:tx><c:v>Series {index + 1}</c:v></c:tx><c:cat><c:strLit><c:ptCount val="2"/><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>B</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>{first}</c:v></c:pt><c:pt idx="1"><c:v>{second}</c:v></c:pt></c:numLit></c:val></c:ser>'


def _cat_axis(axis: int, crossing: int) -> str:
    return f'<c:catAx><c:axId val={quoteattr(str(axis))}/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:tickLblPos val="nextTo"/><c:crossAx val={quoteattr(str(crossing))}/><c:crosses val="autoZero"/><c:auto val="1"/><c:lblAlgn val="ctr"/><c:lblOffset val="100"/></c:catAx>'


def _value_axis(axis: int, crossing: int) -> str:
    return f'<c:valAx><c:axId val={quoteattr(str(axis))}/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:numFmt formatCode="General" sourceLinked="1"/><c:tickLblPos val="nextTo"/><c:crossAx val={quoteattr(str(crossing))}/><c:crosses val="autoZero"/><c:crossBetween val="between"/></c:valAx>'


def _series_axis(axis: int, crossing: int) -> str:
    return f'<c:serAx><c:axId val={quoteattr(str(axis))}/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:tickLblPos val="nextTo"/><c:crossAx val={quoteattr(str(crossing))}/><c:crosses val="autoZero"/></c:serAx>'


def _direct_bar() -> bytes:
    chart = f'<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>{_series(0)}<c:axId val="10"/><c:axId val="20"/></c:barChart>'
    return _space(chart + _cat_axis(10, 20) + _value_axis(20, 10))


def _preview_surface() -> bytes:
    chart = f'<c:surface3DChart><c:wireframe val="0"/>{_series(0)}<c:axId val="30"/><c:axId val="40"/><c:axId val="50"/></c:surface3DChart>'
    return _space(
        chart + _cat_axis(30, 40) + _value_axis(40, 30) + _series_axis(50, 40)
    )


def _placeholder_stock() -> bytes:
    series = "".join(_series(index) for index in range(3))
    chart = f'<c:stockChart>{series}<c:hiLowLines/><c:axId val="60"/><c:axId val="70"/></c:stockChart>'
    return _space(chart + _cat_axis(60, 70) + _value_axis(70, 60))
