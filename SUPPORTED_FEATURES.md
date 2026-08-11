# Supported PPTX Features

## DrawingML tables

- Parses `ppt/tableStyles.xml`, `a:tableStyleId`, the six table flags, whole/row/column/corner regions, `tblBg`, theme-aware fill/text styles, outer borders, and `insideH`/`insideV` borders.
- Applies regions in the Microsoft Office order and lets explicit cell fill, `noFill`, and side-border presence override style regions.
- Uses logical grid coordinates across `gridSpan`, `hMerge`, and `vMerge` cells.
- Preserves unavailable built-in and invalid IDs without inventing an appearance and emits `TABLE_STYLE_DEFINITION_UNAVAILABLE` with ID/source kind/six flags.
- `[교차검증 필요]` The cited Office application-order note does not define whether band numbering restarts after first/last rows; the converter currently uses one-based odd/even physical grid coordinates before later first/last-region overrides.

Status legend: `exact` / `approximate` / `fallback` / `unparsed`

This file is the detailed ECMA-376 element inventory. The authoritative support contract now lives in `docs/architecture/CAPABILITY_MATRIX.md`.

This inventory is in a staged migration from legacy labels to support tiers. Until every row is migrated, interpret legacy labels as follows:

- `Supported` → `approximate`
- `Partial` → `approximate`
- `Placeholder` → `fallback`
- `Not yet` → `unparsed`

Capability stages such as `parsed` and `rendered` belong in `docs/architecture/CAPABILITY_MATRIX.md`, not in the `Status` column here.

## Shapes

| Feature | ECMA-376 Element | Status |
|---------|-----------------|--------|
| Rectangle | `<a:prstGeom prst="rect">` | Supported |
| Rounded Rectangle | `<a:prstGeom prst="roundRect">` | Supported |
| Ellipse | `<a:prstGeom prst="ellipse">` | Supported |
| Triangle | `<a:prstGeom prst="triangle">` | Supported |
| Right Triangle | `<a:prstGeom prst="rtTriangle">` | Supported |
| Diamond | `<a:prstGeom prst="diamond">` | Supported |
| Parallelogram | `<a:prstGeom prst="parallelogram">` | Supported |
| Trapezoid | `<a:prstGeom prst="trapezoid">` | Supported |
| Pentagon | `<a:prstGeom prst="pentagon">` | Supported |
| Hexagon | `<a:prstGeom prst="hexagon">` | Supported |
| Octagon | `<a:prstGeom prst="octagon">` | Supported |
| Right Arrow | `<a:prstGeom prst="rightArrow">` | Supported |
| Left Arrow | `<a:prstGeom prst="leftArrow">` | Supported |
| Up Arrow | `<a:prstGeom prst="upArrow">` | Supported |
| Down Arrow | `<a:prstGeom prst="downArrow">` | Supported |
| Left-Right Arrow | `<a:prstGeom prst="leftRightArrow">` | Supported |
| Up-Down Arrow | `<a:prstGeom prst="upDownArrow">` | Supported |
| Chevron | `<a:prstGeom prst="chevron">` | Supported |
| Bent Arrow | `<a:prstGeom prst="bentArrow">` | Supported |
| Callouts | `<a:prstGeom prst="wedge*Callout">` | Supported |
| Stars | `<a:prstGeom prst="star*">` | Supported |
| Plus | `<a:prstGeom prst="mathPlus">` | Supported |
| Minus | `<a:prstGeom prst="mathMinus">` | Supported |
| Cross | `<a:prstGeom prst="plus">` | Supported |
| Heart | `<a:prstGeom prst="heart">` | Supported |
| Lightning Bolt | `<a:prstGeom prst="lightningBolt">` | Supported |
| Custom Geometry | `<a:custGeom>` | Partial |
| Adjust Values / Guide Formulas | `<a:gdLst><a:gd fmla="...">` | Partial |
| Custom geometry text rectangle | `<a:custGeom><a:rect .../>` | Partial |
| Custom geometry adjust handles | `<a:ahLst><a:ahXY>` / `<a:ahPolar>` | Partial |
| Custom geometry connection sites | `<a:cxnLst><a:cxn>` | Partial |

