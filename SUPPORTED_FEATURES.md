# Supported PPTX Features

## DrawingML tables

- Parses `ppt/tableStyles.xml`, `a:tableStyleId`, the six table flags, whole/row/column/corner regions, `tblBg`, theme-aware fill/text styles, outer borders, and `insideH`/`insideV` borders.
- Applies regions in the Microsoft Office order and lets explicit cell fill, `noFill`, and side-border presence override style regions.
- Uses logical grid coordinates across `gridSpan`, `hMerge`, and `vMerge` cells.
- Preserves unavailable built-in and invalid IDs without inventing an appearance and emits `TABLE_STYLE_DEFINITION_UNAVAILABLE` with ID/source kind/six flags.
- The converter uses one-based odd/even physical grid coordinates before later first/last-region overrides and does not claim exact PowerPoint equivalence for header/footer-relative band origins.

Status legend: `exact` / `approximate` / `fallback` / `unparsed`

This file is the detailed ECMA-376 element inventory. The authoritative support contract now lives in `docs/architecture/CAPABILITY_MATRIX.md`.

This inventory is in a staged migration from legacy labels to support tiers. Until every row is migrated, interpret legacy labels as follows:

- `Supported` → `approximate`
- `Partial` → `approximate`
- `Placeholder` → `fallback`
- `Not yet` → `unparsed`

Capability stages such as `parsed` and `rendered` belong in `docs/architecture/CAPABILITY_MATRIX.md`, not in the `Status` column here.

## Shapes

| Feature | ECMA-376 Element | Current handling |
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

| Feature | ECMA-376 Element | Current handling |
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
| RTL text | `<a:pPr rtl="1">` | Approximate |

## Bullets and Numbering

| Feature | ECMA-376 Element | Current handling |
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

| Feature | ECMA-376 Element | Current handling |
|---------|-----------------|--------|
| Solid fill (RGB) | `<a:solidFill><a:srgbClr>` | Supported |
| Solid fill (theme) | `<a:solidFill><a:schemeClr>` | Supported |
| Gradient fill | `<a:gradFill>` | Supported |
| Image fill | `<a:blipFill>` | Supported |
| Pattern fill | `<a:pattFill>` | Supported for all 54 presets with approximate repeated SVG tiles; unknown or unresolved patterns preserve raw semantics in `DRAWINGML_PATTERN_UNSUPPORTED` diagnostics |
| No fill | `<a:noFill>` | Supported |
| Fill style reference | `<a:fillRef>` | Supported |

## Borders and Lines

| Feature | ECMA-376 Element | Current handling |
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

| Feature | ECMA-376 Element | Current handling |
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

| Feature | ECMA-376 Element | Current handling |
|---------|-----------------|--------|
| Table rendering | `<a:tbl>` | Supported |
| Cell fill | `<a:tcPr>` fill | Supported |
| Cell borders | `<a:tcPr>` borders | Supported |
| Column widths | `<a:gridCol>` | Supported |
| Row heights | `<a:tr h="...">` | Supported |
| Column span | `gridSpan` | Supported |
| Row span | `rowSpan` + `vMerge` | Supported |
| Table styles | `<a:tblStyle>` | Partial - package definitions, official region order, strict namespace/context diagnostics |
| Table-style fill reference | `<a:fillRef>` | Partial - index/color/modifiers preserved; parsed theme fills resolve; unavailable non-solid fills emit `TABLE_STYLE_PRIMITIVE_UNSUPPORTED` without a replacement or exactness claim |
| Table-style effect/line reference | `<a:tblBg>/<a:effectRef>`, border side `<a:lnRef>` | Preserved only - scoped index/color/modifiers emit `TABLE_STYLE_PRIMITIVE_UNSUPPORTED`; no effect or line is invented |

## Images

| Feature | ECMA-376 Element | Current handling |
|---------|-----------------|--------|
| Embedded images | `<p:pic>` | Supported |
| Image cropping | `<a:srcRect>` | Supported |
| Base64 embedding | — | Supported |
| External references | — | Supported |
| Background image fill | `<a:blipFill>` in `<p:bg>` | Supported |

