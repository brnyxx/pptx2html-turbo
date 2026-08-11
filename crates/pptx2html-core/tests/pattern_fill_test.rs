mod fixtures;

use std::collections::HashSet;

use fixtures::MinimalPptx;
use pptx2html_core::model::presentation::{ClrMap, ColorScheme};
use pptx2html_core::model::{Color, ColorKind, ColorModifier, Fill, PatternPreset, StyleRef};
use pptx2html_core::parser::PptxParser;
use pptx2html_core::resolver::style_ref::resolve_fill_ref;
use pptx2html_core::{convert_bytes, convert_bytes_with_metadata};

const OFFICIAL_PATTERNS: [&str; 54] = [
    "pct5",
    "pct10",
    "pct20",
    "pct25",
    "pct30",
    "pct40",
    "pct50",
    "pct60",
    "pct70",
    "pct75",
    "pct80",
    "pct90",
    "horz",
    "vert",
    "ltHorz",
    "ltVert",
    "dkHorz",
    "dkVert",
    "narHorz",
    "narVert",
    "dashHorz",
    "dashVert",
    "cross",
    "dnDiag",
    "upDiag",
    "ltDnDiag",
    "ltUpDiag",
    "dkDnDiag",
    "dkUpDiag",
    "wdDnDiag",
    "wdUpDiag",
    "dashDnDiag",
    "dashUpDiag",
    "diagCross",
    "smCheck",
    "lgCheck",
    "smGrid",
    "lgGrid",
    "dotGrid",
    "smConfetti",
    "lgConfetti",
    "horzBrick",
    "diagBrick",
    "solidDmnd",
    "openDmnd",
    "dotDmnd",
    "plaid",
    "sphere",
    "weave",
    "divot",
    "shingle",
    "wave",
    "trellis",
    "zigZag",
];

fn shape_with_pattern(preset: &str, colors: &str) -> String {
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Pattern"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="914400" y="914400"/><a:ext cx="1828800" cy="1828800"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:pattFill prst="{preset}">{colors}</a:pattFill></p:spPr></p:sp>"#
    )
}

#[test]
fn classifies_the_exact_official_pattern_inventory() {
    let parsed: Vec<String> = OFFICIAL_PATTERNS
        .iter()
        .map(|name| PatternPreset::from_ooxml(name).as_ooxml().to_owned())
        .collect();

    assert_eq!(parsed, OFFICIAL_PATTERNS);
    assert!(matches!(
        PatternPreset::from_ooxml("futurePattern"),
        PatternPreset::Unknown(raw) if raw == "futurePattern"
    ));
}

#[test]
fn parses_pattern_colors_and_preserves_missing_colors() {
    let colors = r#"<a:fgClr><a:schemeClr val="accent1"><a:tint val="70000"/><a:alpha val="80000"/></a:schemeClr></a:fgClr><a:bgClr><a:prstClr val="white"/></a:bgClr>"#;
    let bytes = MinimalPptx::new(&shape_with_pattern("pct20", colors)).build();
    let presentation = PptxParser::parse_bytes(&bytes).unwrap();

    let Fill::Pattern(pattern) = &presentation.slides[0].shapes[0].fill else {
        panic!("expected parsed pattern fill");
    };
    assert_eq!(pattern.preset.as_ooxml(), "pct20");
    assert!(
        matches!(pattern.foreground.as_ref().map(|c| &c.kind), Some(ColorKind::Theme(name)) if name == "accent1")
    );
    assert_eq!(pattern.foreground.as_ref().unwrap().modifiers.len(), 2);
    assert!(
        matches!(pattern.background.as_ref().map(|c| &c.kind), Some(ColorKind::Preset(name)) if name == "white")
    );

    let missing = MinimalPptx::new(&shape_with_pattern("cross", "")).build();
    let presentation = PptxParser::parse_bytes(&missing).unwrap();
    let Fill::Pattern(pattern) = &presentation.slides[0].shapes[0].fill else {
        panic!("expected parsed pattern fill");
    };
    assert!(pattern.foreground.is_none());
    assert!(pattern.background.is_none());
}

#[test]
fn renders_known_pattern_as_a_deterministic_repeated_tile() {
    let colors = r#"<a:fgClr><a:srgbClr val="336699"/></a:fgClr><a:bgClr><a:srgbClr val="F2F2F2"/></a:bgClr>"#;
    let bytes = MinimalPptx::new(&shape_with_pattern("diagCross", colors)).build();

    let first = convert_bytes(&bytes).unwrap();
    let second = convert_bytes(&bytes).unwrap();

    assert_eq!(first, second);
    assert!(first.contains("data:image/svg+xml;base64,"));
    assert!(first.contains("background-repeat: repeat"));
    assert!(!first.contains("background-color: #336699"));
}