## Text

| Feature | ECMA-376 Element | Status |
|---------|-----------------|--------|
| Plain text | `<a:t>` | Supported |
| Bold | `<a:rPr b="1">` | Supported |
| Italic | `<a:rPr i="1">` | Supported |
| Underline | `<a:rPr u="sng">` | Supported |
| Strikethrough | `<a:rPr strike="sngStrike">` | Supported |
| Font size | `<a:rPr sz="2400">` | Supported |
| Font family | `<a:latin typeface="...">` | Supported |
| East Asian font | `<a:ea typeface="...">` | Supported |
| Text color (RGB) | `<a:solidFill><a:srgbClr>` | Supported |
| Text color (theme) | `<a:solidFill><a:schemeClr>` | Supported |
| Superscript / Subscript | `<a:rPr baseline="...">` | Supported |
| Letter spacing | `<a:rPr spc="...">` | Supported |
| Text highlight | `<a:highlight>` | Supported |
| Text shadow | `<a:effectLst><a:outerShdw>` | Supported |
| Line break | `<a:br>` | Supported |
| Hyperlink | `<a:hlinkClick>` | Approximate |
| Mouse-over action metadata | `<a:hlinkMouseOver>` | Approximate |
| Internal slide / first / last / next / previous action | `<a:hlinkClick action="ppaction://...">` | Approximate |
| Media, program, macro, file, and custom action preservation | `<a:hlinkClick action="...">` | fallback |
| Group and table graphic-frame action ownership | `p:grpSp/p:nvGrpSpPr/p:cNvPr`, `p:graphicFrame/p:nvGraphicFramePr/p:cNvPr` | Approximate |
| Text alignment | `<a:pPr algn="...">` | Supported |
| Line spacing | `<a:lnSpc>` | Supported |
| Space before/after | `<a:spcBef>` / `<a:spcAft>` | Supported |
| Paragraph indent | `<a:pPr indent="...">` | Supported |
| Paragraph margin | `<a:pPr marL="...">` | Supported |
| Vertical text | `<a:bodyPr vert="...">` | Supported |
| Vertical alignment | `<a:bodyPr anchor="...">` | Supported |
| Text wrapping | `<a:bodyPr wrap="...">` | Supported |
| Auto-fit / Shrink | `<a:normAutofit>` | Approximate |
| Text margins (insets) | `<a:bodyPr lIns="...">` | Supported |
| RTL text | `<a:pPr rtl="1">` | Unparsed |

## Bullets and Numbering

| Feature | ECMA-376 Element | Status |
|---------|-----------------|--------|
| Character bullet | `<a:buChar char="...">` | Supported |
| Auto-numbered bullet | `<a:buAutoNum type="...">` | Supported |
| Bullet font | `<a:buFont typeface="...">` | Supported |
| Bullet size | `<a:buSzPct>` / `<a:buSzPts>` / `<a:buSzTx>` | Supported |
| Bullet color | `<a:buClr>` | Supported |
| No bullet | `<a:buNone>` | Supported |
| Picture bullet (slide paragraph, table cell, slide-owned `lstStyle`) | `<a:buBlip><a:blip r:embed="...">` | Supported for PNG/JPEG/GIF/WebP; deterministic marker + `PICTURE_BULLET_IMAGE_MISSING` otherwise |
| Picture bullet (master `txStyles`, master/layout shape style, `defaultTextStyle`) | `<a:buBlip>` in inherited owner part | Fallback with `PICTURE_BULLET_INHERITANCE_UNSUPPORTED`; paragraph text is preserved |

## Fills