## Layout and Hierarchy

| Feature | ECMA-376 Element | Current handling |
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

## Notes and comments

| Feature | ECMA-376 / MS-PPTX element or part | Current handling |
|---------|------------------------------------|--------|
| Slide notes text and one-based slide association | Notes Slide `<p:notes>` / `<a:t>` | parsed off-canvas fallback metadata |
| Notes-master association | Notes Slide relationship to Notes Master part | parsed off-canvas fallback metadata |
| Legacy comments and authors | `<p:cmLst>/<p:cm>/<p:text>`, `<p:cmAuthorLst>` | parsed off-canvas fallback metadata |
| Modern comments and authors | `p188:cmLst/p188:cm/p188:txBody`, Author part | parsed off-canvas fallback metadata |
| Missing comment author | unresolved `authorId` | fallback with exact `COMMENT_AUTHOR_UNRESOLVED`; text retained |
| Unknown modern comment extension | `p188:extLst` foreign payload | fallback raw XML with no exact semantic claim |
| External, unsafe, duplicate, malformed, or namespace-spoofed annotation relation | package relationship | fallback diagnostic; target is not opened or exposed |

Notes and comments are metadata only and are never inserted into the visible slide canvas. The relationship and element bounds are limited to the Microsoft Notes Slide, legacy comments, PresentationML structure, modern `CT_Comment`, Comment Part, and Author Part documentation linked from the README.

## Charts and Embedded Content

| Feature | ECMA-376 Element | Current handling |
|---------|-----------------|--------|
| Namespace- and ancestry-aware classic/ChartEx classification | exact chart-part roots; classic direct families only at `c:chartSpace/c:chart/c:plotArea/<family>`, with foreign and nested out-of-schema branches excluded from axes, series, and cache semantics | Supported |
| Chart detection | exact classic `<c:chart>` or ChartEx graphic URI | Supported |
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
| Chart preview image | deterministic first safe internal image relationship; bounded PNG/JPEG/GIF/WebP payloads require matching structural signatures | Fallback |
| ChartEx relationship | exact internal `http://schemas.microsoft.com/office/2014/relationships/chartEx` target; qualified encountered subtype, namespace-qualified element inventory, raw XML, series summary, and preview metadata are preserved without claiming direct support for any ChartEx subtype | Fallback |
| Invalid chart frame ancestry | only exact `p:graphicFrame/a:graphic/a:graphicData/(c:chart or cx:chart)` ancestry dispatches chart semantics; foreign wrappers remain one unsupported-element diagnostic and malformed official ancestry remains one chart fallback diagnostic | Fallback |
| Combination charts and incompatible axes/series | preserved chart XML with one `CHART_STRUCTURE_UNSUPPORTED` diagnostic | Fallback |
| ChartEx, unsupported/complex families, invalid preview payloads, and broken references | preserved chart XML with one typed chart fallback diagnostic | Fallback |
| Chart placeholder | no usable preview | Fallback |
| SmartArt | `<dgm:relIds>` plus diagram relationship closure | Fallback with a bounded, structurally validated PNG preview only when it uses the strict safe subset (IHDR/IDAT/IEND-only, 8-bit non-interlaced RGBA, stored-zlib blocks, filter-0 scanlines, valid CRC and Adler-32); other PNG encodings and all other image/container forms fall back to a bounded placeholder; no SmartArt auto-layout fidelity claim |
| OLE objects | `<p:oleObj>` | Fallback with a package-provided preview only when it matches the same strict safe PNG subset, otherwise a bounded placeholder; payload bytes remain inert and are never emitted |
| Math equations | `<m:oMath>` / `<m:oMathPara>` | Fallback with script-safe bounded OMML metadata and visible placeholder; no native equation layout claim |
| Markup Compatibility branches | `<mc:AlternateContent>` / `<mc:Choice Requires>` / `<mc:Fallback>` | Namespace-aware deterministic selection of exactly one supported branch; every ordered branch and requirement token remains typed fallback metadata |
| Unknown package extensions | unknown parts, relationships, and qualified elements | Stable deterministic diagnostics with bounded references; external targets are redacted |