#[test]
fn preserves_unknown_pattern_semantics_without_a_solid_fallback() {
    let hostile = "futurePattern&lt;/script&gt;";
    let colors = r#"<a:fgClr><a:sysClr val="windowText" lastClr="112233"><a:shade val="50000"/></a:sysClr></a:fgClr>"#;
    let bytes = MinimalPptx::new(&shape_with_pattern(hostile, colors)).build();

    let result = convert_bytes_with_metadata(&bytes).unwrap();
    let diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "DRAWINGML_PATTERN_UNSUPPORTED")
        .collect();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].location.qualified_element_name.as_deref(),
        Some("a:pattFill")
    );
    assert!(
        diagnostics[0]
            .raw_reference
            .as_deref()
            .unwrap()
            .contains(r"futurePattern\u003C/script\u003E")
    );
    assert!(!result.html.contains("background-color: #112233"));
    assert!(!result.html.contains("</script><script"));
}

#[test]
fn renders_svg_table_cell_and_direct_background_patterns() {
    let pattern = r#"<a:pattFill prst="cross"><a:fgClr><a:srgbClr val="336699"/></a:fgClr><a:bgClr><a:srgbClr val="F2F2F2"/></a:bgClr></a:pattFill>"#;
    let slide = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:bg><p:bgPr>{pattern}</p:bgPr></p:bg><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Ellipse"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="ellipse"/>{pattern}</p:spPr></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Custom"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="914400" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l="0" t="0" r="100" b="100"/><a:pathLst><a:path w="100" h="100"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="100" y="0"/></a:lnTo><a:lnTo><a:pt x="50" y="100"/></a:lnTo><a:close/></a:path></a:pathLst></a:custGeom>{pattern}</p:spPr></p:sp>
<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="4" name="Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="914400"/><a:ext cx="1828800" cy="914400"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="1828800"/></a:tblGrid><a:tr h="914400"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>Pattern</a:t></a:r></a:p></a:txBody><a:tcPr>{pattern}</a:tcPr></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>
</p:spTree></p:cSld></p:sld>"#
    );
    let bytes = MinimalPptx::new("").with_raw_slide(&slide).build();

    let html = convert_bytes(&bytes).unwrap();

    assert!(html.matches("<pattern id=\"pattern-s1-cross-").count() >= 2);
    assert!(html.matches("data:image/svg+xml;base64,").count() >= 2);
    assert!(html.contains("<td style=\"background-image:"));
}

#[test]
fn keeps_identical_unknown_pattern_diagnostics_distinct() {
    let colors = r#"<a:fgClr><a:srgbClr val="112233"/></a:fgClr><a:bgClr><a:srgbClr val="FFFFFF"/></a:bgClr>"#;
    let body = format!(
        "{}{}",
        shape_with_pattern("sameUnknown", colors),
        shape_with_pattern("sameUnknown", colors)
    );
    let result = convert_bytes_with_metadata(&MinimalPptx::new(&body).build()).unwrap();
    let diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "DRAWINGML_PATTERN_UNSUPPORTED")
        .collect();

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(
        diagnostics[0].location.relationship_id.as_deref(),
        Some("pattern-s0-e0")
    );
    assert_eq!(
        diagnostics[1].location.relationship_id.as_deref(),
        Some("pattern-s0-e1")
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        !diagnostic
            .location
            .relationship_id
            .as_deref()
            .unwrap()
            .contains("sameUnknown")
    }));
}

#[test]
fn renders_all_official_presets_with_distinct_tile_signatures() {
    let colors = r#"<a:fgClr><a:srgbClr val="224466"/></a:fgClr><a:bgClr><a:srgbClr val="F8F8F8"/></a:bgClr>"#;
    let body: String = OFFICIAL_PATTERNS
        .iter()
        .map(|preset| shape_with_pattern(preset, colors))
        .collect();

    let html = convert_bytes(&MinimalPptx::new(&body).build()).unwrap();
    let signatures: Vec<&str> = html
        .split("data:image/svg+xml;base64,")
        .skip(1)
        .filter_map(|rest| rest.split_once(')').map(|(signature, _)| signature))
        .collect();
    let unique: HashSet<&str> = signatures.iter().copied().collect();

    assert_eq!(signatures.len(), OFFICIAL_PATTERNS.len());
    assert_eq!(unique.len(), OFFICIAL_PATTERNS.len());
}