| Feature | ECMA-376 Element | Status |
|---------|-----------------|--------|
| Solid fill (RGB) | `<a:solidFill><a:srgbClr>` | Supported |
| Solid fill (theme) | `<a:solidFill><a:schemeClr>` | Supported |
| Gradient fill | `<a:gradFill>` | Supported |
| Image fill | `<a:blipFill>` | Supported |
| Pattern fill | `<a:pattFill>` | Supported for all 54 presets with approximate repeated SVG tiles; unknown or unresolved patterns preserve raw semantics in `DRAWINGML_PATTERN_UNSUPPORTED` diagnostics |
| No fill | `<a:noFill>` | Supported |
| Fill style reference | `<a:fillRef>` | Supported |

## Borders and Lines

| Feature | ECMA-376 Element | Status |
|---------|-----------------|--------|
| Shape outline | `<a:ln>` | Partial |
| Line width | `<a:ln w="...">` | Supported |
| Line color (RGB/theme) | `<a:ln><a:solidFill>` | Supported |
| Dash style (solid/dash/dot/dashDot) | `<a:prstDash>` | Supported |
| Line style reference | `<a:lnRef>` | Supported |
| Arrow head (line start) | `<a:headEnd>` | Supported |
| Arrow tail (line end) | `<a:tailEnd>` | Supported |
| Arrow types (arrow/triangle/stealth/diamond/oval) | `type` attr | Supported |
| Arrow size (sm/med/lg) | `w` / `len` attrs | Supported |
| No fill (transparent line) | `<a:noFill>` in `<a:ln>` | Supported |

## Colors

| Feature | ECMA-376 Element | Status |
|---------|-----------------|--------|
| RGB color | `<a:srgbClr>` | Supported |
| Theme color | `<a:schemeClr>` | Supported |
| System color | `<a:sysClr>` | Supported |
| Preset color | `<a:prstClr>` | Supported |
| Tint modifier | `<a:tint>` | Supported |
| Shade modifier | `<a:shade>` | Supported |
| Alpha modifier | `<a:alpha>` | Supported |
| LumMod / LumOff | `<a:lumMod>` / `<a:lumOff>` | Supported |
| SatMod / SatOff | `<a:satMod>` / `<a:satOff>` | Supported |
| HueMod / HueOff | `<a:hueMod>` / `<a:hueOff>` | Supported |
| Complement | `<a:comp>` | Supported |
| Inverse | `<a:inv>` | Supported |
| Grayscale | `<a:gray>` | Supported |

## Tables

| Feature | ECMA-376 Element | Status |
|---------|-----------------|--------|
| Table rendering | `<a:tbl>` | Supported |
| Cell fill | `<a:tcPr>` fill | Supported |
| Cell borders | `<a:tcPr>` borders | Supported |
| Column widths | `<a:gridCol>` | Supported |
| Row heights | `<a:tr h="...">` | Supported |
| Column span | `gridSpan` | Supported |
| Row span | `rowSpan` + `vMerge` | Supported |
| Table styles | `<a:tblStyle>` | Partial - package definitions, official region order, strict namespace/context diagnostics |
| Table-style fill reference | `<a:fillRef>` | Partial - index/color/modifiers preserved; parsed theme fills resolve; unavailable non-solid fills emit `TABLE_STYLE_PRIMITIVE_UNSUPPORTED` without a replacement (`[교차검증 필요]`) |
| Table-style effect/line reference | `<a:tblBg>/<a:effectRef>`, border side `<a:lnRef>` | Preserved only - scoped index/color/modifiers emit `TABLE_STYLE_PRIMITIVE_UNSUPPORTED`; no effect or line is invented |

## Images

| Feature | ECMA-376 Element | Status |
|---------|-----------------|--------|
| Embedded images | `<p:pic>` | Supported |
| Image cropping | `<a:srcRect>` | Supported |
| Base64 embedding | — | Supported |
| External references | — | Supported |
| Background image fill | `<a:blipFill>` in `<p:bg>` | Supported |