## Transitions and timing

| Feature | ECMA-376 Element | Current handling |
|---------|-----------------|--------|
| Click-triggered cut/fade slide transitions | `<p:transition><p:cut|fade>` | Approximate; automatic advance is never executed |
| Click, with-previous, and after-previous groups | `<p:cTn nodeType="...">` | Approximate; interaction-driven only, with no autoplay or loops |
| Appear, disappear, and fade effects on resolved slide shapes | `<p:set>` / `<p:animEffect>` | Approximate for finite durations and start-condition delays from 0 through 10000 ms |
| Other timing commands, unresolved targets, unbounded durations, and automatic advance | other `<p:timing>` content | Fallback metadata; targets remain statically visible |

The original top-level transition and timing XML is retained in source order with stable generated identities. Unsupported behavior emits `PRESENTATIONML_TIMING_FALLBACK`; this is not an exact PowerPoint timing implementation.

## Effects

| Feature | ECMA-376 Element | Current handling |
|---------|-----------------|--------|
| Text shadow | `<a:outerShdw>` | Approximate |
| Shape shadow | `<a:effectLst>` | Approximate |
| Reflection | `<a:reflection>` | Approximate for namespace-validated direct shapes (bounded CSS mirror/mask; typed attributes plus raw XML; no PowerPoint-fidelity claim), fallback otherwise |
| Glow | `<a:glow>` | Approximate |
| 3D scene and shape effects | `<a:scene3d>`, `<a:sp3d>` | Fallback (typed camera, camera rotation, light rig/rotation, material, depth/extrusion/contour, and top/bottom bevel properties plus context-qualified raw XML) |
| Effect DAG | `<a:effectDag>` | Fallback (typed name plus numerically source-ordered raw metadata) |

Advanced-effect metadata uses the private `drawingml-effect-metadata-v1` envelope and does not add fields to public `ShapeEffects`. Typed descendants require the exact DrawingML namespace and schema owner path; schema attributes must be unqualified, so foreign-prefixed descendants and attributes remain raw-only unsupported content and never feed rendering metadata. Encounter order is stored and sorted numerically. Raw XML is capped at 65,536 UTF-8 bytes per effect; typed string values are capped at 1,024 UTF-8 bytes. Truncated values retain their original byte length and deterministic FNV-1a 64-bit hash, and the diagnostic reason explicitly reports raw XML truncation.


## Generated PPTX capability registry