#[test]
fn parses_theme_pattern_and_substitutes_phclr_with_ordered_modifiers() {
    let theme = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="PatternTheme"><a:themeElements>
<a:clrScheme name="Colors"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:accent1><a:srgbClr val="4472C4"/></a:accent1></a:clrScheme>
<a:fontScheme name="Fonts"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
<a:fmtScheme name="Patterns"><a:fillStyleLst><a:pattFill prst="wave"><a:fgClr><a:schemeClr val="phClr"><a:tint val="70000"/><a:alpha val="80000"/></a:schemeClr></a:fgClr><a:bgClr><a:srgbClr val="FFFFFF"/></a:bgClr></a:pattFill></a:fillStyleLst><a:lnStyleLst/><a:effectStyleLst/><a:bgFillStyleLst/></a:fmtScheme>
</a:themeElements></a:theme>"#;
    let presentation =
        PptxParser::parse_bytes(&MinimalPptx::new("").with_full_theme(theme).build()).unwrap();
    let fmt = &presentation.primary_theme().unwrap().fmt_scheme;
    let reference = StyleRef {
        idx: 1,
        color: Color::theme("accent1"),
    };

    let Fill::Pattern(pattern) =
        resolve_fill_ref(&reference, fmt, &ColorScheme::default(), &ClrMap::default()).unwrap()
    else {
        panic!("expected theme pattern fill");
    };
    let foreground = pattern.foreground.unwrap();
    assert!(matches!(foreground.kind, ColorKind::Theme(ref name) if name == "accent1"));
    assert!(matches!(
        foreground.modifiers.as_slice(),
        [ColorModifier::Tint(70000), ColorModifier::Alpha(80000)]
    ));
}

#[test]
fn inherits_pattern_fills_from_layout_shape_and_master_background() {
    let colors = r#"<a:fgClr><a:srgbClr val="336699"/></a:fgClr><a:bgClr><a:srgbClr val="FFFFFF"/></a:bgClr>"#;
    let master = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:bg><p:bgPr><a:pattFill prst="smGrid">{colors}</a:pattFill></p:bgPr></p:bg><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/></p:sldMaster>"#
    );
    let layout = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="Layout Placeholder"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="ellipse"/><a:pattFill prst="wave">{colors}</a:pattFill></p:spPr></p:sp></p:spTree></p:cSld></p:sldLayout>"#
    );
    let slide = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Slide Placeholder"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="ellipse"/></p:spPr></p:sp>"#;
    let bytes = MinimalPptx::new(slide)
        .with_full_master(&master)
        .with_layout(&layout)
        .build();

    let presentation = PptxParser::parse_bytes(&bytes).unwrap();
    assert!(matches!(
        presentation.masters[0].background,
        Some(Fill::Pattern(_))
    ));
    assert!(matches!(
        presentation.layouts[0].shapes[0].fill,
        Fill::Pattern(_)
    ));
    let html = convert_bytes(&bytes).unwrap();
    assert!(html.contains("pattern-s1-wave-"));
    assert!(html.contains("data:image/svg+xml;base64,"));
}

#[test]
fn assigns_empty_pattern_elements_to_shape_table_and_background_surfaces() {
    let slide = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:bg><p:bgPr><a:pattFill prst="horz"/></p:bgPr></p:bg><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:prstGeom prst="rect"/><a:pattFill prst="vert"/></p:spPr></p:sp><p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="3" name="table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="1000"/></a:tblGrid><a:tr h="1000"><a:tc><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr><a:pattFill prst="cross"/></a:tcPr></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame></p:spTree></p:cSld></p:sld>"#;
    let presentation =
        PptxParser::parse_bytes(&MinimalPptx::new("").with_raw_slide(slide).build()).unwrap();

    assert!(matches!(
        presentation.slides[0].background,
        Some(Fill::Pattern(_))
    ));
    assert!(matches!(
        presentation.slides[0].shapes[0].fill,
        Fill::Pattern(_)
    ));
    let pptx2html_core::model::ShapeType::Table(table) =
        &presentation.slides[0].shapes[1].shape_type
    else {
        panic!("expected table shape");
    };
    assert!(matches!(table.rows[0].cells[0].fill, Fill::Pattern(_)));
}

#[test]
fn reports_missing_pattern_colors_without_nonfinite_or_solid_output() {
    let bytes = MinimalPptx::new(&shape_with_pattern("wave", "")).build();
    let result = convert_bytes_with_metadata(&bytes).unwrap();

    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "DRAWINGML_PATTERN_UNSUPPORTED")
            .count(),
        1
    );
    assert!(!result.html.contains("data:image/svg+xml;base64,"));
    assert!(!result.html.contains("NaN"));
    assert!(!result.html.contains("Infinity"));
}
