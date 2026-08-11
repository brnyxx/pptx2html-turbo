from __future__ import annotations


COLOR_SCHEME = '<a:clrScheme name="Completion"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="1F497D"/></a:dk2><a:lt2><a:srgbClr val="EEECE1"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme>'
FONT_SCHEME = '<a:fontScheme name="Completion"><a:majorFont><a:latin typeface="Arial"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Arial"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme>'
SOLID = '<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>'
GRADIENT = '<a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"/></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"/></a:gs></a:gsLst><a:lin ang="5400000" scaled="0"/></a:gradFill>'
FILLS = f"<a:fillStyleLst>{SOLID}{GRADIENT}{SOLID}</a:fillStyleLst>"
LINE = f'<a:ln w="12700">{SOLID}<a:prstDash val="solid"/></a:ln>'
LINES = f"<a:lnStyleLst>{LINE}{LINE}{LINE}</a:lnStyleLst>"
EFFECTS = "<a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst>"
BACKGROUNDS = f"<a:bgFillStyleLst>{SOLID}{GRADIENT}{SOLID}</a:bgFillStyleLst>"


def theme_xml(name: str = "Completion") -> bytes:
    return (
        '<?xml version="1.0"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" '
        f'name="{name}"><a:themeElements>{COLOR_SCHEME}{FONT_SCHEME}'
        f'<a:fmtScheme name="Completion">{FILLS}{LINES}{EFFECTS}{BACKGROUNDS}'
        "</a:fmtScheme></a:themeElements></a:theme>"
    ).encode()