<!-- BEGIN GENERATED PPTX CAPABILITY MATRIX -->
<!-- manifest-sha256: dd24142f66dbd737b6ef27f77ac4bc433053bc1249e86965c34033a19b32da47 -->
| Feature | Current S/V/B | Target S/V/B | Verification SHA256 | Status SHA256 |
|---|---|---|---|---|
| <a id="capability-presentation"></a>`presentation` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `c07e2810b8d5e13a63436f7b11c3ee961e11b15f61bdc50a1ca260c0738e4a4f` | `29665c44b1b28428449e05099e8b3f5d22f1e577d8eaaf700a7f1c9a1b347de5` |
| <a id="capability-presentation-properties"></a>`presentation-properties` | approximate/parsed<br>fallback/not-applicable<br>fallback/not-applicable | approximate/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `03b3697960c6db57bc2d101452d5e8abc0a9ecd7ed2048d867a97032ccb94e5b` | `cf3d3cadc4899f4321326655a859005131cd42d60dc1e24accad86220543b42d` |
| <a id="capability-slide-master"></a>`slide-master` | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | `d26c42cad024a240ba42584139d32b0485d45f86a946ebc65d2cf2c2d9c920eb` | `2fcbe53ce1225a110400f235335397da53ab763ef52d242204931561cf098958` |
| <a id="capability-slide-layout"></a>`slide-layout` | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | `80a9fec92635d749ef0271cfb91e56a7c2b642a42f42a3719badde4160d0e329` | `fd2002a3e42946c1a1212cdb072c36fdc16f6aa2f56c1c6ae6920649413f4792` |
| <a id="capability-slide"></a>`slide` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `d7216600198cf446aa21948013131473434b57228017fdd7c2eea16a3aee2ed7` | `9ed1789d738b9c6f29e7712866cb1b72ef0b9798f5f78d5e3210d92d59eeaf4c` |
| <a id="capability-theme"></a>`theme` | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | `a1050e25c09f1b3687932cd923ac2c5e9ac8b8bd04ea694e1af75f7ff6397807` | `70df65b760e43407d76fcadcbc3fb5e52fe68c9cd94624c584352cc2bffb0921` |
| <a id="capability-notes-master"></a>`notes-master` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `7c0f6c034617ee80dfedda6fad705b98bd052084f09a7878d8f44c0b8637b507` | `f2dcd5a888468034bfcb5e696a84f70f017ab138c1727937b79cbbd743f21e3a` |
| <a id="capability-notes"></a>`notes` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `1b5af7f5ec83a70268e65aa5017a47d559c69452cea72f455c343edd4ac94e51` | `1e0e297d3d1c8e823ed852c6eb690944605bbf290c62c24bf39300d901642b7f` |
| <a id="capability-handout-master"></a>`handout-master` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `891c69a9b73e211f98dba58561eff7c132fccbe56f73cc738a94d39aa81c3b4a` | `9d44cff55da2c0159e8c5dcc8ead0ff6e9769ead1dd7e6e0c3efaabb2b811497` |
| <a id="capability-comments"></a>`comments` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `944fa74d1b1a1aec97d94eee1d54feb252a2b139a54939ff9388ded6595591b9` | `2ea3f2aafdfa77fd66c34f43fb85bfb4f993bf50cba40edee5eda1165d8340e9` |
| <a id="capability-comment-authors"></a>`comment-authors` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `3fdf939a544a498dda287c4cbec1ef75ccfbe8b3f5aa080ef114614b91d7900a` | `85ea90cb75643a556bd9dba65f0ce49610b7ff62b985d3ea8636f6cfbaa3ed1b` |
| <a id="capability-shape-tree"></a>`shape-tree` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `039ce2b4d821932f9c2243102b5c97dbcd41d0f4ecfc0f7e01b0fde941e7805e` | `3f86cff8d830a06e21d3779e44a9b21194756e2ad8955aefdeaba3fc9db1162a` |
| <a id="capability-preset-shape"></a>`preset-shape` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `cb84ad4e1f0ca5b1849c7a3331a9a878a3d0b3818352f158c405e19c87a88fd2` | `5d446d085d5c42ea91cc6540d5b83bbfaca15e62afe42e6f9c20d4d59ea9a86f` |
| <a id="capability-custom-geometry"></a>`custom-geometry` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `244b8537a5f7fd49e3fafd5a462a12d5f6cf0408a8cf3235e7645b0baefea8f5` | `99c76b2c42fdf8b00e68efc337816db612d39bb09426e39028af8db8b1051083` |
| <a id="capability-connector"></a>`connector` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `7e7b1b3a0a60e49d6702574dba2a1929d3e4c82abd8f7b60a7d162a0f63fa509` | `f469f88311b3de633ad23f2d8257cd92e2faaa75299ee824ac1279ee1f00367c` |
| <a id="capability-group-shape"></a>`group-shape` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `393472e96359637a79aa7a838f6c16db5b9d71b24cb648fefea81e3a646a41fb` | `e5f16afa6c7699ece99d11402306f0119f415730b8889499312d6be6083db36e` |
| <a id="capability-picture"></a>`picture` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `dffb48ca4b06c68069e0b407c9934ceaceb8dabf447bedac71f10b581a2ac645` | `7199c2265f56c189e0b25a8f38529f37da9174155adb2c46b2e236d3105947f8` |
| <a id="capability-text-body"></a>`text-body` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `262f4fb2d080594a9c78a70b702253e646af04a1e7e86f2d9b8debfe18f15e8b` | `bbbb778196c659c4ba3931d9f51c8383575a005812fde7c4f92a85d90cf53e89` |
| <a id="capability-rtl-text"></a>`rtl-text` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `c503fb10524fa65e82d1d4ea5d4de2579f51949547d1de8ad5cb1b496f0070e5` | `85173066116d7250da3058a7f80b43b147cfeef918f4cf802bbf94dff3613c65` |
| <a id="capability-bullets"></a>`bullets` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `ed157a688196eea774392c88ae5db59cb6cfc0f7167532360488ca899ebdff3d` | `7083d9593322381b21f9ac938277da2637c57b8e9663fe7baf886efe289ff341` |
| <a id="capability-picture-bullets"></a>`picture-bullets` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `7d1c21ce2540da7b56a5a48196f9f4d69d56c985e23afd6772a5b96d1de5508f` | `d4d97387d415bb350ee62522151319c7190d7a60f9fc6a33ad16fd2953d680d0` |
| <a id="capability-fills"></a>`fills` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `f7a7e6203cadf6138eda6a0262ea7f8413a200044cbaa8be71445d6ee0d08e7b` | `27f0d1439c068d3dcdc802df5c98749ad63a526753bf4e411cd97c0a5025cac2` |
| <a id="capability-pattern-fill"></a>`pattern-fill` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `aa65f7e14d906cfa690b48408c5e59168b09e5ec7f29366695a335778beb8fab` | `e7687dc0b1523f4d8d835a27538507091664a2af8daf52c7cefa2253b28a7171` |
| <a id="capability-effects"></a>`effects` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `768bd8e0e131deaf5a963f37a66952f4287ebff4a860cf8d2fda726f2f67968d` | `7e79f784844b8576e35fa68dce69588d336125fab6a6e84caf40373b91880b73` |
| <a id="capability-reflection-and-3d"></a>`reflection-and-3d` | fallback/parsed<br>approximate/rendered<br>fallback/not-applicable | fallback/parsed<br>approximate/rendered<br>fallback/not-applicable | `05625623d02d2afb0f7c3529951fd70e1f3611f7ba5acacb447b5e512abac08d` | `22d06b2ad85fb0a25de923ea582f347b688efbb1330a7e110660f45afea9c183` |
| <a id="capability-table"></a>`table` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `4e5951bb9a4549790b7adc79890517a1225009b40688d246c11850c66101d192` | `7ae399ecfa572df16f042587cb995cdb8c754fbf48cc584ff6c28c79083e8d3b` |
| <a id="capability-table-style"></a>`table-style` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `e87d1531fdab2c0c063de4a617627c411454f05c6359e2b93c499fed5638617e` | `8507e8b5258344ccbf42786395cd9e9c1305007d9abc67292710353c91254cce` |
| <a id="capability-image"></a>`image` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `66ee0ff62f62adf90b2cb61bd3298f76d6db7d7e54e03632ffd5ff38e026714a` | `cf16268eadaa17f2829467c88b11c2858d7c58fd445c2c45d803d7b38ac8c213` |
| <a id="capability-chart-direct-subset"></a>`chart-direct-subset` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `9f1b331a89dc0443e3f4a31837f1ab9da612a9570c789fd9dd8e0503e9600643` | `377a904a5d76d39a2ba0164bfcaa24fe1b451c01555b70940225ffd655df7287` |
| <a id="capability-chart-preview-fallback"></a>`chart-preview-fallback` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `9b15b4f2cefcc9a46086fd4b54264d753f9e874554b4e153c0e4f8f5fb15ea29` | `587a7fd372d58f5da936b784d45cbdfa7536d5c3a5a95d31d5274264c8dc0c73` |
| <a id="capability-chart-placeholder-fallback"></a>`chart-placeholder-fallback` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `5f0b4fdecb60710becd532d16762d18734667afea2cda8d28449a5f25da1f9ad` | `4d57460e0f8ebae9e2e593c40d9876782b2e0bb6cfd1dfb8eb6d8e9730b8d49b` |
| <a id="capability-diagram"></a>`diagram` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `0cecd24eb6161f5bad365f66ddf4877732436c3ca3e0e67dfb2a76475572cf3b` | `2bb9eca9b9fd5342b7090b50836f0832acfe59b7d877dd77a8a172efcd3d2e0b` |
| <a id="capability-diagram-data"></a>`diagram-data` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `33d058f921ab4bf96eb875079f95b5c6a103dfd9fbb60ecb5c6b54684882aa19` | `e63c3b734b25079b0df064d2f74f4f085d4d8e6b345afb3b04b45c6f639625fb` |
| <a id="capability-diagram-layout"></a>`diagram-layout` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `39729d3ac8e6afc2d55966c8170f8fbc9412921364b9c031faf980945f9e08fb` | `5c71485b56affe554eccbc54e7c24d5f8d267033dc34b95065fe6ddab4da9427` |
| <a id="capability-diagram-styles"></a>`diagram-styles` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `0dce3209140ee3800b43953d43b6d77dd727cbe19ea1699d088bf2ffccee8725` | `e5cfd249fd43693753b370d54b8846c9eb397e0583fb734ef665c394db77ee19` |
| <a id="capability-diagram-colors"></a>`diagram-colors` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `835995fb45ba39bcffc948c66a1714647a4dde4f45b860bbf04c6e32918dc681` | `c8048fc748ccbf5216d5c9b3e55fcef0ac3fcd062ca75ee75f10378a49429032` |
| <a id="capability-ole-embedded-object"></a>`ole-embedded-object` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `9a1ea008d8a2422d170624f54c315e1e1ff435dee7a9f7528ab130827840486b` | `03972ca8681ad5adfff52f278be1c4c35b0ebaf19251d75f88b1f4eed8a04cc6` |
| <a id="capability-math"></a>`math` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `f902b654054d7ac1aaea679b5832d73bbb121c6d14a593df127bb97a77df9dbd` | `98676a3ba2f695ae7b3fc77b29d51d0b65cc21c8cdbe976aa61777b5637c29c6` |
| <a id="capability-media-audio"></a>`media-audio` | approximate/parsed<br>approximate/rendered<br>approximate/rendered | approximate/parsed<br>approximate/rendered<br>approximate/rendered | `72f9f2545ef7b485e028296680e9943b5b679f55ec7bfc267a4659fa459c2bdb` | `115a7ac4ad92809c52144bca695530c20c42876eb4cd62a92903a793721370ef` |
| <a id="capability-media-video"></a>`media-video` | approximate/parsed<br>approximate/rendered<br>approximate/rendered | approximate/parsed<br>approximate/rendered<br>approximate/rendered | `2de9f9aa1ac20fdda24dff34d3317856b28bc00dcaee216df808cee57158ae08` | `55c5b1bd4d7d05b9e7f5297572607be5fd9e1607eb98bec331ea411c041b83db` |
| <a id="capability-hyperlink-run-and-cell"></a>`hyperlink-run-and-cell` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `8078ebcf0df8602a6acf21547e7e42a8ade526d127fd7e921d249ae07b88d993` | `57dc2d2d733cbce264d1b225496048d1d95072ddb15fce3c38f4b8728124983b` |
| <a id="capability-shape-hyperlink-and-action"></a>`shape-hyperlink-and-action` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `d240b3956c52dba4526750cddd4d9c7a2690f59c295075997f6e3bb46b71664f` | `db947d63b09b3d26c18ff44bc685731344501a9beb14ad530512501e04230603` |
| <a id="capability-timing-and-animation"></a>`timing-and-animation` | approximate/parsed<br>approximate/rendered<br>approximate/rendered | approximate/parsed<br>approximate/rendered<br>approximate/rendered | `ee976c5f050029d337e0ea3a1ff5cfe3351b9aa59f3da5042e507eeaecfa521f` | `30e10705c96190b94004219490a97a7116fcf5f49a9c0b45ca5730fe39f1ce35` |
| <a id="capability-transitions"></a>`transitions` | approximate/parsed<br>approximate/rendered<br>approximate/rendered | approximate/parsed<br>approximate/rendered<br>approximate/rendered | `e06c8a2724ec2b5c11b4f4fbea9c88c66d4957fa756d15c2d27a543f6cf6719c` | `bdc5bc99fe9a448365a3d9721e6e67ca4df2fea07b674856df476f9aedfaef1c` |
| <a id="capability-extensions"></a>`extensions` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `4894bba77c5de06b7102b327fc78201befcc59a7b37cf9aa2f85c1f8e6ac0305` | `b36f463983b9b6f31f21ee7624b8179f3c336069e97235efd07f4c6933e6ad25` |
| <a id="capability-alternate-content"></a>`alternate-content` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `568918777892e84262c3bf521a5297a698db8831598d085a54cbf2840280c221` | `fb843b603490ab7412c7d1c34c18389bbfb9b5d8b973116d530064eec8caee18` |
| <a id="capability-bibliography"></a>`bibliography` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `e23a461817a9ded877cb7eb1e4979501178769765e246971ab74578a4ffe4ebb` | `30ae0425dec8aa78fc8c534d721be0277cce56b84761cfbdc4562175005a5f25` |
| <a id="capability-additional-characteristics"></a>`additional-characteristics` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `f10ed7446d28df9e489140d5c04044a23d86d782cdcfad33eaf6fb000fc8aaf2` | `3be632f7c8a7c60cff5633dd014bdf1f7e036a8c6431adc7bec1e6b8ec3ab2af` |
| <a id="capability-custom-xml"></a>`custom-xml` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `6328918018db2ff76d4aa2d8c8b27bbdace4bc71d46fba6b764209026b2c94c6` | `0e27bb416ec6d01d306d50e4976418e0743916f7531705fd88f99aa855983008` |
| <a id="capability-thumbnail"></a>`thumbnail` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `eea1202e0937556ba322f690e25073981337ab75cb3f640432aed42981fb1a83` | `ac63bbea2b37bedfb131e943838539e1d7373e7a3686fea14f95ee8dfed820c3` |
| <a id="capability-theme-override"></a>`theme-override` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `a6c3565ab75b88f7bcd341876512c4752c266a017d1d6d5ba08aa37b5cda995d` | `8dbd8139836a153e1e69009efcb939bd980708c67e19e790aafba14bb2c71dc9` |
| <a id="capability-slide-synchronization"></a>`slide-synchronization` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `d251deca6b42414d070751e4e079abd3c75abebf6fb296bb9c61d48be6e604d1` | `7108f8d030277f501eccd5e01cfef2389496178cf6128c7ae5248a8b067d1d42` |
| <a id="capability-content-part"></a>`content-part` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `2f7dce33c2e6808355a43fe0820855450ade1abd9fffe83bf6989965dc3da5d9` | `a6dc798a71b64907ffa02c9c93548a78f91ec78b0fca9852ffa861abd11f649e` |
| <a id="capability-embedded-package"></a>`embedded-package` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `9c9a08d8fb4442f66df36bc3de23ca6a0d0448bab2260996ed41c262cca6d5c0` | `1027870090ccee53b686f31b5098514211c5799b534f317f604406e734c57627` |
| <a id="capability-embedded-control-persistence"></a>`embedded-control-persistence` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `8cf43f357e46ee3defd6250fa099d6f88a37f4ac976b58cb6e5c6898c1785ce2` | `9353e1d1789f94b67689440757d4617fa6f283426188298e1914fb12f0922f82` |
| <a id="capability-user-defined-tags"></a>`user-defined-tags` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `d2bd311c48e46b4ba4449d05eb1b99762d2cc782adb325ec275a07b84c29a6d7` | `c5d90044021cd20e3c67fe72a821ca0073e55a3dde89af7916abfc57ce31f26c` |
<!-- END GENERATED PPTX CAPABILITY MATRIX -->