## Layout and Hierarchy

| Feature | ECMA-376 Element | Status |
|---------|-----------------|--------|
| Slide size | `<p:sldSz>` | Supported |
| Shape position / size | `<a:xfrm>` | Supported |
| Shape rotation | `<a:xfrm rot="...">` | Supported |
| Group shapes | `<p:grpSp>` | Supported |
| Connectors | `<p:cxnSp>` | Partial |
| Connector anchoring to custom geometry sites | `<a:stCxn>` / `<a:endCxn>` + `<a:cxnLst>` | Partial |
| Placeholder matching | `<p:ph type="..." idx="...">` | Supported |
| Slide → Layout inheritance | slide.rels → slideLayout | Supported |
| Layout → Master inheritance | layout.rels → slideMaster | Supported |
| Master → Theme reference | master.rels → theme | Supported |
| ClrMap | `<p:clrMap>` | Supported |
| ClrMap override | `<p:clrMapOvr>` | Supported |
| Background inheritance | `<p:bg>` cascade | Supported |
| TxStyles (title/body/other) | `<p:txStyles>` | Supported |
| defaultTextStyle | `<p:defaultTextStyle>` | Supported |
| Show master shapes | `showMasterSp` | Supported |
| Hidden slides | `show="0"` | Supported |
| Multiple themes | theme1.xml, theme2.xml, ... | Supported |

## Charts and Embedded Content

| Feature | ECMA-376 Element | Status |
|---------|-----------------|--------|
| Chart detection | `<c:chart>` URI | Supported |
| Direct bar/column charts | `<c:barChart>` | Approximate |
| Bar/column spacing controls | `<c:gapWidth>`, `<c:overlap>` | Approximate |
| Bar/column data labels | `<c:dLbls>` | Approximate |
| Direct line charts | `<c:lineChart>` | Approximate |
| Line series markers | `<c:marker>` | Approximate |
| Line/area point labels | `<c:dLbls>` | Approximate |
| Chart axis titles | `<c:catAx>/<c:valAx> <c:title>` | Approximate |
| Direct area charts (standard) | `<c:areaChart>` | Approximate |
| Direct area3D charts (flat render) | `<c:area3DChart>` | Approximate |
| Direct scatter charts | `<c:scatterChart>` | Approximate |
| Direct bubble charts (single-series, no `<c:dLbls>`, non-negative sizes, area semantics, approximate bubbleScale support) | `<c:bubbleChart>` | Approximate |
| Direct radar charts (multi-series, no `<c:dLbls>`, approximate marker handling) | `<c:radarChart>` | Approximate |
| Direct ofPie charts (single-series, no `<c:dLbls>`, `ofPieType=pie`, `splitType=pos`) | `<c:ofPieChart>` | Approximate |
| Scatter point labels | `<c:scatterChart> <c:dLbls>` | Approximate |
| Direct pie charts (single-series) | `<c:pieChart>` | Approximate |
| Direct doughnut charts (single-series) | `<c:doughnutChart>` | Approximate |
| Direct pie3D charts (single-series, flat render) | `<c:pie3DChart>` | Approximate |
| Chart preview image | embedded preview | Fallback |
| Unsupported / complex chart families | other chart spaces | Fallback |
| Chart placeholder | — | Fallback |
| SmartArt | `<dgm:*>` | Fallback |
| OLE objects | `<p:oleObj>` | Fallback |
| Math equations | `<m:*>` | Fallback |

## Effects

| Feature | ECMA-376 Element | Status |
|---------|-----------------|--------|
| Text shadow | `<a:outerShdw>` | Approximate |
| Shape shadow | `<a:effectLst>` | Approximate |
| Reflection | `<a:reflection>` | Unparsed |
| Glow | `<a:glow>` | Approximate |
| 3D effects | `<a:sp3d>` | Unparsed |
