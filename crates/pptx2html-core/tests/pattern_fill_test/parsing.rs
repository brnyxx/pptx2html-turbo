use pptx2html_core::convert_bytes;
use pptx2html_core::model::presentation::{ClrMap, ColorScheme};
use pptx2html_core::model::{Color, ColorKind, ColorModifier, Fill, StyleRef};
use pptx2html_core::parser::PptxParser;
use pptx2html_core::resolver::style_ref::resolve_fill_ref;

use super::{fixtures::MinimalPptx, shape_with_pattern};

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
