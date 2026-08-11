mod fixtures;

use std::io::{Cursor, Write};

use pptx2html_core::ConversionOptions;
use pptx2html_core::ProvenanceSource;
use pptx2html_core::model::presentation::{ClrMap, ColorScheme};
use pptx2html_core::model::{
    Alignment, AutoFit, Border, BorderStyle, Bullet, ChartData, ChartDataLabelPosition,
    ChartDataLabelSettings, ChartGrouping, ChartOfPieType, ChartSeries, ChartSpec, ChartSplitType,
    ChartType, ClrMapOverride, Color, ColorKind, CompoundLine, ConnectionRef, ConnectionSite,
    CropRect, CustomGeometry, DashStyle, Emu, Fill, FmtScheme, GeomRect, GeometryPath,
    GradientFill, GradientStop, GradientType, GroupData, ImageFill, LineAlignment, LineCap,
    LineEnd, LineEndSize, LineEndType, LineJoin, ListStyle, ParagraphDefaults, PathFill,
    PictureData, PlaceholderInfo, PlaceholderType, Presentation, RunDefaults, Shape, ShapeStyleRef,
    ShapeType, Size, Slide, SlideLayout, SlideMaster, SolidFill, SpacingValue, StrikethroughType,
    StyleRef, TableCell, TableData, TableRow, TextBody, TextCapitalization, TextMargins,
    TextParagraph, TextRun, TextStyle, UnderlineType, VerticalAlign,
};
use pptx2html_core::parser::PptxParser;
use pptx2html_core::parser::master_parser;
use pptx2html_core::renderer::HtmlRenderer;
use pptx2html_core::resolver::inheritance;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

fn parse_pptx(data: &[u8]) -> pptx2html_core::model::Presentation {
    PptxParser::parse_bytes(data).expect("PPTX parsing failed")
}

fn render_with_metadata(
    data: &[u8],
) -> pptx2html_core::error::PptxResult<pptx2html_core::ConversionResult> {
    pptx2html_core::convert_bytes_with_metadata(data)
}

fn render_model_shapes(shapes: Vec<Shape>) -> String {
    let mut presentation = Presentation {
        slide_size: Size {
            width: Emu(9_144_000),
            height: Emu(6_858_000),
        },
        ..Default::default()
    };
    presentation.slides.push(Slide {
        shapes,
        ..Default::default()
    });
    HtmlRenderer::render(&presentation).expect("HTML rendering should succeed")
}

fn renderer_chart_shape(spec: ChartSpec) -> Shape {
    Shape {
        shape_type: ShapeType::Chart(ChartData {
            rel_id: "rIdChart".to_string(),
            preview_image: None,
            preview_mime: None,
            direct_spec: Some(spec),
        }),
        size: Size {
            width: Emu(1_828_800),
            height: Emu(914_400),
        },
        ..Default::default()
    }
}

fn zip_entries(entries: &[(&str, String)]) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();
    for (path, xml) in entries {
        zip.start_file(path, options).expect("start zip entry");
        zip.write_all(xml.as_bytes()).expect("write zip entry");
    }
    zip.finish().expect("finish zip").into_inner()
}

#[test]
fn parses_presentation_relationship_fallbacks_through_public_parser() {
    let many_shapes = (0..105)
        .map(|idx| {
            format!(
                r#"<p:sp><p:nvSpPr><p:cNvPr id="{}" name="Shape {}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr></p:sp>"#,
                idx + 10,
                idx
            )
        })
        .collect::<String>();
    let presentation_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rIdMaster"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rIdSlide"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:defaultTextStyle>
    <a:lvl1pPr>
      <a:lnSpc><a:spcPct val="150000"/></a:lnSpc>
      <a:spcBef><a:spcPts val="200"/></a:spcBef>
      <a:spcAft><a:spcPts val="400"/></a:spcAft>
      <a:defRPr sz="1800"><a:schemeClr val="accent2"></a:schemeClr></a:defRPr>
    </a:lvl1pPr>
  </p:defaultTextStyle>
</p:presentation>"#
        .to_string();
    let presentation_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdMaster" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rIdSlide" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#
        .to_string();
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#
        .to_string();
    let master_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="ppt/theme/theme1.xml"/>
  <Relationship Id="rIdLayout1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
  <Relationship Id="rIdLayout2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout2.xml"/>
  <Relationship Id="rIdLayoutDuplicate" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
  <Relationship Id="rIdLayoutMissing" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/missingLayout.xml"/>
</Relationships>"#
        .to_string();
    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg><p:bgPr><a:srgbClr val="ABCDEF"></a:srgbClr></p:bgPr></p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="Layout Edge Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:ln w="12700"><a:prstDash val="unknownDash"/></a:ln></p:spPr>
        <p:txBody><a:bodyPr/><a:lstStyle><a:lvl1pPr><a:spcBef><a:spcPct val="70000"/></a:spcBef><a:lnSpc><a:spcPts val="600"/></a:lnSpc><a:spcAft><a:spcPts val="300"/></a:spcAft></a:lvl1pPr></a:lstStyle></p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sldLayout>"#
        .to_string();
    let layout2_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg><p:bgPr><a:schemeClr val="accent1"></a:schemeClr></p:bgPr></p:bg>
    <p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree>
  </p:cSld>
</p:sldLayout>"#
        .to_string();
    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Theme">
  <a:themeElements>
    <a:clrScheme name="Scheme"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2></a:clrScheme>
    <a:fontScheme name="Fonts"><a:majorFont><a:latin typeface="Aptos"/></a:majorFont><a:minorFont><a:latin typeface="Aptos"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#
        .to_string();
    let slide_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{many_shapes}</p:spTree></p:cSld>
</p:sld>"#
    );
    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdLayout" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#
        .to_string();

    let pptx = zip_entries(&[
        ("ppt/presentation.xml", presentation_xml),
        ("ppt/_rels/presentation.xml.rels", presentation_rels),
        ("ppt/slideMasters/slideMaster1.xml", master_xml),
        ("ppt/slideMasters/_rels/slideMaster1.xml.rels", master_rels),
        ("ppt/slideLayouts/slideLayout1.xml", layout_xml),
        ("ppt/slideLayouts/slideLayout2.xml", layout2_xml),
        ("ppt/theme/theme1.xml", theme_xml),
        ("ppt/slides/slide1.xml", slide_xml),
        ("ppt/slides/_rels/slide1.xml.rels", slide_rels),
    ]);

    let presentation = parse_pptx(&pptx);
    assert_eq!(presentation.slides.len(), 1);
    assert_eq!(presentation.layouts.len(), 2);
    assert_eq!(presentation.slides[0].shapes.len(), 105);
    assert_eq!(presentation.slides[0].layout_idx, Some(0));
    assert!(presentation.default_text_style.is_some());
    let defaults = presentation
        .default_text_style
        .as_ref()
        .and_then(|style| style.levels[0].as_ref())
        .expect("level 1 defaults");
    assert!(matches!(
        defaults.line_spacing,
        Some(SpacingValue::Percent(v)) if (v - 1.5).abs() < 1e-6
    ));
    assert!(matches!(
        defaults.space_before,
        Some(SpacingValue::Points(v)) if (v - 2.0).abs() < 1e-6
    ));
    assert!(matches!(
        defaults.space_after,
        Some(SpacingValue::Points(v)) if (v - 4.0).abs() < 1e-6
    ));
    assert!(matches!(
        defaults
            .def_run_props
            .as_ref()
            .and_then(|rd| rd.color.as_ref())
            .map(|color| &color.kind),
        Some(ColorKind::Theme(name)) if name == "accent2"
    ));
}

#[test]
fn parses_invalid_presentation_xml_through_public_parser() {
    let pptx = zip_entries(&[(
        "ppt/presentation.xml",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><"#
            .to_string(),
    )]);

    let err = PptxParser::parse_bytes(&pptx).expect_err("invalid presentation xml should fail");
    assert!(matches!(err, pptx2html_core::error::PptxError::Xml(_)));
}

#[test]
fn parses_master_remaining_start_tag_and_error_paths_through_public_parser() {
    let mut empty_archive = zip::ZipArchive::new(Cursor::new(zip_entries(&[]))).expect("empty zip");
    let start_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg><p:bgPr><a:schemeClr val="accent2"></a:schemeClr></p:bgPr></p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="Master Shape"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr>
        <p:spPr><a:ln w="12700"><a:prstDash val="unknownDash"/></a:ln></p:spPr>
        <p:txBody><a:bodyPr/><a:lstStyle><a:lvl1pPr><a:spcAft><a:spcPts val="400"></a:spcPts></a:spcAft><a:defRPr><a:srgbClr val="111111"></a:srgbClr></a:defRPr></a:lvl1pPr></a:lstStyle></p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:txStyles><p:titleStyle><a:lvl1pPr><a:spcAft><a:spcPts val="800"></a:spcPts></a:spcAft><a:defRPr><a:srgbClr val="222222"></a:srgbClr></a:defRPr></a:lvl1pPr></p:titleStyle></p:txStyles>
</p:sldMaster>"#;
    let master = master_parser::parse_slide_master(
        start_xml,
        &std::collections::HashMap::new(),
        &mut empty_archive,
    )
    .expect("master start-tag matrix parses");
    assert_eq!(master.shapes.len(), 1);
    assert!(matches!(
        master.shapes[0].border.dash_style,
        DashStyle::Solid
    ));
    assert!(master.tx_styles.title_style.is_some());

    let mut empty_archive = zip::ZipArchive::new(Cursor::new(zip_entries(&[]))).expect("empty zip");
    let empty_bg_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:bg><p:bgPr><a:srgbClr val="ABCDEF"/></p:bgPr></p:bg><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sldMaster>"#;
    let master = master_parser::parse_slide_master(
        empty_bg_xml,
        &std::collections::HashMap::new(),
        &mut empty_archive,
    )
    .expect("master empty bg parses");
    assert!(matches!(
        master.background,
        Some(Fill::Solid(fill)) if fill.color.to_css().as_deref() == Some("#ABCDEF")
    ));

    let mut empty_archive = zip::ZipArchive::new(Cursor::new(zip_entries(&[]))).expect("empty zip");
    assert!(
        master_parser::parse_slide_master(
            "<p:sldMaster xmlns:p=\"p\"><p:cSld><",
            &std::collections::HashMap::new(),
            &mut empty_archive,
        )
        .is_err()
    );
}

#[test]
fn covers_hierarchy_and_inheritance_edge_paths_through_public_api() {
    assert!(matches!(
        PlaceholderType::from_ooxml("sldNum"),
        Some(PlaceholderType::SldNum)
    ));
    assert!(matches!(
        PlaceholderType::from_ooxml("hdr"),
        Some(PlaceholderType::Hdr)
    ));
    assert!(matches!(
        PlaceholderType::from_ooxml("sldImg"),
        Some(PlaceholderType::SldImg)
    ));
    assert!(PlaceholderType::from_ooxml("unknown-placeholder").is_none());

    let red = Fill::Solid(SolidFill {
        color: Color::rgb("FF0000"),
    });
    let green = Fill::Solid(SolidFill {
        color: Color::rgb("00FF00"),
    });
    let layout = SlideLayout {
        background: Some(red.clone()),
        ..Default::default()
    };
    let master = SlideMaster {
        background: Some(green.clone()),
        ..Default::default()
    };
    assert!(matches!(
        inheritance::background_source(&Slide::default(), Some(&layout), None),
        ProvenanceSource::LayoutBackground
    ));
    assert!(matches!(
        inheritance::background_source(&Slide::default(), None, Some(&master)),
        ProvenanceSource::MasterBackground
    ));

    let no_fill_shape = Shape {
        fill: Fill::NoFill,
        ..Default::default()
    };
    assert!(matches!(
        inheritance::resolve_shape_fill(&no_fill_shape, None, None),
        Fill::NoFill
    ));
    let no_fill_layout = Shape {
        fill: Fill::NoFill,
        ..Default::default()
    };
    assert!(matches!(
        inheritance::resolve_shape_fill(&Shape::default(), Some(&no_fill_layout), None),
        Fill::NoFill
    ));
    let no_fill_master = Shape {
        fill: Fill::NoFill,
        ..Default::default()
    };
    assert!(matches!(
        inheritance::resolve_shape_fill(
            &Shape::default(),
            Some(&Shape::default()),
            Some(&no_fill_master)
        ),
        Fill::NoFill
    ));
    assert!(matches!(
        inheritance::resolve_shape_fill_with_theme(&no_fill_shape, None, None, None, None, None),
        Fill::NoFill
    ));
    assert!(matches!(
        inheritance::shape_fill_source(&Shape::default(), Some(&no_fill_layout), None, false),
        Some(ProvenanceSource::LayoutPlaceholder)
    ));
    assert!(matches!(
        inheritance::shape_fill_source(
            &Shape::default(),
            Some(&Shape::default()),
            Some(&no_fill_master),
            false
        ),
        Some(ProvenanceSource::MasterPlaceholder)
    ));

    let shape_with_style_line = Shape {
        style_ref: Some(ShapeStyleRef {
            ln_ref: Some(StyleRef {
                idx: 1,
                color: Color::rgb("123456"),
            }),
            ..Default::default()
        }),
        border: Border {
            dash_style: DashStyle::DashDot,
            cap: LineCap::Round,
            compound: CompoundLine::Double,
            alignment: LineAlignment::Inset,
            join: LineJoin::Bevel,
            miter_limit: Some(2.5),
            head_end: Some(LineEnd {
                end_type: LineEndType::Triangle,
                width: LineEndSize::Large,
                length: LineEndSize::Small,
            }),
            tail_end: Some(LineEnd {
                end_type: LineEndType::Oval,
                width: LineEndSize::Small,
                length: LineEndSize::Large,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let fmt = FmtScheme {
        ln_style_lst: vec![Border {
            width: 2.0,
            color: Color::none(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let scheme = ColorScheme::default();
    let clr_map = ClrMap::default();
    let resolved_border = inheritance::resolve_border_with_theme(
        &shape_with_style_line,
        None,
        None,
        Some(&fmt),
        Some(&scheme),
        Some(&clr_map),
    );
    assert_eq!(resolved_border.color.to_css().as_deref(), Some("#123456"));
    assert_eq!(resolved_border.width, 2.0);
    assert!(matches!(resolved_border.dash_style, DashStyle::DashDot));
    assert!(matches!(resolved_border.cap, LineCap::Round));
    assert!(matches!(resolved_border.compound, CompoundLine::Double));
    assert!(matches!(resolved_border.alignment, LineAlignment::Inset));
    assert!(matches!(resolved_border.join, LineJoin::Bevel));
    assert_eq!(resolved_border.miter_limit, Some(2.5));

    let layout_no_border = Shape {
        border: Border {
            no_fill: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let master_no_border = layout_no_border.clone();
    assert!(matches!(
        inheritance::resolve_border_with_theme(
            &Shape::default(),
            Some(&layout_no_border),
            None,
            None,
            None,
            None
        )
        .style,
        BorderStyle::None
    ));
    assert!(matches!(
        inheritance::resolve_border_with_theme(
            &Shape::default(),
            Some(&Shape::default()),
            Some(&master_no_border),
            None,
            None,
            None
        )
        .style,
        BorderStyle::None
    ));

    let mut master_map = ClrMap::default();
    master_map.set("tx1", "dk1");
    let layout = SlideLayout {
        clr_map_ovr: Some(ClrMapOverride::UseMaster),
        ..Default::default()
    };
    let master = SlideMaster {
        clr_map: master_map,
        ..Default::default()
    };
    assert_eq!(
        inheritance::resolve_clr_map(&Slide::default(), Some(&layout), &master).get("tx1"),
        Some(&"dk1".to_string())
    );
}

#[test]
fn renders_chart_zero_label_and_crop_fallback_matrix_through_public_renderer() {
    let labels = |position| ChartDataLabelSettings {
        show_value: true,
        position: Some(position),
        ..Default::default()
    };
    let empty_labels = ChartDataLabelSettings {
        show_value: false,
        show_category_name: false,
        show_series_name: false,
        show_percent: false,
        position: Some(ChartDataLabelPosition::Center),
    };
    let series = |name: &str, values: Vec<f64>| ChartSeries {
        name: Some(name.to_string()),
        categories: vec![
            "Zero".to_string(),
            "Positive".to_string(),
            "Tail".to_string(),
        ],
        values,
        x_values: vec![1.0, 2.0, 3.0],
        ..Default::default()
    };

    let html = render_model_shapes(vec![
        renderer_chart_shape(ChartSpec {
            chart_type: ChartType::Column,
            grouping: ChartGrouping::PercentStacked,
            data_labels: Some(labels(ChartDataLabelPosition::Center)),
            series: vec![series("ColumnStacked", vec![0.0, 5.0, 1.0])],
            ..Default::default()
        }),
        renderer_chart_shape(ChartSpec {
            chart_type: ChartType::Column,
            grouping: ChartGrouping::Clustered,
            data_labels: Some(labels(ChartDataLabelPosition::InEnd)),
            series: vec![series("ColumnClustered", vec![0.0, 4.0, 1.0])],
            ..Default::default()
        }),
        renderer_chart_shape(ChartSpec {
            chart_type: ChartType::Bar,
            grouping: ChartGrouping::PercentStacked,
            data_labels: Some(labels(ChartDataLabelPosition::Center)),
            series: vec![series("BarStacked", vec![0.0, 5.0, 1.0])],
            ..Default::default()
        }),
        renderer_chart_shape(ChartSpec {
            chart_type: ChartType::Bar,
            grouping: ChartGrouping::Clustered,
            data_labels: Some(labels(ChartDataLabelPosition::Center)),
            series: vec![series("BarClustered", vec![0.0, 4.0, 1.0])],
            ..Default::default()
        }),
        renderer_chart_shape(ChartSpec {
            chart_type: ChartType::Line,
            data_labels: Some(labels(ChartDataLabelPosition::InEnd)),
            series: vec![series("LineInEnd", vec![0.0, 3.0, 1.0])],
            ..Default::default()
        }),
        renderer_chart_shape(ChartSpec {
            chart_type: ChartType::Scatter,
            data_labels: Some(labels(ChartDataLabelPosition::OutEnd)),
            series: vec![series("ScatterOutEnd", vec![0.0, 2.0, 1.0])],
            ..Default::default()
        }),
        renderer_chart_shape(ChartSpec {
            chart_type: ChartType::OfPie,
            of_pie_type: Some(ChartOfPieType::Pie),
            split_type: Some(ChartSplitType::Pos),
            split_pos: Some(2.0),
            series: vec![series("OfPieSkip", vec![5.0, 0.0, 3.0])],
            ..Default::default()
        }),
        renderer_chart_shape(ChartSpec {
            chart_type: ChartType::Pie,
            data_labels: Some(empty_labels),
            series: vec![series("PieSkip", vec![5.0, 0.0, 3.0])],
            ..Default::default()
        }),
        Shape {
            shape_type: ShapeType::Picture(PictureData {
                rel_id: "rIdDegenerateCrop".to_string(),
                data: b"degenerate".to_vec(),
                crop: Some(CropRect {
                    left: 0.6,
                    top: 0.0,
                    right: 0.6,
                    bottom: 0.0,
                }),
                ..Default::default()
            }),
            size: Size {
                width: Emu(914_400),
                height: Emu(457_200),
            },
            ..Default::default()
        },
        Shape {
            shape_type: ShapeType::Picture(PictureData {
                rel_id: "rIdNoCrop".to_string(),
                content_type: "image/png".to_string(),
                data: b"plain".to_vec(),
                ..Default::default()
            }),
            size: Size {
                width: Emu(914_400),
                height: Emu(457_200),
            },
            ..Default::default()
        },
    ]);

    assert!(html.contains("data-label-position=\"ctr\""));
    assert!(html.contains("data-label-position=\"inEnd\""));
    assert!(html.contains("data-label-position=\"outEnd\""));
    assert!(html.contains("chart-of-pie-secondary"));
    assert!(html.contains("data:image/png;base64"));
    assert!(html.contains("shape-image"));
}

#[test]
fn renders_swapped_connector_path_variants_through_public_renderer() {
    let connector = |name: &str, rotation: f64, flip_h: bool, flip_v: bool| Shape {
        shape_type: ShapeType::Custom(name.to_string()),
        rotation,
        flip_h,
        flip_v,
        size: Size {
            width: Emu(914_400),
            height: Emu(457_200),
        },
        ..Default::default()
    };

    let html = render_model_shapes(vec![
        connector("straightConnector1", 90.0, true, false),
        connector("bentConnector2", 270.0, true, false),
        connector("bentConnector2", 270.0, false, true),
        connector("bentConnector2", 270.0, false, false),
        connector("bentConnector3", 270.0, true, false),
        connector("bentConnector3", 270.0, false, false),
        connector("curvedConnector2", 270.0, true, false),
    ]);

    assert!(html.contains("shape-svg"));
    assert!(html.contains("transform=\"translate"));
    assert!(html.contains("M0,0 L0,"));
}

#[test]
fn renders_table_paragraph_and_external_picture_fallbacks_through_public_renderer() {
    fn text_run(text: &str) -> TextRun {
        TextRun {
            text: text.to_string(),
            ..Default::default()
        }
    }
    fn paragraph(text: &str, level: u32, bullet: Option<Bullet>) -> TextParagraph {
        TextParagraph {
            runs: vec![text_run(text)],
            level,
            bullet,
            space_after: Some(SpacingValue::Percent(1.2)),
            ..Default::default()
        }
    }
    fn cell(vertical_align: VerticalAlign, text_body: TextBody) -> TableCell {
        TableCell {
            text_body: Some(text_body),
            vertical_align,
            margin_left: 1.0,
            margin_right: 2.0,
            margin_top: 3.0,
            margin_bottom: 4.0,
            ..Default::default()
        }
    }

    let table_shape = Shape {
        shape_type: ShapeType::Table(TableData {
            col_widths: vec![0.0, 0.0],
            band_row: true,
            band_col: true,
            first_col: true,
            last_col: true,
            rows: vec![
                TableRow {
                    height: 20.0,
                    cells: vec![
                        cell(
                            VerticalAlign::Top,
                            TextBody {
                                paragraphs: vec![
                                    paragraph(
                                        "Char bullet",
                                        2,
                                        Some(Bullet::Char(pptx2html_core::model::BulletChar {
                                            char: "•".to_string(),
                                            font: Some("Symbol".to_string()),
                                            size_pct: Some(-14.0),
                                            color: Some(Color::rgb("FF0000")),
                                        })),
                                    ),
                                    paragraph(
                                        "Auto bullet",
                                        1,
                                        Some(Bullet::AutoNum(
                                            pptx2html_core::model::BulletAutoNum {
                                                num_type: "arabicPeriod".to_string(),
                                                start_at: Some(3),
                                                font: Some("Arial".to_string()),
                                                size_pct: Some(-10.0),
                                                color: Some(Color::rgb("00AA00")),
                                            },
                                        )),
                                    ),
                                    paragraph("No bullet", 1, Some(Bullet::None)),
                                    TextParagraph::default(),
                                ],
                                ..Default::default()
                            },
                        ),
                        cell(
                            VerticalAlign::Bottom,
                            TextBody {
                                paragraphs: vec![paragraph("Bottom cell", 0, None)],
                                ..Default::default()
                            },
                        ),
                    ],
                },
                TableRow {
                    height: 24.0,
                    cells: vec![
                        cell(
                            VerticalAlign::Middle,
                            TextBody {
                                paragraphs: vec![paragraph("Band row", 0, None)],
                                ..Default::default()
                            },
                        ),
                        TableCell {
                            v_merge: true,
                            ..Default::default()
                        },
                    ],
                },
            ],
            ..Default::default()
        }),
        size: Size {
            width: Emu(1_828_800),
            height: Emu(914_400),
        },
        ..Default::default()
    };

    let picture_shape = Shape {
        shape_type: ShapeType::Picture(PictureData {
            rel_id: "rIdExternalPicture".to_string(),
            content_type: "image/png".to_string(),
            data: b"external-picture".to_vec(),
            ..Default::default()
        }),
        size: Size {
            width: Emu(914_400),
            height: Emu(457_200),
        },
        ..Default::default()
    };

    let mut presentation = Presentation {
        slide_size: Size {
            width: Emu(9_144_000),
            height: Emu(6_858_000),
        },
        ..Default::default()
    };
    presentation.slides.push(Slide {
        shapes: vec![table_shape, picture_shape],
        ..Default::default()
    });
    let html = HtmlRenderer::render_with_options(
        &presentation,
        &ConversionOptions {
            embed_images: false,
            ..Default::default()
        },
    )
    .expect("renderer table matrix should render");

    assert!(html.contains("font-size: 14.0pt"));
    assert!(html.contains("font-size: 10.0pt"));
    assert!(html.contains("padding-left: 72.0pt"));
    assert!(html.contains("margin-bottom: 1.2em"));
    assert!(html.contains("vertical-align: top"));
    assert!(html.contains("vertical-align: bottom"));
    assert!(html.contains("&nbsp;"));
    assert!(html.contains("images/slide-1/image-0.png"));
}

#[test]
fn renders_remaining_chart_label_and_gradient_branches_through_public_renderer() {
    let labels = |position| ChartDataLabelSettings {
        show_value: true,
        position: Some(position),
        ..Default::default()
    };
    let series = |values: Vec<f64>| ChartSeries {
        name: Some("Series".to_string()),
        categories: vec!["A".to_string(), "B".to_string()],
        x_values: vec![1.0, 2.0],
        values,
        ..Default::default()
    };
    let marker_none_series = |values: Vec<f64>| ChartSeries {
        marker: Some(pptx2html_core::model::ChartMarkerSpec {
            symbol: Some("none".to_string()),
            size: Some(6),
        }),
        ..series(values)
    };
    let gradient_fill = |gradient_type| {
        Fill::Gradient(GradientFill {
            gradient_type,
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: Color::rgb("112233"),
                },
                GradientStop {
                    position: 1.0,
                    color: Color::rgb("445566"),
                },
            ],
            angle: 45.0,
        })
    };
    let gradient_shape = |gradient_type| Shape {
        shape_type: ShapeType::TextBox,
        fill: gradient_fill(gradient_type),
        size: Size {
            width: Emu(914_400),
            height: Emu(457_200),
        },
        ..Default::default()
    };

    let mut presentation = Presentation {
        slide_size: Size {
            width: Emu(9_144_000),
            height: Emu(6_858_000),
        },
        ..Default::default()
    };
    presentation.slides.push(Slide {
        background: Some(gradient_fill(GradientType::Radial)),
        shapes: vec![
            renderer_chart_shape(ChartSpec {
                chart_type: ChartType::Column,
                grouping: ChartGrouping::PercentStacked,
                data_labels: Some(labels(ChartDataLabelPosition::InEnd)),
                series: vec![series(vec![2.0, 1.0])],
                ..Default::default()
            }),
            renderer_chart_shape(ChartSpec {
                chart_type: ChartType::Bar,
                grouping: ChartGrouping::PercentStacked,
                data_labels: Some(labels(ChartDataLabelPosition::InEnd)),
                series: vec![series(vec![2.0, 1.0])],
                ..Default::default()
            }),
            renderer_chart_shape(ChartSpec {
                chart_type: ChartType::Radar,
                radar_style: Some(pptx2html_core::model::ChartRadarStyle::Marker),
                series: vec![series(vec![0.0, 3.0])],
                ..Default::default()
            }),
            renderer_chart_shape(ChartSpec {
                chart_type: ChartType::Line,
                data_labels: Some(labels(ChartDataLabelPosition::InEnd)),
                series: vec![marker_none_series(vec![2.0, 1.0])],
                ..Default::default()
            }),
            renderer_chart_shape(ChartSpec {
                chart_type: ChartType::Line,
                data_labels: Some(labels(ChartDataLabelPosition::OutEnd)),
                series: vec![marker_none_series(vec![2.0, 1.0])],
                ..Default::default()
            }),
            renderer_chart_shape(ChartSpec {
                chart_type: ChartType::Scatter,
                scatter_style: Some(pptx2html_core::model::ChartScatterStyle::Marker),
                data_labels: Some(labels(ChartDataLabelPosition::InEnd)),
                series: vec![marker_none_series(vec![2.0, 1.0])],
                ..Default::default()
            }),
            renderer_chart_shape(ChartSpec {
                chart_type: ChartType::Scatter,
                scatter_style: Some(pptx2html_core::model::ChartScatterStyle::Marker),
                data_labels: Some(labels(ChartDataLabelPosition::OutEnd)),
                series: vec![marker_none_series(vec![2.0, 1.0])],
                ..Default::default()
            }),
            gradient_shape(GradientType::Radial),
            gradient_shape(GradientType::Rectangular),
            gradient_shape(GradientType::Shape),
            Shape {
                shape_type: ShapeType::TextBox,
                fill: Fill::Image(ImageFill {
                    rel_id: "rIdDefaultMime".to_string(),
                    data: b"image-data".to_vec(),
                    ..Default::default()
                }),
                size: Size {
                    width: Emu(914_400),
                    height: Emu(457_200),
                },
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    presentation.slides.push(Slide {
        background: Some(gradient_fill(GradientType::Rectangular)),
        ..Default::default()
    });
    presentation.slides.push(Slide {
        background: Some(gradient_fill(GradientType::Shape)),
        ..Default::default()
    });

    let html = HtmlRenderer::render(&presentation).expect("chart/gradient matrix should render");
    assert!(html.contains("data-label-position=\"inEnd\""));
    assert!(html.contains("data-label-position=\"outEnd\""));
    assert!(html.contains("radial-gradient(circle"));
    assert!(html.contains("radial-gradient(ellipse"));
    assert!(html.contains("radial-gradient(closest-side"));
    assert!(html.contains("data:image/png;base64"));
}

#[test]
fn renders_master_text_body_inheritance_paths_through_public_renderer() {
    let placeholder = PlaceholderInfo {
        ph_type: Some(PlaceholderType::Body),
        idx: Some(7),
    };
    let mut inherited_levels = ListStyle::default();
    inherited_levels.levels[0] = Some(ParagraphDefaults {
        def_run_props: Some(RunDefaults {
            font_size: Some(22.0),
            bold: Some(true),
            font_latin: Some("".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    });
    let master_text = TextBody {
        vertical_align: VerticalAlign::Bottom,
        vertical_align_explicit: true,
        word_wrap: false,
        word_wrap_explicit: true,
        anchor_center: true,
        text_rotation_deg: 22.0,
        margin_top_explicit: true,
        margins: TextMargins {
            top: 12.0,
            ..Default::default()
        },
        list_style: Some(inherited_levels),
        ..Default::default()
    };
    let master_shape = Shape {
        placeholder: Some(placeholder.clone()),
        text_body: Some(master_text),
        vertical_text: Some("mongolianVert".to_string()),
        vertical_text_explicit: true,
        ..Default::default()
    };
    let slide_shape = Shape {
        placeholder: Some(placeholder),
        text_body: Some(TextBody {
            paragraphs: vec![TextParagraph {
                runs: vec![TextRun {
                    text: "Inherited text".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }),
        size: Size {
            width: Emu(1_828_800),
            height: Emu(914_400),
        },
        ..Default::default()
    };
    let unknown_vertical = Shape {
        text_body: Some(TextBody {
            paragraphs: vec![TextParagraph {
                runs: vec![TextRun {
                    text: "Unknown vertical".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }),
        vertical_text: Some("mysteryVerticalMode".to_string()),
        vertical_text_explicit: true,
        size: Size {
            width: Emu(914_400),
            height: Emu(457_200),
        },
        ..Default::default()
    };

    let mut presentation = Presentation {
        slide_size: Size {
            width: Emu(9_144_000),
            height: Emu(6_858_000),
        },
        ..Default::default()
    };
    presentation.masters.push(SlideMaster {
        shapes: vec![master_shape],
        ..Default::default()
    });
    presentation.layouts.push(SlideLayout {
        master_idx: 0,
        ..Default::default()
    });
    presentation.slides.push(Slide {
        layout_idx: Some(0),
        shapes: vec![slide_shape, unknown_vertical],
        ..Default::default()
    });

    let html = HtmlRenderer::render(&presentation).expect("master text inheritance should render");
    assert!(html.contains("writing-mode: vertical-lr"));
    assert!(html.contains("transform: rotate(22.0deg)"));
    assert!(html.contains("font-size: 22.0pt"));
    assert!(html.contains("font-weight: bold"));
    assert!(html.contains("h-center"));
}

#[test]
fn renders_anchor_geometry_and_fill_fallbacks_through_public_renderer() {
    let connection_shape = |id, x, y, flip_h, flip_v, rotation| Shape {
        id,
        shape_type: ShapeType::CustomGeom(CustomGeometry {
            paths: vec![GeometryPath {
                width: 100_000.0,
                height: 100_000.0,
                commands: Vec::new(),
                fill: PathFill::Norm,
            }],
            text_rect: Some(GeomRect {
                left: 10_000.0,
                top: 10_000.0,
                right: 90_000.0,
                bottom: 90_000.0,
            }),
            adjust_handles: Vec::new(),
            connection_sites: vec![ConnectionSite {
                x: 25_000.0,
                y: 75_000.0,
                angle: 0.0,
            }],
        }),
        position: pptx2html_core::model::Position {
            x: Emu(x),
            y: Emu(y),
        },
        size: Size {
            width: Emu(914_400),
            height: Emu(457_200),
        },
        flip_h,
        flip_v,
        rotation,
        ..Default::default()
    };

    let anchored_connector = Shape {
        shape_type: ShapeType::Custom("bentConnector3".to_string()),
        start_connection: Some(ConnectionRef {
            shape_id: 301,
            site_idx: 0,
        }),
        end_connection: Some(ConnectionRef {
            shape_id: 302,
            site_idx: 0,
        }),
        size: Size {
            width: Emu(914_400),
            height: Emu(457_200),
        },
        ..Default::default()
    };

    let mut presentation = Presentation {
        slide_size: Size {
            width: Emu(9_144_000),
            height: Emu(6_858_000),
        },
        ..Default::default()
    };
    presentation.slides.push(Slide {
        background: Some(Fill::Solid(SolidFill {
            color: Color::none(),
        })),
        shapes: vec![
            connection_shape(301, 0, 0, true, false, 90.0),
            connection_shape(302, 914_400, 914_400, false, true, 180.0),
            anchored_connector,
            Shape {
                shape_type: ShapeType::TextBox,
                fill: Fill::None,
                size: Size {
                    width: Emu(457_200),
                    height: Emu(228_600),
                },
                ..Default::default()
            },
            Shape {
                shape_type: ShapeType::TextBox,
                fill: Fill::Gradient(GradientFill {
                    gradient_type: GradientType::Linear,
                    stops: Vec::new(),
                    angle: 0.0,
                }),
                size: Size {
                    width: Emu(457_200),
                    height: Emu(228_600),
                },
                ..Default::default()
            },
            Shape {
                shape_type: ShapeType::TextBox,
                fill: Fill::Image(ImageFill {
                    rel_id: "rIdEmptyImage".to_string(),
                    data: Vec::new(),
                    ..Default::default()
                }),
                size: Size {
                    width: Emu(457_200),
                    height: Emu(228_600),
                },
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    presentation.slides.push(Slide {
        background: Some(Fill::Gradient(GradientFill {
            gradient_type: GradientType::Linear,
            stops: Vec::new(),
            angle: 0.0,
        })),
        ..Default::default()
    });
    presentation.slides.push(Slide {
        background: Some(Fill::Image(ImageFill {
            rel_id: "rIdEmptyBackground".to_string(),
            data: Vec::new(),
            ..Default::default()
        })),
        ..Default::default()
    });

    let html = HtmlRenderer::render(&presentation).expect("anchor geometry should render");
    assert!(html.contains("shape-svg"));
    assert!(html.contains("M0,0 L0,"));
}

#[test]
fn renders_renderer_none_label_marker_and_group_skip_paths_through_public_renderer() {
    let empty_labels = ChartDataLabelSettings {
        show_value: false,
        show_category_name: false,
        show_series_name: false,
        show_percent: false,
        position: Some(ChartDataLabelPosition::Center),
    };
    let series = ChartSeries {
        name: Some("No Labels".to_string()),
        categories: vec!["A".to_string()],
        values: vec![4.0],
        ..Default::default()
    };
    let no_marker_line = Shape {
        shape_type: ShapeType::Custom("line".to_string()),
        border: Border {
            width: 1.0,
            head_end: Some(LineEnd {
                end_type: LineEndType::None,
                width: LineEndSize::Medium,
                length: LineEndSize::Medium,
            }),
            ..Default::default()
        },
        size: Size {
            width: Emu(914_400),
            height: Emu(0),
        },
        ..Default::default()
    };
    let no_rect_custom_geom = Shape {
        shape_type: ShapeType::CustomGeom(CustomGeometry {
            paths: Vec::new(),
            text_rect: Some(GeomRect {
                left: 0.0,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
            }),
            adjust_handles: Vec::new(),
            connection_sites: Vec::new(),
        }),
        text_body: Some(TextBody {
            paragraphs: vec![TextParagraph {
                runs: vec![TextRun {
                    text: "custom".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }),
        size: Size {
            width: Emu(914_400),
            height: Emu(457_200),
        },
        ..Default::default()
    };
    let hidden_group = Shape {
        shape_type: ShapeType::Group(
            vec![
                Shape {
                    hidden: true,
                    shape_type: ShapeType::TextBox,
                    ..Default::default()
                },
                Shape {
                    shape_type: ShapeType::TextBox,
                    size: Size {
                        width: Emu(457_200),
                        height: Emu(228_600),
                    },
                    ..Default::default()
                },
            ],
            GroupData {
                child_offset: pptx2html_core::model::Position {
                    x: Emu(0),
                    y: Emu(0),
                },
                child_extent: Size {
                    width: Emu(0),
                    height: Emu(0),
                },
            },
        ),
        size: Size {
            width: Emu(914_400),
            height: Emu(457_200),
        },
        ..Default::default()
    };
    let inline_style_text = Shape {
        shape_type: ShapeType::TextBox,
        text_body: Some(TextBody {
            paragraphs: vec![TextParagraph {
                runs: vec![TextRun {
                    text: "inline styled".to_string(),
                    style: TextStyle {
                        italic: true,
                        underline: UnderlineType::Single,
                        strikethrough: StrikethroughType::Single,
                        ..Default::default()
                    },
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }),
        size: Size {
            width: Emu(914_400),
            height: Emu(457_200),
        },
        ..Default::default()
    };

    let html = render_model_shapes(vec![
        renderer_chart_shape(ChartSpec {
            chart_type: ChartType::Column,
            data_labels: Some(empty_labels.clone()),
            series: vec![series.clone()],
            ..Default::default()
        }),
        renderer_chart_shape(ChartSpec {
            chart_type: ChartType::Line,
            data_labels: Some(empty_labels),
            series: vec![series],
            ..Default::default()
        }),
        renderer_chart_shape(ChartSpec {
            chart_type: ChartType::Scatter,
            scatter_style: Some(pptx2html_core::model::ChartScatterStyle::Marker),
            series: vec![ChartSeries {
                name: Some("No finite X".to_string()),
                categories: Vec::new(),
                x_values: Vec::new(),
                values: vec![4.0],
                ..Default::default()
            }],
            ..Default::default()
        }),
        renderer_chart_shape(ChartSpec {
            chart_type: ChartType::Scatter,
            scatter_style: Some(pptx2html_core::model::ChartScatterStyle::Marker),
            series: vec![ChartSeries {
                name: Some("NaN scatter".to_string()),
                x_values: vec![f64::NAN],
                values: vec![f64::NAN],
                ..Default::default()
            }],
            ..Default::default()
        }),
        Shape {
            shape_type: ShapeType::Table(TableData {
                band_col: true,
                last_col: true,
                col_widths: vec![1.0, 1.0],
                rows: vec![TableRow {
                    height: 20.0,
                    cells: vec![TableCell::default(), TableCell::default()],
                }],
                ..Default::default()
            }),
            size: Size {
                width: Emu(914_400),
                height: Emu(457_200),
            },
            ..Default::default()
        },
        no_marker_line,
        no_rect_custom_geom,
        hidden_group,
        inline_style_text,
    ]);

    assert!(html.contains("shape-svg"));
    assert!(html.contains("custom"));
    assert!(html.contains("font-style: italic"));
    assert!(!html.contains("chart-data-label\">4</text>"));
}

#[test]
fn renders_last_renderer_fallback_edges_through_public_renderer() {
    let band_col_table = Shape {
        shape_type: ShapeType::Table(TableData {
            band_col: true,
            last_col: true,
            col_widths: vec![1.0, 1.0],
            rows: vec![TableRow {
                height: 20.0,
                cells: vec![
                    TableCell {
                        text_body: Some(TextBody {
                            paragraphs: vec![TextParagraph {
                                runs: vec![TextRun {
                                    text: "A".to_string(),
                                    ..Default::default()
                                }],
                                ..Default::default()
                            }],
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    TableCell {
                        text_body: Some(TextBody {
                            paragraphs: vec![TextParagraph {
                                runs: vec![TextRun {
                                    text: "B".to_string(),
                                    ..Default::default()
                                }],
                                ..Default::default()
                            }],
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                ],
            }],
            ..Default::default()
        }),
        size: Size {
            width: Emu(914_400),
            height: Emu(457_200),
        },
        ..Default::default()
    };
    let ellipse_empty_gradient = Shape {
        shape_type: ShapeType::Ellipse,
        fill: Fill::Gradient(GradientFill {
            gradient_type: GradientType::Linear,
            stops: Vec::new(),
            angle: 0.0,
        }),
        size: Size {
            width: Emu(457_200),
            height: Emu(457_200),
        },
        ..Default::default()
    };
    let non_custom_anchor = Shape {
        id: 401,
        shape_type: ShapeType::TextBox,
        size: Size {
            width: Emu(457_200),
            height: Emu(457_200),
        },
        ..Default::default()
    };
    let connector_to_non_custom = Shape {
        shape_type: ShapeType::Custom("line".to_string()),
        start_connection: Some(ConnectionRef {
            shape_id: 401,
            site_idx: 0,
        }),
        end_connection: Some(ConnectionRef {
            shape_id: 401,
            site_idx: 0,
        }),
        size: Size {
            width: Emu(914_400),
            height: Emu(0),
        },
        ..Default::default()
    };

    let mut presentation = Presentation {
        slide_size: Size {
            width: Emu(9_144_000),
            height: Emu(6_858_000),
        },
        ..Default::default()
    };
    presentation.slides.push(Slide {
        background: Some(Fill::None),
        shapes: vec![
            band_col_table,
            ellipse_empty_gradient,
            non_custom_anchor,
            connector_to_non_custom,
        ],
        ..Default::default()
    });
    presentation.slides.push(Slide {
        background: Some(Fill::Image(ImageFill {
            rel_id: "rIdBgDefaultMime".to_string(),
            data: b"bg".to_vec(),
            ..Default::default()
        })),
        ..Default::default()
    });

    let html = HtmlRenderer::render(&presentation).expect("last renderer edges should render");
    assert!(html.contains("background-color: rgba(0,0,0,0.04)"));
    assert!(html.contains("data:image/png;base64"));
    assert!(html.contains("shape-svg"));
}

#[test]
fn parses_theme_master_and_layout_branches_through_public_parser() {
    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="CoverageTheme">
  <a:themeElements>
    <a:clrScheme name="CoverageColors">
      <a:dk1><a:sysClr val="windowText" lastClr="111111"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="222222"/></a:dk2>
      <a:lt2><a:srgbClr val="F7F7F7"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="CoverageFonts">
      <a:majorFont>
        <a:latin typeface="Aptos"/>
        <a:ea typeface="Yu Gothic"/>
        <a:cs typeface="Noto Sans Devanagari"/>
      </a:majorFont>
      <a:minorFont>
        <a:latin typeface="Aptos Narrow"/>
        <a:ea typeface="Meiryo"/>
        <a:cs typeface="Noto Sans Arabic"/>
      </a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="CoverageFmt">
      <a:fillStyleLst>
        <a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
        <a:solidFill><a:schemeClr val="accent2"/></a:solidFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="12700" cap="rnd" cmpd="dbl" algn="in">
          <a:solidFill><a:schemeClr val="accent3"/></a:solidFill>
          <a:prstDash val="lgDashDot"/>
          <a:miter lim="200000"/>
        </a:ln>
      </a:lnStyleLst>
      <a:effectStyleLst>
        <a:effectStyle>
          <a:effectLst>
            <a:outerShdw blurRad="12700" dist="25400" dir="5400000">
              <a:schemeClr val="accent4"/>
              <a:alpha val="50000"/>
            </a:outerShdw>
            <a:glow rad="6350">
              <a:srgbClr val="ABCDEF"/>
              <a:alpha val="60000"/>
            </a:glow>
          </a:effectLst>
        </a:effectStyle>
      </a:effectStyleLst>
      <a:bgFillStyleLst>
        <a:noFill/>
        <a:solidFill><a:schemeClr val="accent5"/></a:solidFill>
      </a:bgFillStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#;

    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs>
            <a:gs pos="100000"><a:schemeClr val="accent1"/></a:gs>
          </a:gsLst>
          <a:path path="rect"/>
          <a:lin ang="5400000"/>
        </a:gradFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Master Placeholder"/>
          <p:cNvSpPr/>
          <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="12700" y="25400"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700" cap="rnd" cmpd="dbl" algn="in">
            <a:prstDash val="lgDashDot"/>
            <a:miter lim="200000"/>
            <a:headEnd type="triangle" w="lg" len="sm"/>
            <a:tailEnd type="stealth" w="sm" len="lg"/>
            <a:schemeClr val="accent3"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr" anchorCtr="1" rot="5400000" vert="vert270"
                    lIns="91440" tIns="45720" rIns="182880" bIns="22860" wrap="none"/>
          <a:lstStyle>
            <a:lvl1pPr algn="r">
              <a:spcBef><a:spcPts val="1200"/></a:spcBef>
              <a:defRPr sz="1800"><a:latin typeface="Calibri"/><a:srgbClr val="336699"/></a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p/>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr algn="ctr" marL="457200" indent="-228600">
        <a:lnSpc><a:spcPct val="90000"/></a:lnSpc>
        <a:defRPr sz="2400" spc="200" baseline="30000" cap="all" u="dbl"
                  strike="sngStrike" b="1" i="1">
          <a:latin typeface="Aptos"/>
          <a:ea typeface="Yu Gothic"/>
          <a:cs typeface="Noto Sans Devanagari"/>
          <a:schemeClr val="accent2"/>
        </a:defRPr>
      </a:lvl1pPr>
    </p:titleStyle>
    <p:bodyStyle><a:lvl1pPr algn="just"/></p:bodyStyle>
    <p:otherStyle><a:lvl1pPr algn="r"/></p:otherStyle>
  </p:txStyles>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2"
            accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6"
            hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             type="title" showMasterSp="0">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="112233"/></a:gs>
            <a:gs pos="100000"><a:schemeClr val="accent2"/></a:gs>
          </a:gsLst>
          <a:path path="shape"/>
          <a:lin ang="2700000"/>
        </a:gradFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Layout Placeholder"/>
          <p:cNvSpPr/>
          <p:nvPr><p:ph type="title" idx="3"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="12700" y="38100"/><a:ext cx="254000" cy="381000"/></a:xfrm>
          <a:ln w="12700" cap="flat" cmpd="tri" algn="in">
            <a:prstDash val="sysDot"/>
            <a:bevel/>
            <a:headEnd type="arrow" w="sm" len="sm"/>
            <a:tailEnd type="diamond" w="lg" len="lg"/>
            <a:srgbClr val="445566"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="b" anchorCtr="1" rot="1800000" vert="vert"
                    lIns="45720" tIns="91440" rIns="137160" bIns="182880" wrap="none"/>
          <a:normAutofit fontScale="70000" lnSpcReduction="15000"/>
          <a:lstStyle>
            <a:lvl1pPr algn="just">
              <a:spcAft><a:spcPct val="150000"/></a:spcAft>
              <a:defRPr sz="2000"><a:latin typeface="Calibri"/><a:schemeClr val="accent4"/></a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p/>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:overrideClrMapping bg1="lt2" tx1="dk2" accent1="accent6"/></p:clrMapOvr>
</p:sldLayout>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_full_theme(theme_xml)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .with_slide_layout_rel()
        .build();

    let presentation = parse_pptx(&pptx);

    let theme = &presentation.themes[0];
    assert_eq!(theme.font_scheme.major_latin, "Aptos");
    assert_eq!(theme.font_scheme.minor_latin, "Aptos Narrow");
    assert_eq!(
        theme.font_scheme.major_east_asian.as_deref(),
        Some("Yu Gothic")
    );
    assert_eq!(
        theme.font_scheme.minor_complex_script.as_deref(),
        Some("Noto Sans Arabic")
    );
    assert_eq!(theme.fmt_scheme.fill_style_lst.len(), 2);
    assert_eq!(theme.fmt_scheme.ln_style_lst.len(), 1);
    assert_eq!(theme.fmt_scheme.effect_style_lst.len(), 1);

    let master = &presentation.masters[0];
    assert!(matches!(
        &master.background,
        Some(Fill::Gradient(fill)) if fill.stops.len() == 2
    ));
    assert!(master.tx_styles.title_style.is_some());
    let master_shape = &master.shapes[0];
    assert!(matches!(
        master_shape
            .placeholder
            .as_ref()
            .and_then(|ph| ph.ph_type.as_ref()),
        Some(PlaceholderType::Body)
    ));
    assert!(matches!(master_shape.border.cap, LineCap::Round));
    assert!(matches!(master_shape.border.compound, CompoundLine::Double));
    assert!(matches!(
        master_shape.border.alignment,
        LineAlignment::Inset
    ));
    assert!(matches!(master_shape.border.join, LineJoin::Miter));
    assert!(matches!(
        master_shape.border.dash_style,
        DashStyle::LongDashDot
    ));
    assert_eq!(master_shape.vertical_text.as_deref(), Some("vert270"));
    let master_text = master_shape.text_body.as_ref().expect("master text body");
    assert!(matches!(master_text.vertical_align, VerticalAlign::Middle));
    assert!(master_text.anchor_center);

    let layout = &presentation.layouts[0];
    assert_eq!(layout.layout_type.as_deref(), Some("title"));
    assert!(!layout.show_master_sp);
    assert!(matches!(
        &layout.background,
        Some(Fill::Gradient(fill)) if fill.stops.len() == 2
    ));
    assert!(matches!(
        &layout.clr_map_ovr,
        Some(ClrMapOverride::Override(map))
            if map.get("bg1").map(String::as_str) == Some("lt2")
                && map.get("tx1").map(String::as_str) == Some("dk2")
                && map.get("accent1").map(String::as_str) == Some("accent6")
    ));
    let layout_shape = &layout.shapes[0];
    assert!(matches!(
        layout_shape
            .placeholder
            .as_ref()
            .and_then(|ph| ph.ph_type.as_ref()),
        Some(PlaceholderType::Title)
    ));
    let layout_text = layout_shape.text_body.as_ref().expect("layout text body");
    assert!(matches!(layout_text.vertical_align, VerticalAlign::Bottom));
    assert!(layout_text.anchor_center);
    assert!(matches!(
        layout_text.auto_fit,
        AutoFit::Normal {
            font_scale: Some(v),
            line_spacing_reduction: Some(lsr)
        } if (v - 0.7).abs() < 1e-6 && (lsr - 0.15).abs() < 1e-6
    ));
}

#[test]
fn parses_theme_and_master_start_tag_variants_through_public_parser() {
    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="StartTagTheme">
  <a:themeElements>
    <a:clrScheme name="StartTagColors">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="StartTagFonts">
      <a:majorFont><a:latin typeface="Aptos"/></a:majorFont>
      <a:minorFont><a:latin typeface="Aptos Narrow"/></a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="StartTagFmt">
      <a:fillStyleLst>
        <a:solidFill><a:srgbClr val="AA0000"></a:srgbClr></a:solidFill>
        <a:solidFill><a:schemeClr val="accent2"></a:schemeClr></a:solidFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="12700" cap="flat" cmpd="tri" algn="in">
          <a:solidFill><a:schemeClr val="accent3"></a:schemeClr></a:solidFill>
          <a:prstDash val="sysDashDotDot"/>
          <a:bevel/>
        </a:ln>
        <a:ln w="6350" cmpd="thickThin">
          <a:solidFill><a:srgbClr val="00FF00"></a:srgbClr></a:solidFill>
          <a:prstDash val="lgDashDotDot"/>
          <a:round/>
        </a:ln>
        <a:ln w="6350" cmpd="thinThick">
          <a:solidFill><a:srgbClr val="0000FF"></a:srgbClr></a:solidFill>
          <a:prstDash val="sysDash"/>
        </a:ln>
      </a:lnStyleLst>
      <a:effectStyleLst>
        <a:effectStyle>
          <a:effectLst>
            <a:outerShdw blurRad="12700" dist="25400" dir="5400000">
              <a:srgbClr val="112233"></a:srgbClr>
              <a:alpha val="40000"/>
            </a:outerShdw>
            <a:glow rad="6350">
              <a:schemeClr val="accent6"></a:schemeClr>
              <a:alpha val="50000"/>
            </a:glow>
          </a:effectLst>
        </a:effectStyle>
      </a:effectStyleLst>
      <a:bgFillStyleLst>
        <a:solidFill><a:schemeClr val="accent5"></a:schemeClr></a:solidFill>
      </a:bgFillStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#;

    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="334455"></a:srgbClr></a:gs>
            <a:gs pos="100000"><a:schemeClr val="accent1"></a:schemeClr></a:gs>
          </a:gsLst>
          <a:path path="rect"></a:path>
          <a:lin ang="1800000"/>
        </a:gradFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="Master Normal"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="12700" y="25400"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700" cap="flat" cmpd="thinThick" algn="in">
            <a:prstDash val="sysDashDotDot"/>
            <a:bevel/>
            <a:schemeClr val="accent3"></a:schemeClr>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr" anchorCtr="true" rot="60000" vert="horz" wrap="square"></a:bodyPr>
          <a:normAutofit fontScale="65000" lnSpcReduction="12000"></a:normAutofit>
          <a:lstStyle>
            <a:lvl1pPr algn="ctr">
              <a:spcBef><a:spcPct val="25000"/></a:spcBef>
              <a:lnSpc><a:spcPts val="900"/></a:lnSpc>
              <a:defRPr sz="1800"><a:srgbClr val="112233"></a:srgbClr></a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="Master NoAuto"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="25400" y="38100"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700" cmpd="thickThin"><a:prstDash val="dash"/></a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr></a:bodyPr>
          <a:noAutofit></a:noAutofit>
          <a:lstStyle>
            <a:lvl1pPr>
              <a:defRPr sz="1600"><a:schemeClr val="accent2"></a:schemeClr></a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="4" name="Master Shrink"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="38100" y="50800"/><a:ext cx="381000" cy="254000"/></a:xfrm></p:spPr>
        <p:txBody>
          <a:bodyPr></a:bodyPr>
          <a:spAutoFit></a:spAutoFit>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="5" name="Implicit Solid"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="50800" y="63500"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700"></a:ln>
        </p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
    <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr algn="ctr">
        <a:spcBef><a:spcPct val="25000"/></a:spcBef>
        <a:defRPr sz="2000"><a:schemeClr val="accent2"></a:schemeClr></a:defRPr>
      </a:lvl1pPr>
    </p:titleStyle>
    <p:bodyStyle>
      <a:lvl1pPr>
        <a:lnSpc><a:spcPts val="1200"/></a:lnSpc>
      </a:lvl1pPr>
    </p:bodyStyle>
    <p:otherStyle>
      <a:lvl1pPr>
        <a:spcAft><a:spcPct val="50000"/></a:spcAft>
      </a:lvl1pPr>
    </p:otherStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_full_theme(theme_xml)
        .with_full_master(master_xml)
        .build();

    let presentation = parse_pptx(&pptx);

    let theme = &presentation.themes[0];
    assert_eq!(theme.fmt_scheme.ln_style_lst.len(), 3);
    assert!(matches!(
        theme.fmt_scheme.ln_style_lst[0].compound,
        CompoundLine::Triple
    ));
    assert!(matches!(
        theme.fmt_scheme.ln_style_lst[0].dash_style,
        DashStyle::SystemDashDotDot
    ));
    assert!(matches!(
        theme.fmt_scheme.ln_style_lst[1].compound,
        CompoundLine::ThickThin
    ));
    assert!(matches!(
        theme.fmt_scheme.ln_style_lst[1].dash_style,
        DashStyle::LongDashDotDot
    ));
    assert!(matches!(
        theme.fmt_scheme.ln_style_lst[2].compound,
        CompoundLine::ThinThick
    ));
    assert!(matches!(
        theme.fmt_scheme.ln_style_lst[2].dash_style,
        DashStyle::SystemDash
    ));
    let effect = &theme.fmt_scheme.effect_style_lst[0];
    assert!(matches!(
        effect
            .outer_shadow
            .as_ref()
            .map(|shadow| shadow.color.kind.clone()),
        Some(ColorKind::Rgb(rgb)) if rgb == "112233"
    ));
    assert!(matches!(
        effect.glow.as_ref().map(|glow| glow.color.kind.clone()),
        Some(ColorKind::Theme(name)) if name == "accent6"
    ));

    let master = &presentation.masters[0];
    assert!(matches!(
        &master.background,
        Some(Fill::Gradient(fill)) if fill.stops.len() == 2
    ));
    let title_style = master.tx_styles.title_style.as_ref().expect("title style");
    assert!(matches!(
        title_style.levels[0]
            .as_ref()
            .and_then(|lvl| lvl.space_before.clone()),
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 0.25).abs() < 1e-6
    ));
    let body_style = master.tx_styles.body_style.as_ref().expect("body style");
    assert!(matches!(
        body_style.levels[0]
            .as_ref()
            .and_then(|lvl| lvl.line_spacing.clone()),
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 12.0).abs() < 1e-6
    ));
    let other_style = master.tx_styles.other_style.as_ref().expect("other style");
    assert!(matches!(
        other_style.levels[0]
            .as_ref()
            .and_then(|lvl| lvl.space_after.clone()),
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 0.5).abs() < 1e-6
    ));

    assert_eq!(master.shapes.len(), 4);
    let normal_shape = &master.shapes[0];
    assert!(matches!(
        normal_shape.border.compound,
        CompoundLine::ThinThick
    ));
    assert!(matches!(
        normal_shape.border.dash_style,
        DashStyle::SystemDashDotDot
    ));
    assert!(matches!(normal_shape.border.join, LineJoin::Bevel));
    assert!(matches!(
        normal_shape
            .text_body
            .as_ref()
            .expect("master normal text")
            .auto_fit,
        AutoFit::Normal {
            font_scale: Some(v),
            line_spacing_reduction: Some(lsr)
        } if (v - 0.65).abs() < 1e-6 && (lsr - 0.12).abs() < 1e-6
    ));
    let no_auto_shape = &master.shapes[1];
    assert!(matches!(
        no_auto_shape.border.compound,
        CompoundLine::ThickThin
    ));
    assert!(matches!(
        no_auto_shape
            .text_body
            .as_ref()
            .expect("master no-auto text")
            .auto_fit,
        AutoFit::NoAutoFit
    ));
    let shrink_shape = &master.shapes[2];
    assert!(matches!(
        shrink_shape
            .text_body
            .as_ref()
            .expect("master shrink text")
            .auto_fit,
        AutoFit::Shrink
    ));
    let implicit_line_shape = &master.shapes[3];
    assert!(matches!(
        implicit_line_shape.border.style,
        BorderStyle::Solid
    ));
}

#[test]
fn parses_start_tag_layout_defaults_and_background_relations_through_public_parser() {
    let presentation_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000" foo="ignored"/>
    <p:defaultTextStyle>
    <a:lvl1pPr algn="ctr">
      <a:spcAft><a:spcPts val="600"/></a:spcAft>
      <a:defRPr sz="2000">
        <a:srgbClr val="112233"></a:srgbClr>
      </a:defRPr>
    </a:lvl1pPr>
    <a:lvl2pPr algn="just">
      <a:lnSpc><a:spcPct val="150000"/></a:lnSpc>
      <a:defRPr sz="1800">
        <a:schemeClr val="accent2"></a:schemeClr>
      </a:defRPr>
    </a:lvl2pPr>
    <a:lvl3pPr algn="r">
      <a:spcBef><a:spcPct val="25000"/></a:spcBef>
    </a:lvl3pPr>
    <a:lvl4pPr algn="l">
      <a:lnSpc><a:spcPts val="900"/></a:lnSpc>
    </a:lvl4pPr>
    <a:lvl5pPr algn="just">
      <a:spcAft><a:spcPct val="50000"/></a:spcAft>
    </a:lvl5pPr>
  </p:defaultTextStyle>
</p:presentation>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             type="title" showMasterSp="false">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="FF0000"></a:srgbClr></a:gs>
            <a:gs pos="100000"><a:schemeClr val="accent2"></a:schemeClr></a:gs>
          </a:gsLst>
          <a:path path="rect"></a:path>
          <a:lin ang="1800000"/>
        </a:gradFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="StartTagNormal"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="12700" y="25400"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700" cap="flat" cmpd="thickThin" algn="in">
            <a:prstDash val="sysDashDotDot"/>
            <a:round/>
            <a:schemeClr val="accent3"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr" anchorCtr="true" rot="60000" vert="horz"
                    lIns="91440" tIns="45720" rIns="182880" bIns="22860" wrap="square"></a:bodyPr>
          <a:normAutofit fontScale="65000" lnSpcReduction="12000"></a:normAutofit>
          <a:lstStyle>
            <a:lvl1pPr algn="ctr">
              <a:defRPr sz="1800">
                <a:srgbClr val="112233"></a:srgbClr>
              </a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p/>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="3" name="StartTagNoAuto"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="25400" y="38100"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700" cmpd="thinThick">
            <a:prstDash val="dash"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr></a:bodyPr>
          <a:noAutofit></a:noAutofit>
          <a:lstStyle>
            <a:lvl1pPr>
              <a:defRPr sz="1600">
                <a:schemeClr val="accent2"></a:schemeClr>
              </a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p/>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="4" name="StartTagShrink"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="38100" y="50800"/><a:ext cx="381000" cy="254000"/></a:xfrm>
        </p:spPr>
        <p:txBody>
          <a:bodyPr></a:bodyPr>
          <a:spAutoFit></a:spAutoFit>
          <a:p/>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="5" name="ImplicitSolidLine"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="50800" y="63500"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700"></a:ln>
        </p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sldLayout>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_presentation_xml(presentation_xml)
        .with_clr_map(
            r#"bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink""#,
        )
        .with_layout(layout_xml)
        .build();

    let presentation = parse_pptx(&pptx);

    let defaults = presentation
        .default_text_style
        .as_ref()
        .expect("default text style should be parsed");
    let lvl1 = defaults.levels[0].as_ref().expect("level 1 defaults");
    assert!(matches!(
        lvl1.space_after,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 6.0).abs() < 1e-6
    ));
    assert_eq!(
        lvl1.def_run_props
            .as_ref()
            .and_then(|rd| rd.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#112233")
    );
    let lvl2 = defaults.levels[1].as_ref().expect("level 2 defaults");
    assert!(matches!(
        lvl2.line_spacing,
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 1.5).abs() < 1e-6
    ));
    assert_eq!(
        lvl2.def_run_props
            .as_ref()
            .and_then(|rd| rd.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#ED7D31")
    );
    let lvl3 = defaults.levels[2].as_ref().expect("level 3 defaults");
    assert!(matches!(
        lvl3.space_before,
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 0.25).abs() < 1e-6
    ));
    let lvl4 = defaults.levels[3].as_ref().expect("level 4 defaults");
    assert!(matches!(
        lvl4.line_spacing,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 9.0).abs() < 1e-6
    ));
    let lvl5 = defaults.levels[4].as_ref().expect("level 5 defaults");
    assert!(matches!(
        lvl5.space_after,
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 0.5).abs() < 1e-6
    ));

    let layout = &presentation.layouts[0];
    assert_eq!(layout.layout_type.as_deref(), Some("title"));
    assert!(!layout.show_master_sp);
    assert!(matches!(
        &layout.background,
        Some(Fill::Gradient(fill)) if fill.stops.len() == 2
    ));

    assert_eq!(layout.shapes.len(), 4);
    let normal_shape = &layout.shapes[0];
    assert!(matches!(normal_shape.border.cap, LineCap::Flat));
    assert!(matches!(
        normal_shape.border.compound,
        CompoundLine::ThickThin
    ));
    assert!(matches!(normal_shape.border.join, LineJoin::Round));
    assert!(matches!(
        normal_shape.border.dash_style,
        DashStyle::SystemDashDotDot
    ));
    assert_eq!(
        normal_shape.border.color.to_css().as_deref(),
        Some("#A5A5A5")
    );
    assert_eq!(normal_shape.vertical_text, None);
    let normal_text = normal_shape
        .text_body
        .as_ref()
        .expect("normal shape text body");
    assert!(matches!(normal_text.vertical_align, VerticalAlign::Middle));
    assert!(normal_text.anchor_center);
    assert!(normal_text.word_wrap);
    assert!(matches!(
        normal_text.auto_fit,
        AutoFit::Normal {
            font_scale: Some(v),
            line_spacing_reduction: Some(lsr)
        } if (v - 0.65).abs() < 1e-6 && (lsr - 0.12).abs() < 1e-6
    ));
    assert_eq!(
        normal_text
            .list_style
            .as_ref()
            .and_then(|style| style.levels[0].as_ref())
            .and_then(|level| level.def_run_props.as_ref())
            .and_then(|rd| rd.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#112233")
    );

    let no_auto_shape = &layout.shapes[1];
    assert!(matches!(
        no_auto_shape.border.compound,
        CompoundLine::ThinThick
    ));
    let no_auto_text = no_auto_shape.text_body.as_ref().expect("no-auto text body");
    assert!(matches!(no_auto_text.auto_fit, AutoFit::NoAutoFit));
    assert_eq!(
        no_auto_text
            .list_style
            .as_ref()
            .and_then(|style| style.levels[0].as_ref())
            .and_then(|level| level.def_run_props.as_ref())
            .and_then(|rd| rd.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#ED7D31")
    );

    let shrink_shape = &layout.shapes[2];
    assert!(matches!(
        shrink_shape
            .text_body
            .as_ref()
            .expect("shrink text body")
            .auto_fit,
        AutoFit::Shrink
    ));
    let implicit_line_shape = &layout.shapes[3];
    assert!(matches!(
        implicit_line_shape.border.style,
        BorderStyle::Solid
    ));
}

#[test]
fn parses_layout_background_image_start_tags_through_public_parser() {
    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             type="body">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:blipFill>
          <a:blip r:embed="rIdBg"></a:blip>
        </a:blipFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sldLayout>"#;

    let layout_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
  <Relationship Id="rIdBg" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/layout-bg.png"/>
</Relationships>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_clr_map(
            r#"bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink""#,
        )
        .with_layout(layout_xml)
        .with_layout_rels(layout_rels)
        .with_extra_file("ppt/media/layout-bg.png", b"layout-background")
        .build();

    let presentation = parse_pptx(&pptx);
    let layout = &presentation.layouts[0];
    assert_eq!(layout.layout_type.as_deref(), Some("body"));
    assert!(matches!(
        &layout.background,
        Some(Fill::Image(image))
            if image.content_type == "image/png" && image.data == b"layout-background"
    ));
}

#[test]
fn parses_master_solid_background_and_dashdot_start_variants_through_public_parser() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:solidFill><a:srgbClr val="334455"></a:srgbClr></a:solidFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="DashDot"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="12700" y="25400"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700"><a:prstDash val="dashDot"/></a:ln>
        </p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sldMaster>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_full_master(master_xml)
        .build();

    let presentation = parse_pptx(&pptx);
    let master = &presentation.masters[0];
    assert!(matches!(
        &master.background,
        Some(Fill::Solid(fill)) if matches!(fill.color.kind, ColorKind::Rgb(ref rgb) if rgb == "334455")
    ));
    let shape = &master.shapes[0];
    assert!(matches!(shape.border.style, BorderStyle::Solid));
    assert!(matches!(shape.border.dash_style, DashStyle::DashDot));

    let empty_master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:solidFill><a:schemeClr val="accent2"/></a:solidFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sldMaster>"#;

    let empty_pptx = fixtures::MinimalPptx::new("")
        .with_full_master(empty_master_xml)
        .build();
    let empty_presentation = parse_pptx(&empty_pptx);
    assert!(matches!(
        &empty_presentation.masters[0].background,
        Some(Fill::Solid(fill))
            if matches!(fill.color.kind, ColorKind::Theme(ref name) if name == "accent2")
    ));
}

#[test]
fn parse_slide_master_directly_covers_solid_background_and_dashdot_branches() {
    use std::collections::HashMap;
    use std::io::{Cursor, Write};

    use pptx2html_core::parser::master_parser::parse_slide_master;
    use zip::write::SimpleFileOptions;
    use zip::{ZipArchive, ZipWriter};

    fn empty_archive() -> ZipArchive<Cursor<Vec<u8>>> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        zip.start_file("ppt/empty.txt", options)
            .expect("start archive file");
        zip.write_all(b"").expect("write empty archive payload");
        ZipArchive::new(Cursor::new(
            zip.finish().expect("finish archive").into_inner(),
        ))
        .expect("open archive")
    }

    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:solidFill><a:schemeClr val="accent2"/></a:solidFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="DashDot"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="12700" y="25400"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700" cap="flat" cmpd="tri" algn="in">
            <a:prstDash val="dashDot"/>
            <a:bevel/>
          </a:ln>
        </p:spPr>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="ImplicitSolid"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="25400" y="38100"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700"></a:ln>
        </p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sldMaster>"#;

    let mut archive = empty_archive();
    let master = parse_slide_master(xml, &HashMap::new(), &mut archive).expect("master parses");

    assert!(matches!(
        &master.background,
        Some(Fill::Solid(fill))
            if matches!(fill.color.kind, ColorKind::Theme(ref name) if name == "accent2")
    ));
    assert!(matches!(master.shapes[0].border.cap, LineCap::Flat));
    assert!(matches!(
        master.shapes[0].border.compound,
        CompoundLine::Triple
    ));
    assert!(matches!(master.shapes[0].border.style, BorderStyle::Solid));
    assert!(matches!(
        master.shapes[0].border.dash_style,
        DashStyle::DashDot
    ));
    assert!(matches!(master.shapes[0].border.join, LineJoin::Bevel));
    assert!(matches!(master.shapes[1].border.style, BorderStyle::Solid));
}

#[test]
fn parses_tables_and_unresolved_graphic_frames_and_renders_markers() {
    let slide = r#"
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="SmartArt"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></p:xfrm>
        <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram">
          <dgm:relIds r:dm="1"/>
        </a:graphicData></a:graphic>
      </p:graphicFrame>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="3" name="Math"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></p:xfrm>
        <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/officeDocument/2006/math">
          <m:oMath/>
        </a:graphicData></a:graphic>
      </p:graphicFrame>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="4" name="Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1" bandCol="1" firstRow="1" lastRow="1" firstCol="1" lastCol="1"></a:tblPr>
              <a:tblGrid>
                <a:gridCol w="914400"/>
                <a:gridCol w="457200"></a:gridCol>
              </a:tblGrid>
              <a:tr h="457200">
                <a:tc gridSpan="2" rowSpan="2" vMerge="1">
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr algn="ctr" lvl="1" indent="91440" marL="45720"/>
                      <a:defRPr sz="2000" cap="small" b="1"/>
                      <a:buClr><a:schemeClr val="accent2"/></a:buClr>
                      <a:r>
                        <a:rPr sz="1800" u="sng" strike="dblStrike" cap="all" baseline="20000" spc="100">
                          <a:hlinkClick r:id="rIdLink"/>
                        </a:rPr>
                        <a:t>Cell</a:t>
                      </a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr marL="91440" marR="137160" marT="45720" marB="22860" anchor="ctr">
                    <a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>
                    <a:lnL w="12700"><a:prstDash val="dash"/><a:srgbClr val="FF0000"/></a:lnL>
                    <a:lnR w="12700"><a:schemeClr val="accent3"/></a:lnR>
                    <a:lnT w="12700"><a:solidFill><a:srgbClr val="0000FF"/></a:solidFill></a:lnT>
                    <a:lnB w="12700"><a:noFill/></a:lnB>
                  </a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:cxnSp>
        <p:nvCxnSpPr>
          <p:cNvPr id="5" name="Connector"/>
          <p:cNvCxnSpPr/>
          <p:nvPr/>
        </p:nvCxnSpPr>
        <p:spPr>
          <a:xfrm rot="5400000"><a:off x="0" y="0"/><a:ext cx="0" cy="914400"/></a:xfrm>
          <a:ln w="12700">
            <a:headEnd type="triangle" w="lg" len="sm"/>
            <a:tailEnd type="oval" w="sm" len="lg"/>
            <a:schemeClr val="accent1"/>
          </a:ln>
        </p:spPr>
        <p:stCxn id="10" idx="1"/>
        <p:endCxn id="11" idx="2"/>
      </p:cxnSp>
    "#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/>
</Relationships>"#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_slide_rels(slide_rels)
        .build();

    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];
    assert_eq!(slide.shapes.len(), 4);
    assert!(slide.shapes.iter().any(|shape| matches!(
        &shape.shape_type,
        ShapeType::Unsupported(data)
            if data.raw_xml.as_deref().is_some_and(|raw| raw.contains("relIds"))
    )));
    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    assert!(table.band_row && table.band_col && table.first_row && table.last_row);
    assert_eq!(table.col_widths.len(), 2);
    let cell = &table.rows[0].cells[0];
    assert_eq!(cell.col_span, 2);
    assert_eq!(cell.row_span, 2);
    assert!(cell.v_merge);
    assert!(matches!(cell.vertical_align, VerticalAlign::Middle));
    let para = &cell.text_body.as_ref().expect("cell text").paragraphs[0];
    assert!(matches!(para.alignment, Alignment::Center));
    assert_eq!(para.level, 1);
    assert_eq!(
        para.runs[0].hyperlink.as_deref(),
        Some("https://example.com")
    );

    let result = render_with_metadata(&pptx).expect("render with metadata");
    assert!(
        result.html.contains("marker-"),
        "expected SVG marker defs in HTML"
    );
    assert!(
        result.html.contains("unresolved-element"),
        "expected unresolved placeholders in HTML"
    );
    assert!(
        result.unresolved_elements.iter().any(|elem| elem
            .raw_xml
            .as_deref()
            .is_some_and(|raw| raw.contains("oMath"))),
        "expected raw XML to survive into unresolved metadata"
    );
}

#[test]
fn parses_group_and_dash_empty_variants_through_public_parser() {
    let slide = r#"
      <p:grpSp>
        <p:nvGrpSpPr><p:cNvPr id="10" name="Group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
        <p:grpSpPr>
          <a:xfrm>
            <a:off x="100000" y="100000"/>
            <a:ext cx="5000000" cy="3000000"/>
            <a:chOff><a:off x="0" y="0"/></a:chOff>
            <a:chExt><a:ext cx="5000000" cy="3000000"/></a:chExt>
          </a:xfrm>
        </p:grpSpPr>
        <p:sp>
          <p:nvSpPr><p:cNvPr id="11" name="InnerShape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
          <p:spPr>
            <a:xfrm><a:off x="500000" y="500000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
            <a:prstGeom prst="rect"/>
            <a:solidFill><a:srgbClr val="AABB00"/></a:solidFill>
          </p:spPr>
        </p:sp>
      </p:grpSp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="20" name="SolidDash"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="500000"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:noFill/>
          <a:ln w="12700"><a:prstDash val="solid"/><a:sysClr lastClr="111111"/></a:ln>
        </p:spPr>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="21" name="DotDash"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="500000"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:ln w="12700"><a:prstDash val="dot"/><a:sysClr val="windowText"/></a:ln>
        </p:spPr>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="22" name="LongDash"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="500000"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:ln w="12700"><a:prstDash val="lgDash"/><a:srgbClr val="223344"/></a:ln>
        </p:spPr>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="23" name="SystemDot"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="500000"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:ln w="12700"><a:prstDash val="sysDot"/><a:srgbClr val="445566"/></a:ln>
        </p:spPr>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="24" name="SystemDashDotDot"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm rot="5400000" flipH="true" flipV="1"/>
          <a:prstGeom prst="rect"/>
          <a:ln w="12700"><a:prstDash val="sysDashDotDot"/><a:srgbClr val="778899"/></a:ln>
        </p:spPr>
      </p:sp>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shapes = &pres.slides[0].shapes;

    let group = shapes
        .iter()
        .find(|shape| matches!(shape.shape_type, ShapeType::Group(_, _)))
        .expect("group shape");
    assert!(matches!(&group.shape_type, ShapeType::Group(children, _) if children.len() == 1));

    let solid_dash = shapes
        .iter()
        .find(|shape| shape.name == "SolidDash")
        .expect("solid dash shape");
    assert!(matches!(solid_dash.fill, Fill::NoFill));
    assert!(matches!(solid_dash.border.style, BorderStyle::Solid));
    assert!(matches!(solid_dash.border.dash_style, DashStyle::Solid));

    let dot_dash = shapes
        .iter()
        .find(|shape| shape.name == "DotDash")
        .expect("dot dash shape");
    assert!(matches!(dot_dash.border.style, BorderStyle::Dotted));
    assert!(matches!(dot_dash.border.dash_style, DashStyle::Dot));

    let long_dash = shapes
        .iter()
        .find(|shape| shape.name == "LongDash")
        .expect("long dash shape");
    assert!(matches!(long_dash.border.style, BorderStyle::Dashed));
    assert!(matches!(long_dash.border.dash_style, DashStyle::LongDash));

    let system_dot = shapes
        .iter()
        .find(|shape| shape.name == "SystemDot")
        .expect("system dot shape");
    assert!(matches!(system_dot.border.style, BorderStyle::Dotted));
    assert!(matches!(system_dot.border.dash_style, DashStyle::SystemDot));

    let system_dash_dot_dot = shapes
        .iter()
        .find(|shape| shape.name == "SystemDashDotDot")
        .expect("system dash dot dot shape");
    assert!(system_dash_dot_dot.flip_h);
    assert!(system_dash_dot_dot.flip_v);
    assert!(matches!(
        system_dash_dot_dot.border.style,
        BorderStyle::Dotted
    ));
    assert!(matches!(
        system_dash_dot_dot.border.dash_style,
        DashStyle::SystemDashDotDot
    ));
}

#[test]
fn parses_empty_event_color_and_dash_matrix_through_public_parser() {
    let slide = r#"
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="30" name="Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1" bandCol="true" firstRow="1" lastRow="true" firstCol="1" lastCol="true"></a:tblPr>
              <a:tblGrid>
                <a:gridCol w="914400"/>
                <a:gridCol w="457200"/>
              </a:tblGrid>
              <a:tr h="457200">
                <a:tc gridSpan="2" rowSpan="1" vMerge="1">
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr algn="ctr" lvl="1" indent="91440" marL="45720"/>
                      <a:defRPr sz="2000" spc="100" baseline="10000" cap="small" u="dashLong" strike="dblStrike" b="true" i="true"/>
                      <a:buClr><a:prstClr val="orange"/></a:buClr>
                      <a:r><a:rPr sz="1800"/><a:t>Cell One</a:t></a:r>
                      <a:r><a:rPr sz="1800"><a:hlinkClick r:id="rIdCell"/></a:rPr><a:t>Cell Two</a:t></a:r>
                      <a:br/>
                    </a:p>
                  </a:txBody>
                  <a:tcPr marL="91440" marR="137160" marT="45720" marB="22860" anchor="ctr">
                    <a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>
                    <a:lnL w="12700"><a:prstDash val="solid"/><a:srgbClr val="FF0000"/></a:lnL>
                    <a:lnR w="12700"><a:prstDash val="dashDot"/><a:srgbClr val="0000FF"/></a:lnR>
                    <a:lnT w="12700"><a:prstDash val="lgDash"/><a:srgbClr val="123456"/></a:lnT>
                    <a:lnB w="12700"><a:prstDash val="sysDashDotDot"/><a:srgbClr val="654321"/></a:lnB>
                  </a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="31" name="Shape Matrix"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm rot="5400000" flipH="true"/>
          <a:gradFill>
            <a:gsLst>
              <a:gs pos="0"><a:prstClr val="orange"/></a:gs>
              <a:gs pos="100000"><a:sysClr lastClr="112233"/></a:gs>
            </a:gsLst>
            <a:lin ang="5400000"/>
            <a:path path="shape"/>
          </a:gradFill>
          <a:ln w="12700" cap="rnd" cmpd="dbl" algn="in">
            <a:prstDash val="sysDashDot"/>
            <a:bevel/>
            <a:headEnd type="triangle" w="lg" len="sm"/>
            <a:tailEnd type="diamond" w="sm" len="lg"/>
            <a:sysClr val="windowText"/>
          </a:ln>
          <a:effectLst>
            <a:outerShdw blurRad="12700" dist="25400" dir="5400000"><a:schemeClr val="accent1"/></a:outerShdw>
            <a:glow rad="6350"><a:sysClr lastClr="123456"/></a:glow>
          </a:effectLst>
          <a:custGeom>
            <a:avLst><a:gd name="adj1" fmla="val 50000"/></a:avLst>
            <a:gdLst><a:gd name="x1" fmla="val 100000"/></a:gdLst>
            <a:ahLst>
              <a:ahXY gdRefX="adj1" minX="0" maxX="100000" gdRefY="adj1" minY="0" maxY="100000"><a:pos x="50000" y="50000"/></a:ahXY>
              <a:ahPolar gdRefR="adj1" minR="0" maxR="100000" gdRefAng="adj1" minAng="0" maxAng="100000"><a:pos x="50000" y="50000"/></a:ahPolar>
            </a:ahLst>
            <a:cxnLst><a:cxn ang="0"><a:pos x="0" y="0"/></a:cxn></a:cxnLst>
            <a:rect l="0" t="0" r="100000" b="100000"/>
            <a:pathLst>
              <a:path w="100000" h="100000" fill="lighten"><a:moveTo><a:pt x="0" y="0"/></a:moveTo></a:path>
              <a:path w="100000" h="100000" fill="darken"><a:moveTo><a:pt x="0" y="0"/></a:moveTo></a:path>
              <a:path w="100000" h="100000" fill="lightenLess"><a:moveTo><a:pt x="0" y="0"/></a:moveTo></a:path>
            </a:pathLst>
          </a:custGeom>
        </p:spPr>
        <p:style>
          <a:lnRef idx="1"/>
          <a:fillRef idx="2"/>
          <a:effectRef idx="3"/>
          <a:fontRef idx="minor"/>
        </p:style>
        <p:txBody>
          <a:bodyPr anchor="ctr" wrap="none"/>
          <a:normAutofit fontScale="80000" lnSpcReduction="10000"/>
          <a:p>
            <a:pPr algn="r" lvl="1" indent="12700" marL="25400"/>
            <a:defRPr sz="2400" spc="200" baseline="30000" cap="all" u="dbl" strike="sngStrike" b="true" i="1"/>
            <a:buClr><a:sysClr lastClr="445566"/></a:buClr>
            <a:r><a:rPr sz="1800"/><a:t>Shape One</a:t></a:r>
            <a:r><a:rPr sz="1800"><a:hlinkClick r:id="rIdShape"/></a:rPr><a:t>Shape Two</a:t></a:r>
            <a:br/>
          </a:p>
        </p:txBody>
      </p:sp>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_slide_rels(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdCell" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/cell" TargetMode="External"/>
  <Relationship Id="rIdShape" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/shape" TargetMode="External"/>
</Relationships>"#,
        )
        .build();

    let presentation = parse_pptx(&pptx);
    let shapes = &presentation.slides[0].shapes;

    let table = shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    let cell = &table.rows[0].cells[0];
    assert!(table.band_row && table.band_col && table.first_row && table.last_row);
    assert!(table.first_col && table.last_col);
    assert_eq!(cell.col_span, 2);
    assert!(matches!(cell.border_left.style, BorderStyle::Solid));
    assert!(matches!(cell.border_left.dash_style, DashStyle::Solid));
    assert!(cell.border_right.width > 0.0);
    assert!(cell.border_top.width > 0.0);
    assert!(cell.border_bottom.width > 0.0);
    let cell_para = &cell.text_body.as_ref().expect("cell text").paragraphs[0];
    assert_eq!(cell_para.runs.len(), 3);
    assert_eq!(
        cell_para.runs[1].hyperlink.as_deref(),
        Some("https://example.com/cell")
    );
    let cell_def = cell_para.def_rpr.as_ref().expect("cell defRPr");
    assert_eq!(cell_def.font_size, Some(20.0));
    assert_eq!(cell_def.letter_spacing, Some(1.0));
    assert_eq!(cell_def.baseline, Some(10000));
    assert_eq!(cell_def.bold, Some(true));
    assert_eq!(cell_def.italic, Some(true));
    assert!(matches!(
        cell_def.capitalization,
        Some(TextCapitalization::Small)
    ));
    assert!(matches!(cell_def.underline, Some(UnderlineType::DashLong)));
    assert!(matches!(
        cell_def.strikethrough,
        Some(StrikethroughType::Double)
    ));

    let shape = shapes
        .iter()
        .find(|shape| shape.name == "Shape Matrix")
        .expect("shape matrix");
    assert!(matches!(
        shape.fill,
        Fill::Gradient(ref fill)
            if fill.stops.len() == 2
                && matches!(fill.gradient_type, GradientType::Shape)
                && matches!(fill.stops[0].color.kind, ColorKind::Preset(_))
                && matches!(fill.stops[1].color.kind, ColorKind::Rgb(_))
    ));
    assert!(matches!(shape.border.cap, LineCap::Round));
    assert!(matches!(shape.border.compound, CompoundLine::Double));
    assert!(matches!(shape.border.alignment, LineAlignment::Inset));
    assert!(matches!(shape.border.join, LineJoin::Bevel));
    assert!(matches!(shape.border.dash_style, DashStyle::SystemDashDot));
    assert!(shape.style_ref.as_ref().is_some());
    let text_body = shape.text_body.as_ref().expect("shape text body");
    assert!(matches!(
        text_body.auto_fit,
        AutoFit::Normal {
            font_scale: Some(v),
            line_spacing_reduction: Some(lsr)
        } if (v - 0.8).abs() < 1e-6 && (lsr - 0.1).abs() < 1e-6
    ));
    let para = &text_body.paragraphs[0];
    assert_eq!(para.runs.len(), 3);
    assert_eq!(
        para.runs[1].hyperlink.as_deref(),
        Some("https://example.com/shape")
    );
    let def = para.def_rpr.as_ref().expect("shape defRPr");
    assert_eq!(def.font_size, Some(24.0));
    assert!(matches!(def.capitalization, Some(TextCapitalization::All)));
    assert!(matches!(def.underline, Some(UnderlineType::Double)));
    assert!(matches!(def.strikethrough, Some(StrikethroughType::Single)));

    let custom_geom = match &shape.shape_type {
        ShapeType::CustomGeom(geom) => geom,
        other => panic!("expected custom geometry, got {other:?}"),
    };
    assert_eq!(custom_geom.adjust_handles.len(), 2);
    assert_eq!(custom_geom.connection_sites.len(), 1);
    assert_eq!(custom_geom.paths.len(), 3);
    assert!(matches!(custom_geom.paths[0].fill, PathFill::Lighten));
    assert!(matches!(custom_geom.paths[1].fill, PathFill::Darken));
    assert!(matches!(custom_geom.paths[2].fill, PathFill::LightenLess));

    let rendered = render_with_metadata(&pptx).expect("render with metadata");
    assert!(
        !rendered.html.is_empty(),
        "rendered HTML should still be produced for the fixture"
    );
}

#[test]
fn parses_empty_event_bullet_font_highlight_and_style_ref_matrix_through_public_parser() {
    let slide = r#"
      <p:bg>
        <p:bgPr>
          <a:solidFill><a:prstClr val="orange"/></a:solidFill>
        </p:bgPr>
      </p:bg>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="40" name="Style Ref Matrix"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rect"/>
        </p:spPr>
        <p:style>
          <a:lnRef idx="1"><a:schemeClr val="accent1"/></a:lnRef>
          <a:fillRef idx="2"><a:prstClr val="orange"/></a:fillRef>
          <a:effectRef idx="3"><a:sysClr lastClr="112233"/></a:effectRef>
          <a:fontRef idx="minor"><a:schemeClr val="accent2"/></a:fontRef>
        </p:style>
        <p:txBody>
          <a:bodyPr/>
          <a:p>
            <a:pPr algn="ctr">
              <a:lnSpc><a:spcPct val="110000"/></a:lnSpc>
              <a:spcAft><a:spcPts val="1400"/></a:spcAft>
              <a:buChar char="•"/>
              <a:buClr><a:schemeClr val="accent4"/></a:buClr>
              <a:buFont typeface="Wingdings"/>
              <a:buSzPct val="125000"/>
              <a:defRPr sz="2000">
                <a:latin typeface="Aptos"/>
                <a:ea typeface="Yu Gothic"/>
                <a:cs typeface="Noto Sans Arabic"/>
              </a:defRPr>
            </a:pPr>
            <a:r>
              <a:rPr sz="1800">
                <a:hlinkClick r:id="rIdStart"></a:hlinkClick>
                <a:highlight><a:schemeClr val="accent6"/></a:highlight>
                <a:latin typeface="Calibri"/>
                <a:ea typeface="Meiryo"/>
                <a:cs typeface="Noto Sans Devanagari"/>
                <a:sysClr lastClr="223344"/>
              </a:rPr>
              <a:t>Styled Run</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="41" name="Cell Matrix"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1"/>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr algn="l">
                        <a:lnSpc><a:spcPts val="900"/></a:lnSpc>
                        <a:spcBef><a:spcPct val="25000"/></a:spcBef>
                        <a:spcAft><a:spcPts val="1200"/></a:spcAft>
                        <a:buChar char="◦"/>
                        <a:buClr><a:prstClr val="orange"/></a:buClr>
                        <a:buFont typeface="Symbol"/>
                        <a:buSzPts val="1600"/>
                        <a:defRPr sz="1900">
                          <a:latin typeface="Cell Latin"/>
                          <a:ea typeface="Cell EA"/>
                          <a:cs typeface="Cell CS"/>
                        </a:defRPr>
                      </a:pPr>
                      <a:r>
                        <a:rPr sz="1700">
                          <a:highlight><a:srgbClr val="FFFF00"/></a:highlight>
                          <a:latin typeface="CellRun Latin"/>
                          <a:ea typeface="CellRun EA"/>
                          <a:cs typeface="CellRun CS"/>
                          <a:prstClr val="orange"/>
                        </a:rPr>
                        <a:t>Cell Run</a:t>
                      </a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr anchor="ctr">
                    <a:solidFill><a:schemeClr val="accent5"/></a:solidFill>
                    <a:lnL w="12700"><a:sysClr val="windowText"/></a:lnL>
                    <a:lnR w="12700"><a:prstClr val="orange"/></a:lnR>
                  </a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_slide_rels(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdStart" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/start" TargetMode="External"/>
</Relationships>"#,
        )
        .build();

    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];
    assert!(matches!(
        &slide.background,
        Some(Fill::Solid(fill))
            if matches!(fill.color.kind, ColorKind::Preset(ref name) if name == "orange")
    ));

    let shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Style Ref Matrix")
        .expect("style ref shape");
    let style_ref = shape.style_ref.as_ref().expect("shape style ref");
    assert!(matches!(
        style_ref
            .ln_ref
            .as_ref()
            .map(|style| style.color.kind.clone()),
        Some(ColorKind::Theme(name)) if name == "accent1"
    ));
    assert!(matches!(
        style_ref
            .fill_ref
            .as_ref()
            .map(|style| style.color.kind.clone()),
        Some(ColorKind::Preset(name)) if name == "orange"
    ));
    assert!(matches!(
        style_ref
            .effect_ref
            .as_ref()
            .map(|style| style.color.kind.clone()),
        Some(ColorKind::Rgb(rgb)) if rgb == "112233"
    ));
    assert!(matches!(
        style_ref
            .font_ref
            .as_ref()
            .map(|style| style.color.kind.clone()),
        Some(ColorKind::Theme(name)) if name == "accent2"
    ));

    let para = &shape
        .text_body
        .as_ref()
        .expect("shape text body")
        .paragraphs[0];
    assert!(matches!(
        para.line_spacing,
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 1.1).abs() < 1e-6
    ));
    assert!(matches!(
        para.space_after,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 14.0).abs() < 1e-6
    ));
    assert!(matches!(
        &para.bullet,
        Some(Bullet::Char(bullet))
            if bullet.char == "•"
    ));
    let shape_def = para.def_rpr.as_ref().expect("shape defRPr");
    assert_eq!(shape_def.font_latin.as_deref(), Some("Aptos"));
    assert_eq!(shape_def.font_ea.as_deref(), Some("Yu Gothic"));
    assert_eq!(shape_def.font_cs.as_deref(), Some("Noto Sans Arabic"));
    let run = &para.runs[0];
    assert_eq!(run.hyperlink.as_deref(), Some("https://example.com/start"));
    assert_eq!(run.font.latin.as_deref(), Some("Calibri"));
    assert_eq!(run.font.east_asian.as_deref(), Some("Meiryo"));
    assert_eq!(
        run.font.complex_script.as_deref(),
        Some("Noto Sans Devanagari")
    );
    assert_eq!(run.style.color.to_css().as_deref(), Some("#223344"));
    assert_eq!(
        run.style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#70AD47")
    );

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    let cell = &table.rows[0].cells[0];
    assert!(matches!(
        &cell.fill,
        Fill::Solid(fill) if fill.color.to_css().as_deref() == Some("#5B9BD5")
    ));
    assert_eq!(cell.border_left.color.to_css().as_deref(), Some("#000000"));
    assert_eq!(cell.border_right.color.to_css().as_deref(), Some("#FFA500"));
    let cell_para = &cell.text_body.as_ref().expect("cell text body").paragraphs[0];
    assert!(matches!(
        cell_para.line_spacing,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 9.0).abs() < 1e-6
    ));
    assert!(matches!(
        cell_para.space_before,
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 0.25).abs() < 1e-6
    ));
    assert!(matches!(
        cell_para.space_after,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 12.0).abs() < 1e-6
    ));
    assert!(matches!(
        &cell_para.bullet,
        Some(Bullet::Char(bullet))
            if bullet.char == "◦"
    ));
    let cell_def = cell_para.def_rpr.as_ref().expect("cell defRPr");
    assert_eq!(cell_def.font_latin.as_deref(), Some("Cell Latin"));
    assert_eq!(cell_def.font_ea.as_deref(), Some("Cell EA"));
    assert_eq!(cell_def.font_cs.as_deref(), Some("Cell CS"));
    let cell_run = &cell_para.runs[0];
    assert_eq!(cell_run.font.latin.as_deref(), Some("CellRun Latin"));
    assert_eq!(cell_run.font.east_asian.as_deref(), Some("CellRun EA"));
    assert_eq!(cell_run.font.complex_script.as_deref(), Some("CellRun CS"));
    assert_eq!(cell_run.style.color.to_css().as_deref(), Some("#FFA500"));
    assert_eq!(
        cell_run
            .style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#FFFF00")
    );
}

#[test]
fn parses_empty_event_autonum_bullet_none_and_gradient_stop_matrix_through_public_parser() {
    let slide = r#"
      <p:bg>
        <p:bgPr>
          <a:gradFill>
            <a:gsLst>
              <a:gs pos="0"><a:prstClr val="orange"/></a:gs>
              <a:gs pos="100000"><a:sysClr lastClr="123456"/></a:gs>
            </a:gsLst>
            <a:path path="shape"/>
          </a:gradFill>
        </p:bgPr>
      </p:bg>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="50" name="AutoNum Shape"></p:cNvPr>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:custGeom>
            <a:pathLst>
              <a:path w="100000" h="100000" fill="none"/>
            </a:pathLst>
          </a:custGeom>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:p>
            <a:pPr algn="ctr">
              <a:lnSpc><a:spcPct val="110000"/></a:lnSpc>
              <a:spcBef><a:spcPts val="1200"/></a:spcBef>
              <a:spcAft><a:spcPct val="25000"/></a:spcAft>
              <a:buFont typeface="Wingdings"/>
              <a:buSzPct val="125000"/>
              <a:buClr><a:prstClr val="orange"/></a:buClr>
              <a:buAutoNum type="arabicParenR" startAt="3"/>
            </a:pPr>
            <a:r>
              <a:rPr sz="1800">
                <a:hlinkClick r:id="rIdRun"></a:hlinkClick>
                <a:highlight><a:schemeClr val="accent6"/></a:highlight>
                <a:sysClr val="windowText"/>
              </a:rPr>
              <a:t>AutoNum</a:t>
            </a:r>
          </a:p>
          <a:p>
            <a:pPr><a:buNone/></a:pPr>
            <a:r><a:t>No Bullet</a:t></a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="51" name="AutoNum Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1"/>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr algn="l">
                        <a:lnSpc><a:spcPts val="900"/></a:lnSpc>
                        <a:spcBef><a:spcPct val="25000"/></a:spcBef>
                        <a:spcAft><a:spcPts val="1200"/></a:spcAft>
                        <a:buFont typeface="Symbol"/>
                        <a:buSzPts val="1600"/>
                        <a:buClr><a:schemeClr val="accent4"/></a:buClr>
                        <a:buAutoNum type="alphaLcParenR" startAt="2"/>
                      </a:pPr>
                      <a:r>
                        <a:rPr sz="1700">
                          <a:highlight><a:schemeClr val="accent6"/></a:highlight>
                          <a:prstClr val="orange"/>
                          <a:latin typeface="CellRun Latin"/>
                        </a:rPr>
                        <a:t>Cell Auto</a:t>
                      </a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr anchor="ctr">
                    <a:solidFill><a:sysClr lastClr="ABCDEF"/></a:solidFill>
                    <a:lnL w="12700"><a:sysClr val="windowText"/></a:lnL>
                    <a:lnR w="12700"><a:prstClr val="orange"/></a:lnR>
                  </a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_slide_rels(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdRun" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/auto" TargetMode="External"/>
</Relationships>"#,
        )
        .build();

    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];
    assert!(matches!(
        &slide.background,
        Some(Fill::Gradient(fill))
            if fill.stops.len() == 2
                && matches!(fill.gradient_type, GradientType::Shape)
                && matches!(fill.stops[0].color.kind, ColorKind::Preset(_))
                && matches!(fill.stops[1].color.kind, ColorKind::Rgb(_))
    ));

    let shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "AutoNum Shape")
        .expect("auto-num shape");
    let text_body = shape.text_body.as_ref().expect("shape text body");
    let auto_num_para = &text_body.paragraphs[0];
    assert!(matches!(
        auto_num_para.line_spacing,
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 1.1).abs() < 1e-6
    ));
    assert!(matches!(
        auto_num_para.space_before,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 12.0).abs() < 1e-6
    ));
    assert!(matches!(
        auto_num_para.space_after,
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 0.25).abs() < 1e-6
    ));
    assert!(matches!(
        &auto_num_para.bullet,
        Some(Bullet::AutoNum(bullet))
            if bullet.num_type == "arabicParenR"
                && bullet.start_at == Some(3)
                && bullet.font.as_deref() == Some("Wingdings")
                && bullet.size_pct.is_some_and(|v| (v - 1.25).abs() < 1e-6)
                && bullet.color.as_ref().and_then(|c| c.to_css()).as_deref() == Some("#FFA500")
    ));
    assert_eq!(
        auto_num_para.runs[0].hyperlink.as_deref(),
        Some("https://example.com/auto")
    );
    assert_eq!(
        auto_num_para.runs[0]
            .style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#70AD47")
    );
    assert_eq!(
        auto_num_para.runs[0].style.color.to_css().as_deref(),
        Some("#000000")
    );
    assert!(matches!(
        &text_body.paragraphs[1].bullet,
        Some(Bullet::None)
    ));
    let custom_geom = match &shape.shape_type {
        ShapeType::CustomGeom(geom) => geom,
        other => panic!("expected custom geometry, got {other:?}"),
    };
    assert_eq!(custom_geom.paths.len(), 1);
    assert!(matches!(custom_geom.paths[0].fill, PathFill::None));

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    let cell = &table.rows[0].cells[0];
    assert!(matches!(
        &cell.fill,
        Fill::Solid(fill) if fill.color.to_css().as_deref() == Some("#ABCDEF")
    ));
    assert_eq!(cell.border_left.color.to_css().as_deref(), Some("#000000"));
    assert_eq!(cell.border_right.color.to_css().as_deref(), Some("#FFA500"));
    let cell_para = &cell.text_body.as_ref().expect("cell text body").paragraphs[0];
    assert!(matches!(
        cell_para.line_spacing,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 9.0).abs() < 1e-6
    ));
    assert!(matches!(
        cell_para.space_before,
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 0.25).abs() < 1e-6
    ));
    assert!(matches!(
        cell_para.space_after,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 12.0).abs() < 1e-6
    ));
    assert!(matches!(
        &cell_para.bullet,
        Some(Bullet::AutoNum(bullet))
            if bullet.num_type == "alphaLcParenR"
                && bullet.start_at == Some(2)
                && bullet.font.as_deref() == Some("Symbol")
                && bullet.size_pct.is_some_and(|v| (v + 16.0).abs() < 1e-6)
                && bullet.color.as_ref().and_then(|c| c.to_css()).as_deref() == Some("#FFC000")
    ));
    let cell_run = &cell_para.runs[0];
    assert_eq!(cell_run.font.latin.as_deref(), Some("CellRun Latin"));
    assert_eq!(cell_run.style.color.to_css().as_deref(), Some("#FFA500"));
    assert_eq!(
        cell_run
            .style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#70AD47")
    );
}

#[test]
fn parses_grouped_chart_table_unsupported_and_image_fill_branches_through_public_parser() {
    let slide = r#"
      <p:grpSp>
        <p:nvGrpSpPr><p:cNvPr id="60" name="Outer Group"></p:cNvPr><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
        <p:grpSpPr>
          <a:xfrm>
            <a:off x="100000" y="100000"/>
            <a:ext cx="5000000" cy="3000000"/>
            <a:chOff x="0" y="0"/>
            <a:chExt cx="5000000" cy="3000000"/>
          </a:xfrm>
        </p:grpSpPr>
        <p:grpSp>
          <p:nvGrpSpPr><p:cNvPr id="61" name="Inner Group"></p:cNvPr><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
          <p:grpSpPr>
            <a:xfrm>
              <a:off x="0" y="0"/>
              <a:ext cx="2000000" cy="1000000"/>
              <a:chOff x="0" y="0"/>
              <a:chExt cx="2000000" cy="1000000"/>
            </a:xfrm>
          </p:grpSpPr>
          <p:sp>
            <p:nvSpPr><p:cNvPr id="62" name="Custom Paths"></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
            <p:spPr>
              <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
              <a:custGeom>
                <a:pathLst>
                  <a:path w="100000" h="100000" fill="none"/>
                  <a:path w="100000" h="100000" fill="lighten"/>
                  <a:path w="100000" h="100000" fill="darken"/>
                  <a:path w="100000" h="100000" fill="lightenLess"/>
                  <a:path w="100000" h="100000" fill="darkenLess"/>
                </a:pathLst>
              </a:custGeom>
            </p:spPr>
          </p:sp>
        </p:grpSp>
        <p:graphicFrame>
          <p:nvGraphicFramePr><p:cNvPr id="63" name="Grouped Chart"></p:cNvPr><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
          <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
          <a:graphic>
            <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
              <chart r:id="rIdChart"/>
            </a:graphicData>
          </a:graphic>
        </p:graphicFrame>
        <p:graphicFrame>
          <p:nvGraphicFramePr><p:cNvPr id="64" name="Grouped Math"></p:cNvPr><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
          <p:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></p:xfrm>
          <a:graphic>
            <a:graphicData uri="http://schemas.openxmlformats.org/officeDocument/2006/math">
              <m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"/>
            </a:graphicData>
          </a:graphic>
        </p:graphicFrame>
        <p:graphicFrame>
          <p:nvGraphicFramePr><p:cNvPr id="65" name="Grouped Table"></p:cNvPr><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
          <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
          <a:graphic>
            <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
              <a:tbl>
                <a:tblPr bandRow="1"/>
                <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
                <a:tr h="457200">
                  <a:tc>
                    <a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Grouped Cell</a:t></a:r></a:p></a:txBody>
                    <a:tcPr/>
                  </a:tc>
                </a:tr>
              </a:tbl>
            </a:graphicData>
          </a:graphic>
        </p:graphicFrame>
        <p:sp>
          <p:nvSpPr><p:cNvPr id="66" name="Image Fill Shape"></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
          <p:spPr>
            <a:xfrm><a:off x="25400" y="38100"/><a:ext cx="914400" cy="457200"/></a:xfrm>
            <a:prstGeom prst="rect"/>
            <a:blipFill><a:blip r:embed="rIdFill"/></a:blipFill>
          </p:spPr>
          <p:txBody>
            <a:bodyPr/>
            <a:p>
              <a:r>
                <a:rPr><a:hlinkClick r:id="rIdRun"></a:hlinkClick></a:rPr>
                <a:t>Image Fill</a:t>
              </a:r>
            </a:p>
          </p:txBody>
        </p:sp>
      </p:grpSp>
    "#;

    let chart_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:pieChart>
        <c:varyColors val="1"/>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Series</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="1"/><c:pt idx="0"><c:v>Only</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>42</c:v></c:pt></c:numLit></c:val>
        </c:ser>
      </c:pieChart>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#;

    let chart_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdSkip" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/package" Target="../embeddings/ignored.bin"/>
  <Relationship Id="rIdPreview" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/chart-preview.png"/>
</Relationships>"#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_slide_rels(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdChart" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
  <Relationship Id="rIdFill" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/fill.png"/>
  <Relationship Id="rIdRun" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/grouped" TargetMode="External"/>
</Relationships>"#,
        )
        .with_extra_file("ppt/charts/chart1.xml", chart_xml.as_bytes())
        .with_extra_file("ppt/charts/_rels/chart1.xml.rels", chart_rels.as_bytes())
        .with_extra_file("ppt/media/chart-preview.png", b"preview")
        .with_extra_file("ppt/media/fill.png", b"fill-bytes")
        .build();

    let presentation = parse_pptx(&pptx);
    assert_eq!(presentation.slides.len(), 1);
    assert_eq!(presentation.slides[0].shapes.len(), 1);

    let outer_group = match &presentation.slides[0].shapes[0].shape_type {
        ShapeType::Group(children, group_data) => {
            assert_eq!(group_data.child_offset.x.to_px(), 0.0);
            assert_eq!(
                group_data.child_extent.width.to_px(),
                Emu::parse_emu("5000000").to_px()
            );
            children
        }
        other => panic!("expected outer group, got {other:?}"),
    };
    assert_eq!(outer_group.len(), 5);

    let nested_group = outer_group
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Group(children, _) => Some(children),
            _ => None,
        })
        .expect("nested group");
    let custom_geom = match &nested_group[0].shape_type {
        ShapeType::CustomGeom(geom) => geom,
        other => panic!("expected nested custom geometry, got {other:?}"),
    };
    assert_eq!(custom_geom.paths.len(), 5);
    assert!(matches!(custom_geom.paths[0].fill, PathFill::None));
    assert!(matches!(custom_geom.paths[1].fill, PathFill::Lighten));
    assert!(matches!(custom_geom.paths[2].fill, PathFill::Darken));
    assert!(matches!(custom_geom.paths[3].fill, PathFill::LightenLess));
    assert!(matches!(custom_geom.paths[4].fill, PathFill::DarkenLess));

    let chart = outer_group
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Chart(chart) => Some(chart),
            _ => None,
        })
        .expect("grouped chart");
    assert_eq!(chart.preview_image.as_deref(), Some(b"preview".as_slice()));
    assert_eq!(chart.preview_mime.as_deref(), Some("image/png"));

    assert!(outer_group.iter().any(|shape| matches!(
        &shape.shape_type,
        ShapeType::Unsupported(data)
            if data.raw_xml.as_deref().is_some_and(|raw| raw.contains("oMath"))
    )));

    let table = outer_group
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("grouped table");
    assert_eq!(table.rows.len(), 1);
    assert_eq!(
        table.rows[0].cells[0]
            .text_body
            .as_ref()
            .expect("cell body")
            .paragraphs[0]
            .runs[0]
            .text,
        "Grouped Cell"
    );

    let image_fill_shape = outer_group
        .iter()
        .find(|shape| shape.name == "Image Fill Shape")
        .expect("image fill shape");
    assert_eq!(
        image_fill_shape
            .text_body
            .as_ref()
            .expect("image fill text")
            .paragraphs[0]
            .runs[0]
            .hyperlink
            .as_deref(),
        Some("https://example.com/grouped")
    );
    assert!(matches!(
        &image_fill_shape.fill,
        Fill::Image(ImageFill { data, content_type, .. })
            if data == b"fill-bytes" && content_type == "image/png"
    ));
}

#[test]
fn parses_unsupported_graphic_raw_text_fallthrough_through_public_parser() {
    let slide = r#"
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="170" name="Raw Text Unsupported"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/officeDocument/2006/math">
            <a:txBody>
              <a:bodyPr/>
              <a:p><a:r><a:t>Captured Shape Text</a:t></a:r></a:p>
            </a:txBody>
            <a:tbl>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody><a:bodyPr/><a:p><a:r><a:t>Captured Cell Text</a:t></a:r></a:p></a:txBody>
                  <a:tcPr/>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let presentation = parse_pptx(&pptx);
    let unsupported = presentation.slides[0]
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Unsupported(data) => Some(data),
            _ => None,
        })
        .expect("unsupported math placeholder");

    let raw_xml = unsupported.raw_xml.as_deref().expect("captured raw XML");
    assert!(raw_xml.contains("Captured Shape Text"));
    assert!(raw_xml.contains("Captured Cell Text"));
}

#[test]
fn parses_unknown_dash_defaults_through_public_parser() {
    let slide = r#"
      <p:sp>
        <p:nvSpPr><p:cNvPr id="180" name="Unknown Dash Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:ln w="12700"><a:prstDash val="mysteryDash"/></a:ln>
        </p:spPr>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="181" name="Unknown Dash Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody><a:bodyPr/><a:p><a:r><a:t>Dash</a:t></a:r></a:p></a:txBody>
                  <a:tcPr><a:lnL w="12700"><a:prstDash val="mysteryDash"></a:prstDash></a:lnL></a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];
    let shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Unknown Dash Shape")
        .expect("shape with unknown dash");
    assert!(matches!(shape.border.dash_style, DashStyle::Solid));
    assert!(matches!(shape.border.style, BorderStyle::Solid));

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    assert!(matches!(
        table.rows[0].cells[0].border_left.dash_style,
        DashStyle::Solid
    ));
    assert!(matches!(
        table.rows[0].cells[0].border_left.style,
        BorderStyle::Solid
    ));
}

#[test]
fn parses_start_tag_cell_hyperlink_defrpr_and_nonimage_chart_preview_through_public_parser() {
    let slide = r#"
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="190" name="Non Image Preview Chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <chart r:id="rIdChart"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="191" name="DefRPr Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:p>
            <a:defRPr sz="2000"><a:srgbClr val="112233"></a:srgbClr></a:defRPr>
            <a:r><a:t>Shape</a:t></a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="192" name="Start Hyperlink Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody>
                    <a:bodyPr/>
                    <a:p>
                      <a:defRPr sz="1800"><a:srgbClr val="445566"></a:srgbClr></a:defRPr>
                      <a:r>
                        <a:rPr><a:hlinkClick r:id="rIdCell"></a:hlinkClick></a:rPr>
                        <a:t>Cell Link</a:t>
                      </a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr/>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;
    let chart_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:pieChart>
        <c:ser>
          <c:idx val="0"/><c:order val="0"/>
          <c:cat><c:strLit><c:ptCount val="1"/><c:pt idx="0"><c:v>A</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val>
        </c:ser>
      </c:pieChart>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#;
    let chart_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdWorkbook" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/package" Target="../embeddings/workbook.bin"/>
</Relationships>"#;
    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdChart" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart-nonimage.xml"/>
  <Relationship Id="rIdCell" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/cell-start" TargetMode="External"/>
</Relationships>"#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_slide_rels(slide_rels)
        .with_extra_file("ppt/charts/chart-nonimage.xml", chart_xml.as_bytes())
        .with_extra_file(
            "ppt/charts/_rels/chart-nonimage.xml.rels",
            chart_rels.as_bytes(),
        )
        .with_extra_file("ppt/embeddings/workbook.bin", b"not-an-image")
        .build();
    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];

    let chart = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Chart(chart) => Some(chart),
            _ => None,
        })
        .expect("chart shape");
    assert!(chart.direct_spec.is_some());
    assert!(chart.preview_image.is_none());

    let shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "DefRPr Shape")
        .expect("defRPr shape");
    assert_eq!(
        shape.text_body.as_ref().expect("shape text").paragraphs[0]
            .def_rpr
            .as_ref()
            .and_then(|def_rpr| def_rpr.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#112233")
    );

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    let para = &table.rows[0].cells[0]
        .text_body
        .as_ref()
        .expect("cell text")
        .paragraphs[0];
    assert_eq!(
        para.def_rpr
            .as_ref()
            .and_then(|def_rpr| def_rpr.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#445566")
    );
    assert_eq!(
        para.runs[0].hyperlink.as_deref(),
        Some("https://example.com/cell-start")
    );
}

#[test]
fn parses_start_tag_color_context_matrix_through_public_parser() {
    let slide = r#"
      <p:bg>
        <p:bgPr>
          <a:solidFill><a:srgbClr val="ABCDEF"></a:srgbClr></a:solidFill>
        </p:bgPr>
      </p:bg>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="70" name="Start Tag Color Shape"></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:effectLst><a:glow rad="6350"><a:prstClr val="orange"></a:prstClr></a:glow></a:effectLst>
        </p:spPr>
        <p:style>
          <a:lnRef idx="1"><a:srgbClr val="111111"></a:srgbClr></a:lnRef>
          <a:fillRef idx="2"><a:schemeClr val="accent2"></a:schemeClr></a:fillRef>
          <a:effectRef idx="3"><a:sysClr lastClr="112233"></a:sysClr></a:effectRef>
          <a:fontRef idx="minor"><a:schemeClr val="accent4"></a:schemeClr></a:fontRef>
        </p:style>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle>
            <a:lvl1pPr>
              <a:spcBef><a:spcPct val="25000"/></a:spcBef>
              <a:lnSpc><a:spcPts val="900"/></a:lnSpc>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p>
            <a:pPr algn="ctr">
              <a:spcAft><a:spcPts val="1400"/></a:spcAft>
              <a:buFont typeface="Wingdings"/>
              <a:buSzPts val="1800"/>
              <a:buClr><a:schemeClr val="accent4"></a:schemeClr></a:buClr>
              <a:buChar char="•"/>
            </a:pPr>
            <a:r>
              <a:rPr sz="1800">
                <a:sysClr></a:sysClr>
                <a:highlight><a:prstClr val="orange"></a:prstClr></a:highlight>
                <a:effectLst><a:outerShdw blurRad="12700" dist="25400" dir="5400000"><a:schemeClr val="accent1"></a:schemeClr></a:outerShdw></a:effectLst>
                <a:hlinkClick r:id="rIdRun"></a:hlinkClick>
                <a:srgbClr val="445566"></a:srgbClr>
              </a:rPr>
              <a:t>Color Shape</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="71" name="Start Tag Color Table"></p:cNvPr><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1"/>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr algn="l">
                        <a:buFont typeface="Symbol"/>
                        <a:buSzPct val="125000"/>
                        <a:buClr><a:srgbClr val="334455"></a:srgbClr></a:buClr>
                        <a:buAutoNum type="alphaLcParenR" startAt="2"/>
                      </a:pPr>
                      <a:r>
                        <a:rPr sz="1700">
                          <a:highlight><a:srgbClr val="FFFF00"></a:srgbClr></a:highlight>
                          <a:schemeClr val="accent2"></a:schemeClr>
                        </a:rPr>
                        <a:t>Cell One</a:t>
                      </a:r>
                      <a:r>
                        <a:rPr><a:sysClr val="windowText"></a:sysClr></a:rPr>
                        <a:t>Cell Two</a:t>
                      </a:r>
                    </a:p>
                    <a:p>
                      <a:pPr><a:buNone/></a:pPr>
                      <a:r><a:t>No Bullet</a:t></a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr anchor="ctr">
                    <a:solidFill><a:schemeClr val="accent5"></a:schemeClr></a:solidFill>
                  </a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_slide_rels(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdRun" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/start-tag" TargetMode="External"/>
</Relationships>"#,
        )
        .build();

    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];
    assert!(matches!(
        &slide.background,
        Some(Fill::Solid(fill))
            if matches!(fill.color.kind, ColorKind::Rgb(ref rgb) if rgb == "ABCDEF")
    ));

    let shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Start Tag Color Shape")
        .expect("start-tag color shape");
    let style_ref = shape.style_ref.as_ref().expect("shape style ref");
    assert!(matches!(
        style_ref
            .ln_ref
            .as_ref()
            .map(|style| style.color.kind.clone()),
        Some(ColorKind::Rgb(rgb)) if rgb == "111111"
    ));
    assert!(matches!(
        style_ref
            .fill_ref
            .as_ref()
            .map(|style| style.color.kind.clone()),
        Some(ColorKind::Theme(name)) if name == "accent2"
    ));
    assert!(matches!(
        style_ref
            .effect_ref
            .as_ref()
            .map(|style| style.color.kind.clone()),
        Some(ColorKind::Rgb(rgb)) if rgb == "112233"
    ));
    assert!(matches!(
        style_ref
            .font_ref
            .as_ref()
            .map(|style| style.color.kind.clone()),
        Some(ColorKind::Theme(name)) if name == "accent4"
    ));
    assert!(shape.effects.glow.is_some());
    let list_style = shape
        .text_body
        .as_ref()
        .expect("shape text body")
        .list_style
        .as_ref()
        .expect("shape list style");
    let level = list_style.levels[0].as_ref().expect("shape level 1");
    assert!(matches!(
        level.space_before,
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 0.25).abs() < 1e-6
    ));
    assert!(matches!(
        level.line_spacing,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 9.0).abs() < 1e-6
    ));
    let para = &shape
        .text_body
        .as_ref()
        .expect("shape text body")
        .paragraphs[0];
    assert!(matches!(
        para.space_after,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 14.0).abs() < 1e-6
    ));
    assert!(matches!(
        &para.bullet,
        Some(Bullet::Char(bullet))
            if bullet.char == "•"
                && bullet.font.as_deref() == Some("Wingdings")
                && bullet.size_pct.is_some_and(|v| (v + 18.0).abs() < 1e-6)
                && bullet.color.as_ref().and_then(|c| c.to_css()).as_deref() == Some("#FFC000")
    ));
    let run = &para.runs[0];
    assert_eq!(
        run.hyperlink.as_deref(),
        Some("https://example.com/start-tag")
    );
    assert_eq!(run.style.color.to_css().as_deref(), Some("#445566"));
    assert_eq!(
        run.style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#FFA500")
    );
    assert_eq!(
        run.style
            .shadow
            .as_ref()
            .and_then(|s| s.color.to_css())
            .as_deref(),
        Some("#4472C4")
    );

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    let cell = &table.rows[0].cells[0];
    assert!(matches!(
        &cell.fill,
        Fill::Solid(fill) if fill.color.to_css().as_deref() == Some("#5B9BD5")
    ));
    let table_para = &cell.text_body.as_ref().expect("cell text body").paragraphs[0];
    assert!(matches!(
        &table_para.bullet,
        Some(Bullet::AutoNum(bullet))
            if bullet.num_type == "alphaLcParenR"
                && bullet.start_at == Some(2)
                && bullet.font.as_deref() == Some("Symbol")
                && bullet.size_pct.is_some_and(|v| (v - 1.25).abs() < 1e-6)
                && bullet.color.as_ref().and_then(|c| c.to_css()).as_deref() == Some("#334455")
    ));
    assert_eq!(
        table_para.runs[0].style.color.to_css().as_deref(),
        Some("#ED7D31")
    );
    assert_eq!(
        table_para.runs[0]
            .style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#FFFF00")
    );
    assert_eq!(
        table_para.runs[1].style.color.to_css().as_deref(),
        Some("#000000")
    );
    assert!(matches!(
        &cell.text_body.as_ref().expect("cell text body").paragraphs[1].bullet,
        Some(Bullet::None)
    ));
}

#[test]
fn parses_empty_tag_color_context_matrix_through_public_parser() {
    let slide = r#"
      <p:bg>
        <p:bgPr>
          <a:solidFill><a:schemeClr val="accent2"/></a:solidFill>
        </p:bgPr>
      </p:bg>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="80" name="Empty Tag Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:effectLst>
            <a:outerShdw blurRad="12700" dist="25400" dir="5400000"><a:schemeClr val="accent1"/></a:outerShdw>
            <a:glow rad="6350"><a:prstClr val="orange"/></a:glow>
          </a:effectLst>
          <a:custGeom><a:pathLst><a:path w="100000" h="100000"/></a:pathLst></a:custGeom>
        </p:spPr>
        <p:style>
          <a:lnRef idx="1"><a:srgbClr val="111111"/></a:lnRef>
          <a:fillRef idx="2"><a:schemeClr val="accent2"/></a:fillRef>
          <a:effectRef idx="3"><a:prstClr val="orange"/></a:effectRef>
          <a:fontRef idx="minor"><a:sysClr lastClr="123456"/></a:fontRef>
        </p:style>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle>
            <a:lvl1pPr>
              <a:spcBef><a:spcPct val="25000"/></a:spcBef>
              <a:lnSpc><a:spcPts val="900"/></a:lnSpc>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p>
            <a:pPr>
              <a:spcAft><a:spcPts val="1400"/></a:spcAft>
              <a:buFont typeface="Wingdings"/>
              <a:buSzPts val="1800"/>
              <a:buClr><a:prstClr val="orange"/></a:buClr>
              <a:buAutoNum type="arabicParenR" startAt="3"/>
            </a:pPr>
            <a:r>
              <a:rPr>
                <a:effectLst><a:outerShdw blurRad="12700" dist="25400" dir="5400000"><a:schemeClr val="accent1"/></a:outerShdw></a:effectLst>
                <a:hlinkClick r:id="rIdRun"/>
                <a:highlight><a:srgbClr val="FFA500"/></a:highlight>
                <a:sysClr/>
                <a:srgbClr val="445566"/>
              </a:rPr>
              <a:t>Empty Auto</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="81" name="Empty Tag Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1"/>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr>
                        <a:buFont typeface="Symbol"/>
                        <a:buSzPct val="125000"/>
                        <a:buClr><a:srgbClr val="334455"/></a:buClr>
                        <a:buAutoNum type="alphaLcParenR" startAt="2"/>
                      </a:pPr>
                      <a:r>
                        <a:rPr>
                          <a:highlight><a:srgbClr val="FFFF00"/></a:highlight>
                          <a:schemeClr val="accent2"/></a:rPr>
                        <a:t>Cell One</a:t>
                      </a:r>
                      <a:r>
                        <a:rPr><a:sysClr val="windowText"/></a:rPr>
                        <a:t>Cell Two</a:t>
                      </a:r>
                    </a:p>
                    <a:p>
                      <a:pPr><a:buNone/></a:pPr>
                      <a:r><a:t>No Bullet</a:t></a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr anchor="ctr">
                    <a:solidFill><a:sysClr lastClr="ABCDEF"/></a:solidFill>
                  </a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_slide_rels(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdRun" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/empty-tag" TargetMode="External"/>
</Relationships>"#,
        )
        .build();

    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];
    assert!(matches!(
        &slide.background,
        Some(Fill::Solid(fill)) if fill.color.to_css().as_deref() == Some("#ED7D31")
    ));

    let shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Empty Tag Shape")
        .expect("empty-tag shape");
    let style_ref = shape.style_ref.as_ref().expect("shape style ref");
    assert!(matches!(
        style_ref.ln_ref.as_ref().map(|style| style.color.to_css()),
        Some(Some(color)) if color == "#111111"
    ));
    assert!(matches!(
        style_ref.fill_ref.as_ref().map(|style| style.color.to_css()),
        Some(Some(color)) if color == "#ED7D31"
    ));
    assert!(matches!(
        style_ref.effect_ref.as_ref().map(|style| style.color.to_css()),
        Some(Some(color)) if color == "#FFA500"
    ));
    assert!(matches!(
        style_ref.font_ref.as_ref().map(|style| style.color.to_css()),
        Some(Some(color)) if color == "#123456"
    ));
    assert!(shape.effects.outer_shadow.is_some());
    assert!(shape.effects.glow.is_some());
    let custom_geom = match &shape.shape_type {
        ShapeType::CustomGeom(geom) => geom,
        other => panic!("expected custom geometry, got {other:?}"),
    };
    assert_eq!(custom_geom.paths.len(), 1);
    assert!(matches!(custom_geom.paths[0].fill, PathFill::Norm));
    let list_level = shape
        .text_body
        .as_ref()
        .expect("shape text body")
        .list_style
        .as_ref()
        .and_then(|style| style.levels[0].as_ref())
        .expect("shape list level");
    assert!(matches!(
        list_level.space_before,
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 0.25).abs() < 1e-6
    ));
    assert!(matches!(
        list_level.line_spacing,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 9.0).abs() < 1e-6
    ));
    let para = &shape
        .text_body
        .as_ref()
        .expect("shape text body")
        .paragraphs[0];
    assert!(matches!(
        para.space_after,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 14.0).abs() < 1e-6
    ));
    assert!(matches!(
        &para.bullet,
        Some(Bullet::AutoNum(bullet))
            if bullet.num_type == "arabicParenR"
                && bullet.start_at == Some(3)
                && bullet.font.as_deref() == Some("Wingdings")
                && bullet.size_pct.is_some_and(|v| (v + 18.0).abs() < 1e-6)
                && bullet.color.as_ref().and_then(|c| c.to_css()).as_deref() == Some("#FFA500")
    ));
    let run = &para.runs[0];
    assert_eq!(
        run.hyperlink.as_deref(),
        Some("https://example.com/empty-tag")
    );
    assert_eq!(run.style.color.to_css().as_deref(), Some("#445566"));
    assert_eq!(
        run.style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#FFA500")
    );
    assert_eq!(
        run.style
            .shadow
            .as_ref()
            .and_then(|s| s.color.to_css())
            .as_deref(),
        Some("#4472C4")
    );

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    let cell = &table.rows[0].cells[0];
    assert!(matches!(
        &cell.fill,
        Fill::Solid(fill) if fill.color.to_css().as_deref() == Some("#ABCDEF")
    ));
    let cell_para = &cell.text_body.as_ref().expect("cell text body").paragraphs[0];
    assert!(matches!(
        &cell_para.bullet,
        Some(Bullet::AutoNum(bullet))
            if bullet.num_type == "alphaLcParenR"
                && bullet.start_at == Some(2)
                && bullet.font.as_deref() == Some("Symbol")
                && bullet.size_pct.is_some_and(|v| (v - 1.25).abs() < 1e-6)
                && bullet.color.as_ref().and_then(|c| c.to_css()).as_deref() == Some("#334455")
    ));
    assert_eq!(
        cell_para.runs[0].style.color.to_css().as_deref(),
        Some("#ED7D31")
    );
    assert_eq!(
        cell_para.runs[0]
            .style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#FFFF00")
    );
    assert_eq!(
        cell_para.runs[1].style.color.to_css().as_deref(),
        Some("#000000")
    );
    assert!(matches!(
        &cell.text_body.as_ref().expect("cell text body").paragraphs[1].bullet,
        Some(Bullet::None)
    ));
}

#[test]
fn parses_empty_event_cell_and_shape_dispatch_matrix_through_public_parser() {
    let slide = r#"
      <p:bg>
        <p:bgPr>
          <a:solidFill><a:sysClr lastClr="A1B2C3"/></a:solidFill>
        </p:bgPr>
      </p:bg>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="90" name="Dispatch Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:effectLst><a:glow rad="6350"><a:prstClr val="orange"/></a:glow></a:effectLst>
          <a:custGeom><a:pathLst><a:path w="100000" h="100000"/></a:pathLst></a:custGeom>
        </p:spPr>
        <p:style>
          <a:lnRef idx="1"><a:srgbClr val="111111"/></a:lnRef>
          <a:fillRef idx="2"><a:schemeClr val="accent2"/></a:fillRef>
          <a:effectRef idx="3"><a:prstClr val="orange"/></a:effectRef>
          <a:fontRef idx="minor"><a:sysClr lastClr="123456"/></a:fontRef>
        </p:style>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle>
            <a:lvl1pPr>
              <a:spcBef><a:spcPct val="25000"/></a:spcBef>
              <a:lnSpc><a:spcPts val="900"/></a:lnSpc>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p>
            <a:pPr><a:spcAft><a:spcPts val="1400"/></a:spcAft></a:pPr>
            <a:r><a:rPr><a:srgbClr val="445566"/></a:rPr><a:t>Shape Run</a:t></a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="91" name="Dispatch Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1"/>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr>
                        <a:defRPr><a:schemeClr val="accent3"></a:schemeClr></a:defRPr>
                        <a:buClr><a:srgbClr val="334455"/></a:buClr>
                        <a:buFont typeface="Symbol"/>
                        <a:buSzPct val="125000"/>
                        <a:buAutoNum type="alphaLcParenR" startAt="2"/>
                      </a:pPr>
                      <a:r><a:rPr><a:srgbClr val="224466"/></a:rPr><a:t>RGB</a:t></a:r>
                      <a:r><a:rPr><a:schemeClr val="accent2"/></a:rPr><a:t>Theme</a:t></a:r>
                      <a:r><a:rPr><a:sysClr val="windowText"/></a:rPr><a:t>System</a:t></a:r>
                      <a:r>
                        <a:rPr>
                          <a:effectLst><a:outerShdw blurRad="12700" dist="25400" dir="5400000"><a:schemeClr val="accent1"/></a:outerShdw></a:effectLst>
                          <a:highlight><a:srgbClr val="FFFF00"/></a:highlight>
                        </a:rPr>
                        <a:t>FX</a:t>
                      </a:r>
                    </a:p>
                    <a:p>
                      <a:pPr><a:buClr><a:sysClr lastClr="556677"/></a:buClr><a:buChar char="•"/></a:pPr>
                      <a:r><a:t>Bullet Char</a:t></a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr anchor="ctr"/>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];

    assert!(matches!(
        &slide.background,
        Some(Fill::Solid(fill)) if fill.color.to_css().as_deref() == Some("#A1B2C3")
    ));

    let shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Dispatch Shape")
        .expect("dispatch shape");
    let style_ref = shape.style_ref.as_ref().expect("shape style ref");
    assert!(matches!(
        style_ref
            .ln_ref
            .as_ref()
            .and_then(|style| style.color.to_css())
            .as_deref(),
        Some("#111111")
    ));
    assert!(matches!(
        style_ref
            .fill_ref
            .as_ref()
            .and_then(|style| style.color.to_css())
            .as_deref(),
        Some("#ED7D31")
    ));
    assert!(matches!(
        style_ref
            .effect_ref
            .as_ref()
            .and_then(|style| style.color.to_css())
            .as_deref(),
        Some("#FFA500")
    ));
    assert!(matches!(
        style_ref
            .font_ref
            .as_ref()
            .and_then(|style| style.color.to_css())
            .as_deref(),
        Some("#123456")
    ));
    assert!(shape.effects.glow.is_some());
    let custom_geom = match &shape.shape_type {
        ShapeType::CustomGeom(geom) => geom,
        other => panic!("expected custom geometry, got {other:?}"),
    };
    assert_eq!(custom_geom.paths.len(), 1);
    assert!(matches!(custom_geom.paths[0].fill, PathFill::Norm));
    let list_level = shape
        .text_body
        .as_ref()
        .expect("shape text body")
        .list_style
        .as_ref()
        .and_then(|style| style.levels[0].as_ref())
        .expect("shape list level");
    assert!(matches!(
        list_level.space_before,
        Some(pptx2html_core::model::SpacingValue::Percent(v)) if (v - 0.25).abs() < 1e-6
    ));
    assert!(matches!(
        list_level.line_spacing,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 9.0).abs() < 1e-6
    ));
    let shape_para = &shape
        .text_body
        .as_ref()
        .expect("shape text body")
        .paragraphs[0];
    assert!(matches!(
        shape_para.space_after,
        Some(pptx2html_core::model::SpacingValue::Points(v)) if (v - 14.0).abs() < 1e-6
    ));
    assert_eq!(
        shape_para.runs[0].style.color.to_css().as_deref(),
        Some("#445566")
    );

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    let cell = &table.rows[0].cells[0];
    let para = &cell.text_body.as_ref().expect("cell text body").paragraphs[0];
    assert!(matches!(
        &para.bullet,
        Some(Bullet::AutoNum(bullet))
            if bullet.num_type == "alphaLcParenR"
                && bullet.start_at == Some(2)
                && bullet.font.as_deref() == Some("Symbol")
                && bullet.size_pct.is_some_and(|v| (v - 1.25).abs() < 1e-6)
                && bullet.color.as_ref().and_then(|c| c.to_css()).as_deref() == Some("#334455")
    ));
    assert_eq!(
        para.runs[0].style.color.to_css().as_deref(),
        Some("#224466")
    );
    assert_eq!(
        para.runs[1].style.color.to_css().as_deref(),
        Some("#ED7D31")
    );
    assert_eq!(
        para.runs[2].style.color.to_css().as_deref(),
        Some("#000000")
    );
    assert_eq!(
        para.runs[3]
            .style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#FFFF00")
    );
    assert_eq!(
        para.runs[3]
            .style
            .shadow
            .as_ref()
            .and_then(|s| s.color.to_css())
            .as_deref(),
        Some("#4472C4")
    );
    let second_para = &cell.text_body.as_ref().expect("cell text body").paragraphs[1];
    assert!(matches!(
        &second_para.bullet,
        Some(Bullet::Char(bullet))
            if bullet.char == "•"
                && bullet.color.as_ref().and_then(|c| c.to_css()).as_deref() == Some("#556677")
    ));
}

#[test]
fn parses_cell_run_end_handler_color_paths_through_public_parser() {
    let slide = r#"
      <p:bg>
        <p:bgPr>
          <a:solidFill><a:srgbClr val="13579B"/></a:solidFill>
        </p:bgPr>
      </p:bg>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="100" name="Effect Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:effectLst><a:glow rad="6350"><a:prstClr val="orange"/></a:glow></a:effectLst>
        </p:spPr>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="101" name="End Handler Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1"/>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr>
                        <a:defRPr sz="1900"><a:schemeClr val="accent3"></a:schemeClr></a:defRPr>
                        <a:buClr><a:sysClr val="windowText"/></a:buClr>
                        <a:buFont typeface="Symbol"/>
                        <a:buSzPct val="125000"/>
                        <a:buAutoNum type="alphaLcParenR" startAt="2"/>
                      </a:pPr>
                      <a:r>
                        <a:rPr>
                          <a:highlight><a:srgbClr val="FFFF00"></a:srgbClr></a:highlight>
                          <a:effectLst><a:outerShdw blurRad="12700" dist="25400" dir="5400000"><a:schemeClr val="accent1"></a:schemeClr></a:outerShdw></a:effectLst>
                          <a:srgbClr val="224466"/>
                        </a:rPr>
                        <a:t>First</a:t>
                      </a:r>
                      <a:r>
                        <a:rPr><a:sysClr lastClr="ABCDEF"/></a:rPr>
                        <a:t>Second</a:t>
                      </a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr anchor="ctr"/>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];
    assert!(matches!(
        &slide.background,
        Some(Fill::Solid(fill)) if fill.color.to_css().as_deref() == Some("#13579B")
    ));

    let effect_shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Effect Shape")
        .expect("effect shape");
    assert!(effect_shape.effects.glow.is_some());

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    let para = &table.rows[0].cells[0]
        .text_body
        .as_ref()
        .expect("cell text body")
        .paragraphs[0];
    assert!(matches!(
        &para.bullet,
        Some(Bullet::AutoNum(bullet))
            if bullet.num_type == "alphaLcParenR"
                && bullet.start_at == Some(2)
                && bullet.font.as_deref() == Some("Symbol")
                && bullet.size_pct.is_some_and(|v| (v - 1.25).abs() < 1e-6)
                && bullet.color.as_ref().and_then(|c| c.to_css()).as_deref() == Some("#000000")
    ));
    let first = &para.runs[0];
    assert_eq!(first.style.color.to_css().as_deref(), Some("#224466"));
    assert_eq!(
        first
            .style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#FFFF00")
    );
    assert_eq!(
        first
            .style
            .shadow
            .as_ref()
            .and_then(|s| s.color.to_css())
            .as_deref(),
        Some("#4472C4")
    );
    let second = &para.runs[1];
    assert_eq!(second.style.color.to_css().as_deref(), Some("#ABCDEF"));
}

#[test]
fn parses_custom_geometry_invalid_formula_matrix_through_public_parser() {
    let slide = r#"
      <p:sp>
        <p:nvSpPr><p:cNvPr id="120" name="Formula Matrix"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:custGeom>
            <a:avLst>
              <a:gd name="g0" fmla=""/>
            </a:avLst>
            <a:gdLst>
              <a:gd name="g1" fmla="+- 1 2"/>
              <a:gd name="g2" fmla="*/ 1 2 0"/>
              <a:gd name="g3" fmla="+/ 1 2 0"/>
              <a:gd name="g4" fmla="pin 1 2"/>
              <a:gd name="g5" fmla="min 1"/>
              <a:gd name="g6" fmla="max 1"/>
              <a:gd name="g7" fmla="?: 1 2"/>
              <a:gd name="g8" fmla="abs"/>
              <a:gd name="g9" fmla="sqrt"/>
              <a:gd name="g10" fmla="mod 1 2"/>
              <a:gd name="g11" fmla="sin 1"/>
              <a:gd name="g12" fmla="cos 1"/>
              <a:gd name="g13" fmla="cat2 1 2"/>
              <a:gd name="g14" fmla="sat2 1 2"/>
              <a:gd name="g15" fmla="at2 1"/>
              <a:gd name="g16" fmla="tan 1"/>
              <a:gd name="g17" fmla="mystery 1 2 3"/>
            </a:gdLst>
            <a:pathLst>
              <a:path w="100000" h="100000" fill="none">
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
                <a:lnTo><a:pt x="100000" y="0"/></a:lnTo>
                <a:close/>
              </a:path>
            </a:pathLst>
          </a:custGeom>
        </p:spPr>
      </p:sp>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let presentation = parse_pptx(&pptx);
    let shape = presentation.slides[0]
        .shapes
        .iter()
        .find(|shape| shape.name == "Formula Matrix")
        .expect("formula matrix shape");
    let custom_geom = match &shape.shape_type {
        ShapeType::CustomGeom(geom) => geom,
        other => panic!("expected custom geometry, got {other:?}"),
    };
    assert_eq!(custom_geom.paths.len(), 1);
    assert!(matches!(custom_geom.paths[0].fill, PathFill::None));
}

#[test]
fn parses_formula_short_arity_and_default_line_end_sizes_through_public_parser() {
    let slide = r#"
      <p:sp>
        <p:nvSpPr><p:cNvPr id="130" name="Short Formula Matrix"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:custGeom>
            <a:gdLst>
              <a:gd name="g1" fmla="*/ 1 2"/>
              <a:gd name="g2" fmla="+/ 1 2"/>
            </a:gdLst>
            <a:pathLst>
              <a:path w="100000" h="100000" fill="none">
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
                <a:lnTo><a:pt x="100000" y="0"/></a:lnTo>
                <a:close/>
              </a:path>
            </a:pathLst>
          </a:custGeom>
        </p:spPr>
      </p:sp>
      <p:cxnSp>
        <p:nvCxnSpPr>
          <p:cNvPr id="131" name="Default End Sizes"/>
          <p:cNvCxnSpPr/>
          <p:nvPr/>
        </p:nvCxnSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="0"/></a:xfrm>
          <a:ln>
            <a:headEnd type="triangle"/>
            <a:tailEnd type="diamond"/>
          </a:ln>
        </p:spPr>
      </p:cxnSp>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let presentation = parse_pptx(&pptx);
    let shapes = &presentation.slides[0].shapes;

    let custom_geom = shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::CustomGeom(geom) => Some(geom),
            _ => None,
        })
        .expect("custom geometry");
    assert_eq!(custom_geom.paths.len(), 1);
    assert!(matches!(custom_geom.paths[0].fill, PathFill::None));

    let connector = shapes
        .iter()
        .find(|shape| shape.name == "Default End Sizes")
        .expect("connector");
    assert!(matches!(
        connector
            .border
            .head_end
            .as_ref()
            .map(|end| end.width.clone()),
        Some(LineEndSize::Medium)
    ));
    assert!(matches!(
        connector
            .border
            .tail_end
            .as_ref()
            .map(|end| end.length.clone()),
        Some(LineEndSize::Medium)
    ));
}

#[test]
fn parses_end_handler_color_handoff_matrix_through_public_parser() {
    let slide = r#"
      <p:sp>
        <p:nvSpPr><p:cNvPr id="150" name="Shape Glow"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:effectLst>
            <a:glow rad="6350">
              <a:schemeClr val="accent1"><a:alpha val="50000"/></a:schemeClr>
            </a:glow>
          </a:effectLst>
        </p:spPr>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="151" name="Color Handoff Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1"/>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr>
                        <a:defRPr sz="1900"><a:schemeClr val="accent3"/></a:defRPr>
                      </a:pPr>
                      <a:r>
                        <a:rPr>
                          <a:highlight><a:srgbClr val="FFFF00"/></a:highlight>
                          <a:effectLst>
                            <a:outerShdw blurRad="12700" dist="25400" dir="5400000">
                              <a:schemeClr val="accent1"/>
                            </a:outerShdw>
                          </a:effectLst>
                        </a:rPr>
                        <a:t>Color Handoff</a:t>
                      </a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr anchor="ctr"/>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];

    let shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Shape Glow")
        .expect("shape glow");
    assert_eq!(
        shape
            .effects
            .glow
            .as_ref()
            .and_then(|glow| glow.color.to_css())
            .as_deref(),
        Some("#4472C4")
    );

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    let para = &table.rows[0].cells[0]
        .text_body
        .as_ref()
        .expect("cell text body")
        .paragraphs[0];
    let run = &para.runs[0];
    assert_eq!(
        run.style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#FFFF00")
    );
    assert_eq!(
        run.style
            .shadow
            .as_ref()
            .and_then(|s| s.color.to_css())
            .as_deref(),
        Some("#4472C4")
    );
}

#[test]
fn parses_empty_event_color_dispatch_contexts_through_public_parser() {
    let slide = r#"
      <p:bg>
        <p:bgPr>
          <a:solidFill><a:schemeClr val="accent2"/></a:solidFill>
        </p:bgPr>
      </p:bg>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="140" name="Effect Dispatch"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:effectLst><a:glow rad="6350"><a:prstClr val="orange"/></a:glow></a:effectLst>
        </p:spPr>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="141" name="Color Dispatch Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1"/>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr>
                        <a:buClr><a:sysClr val="windowText"/></a:buClr>
                        <a:buChar char="•"/>
                      </a:pPr>
                      <a:r><a:rPr><a:srgbClr val="224466"/></a:rPr><a:t>RGB</a:t></a:r>
                      <a:r><a:rPr><a:schemeClr val="accent2"/></a:rPr><a:t>Theme</a:t></a:r>
                      <a:r><a:rPr><a:sysClr lastClr="ABCDEF"/></a:rPr><a:t>System</a:t></a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr anchor="ctr"/>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];

    assert!(matches!(
        &slide.background,
        Some(Fill::Solid(fill)) if fill.color.to_css().as_deref() == Some("#ED7D31")
    ));

    let effect_shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Effect Dispatch")
        .expect("effect dispatch shape");
    assert_eq!(
        effect_shape
            .effects
            .glow
            .as_ref()
            .and_then(|glow| glow.color.to_css())
            .as_deref(),
        Some("#FFA500")
    );

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    let para = &table.rows[0].cells[0]
        .text_body
        .as_ref()
        .expect("cell text body")
        .paragraphs[0];
    assert!(matches!(
        &para.bullet,
        Some(Bullet::Char(bullet))
            if bullet.char == "•"
                && bullet.color.as_ref().and_then(|c| c.to_css()).as_deref() == Some("#000000")
    ));
    assert_eq!(
        para.runs[0].style.color.to_css().as_deref(),
        Some("#224466")
    );
    assert_eq!(
        para.runs[1].style.color.to_css().as_deref(),
        Some("#ED7D31")
    );
    assert_eq!(
        para.runs[2].style.color.to_css().as_deref(),
        Some("#ABCDEF")
    );
}

#[test]
fn parses_self_closing_srgb_dispatch_matrix_through_public_parser() {
    let slide = r#"
      <p:bg>
        <p:bgPr>
          <a:solidFill><a:srgbClr val="13579B"/></a:solidFill>
        </p:bgPr>
      </p:bg>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="110" name="Self Closing Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:effectLst><a:glow rad="6350"><a:prstClr val="orange"/></a:glow></a:effectLst>
        </p:spPr>
        <p:style>
          <a:lnRef idx="1"><a:srgbClr val="111111"/></a:lnRef>
        </p:style>
        <p:txBody><a:bodyPr/><a:p><a:r><a:t>Shape</a:t></a:r></a:p></p:txBody>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="111" name="SRGB Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1"/>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr>
                        <a:buClr><a:srgbClr val="334455"/></a:buClr>
                        <a:buChar char="•"/>
                      </a:pPr>
                      <a:r><a:rPr><a:srgbClr val="224466"/></a:rPr><a:t>Cell</a:t></a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr anchor="ctr"/>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];

    assert!(matches!(
        &slide.background,
        Some(Fill::Solid(fill)) if fill.color.to_css().as_deref() == Some("#13579B")
    ));

    let shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Self Closing Shape")
        .expect("self-closing shape");
    assert!(matches!(
        shape
            .style_ref
            .as_ref()
            .and_then(|style| style.ln_ref.as_ref())
            .and_then(|style| style.color.to_css())
            .as_deref(),
        Some("#111111")
    ));
    assert_eq!(
        shape
            .effects
            .glow
            .as_ref()
            .and_then(|glow| glow.color.to_css())
            .as_deref(),
        Some("#FFA500")
    );

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    let para = &table.rows[0].cells[0]
        .text_body
        .as_ref()
        .expect("cell text body")
        .paragraphs[0];
    assert!(matches!(
        &para.bullet,
        Some(Bullet::Char(bullet))
            if bullet.char == "•"
                && bullet.color.as_ref().and_then(|c| c.to_css()).as_deref() == Some("#334455")
    ));
    assert_eq!(
        para.runs[0].style.color.to_css().as_deref(),
        Some("#224466")
    );
}

#[test]
fn parses_shape_text_autofit_connector_and_ole_branches() {
    let slide = r#"
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="5" name="OLE"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/presentationml/2006/oleObject">
            <oleObject progId="Excel.Sheet">
              <oleData sheet="Budget"/>
            </oleObject>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="6" name="Styled Shape"/>
          <p:cNvSpPr/>
          <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm rot="5400000" flipH="1" flipV="true">
            <a:off x="12700" y="25400"/>
            <a:ext cx="381000" cy="254000"/>
          </a:xfrm>
          <a:ln w="12700" cap="flat" cmpd="tri" algn="in">
            <a:prstDash val="sysDash"/>
            <a:solidFill><a:srgbClr val="AA00AA"/></a:solidFill>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr" anchorCtr="1" rot="5400000" vert="vert270"
                    lIns="91440" tIns="45720" rIns="182880" bIns="22860" wrap="none"/>
          <a:noAutofit/>
          <a:lstStyle>
            <a:lvl1pPr algn="r">
              <a:lnSpc><a:spcPct val="90000"/></a:lnSpc>
              <a:spcBef><a:spcPts val="600"/></a:spcBef>
              <a:spcAft><a:spcPts val="800"/></a:spcAft>
              <a:defRPr sz="1800" spc="200" baseline="30000" cap="small" u="dbl"
                        strike="sngStrike" b="1" i="1">
                <a:latin typeface="Calibri"/>
                <a:ea typeface="Yu Gothic"/>
                <a:cs typeface="Noto Sans Arabic"/>
                <a:schemeClr val="accent2"/>
              </a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p>
            <a:pPr algn="just" lvl="1" indent="91440" marL="45720">
              <a:lnSpc><a:spcPct val="110000"/></a:lnSpc>
              <a:spcBef><a:spcPts val="1200"/></a:spcBef>
              <a:spcAft><a:spcPts val="2400"/></a:spcAft>
              <a:buChar char="•"/>
              <a:buClr><a:srgbClr val="00FF00"/></a:buClr>
            </a:pPr>
            <a:defRPr sz="2000" cap="all" b="1" i="1" u="sng" strike="dblStrike"
                      baseline="20000" spc="100">
              <a:latin typeface="Calibri"/>
              <a:ea typeface="Meiryo"/>
              <a:cs typeface="Noto Sans Devanagari"/>
              <a:srgbClr val="123456"/>
            </a:defRPr>
            <a:r>
              <a:rPr sz="1600" cap="all" b="1" i="1" u="sng" strike="dblStrike"
                     baseline="10000" spc="50">
                <a:hlinkClick r:id="rIdLink"/>
                <a:srgbClr val="654321"/>
              </a:rPr>
              <a:t>Styled</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="7" name="Shrink Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="25400" y="38100"/><a:ext cx="254000" cy="127000"/></a:xfrm>
          <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
        </p:spPr>
        <p:txBody>
          <a:bodyPr wrap="none"/>
          <a:spAutoFit/>
          <a:p><a:r><a:t>Shrink</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
      <p:cxnSp>
        <p:nvCxnSpPr>
          <p:cNvPr id="8" name="Connector"/>
          <p:cNvCxnSpPr/>
          <p:nvPr/>
        </p:nvCxnSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="0"/></a:xfrm>
          <a:ln w="12700" cap="rnd" cmpd="dbl" algn="in">
            <a:prstDash val="dashDot"/>
            <a:headEnd type="triangle" w="lg" len="sm"/>
            <a:tailEnd type="oval" w="sm" len="lg"/>
            <a:schemeClr val="accent1"/>
          </a:ln>
        </p:spPr>
        <p:stCxn id="10" idx="1"/>
        <p:endCxn id="11" idx="2"/>
      </p:cxnSp>
    "#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/styled" TargetMode="External"/>
</Relationships>"#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_slide_rels(slide_rels)
        .build();

    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];

    let ole = slide
        .shapes
        .iter()
        .find(|shape| matches!(&shape.shape_type, ShapeType::Unsupported(_)))
        .expect("ole placeholder shape");
    assert!(matches!(
        &ole.shape_type,
        ShapeType::Unsupported(data)
            if data.raw_xml.as_deref().is_some_and(|raw| raw.contains("oleData"))
    ));

    let styled = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Styled Shape")
        .expect("styled shape");
    assert!(styled.flip_h && styled.flip_v);
    assert!(matches!(styled.border.cap, LineCap::Flat));
    assert!(matches!(styled.border.compound, CompoundLine::Triple));
    assert!(matches!(styled.border.alignment, LineAlignment::Inset));
    let text_body = styled.text_body.as_ref().expect("styled text body");
    assert!(matches!(text_body.vertical_align, VerticalAlign::Middle));
    assert!(matches!(text_body.auto_fit, AutoFit::NoAutoFit));
    assert_eq!(styled.vertical_text.as_deref(), Some("vert270"));
    let defaults = text_body
        .list_style
        .as_ref()
        .and_then(|ls| ls.levels[0].as_ref())
        .and_then(|pd| pd.def_run_props.as_ref())
        .expect("shape list-style defaults");
    assert_eq!(defaults.font_size, Some(18.0));
    assert_eq!(defaults.letter_spacing, Some(2.0));
    assert_eq!(defaults.baseline, Some(30000));
    assert_eq!(defaults.font_latin.as_deref(), Some("Calibri"));
    let para = &text_body.paragraphs[0];
    assert!(matches!(para.alignment, Alignment::Justify));
    assert_eq!(para.level, 1);
    assert!(matches!(
        &para.bullet,
        Some(Bullet::Char(bullet)) if bullet.char == "•"
    ));
    let run = &para.runs[0];
    assert_eq!(run.hyperlink.as_deref(), Some("https://example.com/styled"));
    assert_eq!(run.style.color.to_css().as_deref(), Some("#654321"));

    let shrink = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Shrink Shape")
        .expect("shrink shape");
    assert!(matches!(
        shrink.text_body.as_ref().map(|body| &body.auto_fit),
        Some(AutoFit::Shrink)
    ));

    let connector = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Connector")
        .expect("connector");
    assert!(matches!(&connector.shape_type, ShapeType::Custom(name) if name == "line"));
    assert_eq!(
        connector
            .start_connection
            .as_ref()
            .map(|connection| connection.shape_id),
        Some(10)
    );
    assert_eq!(
        connector
            .end_connection
            .as_ref()
            .map(|connection| connection.site_idx),
        Some(2)
    );
}

#[test]
fn parses_slide_table_effect_picture_and_empty_variant_branches() {
    let slide = r#"
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="210" name="Start Variant Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="true" bandCol="1" firstRow="true" lastRow="1" firstCol="1" lastCol="true"></a:tblPr>
              <a:tblGrid>
                <a:gridCol w="914400"></a:gridCol>
                <a:gridCol w="457200"/>
              </a:tblGrid>
              <a:tr h="457200">
                <a:tc rowSpan="2" gridSpan="2" vMerge="1">
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr algn="ctr" lvl="1">
                        <a:lnSpc><a:spcPct val="110000"></a:spcPct></a:lnSpc>
                        <a:spcBef><a:spcPts val="1200"></a:spcPts></a:spcBef>
                        <a:spcAft><a:spcPct val="50000"></a:spcPct></a:spcAft>
                      </a:pPr>
                      <a:defRPr sz="1800" spc="100" baseline="20000" cap="small" u="sng" strike="dblStrike" b="1" i="1"/>
                      <a:r>
                        <a:rPr>
                          <a:ea typeface="CellRun EA"></a:ea>
                          <a:cs typeface="CellRun CS"></a:cs>
                          <a:hlinkClick r:id="rIdCellLink"></a:hlinkClick>
                          <a:highlight><a:schemeClr val="accent6"></a:schemeClr></a:highlight>
                          <a:schemeClr val="accent2"></a:schemeClr>
                        </a:rPr>
                        <a:t>Cell Link</a:t>
                      </a:r>
                    </a:p>
                    <a:p>
                      <a:defRPr sz="1900" spc="150" baseline="10000" cap="all" u="dbl" strike="sngStrike" b="true" i="1">
                        <a:latin typeface="Cell Latin"></a:latin>
                        <a:ea typeface="Cell EA"></a:ea>
                        <a:cs typeface="Cell CS"></a:cs>
                        <a:srgbClr val="334455"></a:srgbClr>
                      </a:defRPr>
                      <a:r>
                        <a:rPr>
                          <a:highlight><a:srgbClr val="FFFF00"></a:srgbClr></a:highlight>
                          <a:prstClr val="orange"></a:prstClr>
                        </a:rPr>
                        <a:t>Cell Start</a:t>
                      </a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr marL="91440" marR="137160" marT="45720" marB="22860" anchor="ctr">
                    <a:lnL w="12700"><a:prstDash val="sysDash"></a:prstDash><a:schemeClr val="accent4"></a:schemeClr></a:lnL>
                    <a:lnR w="12700"><a:prstDash val="sysDot"></a:prstDash><a:srgbClr val="123456"></a:srgbClr></a:lnR>
                    <a:lnT w="12700"><a:prstDash val="lgDashDotDot"></a:prstDash><a:prstClr val="orange"></a:prstClr></a:lnT>
                    <a:lnB w="12700"><a:prstDash val="dash"></a:prstDash><a:sysClr lastClr="ABCDEF"></a:sysClr></a:lnB>
                  </a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:pic>
        <p:nvPicPr><p:cNvPr id="211" name="Picture Start"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
        <p:blipFill><a:blip r:embed="rIdPic"></a:blip></p:blipFill>
        <p:spPr>
          <a:xfrm rot="60000" flipH="1" flipV="true"/>
          <a:prstGeom prst="rect"/>
        </p:spPr>
      </p:pic>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="212" name="Effect Start Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm rot="120000" flipH="true" flipV="1"/>
          <a:prstGeom prst="rect"/>
          <a:ln w="12700" cap="flat" cmpd="thinThick" algn="in">
            <a:prstDash val="sysDashDotDot"/>
            <a:miter lim="300000"/>
            <a:headEnd type="triangle" w="sm" len="lg"/>
            <a:tailEnd type="oval" w="lg" len="sm"/>
            <a:schemeClr val="accent3"></a:schemeClr>
          </a:ln>
          <a:effectLst>
            <a:outerShdw blurRad="12700" dist="25400" dir="5400000"><a:schemeClr val="accent1"></a:schemeClr></a:outerShdw>
            <a:glow rad="6350"><a:prstClr val="orange"></a:prstClr></a:glow>
          </a:effectLst>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle>
            <a:lvl1pPr>
              <a:lnSpc><a:spcPct val="125000"></a:spcPct></a:lnSpc>
              <a:spcBef><a:spcPts val="200"></a:spcPts></a:spcBef>
              <a:spcAft><a:spcPct val="50000"></a:spcPct></a:spcAft>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p>
            <a:defRPr sz="2100" spc="125" baseline="15000" cap="small" u="sng" b="1" i="1"/>
            <a:r>
              <a:rPr>
                <a:ea typeface="Run EA"></a:ea>
                <a:cs typeface="Run CS"></a:cs>
                <a:hlinkClick r:id="rIdShapeLink"></a:hlinkClick>
                <a:highlight><a:srgbClr val="FFFF11"></a:srgbClr></a:highlight>
                <a:sysClr lastClr="112233"></a:sysClr>
              </a:rPr>
              <a:t>Shape Link</a:t>
            </a:r>
          </a:p>
          <a:p>
            <a:defRPr sz="2200">
              <a:latin typeface="Para Latin"></a:latin>
              <a:ea typeface="Para EA"></a:ea>
              <a:cs typeface="Para CS"></a:cs>
              <a:schemeClr val="accent5"></a:schemeClr>
            </a:defRPr>
            <a:r><a:t>Para Defaults</a:t></a:r>
          </a:p>
        </p:txBody>
      </p:sp>
    "#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdCellLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/cell-effect" TargetMode="External"/>
  <Relationship Id="rIdShapeLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/shape-effect" TargetMode="External"/>
  <Relationship Id="rIdPic" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/picture.png"/>
</Relationships>"#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_slide_rels(slide_rels)
        .with_extra_file("ppt/media/picture.png", b"picture-bytes")
        .build();

    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table shape");
    assert!(table.band_row);
    assert!(table.band_col);
    assert!(table.first_row);
    assert!(table.last_row);
    assert!(table.first_col);
    assert!(table.last_col);
    assert_eq!(table.col_widths.len(), 2);
    let cell = &table.rows[0].cells[0];
    assert_eq!(cell.col_span, 2);
    assert_eq!(cell.row_span, 2);
    assert!(cell.v_merge);
    assert!(matches!(cell.vertical_align, VerticalAlign::Middle));
    assert!(
        matches!(cell.border_left.dash_style, DashStyle::SystemDash),
        "left border = {:?}",
        cell.border_left
    );
    assert!(
        matches!(cell.border_right.dash_style, DashStyle::SystemDot),
        "right border = {:?}",
        cell.border_right
    );
    assert!(
        matches!(cell.border_top.dash_style, DashStyle::LongDashDotDot),
        "top border = {:?}",
        cell.border_top
    );
    assert!(
        matches!(cell.border_bottom.dash_style, DashStyle::Dash),
        "bottom border = {:?}",
        cell.border_bottom
    );
    assert!(matches!(cell.border_left.color.kind, ColorKind::Theme(ref name) if name == "accent4"));
    assert_eq!(cell.border_right.color.to_css().as_deref(), Some("#123456"));
    assert_eq!(cell.border_top.color.to_css().as_deref(), Some("#FFA500"));
    assert_eq!(
        cell.border_bottom.color.to_css().as_deref(),
        Some("#ABCDEF")
    );

    let cell_para0 = &cell.text_body.as_ref().expect("cell text body").paragraphs[0];
    let cell_def0 = cell_para0.def_rpr.as_ref().expect("cell empty defRPr");
    assert_eq!(cell_def0.font_size, Some(18.0));
    assert_eq!(cell_def0.letter_spacing, Some(1.0));
    assert_eq!(cell_def0.baseline, Some(20000));
    assert_eq!(cell_def0.bold, Some(true));
    assert_eq!(cell_def0.italic, Some(true));
    let cell_run0 = &cell_para0.runs[0];
    assert_eq!(
        cell_run0.hyperlink.as_deref(),
        Some("https://example.com/cell-effect")
    );
    assert!(matches!(
        cell_run0.style.highlight.as_ref().map(|c| &c.kind),
        Some(ColorKind::Theme(name)) if name == "accent6"
    ));
    assert!(matches!(cell_run0.style.color.kind, ColorKind::Theme(ref name) if name == "accent2"));

    let cell_para1 = &cell.text_body.as_ref().expect("cell text body").paragraphs[1];
    let cell_def1 = cell_para1.def_rpr.as_ref().expect("cell start defRPr");
    assert_eq!(
        cell_def1.color.as_ref().and_then(|c| c.to_css()).as_deref(),
        Some("#334455")
    );
    let cell_run1 = &cell_para1.runs[0];
    assert_eq!(
        cell_run1
            .style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#FFFF00")
    );
    assert_eq!(cell_run1.style.color.to_css().as_deref(), Some("#FFA500"));

    let picture = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Picture Start")
        .expect("picture shape");
    assert!(picture.flip_h && picture.flip_v);
    match &picture.shape_type {
        ShapeType::Picture(pic) => {
            assert_eq!(pic.data, b"picture-bytes");
            assert_eq!(pic.content_type, "image/png");
        }
        other => panic!("expected picture, got {other:?}"),
    }

    let effect_shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Effect Start Shape")
        .expect("effect shape");
    assert!(effect_shape.flip_h && effect_shape.flip_v);
    assert!(matches!(
        effect_shape.border.dash_style,
        DashStyle::SystemDashDotDot
    ));
    assert!(matches!(
        effect_shape.border.compound,
        CompoundLine::ThinThick
    ));
    assert!(matches!(
        effect_shape.border.alignment,
        LineAlignment::Inset
    ));
    assert!(matches!(effect_shape.border.join, LineJoin::Miter));
    assert_eq!(effect_shape.border.miter_limit, Some(3.0));
    assert!(effect_shape.effects.outer_shadow.is_some());
    assert!(effect_shape.effects.glow.is_some());
    let shape_body = effect_shape.text_body.as_ref().expect("shape text body");
    let shape_para0 = &shape_body.paragraphs[0];
    let shape_def0 = shape_para0.def_rpr.as_ref().expect("shape empty defRPr");
    assert_eq!(shape_def0.font_size, Some(21.0));
    assert_eq!(shape_def0.letter_spacing, Some(1.25));
    assert_eq!(shape_def0.baseline, Some(15000));
    let shape_run0 = &shape_para0.runs[0];
    assert_eq!(
        shape_run0.hyperlink.as_deref(),
        Some("https://example.com/shape-effect")
    );
    assert_eq!(
        shape_run0
            .style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#FFFF11")
    );
    assert_eq!(shape_run0.style.color.to_css().as_deref(), Some("#112233"));
    let shape_para1 = &shape_body.paragraphs[1];
    let shape_def1 = shape_para1.def_rpr.as_ref().expect("shape start defRPr");
    assert!(matches!(
        shape_def1.color.as_ref().map(|c| &c.kind),
        Some(ColorKind::Theme(name)) if name == "accent5"
    ));
}

#[test]
fn parses_slide_start_tag_paragraph_defaults_and_run_fonts() {
    let slide = r#"
      <p:sp>
        <p:nvSpPr><p:cNvPr id="220" name="Paragraph Defaults"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rect"/>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:p>
            <a:pPr algn="ctr">
              <a:lnSpc><a:spcPct val="125000"></a:spcPct></a:lnSpc>
              <a:spcBef><a:spcPts val="200"></a:spcPts></a:spcBef>
              <a:spcAft><a:spcPts val="400"></a:spcPts></a:spcAft>
            </a:pPr>
            <a:defRPr sz="2000" spc="150" baseline="15000" cap="small" u="sng" strike="sngStrike" b="1" i="1">
              <a:latin typeface="Para Latin"></a:latin>
              <a:ea typeface="Para EA"></a:ea>
              <a:cs typeface="Para CS"></a:cs>
              <a:schemeClr val="accent2"></a:schemeClr>
            </a:defRPr>
            <a:r>
              <a:rPr>
                <a:ea typeface="Run EA"></a:ea>
                <a:cs typeface="Run CS"></a:cs>
                <a:highlight><a:schemeClr val="accent6"></a:schemeClr></a:highlight>
                <a:prstClr val="orange"></a:prstClr>
              </a:rPr>
              <a:t>Shape Run</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="221" name="Cell Defaults"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblGrid><a:gridCol w="914400"/></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr algn="l">
                        <a:lnSpc><a:spcPct val="150000"></a:spcPct></a:lnSpc>
                        <a:spcBef><a:spcPts val="600"></a:spcPts></a:spcBef>
                        <a:spcAft><a:spcPts val="1200"></a:spcPts></a:spcAft>
                      </a:pPr>
                      <a:defRPr sz="1800" spc="100" baseline="10000" cap="all" u="dbl" strike="dblStrike" b="true" i="1">
                        <a:latin typeface="Cell Latin"></a:latin>
                        <a:ea typeface="Cell EA"></a:ea>
                        <a:cs typeface="Cell CS"></a:cs>
                        <a:srgbClr val="334455"></a:srgbClr>
                      </a:defRPr>
                      <a:r>
                        <a:rPr>
                          <a:ea typeface="CellRun EA"></a:ea>
                          <a:cs typeface="CellRun CS"></a:cs>
                          <a:highlight><a:srgbClr val="FFFF00"></a:srgbClr></a:highlight>
                          <a:sysClr lastClr="ABCDEF"></a:sysClr>
                        </a:rPr>
                        <a:t>Cell Run</a:t>
                      </a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr/>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    "#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];

    let shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Paragraph Defaults")
        .expect("shape");
    let para = &shape
        .text_body
        .as_ref()
        .expect("shape text body")
        .paragraphs[0];
    assert!(matches!(para.alignment, Alignment::Center));
    assert_eq!(
        para.def_rpr
            .as_ref()
            .and_then(|def_rpr| def_rpr.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#ED7D31")
    );
    let shape_run = &para.runs[0];
    assert_eq!(
        shape_run
            .style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#70AD47")
    );
    assert_eq!(shape_run.style.color.to_css().as_deref(), Some("#FFA500"));

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("table");
    let cell_para = &table.rows[0].cells[0]
        .text_body
        .as_ref()
        .expect("cell text body")
        .paragraphs[0];
    assert!(matches!(cell_para.alignment, Alignment::Left));
    assert_eq!(
        cell_para
            .def_rpr
            .as_ref()
            .and_then(|def_rpr| def_rpr.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#334455")
    );
    let cell_run = &cell_para.runs[0];
    assert_eq!(
        cell_run
            .style
            .highlight
            .as_ref()
            .and_then(|c| c.to_css())
            .as_deref(),
        Some("#FFFF00")
    );
    assert_eq!(cell_run.style.color.to_css().as_deref(), Some("#ABCDEF"));
}

#[test]
fn parse_slide_master_directly_covers_background_image_and_shape_text_edge_paths() {
    use std::collections::HashMap;

    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:blipFill>
          <a:blip r:embed="rIdBg"></a:blip>
        </a:blipFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Master Edge Shape"/>
          <p:cNvSpPr/>
          <p:nvPr><p:ph type="body" idx="2"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="12700" y="25400"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700">
            <a:noFill/>
            <a:round/>
            <a:headEnd type="triangle" w="sm" len="lg"/>
            <a:tailEnd type="diamond" w="lg" len="sm"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="b" anchorCtr="1" rot="120000" vert="vert270"
                    lIns="91440" tIns="45720" rIns="182880" bIns="22860"
                    wrap="none"></a:bodyPr>
          <a:lstStyle>
            <a:lvl1pPr algn="ctr">
              <a:spcAft><a:spcPts val="600"/></a:spcAft>
              <a:defRPr sz="1800">
                <a:latin typeface="Calibri"/>
                <a:ea typeface="Malgun Gothic"/>
                <a:cs typeface="Mangal"/>
                <a:schemeClr val="accent4"/>
              </a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="Master Miter Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="25400" y="38100"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700" cmpd="tri">
            <a:prstDash val="mystery"/>
            <a:miter lim="250000"/>
          </a:ln>
        </p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr>
        <a:spcBef><a:spcPct val="25000"/></a:spcBef>
        <a:defRPr sz="2000"><a:srgbClr val="445566"/></a:defRPr>
      </a:lvl1pPr>
    </p:titleStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_extra_file("ppt/media/master-bg.jpg", b"master-image")
        .build();
    let mut archive = zip::ZipArchive::new(Cursor::new(pptx)).expect("fixture archive should open");
    let rels = HashMap::from([("rIdBg".to_string(), "../media/master-bg.jpg".to_string())]);

    let master = master_parser::parse_slide_master(xml, &rels, &mut archive)
        .expect("master edge matrix should parse");

    assert!(matches!(
        &master.background,
        Some(Fill::Image(image))
            if image.content_type == "image/jpeg" && image.data == b"master-image"
    ));
    let title_style = master.tx_styles.title_style.as_ref().expect("title style");
    let title_level = title_style.levels[0].as_ref().expect("title level 1");
    assert!(matches!(
        title_level.space_before,
        Some(SpacingValue::Percent(v)) if (v - 0.25).abs() < 1e-6
    ));
    assert_eq!(
        title_level
            .def_run_props
            .as_ref()
            .and_then(|rd| rd.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#445566")
    );

    assert_eq!(master.shapes.len(), 2);
    let edge_shape = &master.shapes[0];
    assert_eq!(
        edge_shape
            .placeholder
            .as_ref()
            .and_then(|placeholder| placeholder.idx),
        Some(2)
    );
    assert!(matches!(edge_shape.border.style, BorderStyle::None));
    assert!(matches!(edge_shape.border.join, LineJoin::Round));
    assert!(matches!(
        edge_shape
            .border
            .head_end
            .as_ref()
            .map(|line_end| line_end.end_type.clone()),
        Some(LineEndType::Triangle)
    ));
    assert!(matches!(
        edge_shape
            .border
            .tail_end
            .as_ref()
            .map(|line_end| line_end.end_type.clone()),
        Some(LineEndType::Diamond)
    ));
    assert_eq!(edge_shape.vertical_text.as_deref(), Some("vert270"));
    let text = edge_shape.text_body.as_ref().expect("edge text body");
    assert!(matches!(text.vertical_align, VerticalAlign::Bottom));
    assert!(text.anchor_center);
    assert!(!text.word_wrap);
    assert!((text.margins.left - 7.2).abs() < 1e-6);
    assert!((text.margins.top - 3.6).abs() < 1e-6);
    assert!((text.margins.right - 14.4).abs() < 1e-6);
    assert!((text.margins.bottom - 1.8).abs() < 1e-6);
    let defaults = text
        .list_style
        .as_ref()
        .and_then(|list| list.levels[0].as_ref())
        .expect("shape list-style defaults");
    assert!(matches!(
        defaults.space_after,
        Some(SpacingValue::Points(v)) if (v - 6.0).abs() < 1e-6
    ));
    let run_defaults = defaults.def_run_props.as_ref().expect("run defaults");
    assert_eq!(run_defaults.font_latin.as_deref(), Some("Calibri"));
    assert_eq!(run_defaults.font_ea.as_deref(), Some("Malgun Gothic"));
    assert_eq!(run_defaults.font_cs.as_deref(), Some("Mangal"));
    assert!(matches!(
        run_defaults.color.as_ref().map(|color| &color.kind),
        Some(ColorKind::Theme(name)) if name == "accent4"
    ));

    let miter_shape = &master.shapes[1];
    assert!(matches!(miter_shape.border.compound, CompoundLine::Triple));
    assert!(matches!(miter_shape.border.dash_style, DashStyle::Solid));
    assert!(matches!(miter_shape.border.join, LineJoin::Miter));
    assert_eq!(miter_shape.border.miter_limit, Some(2.5));
}

#[test]
fn parses_layout_empty_variant_backgrounds_and_master_color_mapping_through_public_parser() {
    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             type="body" showMasterSp="1">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:blipFill><a:blip r:embed="rIdBg"/></a:blipFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Layout Empty NoAuto"/>
          <p:cNvSpPr/>
          <p:nvPr><p:ph type="body" idx="3"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="12700" y="25400"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700" cap="rnd" cmpd="tri" algn="in">
            <a:noFill/>
            <a:miter lim="300000"/>
            <a:headEnd type="triangle" w="sm" len="lg"/>
            <a:tailEnd type="oval" w="lg" len="sm"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr" anchorCtr="1" rot="60000" vert="vert270"
                    lIns="91440" tIns="45720" rIns="182880" bIns="22860"
                    wrap="none"/>
          <a:noAutofit/>
          <a:lstStyle>
            <a:lvl1pPr>
              <a:lnSpc><a:spcPct val="150000"/></a:lnSpc>
              <a:spcBef><a:spcPts val="1200"/></a:spcBef>
              <a:defRPr sz="1800">
                <a:latin typeface="Calibri"/>
                <a:ea typeface="Malgun Gothic"/>
                <a:cs typeface="Mangal"/>
                <a:schemeClr val="accent4"/>
              </a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p/>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="Layout Empty Shrink"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="25400" y="38100"/><a:ext cx="381000" cy="254000"/></a:xfrm></p:spPr>
        <p:txBody><a:bodyPr/><a:spAutoFit/><a:p/></p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sldLayout>"#;

    let layout_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
  <Relationship Id="rIdBg" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/layout-bg.svg"/>
</Relationships>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_clr_map(
            r#"bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink""#,
        )
        .with_layout(layout_xml)
        .with_layout_rels(layout_rels)
        .with_extra_file("ppt/slideLayouts/media/layout-bg.svg", b"<svg/>")
        .build();

    let presentation = parse_pptx(&pptx);
    let layout = &presentation.layouts[0];

    assert!(matches!(
        &layout.background,
        Some(Fill::Image(image))
            if image.content_type == "image/svg+xml" && image.data == b"<svg/>"
    ));
    assert!(matches!(
        &layout.clr_map_ovr,
        Some(ClrMapOverride::UseMaster)
    ));

    assert_eq!(layout.shapes.len(), 2);
    let no_auto_shape = &layout.shapes[0];
    assert!(matches!(no_auto_shape.border.style, BorderStyle::None));
    assert!(matches!(no_auto_shape.border.join, LineJoin::Miter));
    assert_eq!(no_auto_shape.border.miter_limit, Some(3.0));
    assert!(matches!(
        no_auto_shape
            .border
            .head_end
            .as_ref()
            .map(|line_end| line_end.end_type.clone()),
        Some(LineEndType::Triangle)
    ));
    assert!(matches!(
        no_auto_shape
            .border
            .tail_end
            .as_ref()
            .map(|line_end| line_end.end_type.clone()),
        Some(LineEndType::Oval)
    ));
    let no_auto_text = no_auto_shape.text_body.as_ref().expect("no-auto text body");
    assert!(matches!(no_auto_text.auto_fit, AutoFit::NoAutoFit));
    assert!(!no_auto_text.word_wrap);
    assert_eq!(no_auto_shape.vertical_text.as_deref(), Some("vert270"));
    let no_auto_defaults = no_auto_text
        .list_style
        .as_ref()
        .and_then(|list| list.levels[0].as_ref())
        .expect("layout list-style defaults");
    assert!(matches!(
        no_auto_defaults.line_spacing,
        Some(SpacingValue::Percent(v)) if (v - 1.5).abs() < 1e-6
    ));
    assert!(matches!(
        no_auto_defaults.space_before,
        Some(SpacingValue::Points(v)) if (v - 12.0).abs() < 1e-6
    ));
    let layout_run_defaults = no_auto_defaults
        .def_run_props
        .as_ref()
        .expect("layout run defaults");
    assert_eq!(layout_run_defaults.font_latin.as_deref(), Some("Calibri"));
    assert_eq!(
        layout_run_defaults.font_ea.as_deref(),
        Some("Malgun Gothic")
    );
    assert_eq!(layout_run_defaults.font_cs.as_deref(), Some("Mangal"));
    assert!(matches!(
        layout_run_defaults.color.as_ref().map(|color| &color.kind),
        Some(ColorKind::Theme(name)) if name == "accent4"
    ));

    let shrink_shape = &layout.shapes[1];
    assert!(matches!(
        shrink_shape.text_body.as_ref().map(|body| &body.auto_fit),
        Some(AutoFit::Shrink)
    ));
}

#[test]
fn parses_presentation_fallbacks_for_missing_master_rels_and_default_text_style_levels() {
    let presentation_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst>
    <p:sldMasterId r:id="rIdMaster1"/>
    <p:sldMasterId r:id="rIdMaster2"/>
  </p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rIdSlide"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:defaultTextStyle>
    <a:lvl1pPr algn="ctr"/>
    <a:lvl2pPr algn="just"><a:lnSpc><a:spcPct val="175000"/></a:lnSpc></a:lvl2pPr>
    <a:lvl3pPr><a:spcAft><a:spcPts val="600"/></a:spcAft></a:lvl3pPr>
  </p:defaultTextStyle>
</p:presentation>"#
        .to_string();

    let presentation_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdMaster1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rIdMaster2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster2.xml"/>
  <Relationship Id="rIdSlide" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#
        .to_string();

    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#
        .to_string();

    let master_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/missing-theme.xml"/>
</Relationships>"#
        .to_string();

    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="FallbackTheme">
  <a:themeElements>
    <a:clrScheme name="Scheme"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1></a:clrScheme>
    <a:fontScheme name="Fonts"><a:majorFont><a:latin typeface="Aptos"/></a:majorFont><a:minorFont><a:latin typeface="Aptos"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#
        .to_string();

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sld>"#
        .to_string();

    let pptx = zip_entries(&[
        ("ppt/presentation.xml", presentation_xml),
        ("ppt/_rels/presentation.xml.rels", presentation_rels),
        ("ppt/slideMasters/slideMaster1.xml", master_xml.clone()),
        ("ppt/slideMasters/slideMaster2.xml", master_xml),
        ("ppt/slideMasters/_rels/slideMaster2.xml.rels", master_rels),
        ("ppt/theme/theme1.xml", theme_xml),
        ("ppt/slides/slide1.xml", slide_xml),
    ]);

    let presentation = parse_pptx(&pptx);
    assert_eq!(presentation.masters.len(), 2);
    assert_eq!(presentation.slides.len(), 1);
    assert_eq!(presentation.layouts.len(), 0);
    assert_eq!(presentation.masters[0].theme_idx, 0);
    assert_eq!(presentation.masters[1].theme_idx, 0);

    let defaults = presentation
        .default_text_style
        .as_ref()
        .expect("default text style should be parsed");
    assert!(defaults.levels[0].is_some());
    assert!(matches!(
        defaults.levels[1]
            .as_ref()
            .and_then(|level| level.line_spacing.clone()),
        Some(SpacingValue::Percent(v)) if (v - 1.75).abs() < 1e-6
    ));
    assert!(matches!(
        defaults.levels[2]
            .as_ref()
            .and_then(|level| level.space_after.clone()),
        Some(SpacingValue::Points(v)) if (v - 6.0).abs() < 1e-6
    ));
}

#[test]
fn parse_slide_master_directly_covers_body_pr_scheme_background_and_empty_asset_paths() {
    use std::collections::HashMap;

    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg><p:bgPr><a:schemeClr val="accent5"></a:schemeClr></p:bgPr></p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="OpenBodyPr"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="4"/></p:nvPr></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="12700" y="25400"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="25400">
            <a:round/>
            <a:bevel/>
            <a:miter lim="300000"/>
            <a:schemeClr val="accent2"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr" anchorCtr="1" rot="60000" vert="vert" wrap="none"></a:bodyPr>
          <a:normAutofit fontScale="55000" lnSpcReduction="20000"/>
          <a:lstStyle>
            <a:lvl1pPr>
              <a:spcAft><a:spcPts val="400"/></a:spcAft>
              <a:defRPr><a:schemeClr val="accent4"/></a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p/>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr>
        <a:spcAft><a:spcPts val="800"/></a:spcAft>
        <a:defRPr><a:schemeClr val="accent6"></a:schemeClr></a:defRPr>
      </a:lvl1pPr>
    </p:titleStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let mut archive = zip::ZipArchive::new(Cursor::new(zip_entries(&[]))).expect("empty zip");
    let master = master_parser::parse_slide_master(xml, &HashMap::new(), &mut archive)
        .expect("master direct parser should succeed");

    assert!(matches!(
        &master.background,
        Some(Fill::Solid(fill))
            if matches!(fill.color.kind, ColorKind::Theme(ref name) if name == "accent5")
    ));
    let shape = &master.shapes[0];
    assert!(matches!(
        shape.placeholder.as_ref().and_then(|ph| ph.idx),
        Some(4)
    ));
    assert!(matches!(shape.border.join, LineJoin::Miter));
    assert_eq!(shape.border.miter_limit, Some(3.0));
    assert_eq!(shape.border.color.to_css().as_deref(), Some("#ED7D31"));
    assert_eq!(shape.vertical_text.as_deref(), Some("vert"));
    let body = shape.text_body.as_ref().expect("shape text body");
    assert!(matches!(body.vertical_align, VerticalAlign::Middle));
    assert!(body.anchor_center);
    assert!(!body.word_wrap);
    assert!(matches!(
        body.auto_fit,
        AutoFit::Normal {
            font_scale: Some(v),
            line_spacing_reduction: Some(lsr)
        } if (v - 0.55).abs() < 1e-6 && (lsr - 0.20).abs() < 1e-6
    ));
    let level = body
        .list_style
        .as_ref()
        .and_then(|style| style.levels[0].as_ref())
        .expect("shape level defaults");
    assert!(matches!(
        level.space_after,
        Some(SpacingValue::Points(v)) if (v - 4.0).abs() < 1e-6
    ));
    assert_eq!(
        level
            .def_run_props
            .as_ref()
            .and_then(|rd| rd.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#FFC000")
    );
    let title_defaults = master
        .tx_styles
        .title_style
        .as_ref()
        .and_then(|style| style.levels[0].as_ref())
        .expect("title tx style");
    assert!(matches!(
        title_defaults.space_after,
        Some(SpacingValue::Points(v)) if (v - 8.0).abs() < 1e-6
    ));
    assert_eq!(
        title_defaults
            .def_run_props
            .as_ref()
            .and_then(|rd| rd.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#70AD47")
    );

    let empty_asset_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:blipFill><a:blip r:embed="rIdBg"/></a:blipFill>
        <a:schemeClr val="accent1"/>
      </p:bgPr>
    </p:bg>
    <p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree>
  </p:cSld>
</p:sldMaster>"#;
    let rels = HashMap::from([("rIdBg".to_string(), "../media/master-empty.png".to_string())]);
    let mut archive = zip::ZipArchive::new(Cursor::new(zip_entries(&[(
        "ppt/media/master-empty.png",
        String::new(),
    )])))
    .expect("archive with empty media");
    let master = master_parser::parse_slide_master(empty_asset_xml, &rels, &mut archive)
        .expect("master with empty asset should still parse");
    assert!(master.background.is_none());
}

#[test]
fn parse_slide_master_directly_covers_text_style_color_end_assignment_matrix() {
    use std::collections::HashMap;

    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:blipFill><a:blip r:embed="rIdBg"/></a:blipFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="NoAutoFit Start"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></a:xfrm>
          <a:ln w="12700" cap="rnd" cmpd="dbl" algn="in">
            <a:prstDash val="sysDashDot"/>
            <a:schemeClr val="accent3"/>
            <a:miter lim="200000"/>
            <a:headEnd type="triangle" w="sm" len="lg"/>
            <a:tailEnd type="stealth" w="lg" len="sm"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr" anchorCtr="1" rot="60000" vert="vert" lIns="12700" tIns="12700" rIns="12700" bIns="12700" wrap="none"></a:bodyPr>
          <a:noAutofit></a:noAutofit>
          <a:lstStyle>
            <a:lvl1pPr>
              <a:lnSpc><a:spcPct val="125000"/></a:lnSpc>
              <a:spcBef><a:spcPts val="200"/></a:spcBef>
              <a:spcAft><a:spcPts val="300"/></a:spcAft>
              <a:defRPr><a:srgbClr val="AA0000"></a:srgbClr></a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="Shrink Start"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="12700" y="0"/><a:ext cx="12700" cy="12700"/></a:xfrm>
          <a:ln w="12700" cap="flat" cmpd="tri" algn="ctr">
            <a:prstDash val="sysDashDotDot"/>
            <a:schemeClr val="accent4"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="b" vert="horz"></a:bodyPr>
          <a:spAutoFit></a:spAutoFit>
          <a:lstStyle>
            <a:lvl2pPr>
              <a:defRPr><a:schemeClr val="accent6"></a:schemeClr></a:defRPr>
            </a:lvl2pPr>
          </a:lstStyle>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr>
        <a:defRPr><a:srgbClr val="112233"></a:srgbClr></a:defRPr>
      </a:lvl1pPr>
    </p:titleStyle>
    <p:bodyStyle>
      <a:lvl1pPr>
        <a:lnSpc><a:spcPct val="200000"/></a:lnSpc>
        <a:spcBef><a:spcPts val="100"/></a:spcBef>
        <a:spcAft><a:spcPts val="200"/></a:spcAft>
        <a:defRPr>
          <a:latin typeface="Body Latin"/>
          <a:ea typeface="Body EA"/>
          <a:cs typeface="Body CS"/>
          <a:schemeClr val="accent2"></a:schemeClr>
        </a:defRPr>
      </a:lvl1pPr>
    </p:bodyStyle>
    <p:otherStyle>
      <a:lvl1pPr>
        <a:defRPr>
          <a:latin typeface="Other Latin"/>
          <a:ea typeface="Other EA"/>
          <a:cs typeface="Other CS"/>
        </a:defRPr>
      </a:lvl1pPr>
    </p:otherStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let rels = HashMap::from([(
        "rIdBg".to_string(),
        "../media/empty-background.png".to_string(),
    )]);
    let mut archive = zip::ZipArchive::new(Cursor::new(zip_entries(&[(
        "ppt/media/empty-background.png",
        String::new(),
    )])))
    .expect("archive with empty image");
    let master = master_parser::parse_slide_master(xml, &rels, &mut archive).expect("parse");

    assert!(master.background.is_none());

    let title = master
        .tx_styles
        .title_style
        .as_ref()
        .and_then(|style| style.levels[0].as_ref())
        .and_then(|level| level.def_run_props.as_ref())
        .expect("title defaults");
    assert_eq!(
        title.color.as_ref().and_then(|c| c.to_css()).as_deref(),
        Some("#112233")
    );

    let body = master
        .tx_styles
        .body_style
        .as_ref()
        .and_then(|style| style.levels[0].as_ref())
        .expect("body defaults");
    assert!(matches!(
        body.line_spacing,
        Some(SpacingValue::Percent(v)) if (v - 2.0).abs() < 1e-6
    ));
    assert!(matches!(
        body.space_before,
        Some(SpacingValue::Points(v)) if (v - 1.0).abs() < 1e-6
    ));
    assert!(matches!(
        body.space_after,
        Some(SpacingValue::Points(v)) if (v - 2.0).abs() < 1e-6
    ));
    let body_run = body.def_run_props.as_ref().expect("body defRPr");
    assert!(matches!(
        body_run.color.as_ref().map(|c| &c.kind),
        Some(ColorKind::Theme(name)) if name == "accent2"
    ));

    let other = master
        .tx_styles
        .other_style
        .as_ref()
        .and_then(|style| style.levels[0].as_ref())
        .and_then(|level| level.def_run_props.as_ref())
        .expect("other defaults");
    assert_eq!(other.font_latin.as_deref(), Some("Other Latin"));
    assert_eq!(other.font_ea.as_deref(), Some("Other EA"));
    assert_eq!(other.font_cs.as_deref(), Some("Other CS"));

    assert!(matches!(
        master.shapes[0]
            .text_body
            .as_ref()
            .map(|body| &body.auto_fit),
        Some(AutoFit::NoAutoFit)
    ));
    assert!(matches!(
        master.shapes[1]
            .text_body
            .as_ref()
            .map(|body| &body.auto_fit),
        Some(AutoFit::Shrink)
    ));
    let level0 = master.shapes[0]
        .text_body
        .as_ref()
        .and_then(|body| body.list_style.as_ref())
        .and_then(|style| style.levels[0].as_ref())
        .and_then(|level| level.def_run_props.as_ref())
        .expect("shape level 1");
    assert_eq!(
        level0.color.as_ref().and_then(|c| c.to_css()).as_deref(),
        Some("#AA0000")
    );
    let level1 = master.shapes[1]
        .text_body
        .as_ref()
        .and_then(|body| body.list_style.as_ref())
        .and_then(|style| style.levels[1].as_ref())
        .and_then(|level| level.def_run_props.as_ref())
        .expect("shape level 2");
    assert!(matches!(
        level1.color.as_ref().map(|c| &c.kind),
        Some(ColorKind::Theme(name)) if name == "accent6"
    ));
}

#[test]
fn parse_slide_master_directly_covers_start_tag_defaults_and_dash_variant_matrix() {
    use std::collections::HashMap;

    let dash_shapes = [
        ("solid", "SolidDash", "square", "tri", "ctr"),
        ("dot", "DotDash", "flat", "thickThin", "in"),
        ("lgDash", "LongDash", "rnd", "thinThick", "ctr"),
        ("lgDashDotDot", "LongDashDotDot", "flat", "dbl", "in"),
        ("sysDash", "SystemDash", "square", "tri", "ctr"),
        ("sysDot", "SystemDot", "rnd", "thickThin", "in"),
        ("sysDashDot", "SystemDashDot", "flat", "thinThick", "ctr"),
        ("sysDashDotDot", "SystemDashDotDot", "rnd", "dbl", "in"),
    ]
    .into_iter()
    .enumerate()
    .map(|(idx, (dash, name, cap, compound, algn))| {
        format!(
            r#"<p:sp>
  <p:nvSpPr><p:cNvPr id="{id}" name="{name}"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="{idx}"/></p:nvPr></p:nvSpPr>
  <p:spPr>
    <a:xfrm><a:off x="{x}" y="0"/><a:ext cx="12700" cy="12700"/></a:xfrm>
    <a:ln w="12700" cap="{cap}" cmpd="{compound}" algn="{algn}">
      <a:prstDash val="{dash}"/>
      <a:miter lim="250000"/>
      <a:schemeClr val="accent3"/>
      <a:headEnd type="triangle" w="sm" len="lg"/>
      <a:tailEnd type="stealth" w="lg" len="sm"/>
    </a:ln>
  </p:spPr>
  <p:txBody>
    <a:bodyPr anchor="ctr" anchorCtr="1" rot="120000" vert="vert" lIns="12700" tIns="25400" rIns="38100" bIns="50800" wrap="none"></a:bodyPr>
    <a:normAutofit fontScale="60000" lnSpcReduction="25000"></a:normAutofit>
    <a:lstStyle>
      <a:lvl1pPr>
        <a:lnSpc><a:spcPct val="125000"/></a:lnSpc>
        <a:spcBef><a:spcPct val="50000"/></a:spcBef>
        <a:spcAft><a:spcPts val="300"/></a:spcAft>
        <a:defRPr sz="1800"><a:srgbClr val="AA0000"></a:srgbClr></a:defRPr>
      </a:lvl1pPr>
      <a:lvl2pPr>
        <a:defRPr sz="1600"><a:schemeClr val="accent6"></a:schemeClr></a:defRPr>
      </a:lvl2pPr>
    </a:lstStyle>
  </p:txBody>
</p:sp>"#,
            id = idx + 10,
            idx = idx + 1,
            name = name,
            dash = dash,
            cap = cap,
            compound = compound,
            algn = algn,
            x = idx * 12700
        )
    })
    .collect::<String>();

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:gradFill>
          <a:path path="shape"></a:path>
          <a:gsLst>
            <a:gs pos="25000"><a:srgbClr val="010203"></a:srgbClr></a:gs>
            <a:gs pos="100000"><a:schemeClr val="accent2"></a:schemeClr></a:gs>
          </a:gsLst>
          <a:lin ang="1800000"/>
        </a:gradFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      {dash_shapes}
      <p:sp>
        <p:nvSpPr><p:cNvPr id="99" name="NoAutoFit"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="12700"/><a:ext cx="12700" cy="12700"/></a:xfrm></p:spPr>
        <p:txBody>
          <a:bodyPr anchor="b"></a:bodyPr>
          <a:noAutofit></a:noAutofit>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="100" name="ShrinkAutoFit"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="12700" y="12700"/><a:ext cx="12700" cy="12700"/></a:xfrm></p:spPr>
        <p:txBody>
          <a:bodyPr anchor="t"></a:bodyPr>
          <a:spAutoFit></a:spAutoFit>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:txStyles>
    <p:bodyStyle>
      <a:lvl1pPr>
        <a:lnSpc><a:spcPct val="200000"/></a:lnSpc>
        <a:spcBef><a:spcPts val="100"/></a:spcBef>
        <a:spcAft><a:spcPct val="25000"/></a:spcAft>
        <a:defRPr sz="2400" b="1" i="1">
          <a:latin typeface="Calibri"/>
          <a:ea typeface="Malgun Gothic"/>
          <a:cs typeface="Mangal"/>
          <a:schemeClr val="accent4"></a:schemeClr>
        </a:defRPr>
      </a:lvl1pPr>
      <a:lvl2pPr algn="just"/>
      <a:lvl3pPr><a:defRPr sz="1800"/></a:lvl3pPr>
    </p:bodyStyle>
  </p:txStyles>
</p:sldMaster>"#
    );

    let mut archive = zip::ZipArchive::new(Cursor::new(zip_entries(&[]))).expect("empty zip");
    let master =
        master_parser::parse_slide_master(&xml, &HashMap::new(), &mut archive).expect("parse");

    assert!(matches!(
        &master.background,
        Some(Fill::Gradient(fill))
            if matches!(fill.gradient_type, GradientType::Linear)
                && fill.stops.len() == 2
                && (fill.angle - 30.0).abs() < 1e-6
    ));
    assert_eq!(master.shapes.len(), 10);
    assert!(matches!(
        master.shapes[8]
            .text_body
            .as_ref()
            .map(|body| &body.auto_fit),
        Some(AutoFit::NoAutoFit)
    ));
    assert!(matches!(
        master.shapes[9]
            .text_body
            .as_ref()
            .map(|body| &body.auto_fit),
        Some(AutoFit::Shrink)
    ));
    assert!(
        master
            .shapes
            .iter()
            .any(|shape| matches!(shape.border.dash_style, DashStyle::SystemDash))
    );
    assert!(
        master
            .shapes
            .iter()
            .any(|shape| matches!(shape.border.dash_style, DashStyle::SystemDot))
    );
    assert!(
        master
            .shapes
            .iter()
            .any(|shape| matches!(shape.border.dash_style, DashStyle::SystemDashDot))
    );
    assert!(
        master
            .shapes
            .iter()
            .any(|shape| matches!(shape.border.dash_style, DashStyle::SystemDashDotDot))
    );
    assert!(
        master
            .shapes
            .iter()
            .any(|shape| matches!(shape.border.compound, CompoundLine::ThickThin))
    );
    assert!(
        master
            .shapes
            .iter()
            .any(|shape| matches!(shape.border.compound, CompoundLine::ThinThick))
    );
    let body_defaults = master
        .tx_styles
        .body_style
        .as_ref()
        .and_then(|style| style.levels[0].as_ref())
        .expect("body level defaults");
    assert!(matches!(
        body_defaults.line_spacing,
        Some(SpacingValue::Percent(v)) if (v - 2.0).abs() < 1e-6
    ));
    assert!(matches!(
        body_defaults.space_before,
        Some(SpacingValue::Points(v)) if (v - 1.0).abs() < 1e-6
    ));
    assert!(matches!(
        body_defaults.space_after,
        Some(SpacingValue::Percent(v)) if (v - 0.25).abs() < 1e-6
    ));
    let run = body_defaults
        .def_run_props
        .as_ref()
        .expect("body run defaults");
    assert_eq!(run.font_latin.as_deref(), Some("Calibri"));
    assert_eq!(run.font_ea.as_deref(), Some("Malgun Gothic"));
    assert_eq!(run.font_cs.as_deref(), Some("Mangal"));
    assert_eq!(
        run.color.as_ref().and_then(|c| c.to_css()).as_deref(),
        Some("#FFC000")
    );
    assert!(
        master
            .tx_styles
            .body_style
            .as_ref()
            .and_then(|style| style.levels[1].as_ref())
            .is_some()
    );
    assert!(
        master
            .tx_styles
            .body_style
            .as_ref()
            .and_then(|style| style.levels[2].as_ref())
            .and_then(|level| level.def_run_props.as_ref())
            .is_some()
    );
}

#[test]
fn parse_slide_layout_directly_covers_body_pr_scheme_background_and_empty_asset_paths() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="body">
  <p:cSld>
    <p:bg><p:bgPr><a:schemeClr val="accent3"></a:schemeClr></p:bgPr></p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="OpenLayoutBody"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="2"/></p:nvPr></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700">
            <a:round/>
            <a:bevel/>
            <a:miter lim="250000"/>
            <a:schemeClr val="accent2"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr" anchorCtr="1" rot="120000" vert="vert270" wrap="none"></a:bodyPr>
          <a:normAutofit fontScale="50000" lnSpcReduction="10000"/>
          <a:lstStyle>
            <a:lvl1pPr>
              <a:lnSpc><a:spcPct val="150000"/></a:lnSpc>
              <a:spcAft><a:spcPts val="300"/></a:spcAft>
              <a:defRPr>
                <a:latin typeface="Aptos"/>
                <a:ea typeface="Yu Gothic"/>
                <a:cs typeface="Noto Sans Arabic"/>
                <a:schemeClr val="accent4"/>
              </a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p/>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sldLayout>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_full_master(master_xml)
        .with_layout(xml)
        .with_slide_layout_rel()
        .build();
    let presentation = parse_pptx(&pptx);
    let layout = &presentation.layouts[0];

    assert!(matches!(
        &layout.background,
        Some(Fill::Solid(fill))
            if matches!(fill.color.kind, ColorKind::Theme(ref name) if name == "accent3")
    ));
    assert!(matches!(
        &layout.clr_map_ovr,
        Some(ClrMapOverride::UseMaster)
    ));
    let shape = &layout.shapes[0];
    assert!(matches!(
        shape.placeholder.as_ref().and_then(|ph| ph.idx),
        Some(2)
    ));
    assert!(matches!(shape.border.join, LineJoin::Miter));
    assert_eq!(shape.border.miter_limit, Some(2.5));
    assert_eq!(shape.border.color.to_css().as_deref(), Some("#ED7D31"));
    assert_eq!(shape.vertical_text.as_deref(), Some("vert270"));
    let body = shape.text_body.as_ref().expect("layout text body");
    assert!(matches!(body.vertical_align, VerticalAlign::Middle));
    assert!(body.anchor_center);
    assert!(!body.word_wrap);
    assert!(matches!(
        body.auto_fit,
        AutoFit::Normal {
            font_scale: Some(v),
            line_spacing_reduction: Some(lsr)
        } if (v - 0.50).abs() < 1e-6 && (lsr - 0.10).abs() < 1e-6
    ));
    let level = body
        .list_style
        .as_ref()
        .and_then(|style| style.levels[0].as_ref())
        .expect("layout level defaults");
    assert!(matches!(
        level.line_spacing,
        Some(SpacingValue::Percent(v)) if (v - 1.5).abs() < 1e-6
    ));
    assert!(matches!(
        level.space_after,
        Some(SpacingValue::Points(v)) if (v - 3.0).abs() < 1e-6
    ));
    let defaults = level.def_run_props.as_ref().expect("layout defRPr");
    assert_eq!(defaults.font_latin.as_deref(), Some("Aptos"));
    assert_eq!(defaults.font_ea.as_deref(), Some("Yu Gothic"));
    assert_eq!(defaults.font_cs.as_deref(), Some("Noto Sans Arabic"));
    assert_eq!(
        defaults
            .color
            .as_ref()
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#FFC000")
    );

    let empty_asset_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:blipFill><a:blip r:embed="rIdBg"/></a:blipFill>
        <a:schemeClr val="accent6"/>
      </p:bgPr>
    </p:bg>
    <p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree>
  </p:cSld>
</p:sldLayout>"#;
    let empty_asset_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
  <Relationship Id="rIdBg" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/layout-empty.png"/>
</Relationships>"#;
    let pptx = fixtures::MinimalPptx::new("")
        .with_full_master(master_xml)
        .with_layout(empty_asset_xml)
        .with_slide_layout_rel()
        .with_layout_rels(empty_asset_rels)
        .with_extra_file("ppt/media/layout-empty.png", b"")
        .build();
    let presentation = parse_pptx(&pptx);
    assert!(presentation.layouts[0].background.is_none());
}

#[test]
fn parses_without_master_or_slide_relationship_parts_through_public_parser() {
    let presentation_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rIdMaster"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rIdSlide"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#;
    let presentation_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdMaster" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rIdSlide" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#;
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="Lonely Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let pptx = zip_entries(&[
        ("ppt/presentation.xml", presentation_xml.to_string()),
        (
            "ppt/_rels/presentation.xml.rels",
            presentation_rels.to_string(),
        ),
        ("ppt/slideMasters/slideMaster1.xml", master_xml.to_string()),
        ("ppt/slides/slide1.xml", slide_xml.to_string()),
    ]);

    let presentation = parse_pptx(&pptx);
    assert_eq!(presentation.masters.len(), 1);
    assert!(presentation.layouts.is_empty());
    assert_eq!(presentation.slides.len(), 1);
    assert_eq!(presentation.slides[0].shapes.len(), 1);
}

#[test]
fn parses_presentation_default_text_style_font_and_color_variants_through_public_parser() {
    let presentation_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rIdMaster"/></p:sldMasterIdLst>
  <p:sldIdLst>
    <p:sldId id="256" r:id="rIdSlide1"/>
    <p:sldId id="257" r:id="rIdSlide2" show="false"/>
  </p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:defaultTextStyle>
    <a:lvl1pPr algn="ctr">
      <a:lnSpc><a:spcPct val="150000"/></a:lnSpc>
      <a:spcBef><a:spcPts val="200"/></a:spcBef>
      <a:spcAft><a:spcPct val="50000"/></a:spcAft>
      <a:defRPr sz="1800"><a:srgbClr val="111111"></a:srgbClr></a:defRPr>
    </a:lvl1pPr>
    <a:lvl2pPr algn="just">
      <a:spcAft><a:spcPts val="300"/></a:spcAft>
      <a:defRPr sz="1600"><a:schemeClr val="accent2"></a:schemeClr></a:defRPr>
    </a:lvl2pPr>
    <a:lvl3pPr algn="r"/>
    <a:lvl4pPr algn="l">
      <a:defRPr sz="1400">
        <a:latin typeface="Latin Face"/>
        <a:ea typeface="East Face"/>
        <a:cs typeface="CS Face"/>
        <a:srgbClr val="223344"/>
      </a:defRPr>
    </a:lvl4pPr>
    <a:lvl5pPr algn="just">
      <a:defRPr sz="1500">
        <a:latin typeface="Latin Theme"/>
        <a:ea typeface="EA Theme"/>
        <a:cs typeface="CS Theme"/>
        <a:schemeClr val="accent4"/>
      </a:defRPr>
    </a:lvl5pPr>
    <a:lvl6pPr algn="ctr">
      <a:defRPr sz="1700"/>
    </a:lvl6pPr>
  </p:defaultTextStyle>
</p:presentation>"#;
    let presentation_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdMaster" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rIdSlide1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rIdSlide2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
  <Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#;
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;
    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Theme">
  <a:themeElements>
    <a:clrScheme name="Scheme">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
    </a:clrScheme>
    <a:fontScheme name="Fonts">
      <a:majorFont><a:latin typeface="Aptos"/></a:majorFont>
      <a:minorFont><a:latin typeface="Aptos"/></a:minorFont>
    </a:fontScheme>
  </a:themeElements>
</a:theme>"#;
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let pptx = zip_entries(&[
        ("ppt/presentation.xml", presentation_xml.to_string()),
        (
            "ppt/_rels/presentation.xml.rels",
            presentation_rels.to_string(),
        ),
        ("ppt/slideMasters/slideMaster1.xml", master_xml.to_string()),
        ("ppt/theme/theme1.xml", theme_xml.to_string()),
        ("ppt/slides/slide1.xml", slide_xml.to_string()),
        ("ppt/slides/slide2.xml", slide_xml.to_string()),
    ]);

    let presentation = parse_pptx(&pptx);
    assert_eq!(presentation.slides.len(), 2);
    assert!(!presentation.slides[0].hidden);
    assert!(presentation.slides[1].hidden);

    let defaults = presentation
        .default_text_style
        .as_ref()
        .expect("default text style should parse");
    let lvl1 = defaults.levels[0].as_ref().expect("level 1 defaults");
    assert!(matches!(
        lvl1.space_after,
        Some(SpacingValue::Percent(v)) if (v - 0.5).abs() < 1e-6
    ));
    assert_eq!(
        lvl1.def_run_props
            .as_ref()
            .and_then(|run| run.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#111111")
    );

    let lvl2 = defaults.levels[1].as_ref().expect("level 2 defaults");
    assert!(matches!(
        lvl2.space_after,
        Some(SpacingValue::Points(v)) if (v - 3.0).abs() < 1e-6
    ));
    assert_eq!(
        lvl2.def_run_props
            .as_ref()
            .and_then(|run| run.color.as_ref())
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#ED7D31")
    );

    assert_eq!(
        defaults.levels[2]
            .as_ref()
            .and_then(|level| level.alignment.as_ref())
            .map(|alignment| alignment.to_css()),
        Some("right")
    );

    let lvl4 = defaults.levels[3]
        .as_ref()
        .and_then(|level| level.def_run_props.as_ref())
        .expect("level 4 run defaults");
    assert_eq!(lvl4.font_latin.as_deref(), Some("Latin Face"));
    assert_eq!(lvl4.font_ea.as_deref(), Some("East Face"));
    assert_eq!(lvl4.font_cs.as_deref(), Some("CS Face"));
    assert_eq!(
        lvl4.color
            .as_ref()
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#223344")
    );

    let lvl5 = defaults.levels[4]
        .as_ref()
        .and_then(|level| level.def_run_props.as_ref())
        .expect("level 5 run defaults");
    assert_eq!(lvl5.font_latin.as_deref(), Some("Latin Theme"));
    assert_eq!(lvl5.font_ea.as_deref(), Some("EA Theme"));
    assert_eq!(lvl5.font_cs.as_deref(), Some("CS Theme"));
    assert_eq!(
        lvl5.color
            .as_ref()
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#FFC000")
    );

    assert_eq!(
        defaults.levels[5]
            .as_ref()
            .and_then(|level| level.def_run_props.as_ref())
            .and_then(|run| run.font_size),
        Some(17.0)
    );
}

#[test]
fn parses_core_title_fallbacks_through_public_parser() {
    let presentation_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rIdMaster"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rIdSlide"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#;
    let presentation_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdMaster" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rIdSlide" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#;
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sld>"#;

    let empty_title_pptx = zip_entries(&[
        ("ppt/presentation.xml", presentation_xml.to_string()),
        (
            "ppt/_rels/presentation.xml.rels",
            presentation_rels.to_string(),
        ),
        ("ppt/slideMasters/slideMaster1.xml", master_xml.to_string()),
        ("ppt/slides/slide1.xml", slide_xml.to_string()),
        (
            "docProps/core.xml",
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties">
  <dc:title></dc:title>
</cp:coreProperties>"#
                .to_string(),
        ),
    ]);
    let invalid_title_pptx = zip_entries(&[
        ("ppt/presentation.xml", presentation_xml.to_string()),
        (
            "ppt/_rels/presentation.xml.rels",
            presentation_rels.to_string(),
        ),
        ("ppt/slideMasters/slideMaster1.xml", master_xml.to_string()),
        ("ppt/slides/slide1.xml", slide_xml.to_string()),
        (
            "docProps/core.xml",
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties">
  <dc:title>"#
                .to_string(),
        ),
    ]);

    assert!(parse_pptx(&empty_title_pptx).title.is_none());
    assert!(parse_pptx(&invalid_title_pptx).title.is_none());
}

#[test]
fn parse_slide_directly_covers_graphic_frame_table_and_shape_contexts() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"
       xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="25000"><a:srgbClr val="010203"></a:srgbClr></a:gs>
            <a:gs pos="100000"><a:schemeClr val="accent2"></a:schemeClr></a:gs>
          </a:gsLst>
          <a:path path="rect"></a:path>
          <a:lin ang="1800000"></a:lin>
        </a:gradFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="SmartArt"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram">
            <dgm:relIds r:dm="1"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="3" name="Math"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/officeDocument/2006/math">
            <m:oMath><m:r><m:t>x</m:t></m:r></m:oMath>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="4" name="Direct Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1" bandCol="1" firstRow="1" lastRow="1" firstCol="1" lastCol="1"></a:tblPr>
              <a:tblGrid>
                <a:gridCol w="914400"/>
              </a:tblGrid>
              <a:tr h="457200">
                <a:tc gridSpan="2" rowSpan="2" vMerge="1">
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr algn="ctr"/>
                      <a:defRPr sz="1800" spc="50" baseline="5000" cap="small" u="dashLong" strike="dblStrike" b="1" i="true"/>
                      <a:buClr><a:srgbClr val="00AA00"/></a:buClr>
                      <a:r>
                        <a:rPr><a:hlinkClick r:id="rIdLink"/></a:rPr>
                        <a:t>Cell</a:t>
                      </a:r>
                    </a:p>
                  </a:txBody>
                  <a:tcPr anchor="b">
                    <a:lnB w="12700"><a:prstDash val="dashDot"></a:prstDash><a:srgbClr val="123456"></a:srgbClr></a:lnB>
                  </a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:grpSp>
        <p:nvGrpSpPr><p:cNvPr id="5" name="Outer Group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
        <p:grpSpPr>
          <a:xfrm>
            <a:off x="100000" y="200000"/>
            <a:ext cx="3000000" cy="2000000"/>
            <a:chOff><a:off x="10000" y="20000"/></a:chOff>
            <a:chExt><a:ext cx="1500000" cy="800000"/></a:chExt>
          </a:xfrm>
        </p:grpSpPr>
        <p:sp>
          <p:nvSpPr><p:cNvPr id="6" name="Grouped Child"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
          <p:spPr>
            <a:xfrm><a:off x="500000" y="500000"/><a:ext cx="400000" cy="200000"/></a:xfrm>
            <a:prstGeom prst="rect"></a:prstGeom>
          </p:spPr>
        </p:sp>
      </p:grpSp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="7" name="Direct Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm rot="120000" flipH="1" flipV="true"/>
          <a:prstGeom prst="rect"></a:prstGeom>
          <a:noFill/>
          <a:gradFill>
            <a:gsLst>
              <a:gs pos="0"><a:schemeClr val="accent4"></a:schemeClr></a:gs>
            </a:gsLst>
            <a:path path="shape"></a:path>
            <a:lin ang="60000"></a:lin>
          </a:gradFill>
          <a:ln w="12700" cap="flat" cmpd="thickThin">
            <a:prstDash val="dashDot"></a:prstDash>
            <a:bevel/>
            <a:headEnd type="triangle" w="sm" len="lg"/>
            <a:tailEnd type="stealth" w="lg" len="sm"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr"/>
          <a:noAutofit/>
          <a:lstStyle>
            <a:lvl1pPr>
              <a:defRPr sz="1900"><a:srgbClr val="334455"></a:srgbClr></a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p>
            <a:pPr algn="ctr"/>
            <a:defRPr sz="2000" spc="100" baseline="10000" cap="all" u="dbl" strike="sngStrike" b="1" i="true"/>
            <a:buClr><a:srgbClr val="AA5500"></a:srgbClr></a:buClr>
            <a:r>
              <a:rPr>
                <a:hlinkClick r:id="rIdLink"/>
                <a:highlight><a:srgbClr val="FFFF00"></a:srgbClr></a:highlight>
              </a:rPr>
              <a:t>Linked</a:t>
            </a:r>
            <a:br/>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:pic>
        <p:nvPicPr><p:cNvPr id="8" name="Direct Picture"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
        <p:blipFill><a:blip r:embed="rIdPic"></a:blip></p:blipFill>
        <p:spPr>
          <a:xfrm rot="60000" flipH="1" flipV="true"/>
          <a:prstGeom prst="rect"></a:prstGeom>
        </p:spPr>
      </p:pic>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="9" name="Shrink Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="228600" cy="114300"/></a:xfrm></p:spPr>
        <p:txBody><a:bodyPr/><a:spAutoFit/><a:p/></p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_raw_slide(slide_xml)
        .with_slide_rels(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/direct" TargetMode="External"/>
  <Relationship Id="rIdPic" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/direct-pic.png"/>
</Relationships>"#,
        )
        .with_extra_file("ppt/media/direct-pic.png", b"png-data")
        .build();
    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];

    assert!(matches!(
        &slide.background,
        Some(Fill::Gradient(fill))
            if !fill.stops.is_empty()
    ));

    let unsupported_labels = slide
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape_type {
            ShapeType::Unsupported(data) => Some(data.label.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(unsupported_labels.iter().any(|label| label == "SmartArt"));
    assert!(
        unsupported_labels
            .iter()
            .any(|label| label == "Math Equation")
    );

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("direct table");
    assert!(table.band_row && table.band_col && table.first_row && table.last_row);
    assert!(table.first_col && table.last_col);
    let cell = &table.rows[0].cells[0];
    assert_eq!(cell.col_span, 2);
    assert_eq!(cell.row_span, 2);
    assert!(cell.v_merge);
    assert!(matches!(cell.vertical_align, VerticalAlign::Bottom));
    assert!(matches!(cell.border_bottom.dash_style, DashStyle::DashDot));
    assert_eq!(
        cell.text_body
            .as_ref()
            .and_then(|body| body.paragraphs.first())
            .and_then(|paragraph| paragraph.runs.first())
            .and_then(|run| run.hyperlink.as_deref()),
        Some("https://example.com/direct")
    );

    let group = slide
        .shapes
        .iter()
        .find(|shape| matches!(shape.shape_type, ShapeType::Group(_, _)))
        .expect("group shape");
    match &group.shape_type {
        ShapeType::Group(children, group_data) => {
            assert_eq!(children.len(), 1);
            assert_eq!(
                group_data.child_offset.x.to_pt(),
                Emu::parse_emu("10000").to_pt()
            );
            assert_eq!(
                group_data.child_extent.width.to_pt(),
                Emu::parse_emu("1500000").to_pt()
            );
        }
        other => panic!("expected group, got {other:?}"),
    }

    let direct_shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Direct Shape")
        .expect("direct shape");
    assert!(matches!(direct_shape.fill, Fill::Gradient(_)));
    assert!(matches!(direct_shape.border.join, LineJoin::Bevel));
    let text_body = direct_shape.text_body.as_ref().expect("direct text body");
    assert!(matches!(text_body.auto_fit, AutoFit::NoAutoFit));
    assert_eq!(text_body.paragraphs[0].runs.len(), 2);
    assert_eq!(
        text_body.paragraphs[0].runs[0]
            .style
            .highlight
            .as_ref()
            .and_then(|color| color.to_css())
            .as_deref(),
        Some("#FFFF00")
    );
    assert_eq!(
        text_body.paragraphs[0].runs[0].hyperlink.as_deref(),
        Some("https://example.com/direct")
    );
    assert!(text_body.paragraphs[0].runs[1].is_break);
    let lvl1 = text_body
        .list_style
        .as_ref()
        .and_then(|style| style.levels[0].as_ref())
        .and_then(|level| level.def_run_props.as_ref())
        .expect("direct shape defaults");
    assert_eq!(lvl1.font_size, Some(19.0));

    let picture = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Picture(pic) => Some((shape, pic)),
            _ => None,
        })
        .expect("direct picture");
    assert!((picture.0.rotation - 1.0).abs() < 1e-6);
    assert!(picture.0.flip_h);
    assert!(picture.0.flip_v);
    assert_eq!(picture.1.data, b"png-data");

    let shrink_shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Shrink Shape")
        .expect("shrink shape");
    assert!(matches!(
        shrink_shape.text_body.as_ref().map(|body| &body.auto_fit),
        Some(AutoFit::Shrink)
    ));
}

#[test]
fn parse_slide_directly_covers_start_tag_fallback_and_missing_attr_branches() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:blipFill><a:blip r:embed="rIdBg"></a:blip></a:blipFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="10" name="Chart Without Id"></p:cNvPr><p:cNvGraphicFramePr></p:cNvGraphicFramePr><p:nvPr></p:nvPr></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"></a:off><a:ext cx="1828800" cy="914400"></a:ext></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart></c:chart>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="11" name="Sparse Table"></p:cNvPr><p:cNvGraphicFramePr></p:cNvGraphicFramePr><p:nvPr></p:nvPr></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"></a:off><a:ext cx="1828800" cy="914400"></a:ext></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr></a:tblPr>
              <a:tblGrid><a:gridCol w="914400"></a:gridCol></a:tblGrid>
              <a:tr h="457200">
                <a:tc>
                  <a:txBody>
                    <a:bodyPr></a:bodyPr>
                    <a:lstStyle></a:lstStyle>
                    <a:p>
                      <a:buClr><a:schemeClr val="accent3"><a:tint val="20000"></a:tint></a:schemeClr></a:buClr>
                      <a:buChar char="•"/>
                      <a:r>
                        <a:rPr>
                          <a:highlight><a:schemeClr val="accent1"><a:tint val="10000"></a:tint></a:schemeClr></a:highlight>
                          <a:schemeClr val="accent2"><a:shade val="20000"></a:shade></a:schemeClr>
                        </a:rPr>
                        <a:t>Cell</a:t>
                      </a:r>
                      <a:br></a:br>
                    </a:p>
                  </a:txBody>
                  <a:tcPr><a:solidFill><a:schemeClr val="accent4"><a:shade val="30000"></a:shade></a:schemeClr></a:solidFill></a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="12" name="Fallback Shape"></p:cNvPr><p:cNvSpPr></p:cNvSpPr><p:nvPr></p:nvPr></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"></a:off><a:ext cx="914400" cy="457200"></a:ext></a:xfrm>
          <a:prstGeom prst="rect"></a:prstGeom>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchorCtr="true" vert="horz"></a:bodyPr>
          <a:p>
            <a:buClr><a:schemeClr val="accent5"><a:tint val="5000"></a:tint></a:schemeClr></a:buClr>
            <a:buChar char="◦"/>
            <a:r>
              <a:rPr>
                <a:highlight><a:schemeClr val="accent6"><a:shade val="10000"></a:shade></a:schemeClr></a:highlight>
                <a:schemeClr val="accent4"><a:tint val="15000"></a:tint></a:schemeClr>
              </a:rPr>
              <a:t>Shape</a:t>
            </a:r>
            <a:br></a:br>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:pic>
        <p:nvPicPr><p:cNvPr id="13" name="Crop Top"></p:cNvPr><p:cNvPicPr></p:cNvPicPr><p:nvPr></p:nvPr></p:nvPicPr>
        <p:blipFill><a:blip r:embed="rIdPicTop"></a:blip><a:srcRect t="10000"/></p:blipFill>
        <p:spPr><a:xfrm><a:off x="0" y="0"></a:off><a:ext cx="457200" cy="228600"></a:ext></a:xfrm><a:prstGeom prst="rect"></a:prstGeom></p:spPr>
      </p:pic>
      <p:pic>
        <p:nvPicPr><p:cNvPr id="14" name="Crop Right"></p:cNvPr><p:cNvPicPr></p:cNvPicPr><p:nvPr></p:nvPr></p:nvPicPr>
        <p:blipFill><a:blip r:embed="rIdPicRight"></a:blip><a:srcRect r="20000"/></p:blipFill>
        <p:spPr><a:xfrm><a:off x="0" y="0"></a:off><a:ext cx="457200" cy="228600"></a:ext></a:xfrm><a:prstGeom prst="rect"></a:prstGeom></p:spPr>
      </p:pic>
      <p:pic>
        <p:nvPicPr><p:cNvPr id="15" name="Crop Bottom"></p:cNvPr><p:cNvPicPr></p:cNvPicPr><p:nvPr></p:nvPr></p:nvPicPr>
        <p:blipFill><a:blip r:embed="rIdPicBottom"></a:blip><a:srcRect b="30000"/></p:blipFill>
        <p:spPr><a:xfrm><a:off x="0" y="0"></a:off><a:ext cx="457200" cy="228600"></a:ext></a:xfrm><a:prstGeom prst="rect"></a:prstGeom></p:spPr>
      </p:pic>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdBg" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/empty-bg.png"/>
  <Relationship Id="rIdPicTop" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/crop-top.png"/>
  <Relationship Id="rIdPicRight" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/crop-right.png"/>
  <Relationship Id="rIdPicBottom" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/crop-bottom.png"/>
</Relationships>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_raw_slide(slide_xml)
        .with_slide_rels(slide_rels)
        .with_extra_file("ppt/media/empty-bg.png", b"")
        .with_extra_file("ppt/media/crop-top.png", b"top")
        .with_extra_file("ppt/media/crop-right.png", b"right")
        .with_extra_file("ppt/media/crop-bottom.png", b"bottom")
        .build();

    let presentation = parse_pptx(&pptx);
    let slide = &presentation.slides[0];

    assert!(
        slide.background.is_none(),
        "empty background image should be ignored"
    );

    let chart = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Chart(chart) => Some(chart),
            _ => None,
        })
        .expect("chart fallback shape");
    assert!(chart.rel_id.is_empty());
    assert!(chart.direct_spec.is_none());

    let table = slide
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape_type {
            ShapeType::Table(table) => Some(table),
            _ => None,
        })
        .expect("sparse table");
    assert!(
        !table.band_row
            && !table.band_col
            && !table.first_row
            && !table.last_row
            && !table.first_col
            && !table.last_col
    );
    let cell_para = &table.rows[0].cells[0]
        .text_body
        .as_ref()
        .expect("cell text body")
        .paragraphs[0];
    assert_eq!(cell_para.runs.len(), 2);
    assert!(matches!(
        cell_para.bullet.as_ref(),
        Some(Bullet::Char(bullet)) if bullet.char == "•"
    ));
    let cell_run = &cell_para.runs[0];
    assert!(matches!(
        cell_run.style.color.kind,
        ColorKind::Theme(ref name) if name == "accent2"
    ));

    let fallback_shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Fallback Shape")
        .expect("fallback shape");
    let fallback_body = fallback_shape
        .text_body
        .as_ref()
        .expect("fallback text body");
    assert!(fallback_body.anchor_center);
    assert!(fallback_shape.vertical_text.is_none());
    let fallback_para = &fallback_body.paragraphs[0];
    assert_eq!(fallback_para.runs.len(), 2);
    assert!(matches!(
        fallback_para.bullet.as_ref(),
        Some(Bullet::Char(bullet)) if bullet.char == "◦"
    ));
    let fallback_run = &fallback_para.runs[0];
    assert!(matches!(
        fallback_run.style.color.kind,
        ColorKind::Theme(ref name) if name == "accent4"
    ));

    for (name, expected_top, expected_right, expected_bottom, expected_bytes) in [
        ("Crop Top", Some(0.1), None, None, b"top".as_slice()),
        ("Crop Right", None, Some(0.2), None, b"right".as_slice()),
        ("Crop Bottom", None, None, Some(0.3), b"bottom".as_slice()),
    ] {
        let picture = slide
            .shapes
            .iter()
            .find(|shape| shape.name == name)
            .expect("picture shape");
        let picture = match &picture.shape_type {
            ShapeType::Picture(pic) => pic,
            other => panic!("expected picture for {name}, got {other:?}"),
        };
        assert_eq!(picture.data.as_slice(), expected_bytes);
        let crop = picture.crop.as_ref().expect("picture crop");
        assert_eq!(crop.top, expected_top.unwrap_or(0.0));
        assert_eq!(crop.right, expected_right.unwrap_or(0.0));
        assert_eq!(crop.bottom, expected_bottom.unwrap_or(0.0));
    }
}

#[test]
fn parse_slide_directly_covers_style_ref_effect_defaults_and_custom_geom_handles() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="20" name="Styled Fill Shape"></p:cNvPr>
          <p:cNvSpPr></p:cNvSpPr>
          <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:style>
          <a:lnRef idx="2"/>
          <a:fillRef idx="3"/>
          <a:effectRef idx="4"/>
          <a:fontRef idx="minor"/>
        </p:style>
        <p:spPr>
          <a:xfrm><a:off x="12700" y="25400"></a:off><a:ext cx="914400" cy="457200"></a:ext></a:xfrm>
          <a:prstGeom prst="rect">
            <a:avLst><a:gd name="adj1" fmla="val 50000"/></a:avLst>
          </a:prstGeom>
          <a:blipFill><a:blip r:embed="rIdFill"></a:blip></a:blipFill>
        </p:spPr>
        <p:txBody><a:bodyPr></a:bodyPr><a:p><a:r><a:t>Styled fill</a:t></a:r></a:p></p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="21" name="Handle Shape"></p:cNvPr><p:cNvSpPr></p:cNvSpPr><p:nvPr></p:nvPr></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"></a:off><a:ext cx="914400" cy="457200"></a:ext></a:xfrm>
          <a:effectLst>
            <a:outerShdw blurRad="12700" dist="25400" dir="5400000"><a:srgbClr val="112233"><a:tint val="20000"/></a:srgbClr></a:outerShdw>
            <a:glow rad="6350"><a:schemeClr val="accent4"><a:shade val="50000"/></a:schemeClr></a:glow>
          </a:effectLst>
          <a:custGeom>
            <a:avLst><a:gd name="adj2" fmla="val 40000"/></a:avLst>
            <a:ahLst>
              <a:ahXY gdRefX="adj2" gdRefY="adj2" minX="0" maxX="100000" minY="0" maxY="100000">
                <a:pos x="50000" y="60000"/>
              </a:ahXY>
              <a:ahPolar gdRefR="adj2" gdRefAng="adj2" minR="0" maxR="100000" minAng="0" maxAng="21600000">
                <a:pos x="70000" y="80000"/>
              </a:ahPolar>
            </a:ahLst>
            <a:cxnLst><a:cxn ang="5400000"><a:pos x="100000" y="200000"/></a:cxn></a:cxnLst>
            <a:rect l="0" t="0" r="100000" b="100000"></a:rect>
            <a:pathLst>
              <a:path w="100000" h="100000" fill="darkenLess">
                <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
                <a:quadBezTo><a:pt x="25000" y="25000"/><a:pt x="50000" y="50000"/></a:quadBezTo>
                <a:cubicBezTo><a:pt x="0" y="75000"/><a:pt x="25000" y="100000"/><a:pt x="50000" y="100000"/></a:cubicBezTo>
              </a:path>
            </a:pathLst>
          </a:custGeom>
        </p:spPr>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="22" name="Effect Fallback Shape"></p:cNvPr><p:cNvSpPr></p:cNvSpPr><p:nvPr></p:nvPr></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"></a:off><a:ext cx="457200" cy="228600"></a:ext></a:xfrm>
          <a:effectLst>
            <a:outerShdw blurRad="12700" dist="25400" dir="5400000"></a:outerShdw>
            <a:glow rad="6350"></a:glow>
          </a:effectLst>
          <a:prstGeom prst="rect"></a:prstGeom>
        </p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_raw_slide(slide_xml)
        .with_slide_rels(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdFill" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/fill.png"/>
</Relationships>"#,
        )
        .with_extra_file("ppt/media/fill.png", b"fill-data")
        .build();

    let slide = &parse_pptx(&pptx).slides[0];

    let styled_fill_shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Styled Fill Shape")
        .expect("styled fill shape");
    assert_eq!(
        styled_fill_shape.placeholder.as_ref().and_then(|ph| ph.idx),
        Some(1)
    );
    assert!(matches!(
        styled_fill_shape
            .placeholder
            .as_ref()
            .and_then(|ph| ph.ph_type.as_ref())
            .map(std::mem::discriminant::<PlaceholderType>),
        Some(kind) if kind == std::mem::discriminant(&PlaceholderType::Body)
    ));
    assert!(matches!(
        styled_fill_shape
            .style_ref
            .as_ref()
            .and_then(|style| style.font_ref.as_ref())
            .map(|style| style.idx.as_str()),
        Some("minor")
    ));
    assert!(matches!(
        styled_fill_shape.adjust_values.as_ref().and_then(|map| map.get("adj1")),
        Some(value) if (*value - 50000.0).abs() < 1e-6
    ));
    assert!(matches!(
        &styled_fill_shape.fill,
        Fill::Image(fill)
            if fill.rel_id == "rIdFill"
                && fill.content_type == "image/png"
                && fill.data.as_slice() == b"fill-data"
    ));

    let handle_shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Handle Shape")
        .expect("handle shape");
    let custom_geom = match &handle_shape.shape_type {
        ShapeType::CustomGeom(geom) => geom,
        other => panic!("expected custom geometry, got {other:?}"),
    };
    assert_eq!(custom_geom.adjust_handles.len(), 2);
    assert_eq!(custom_geom.connection_sites.len(), 1);
    assert!(matches!(custom_geom.paths[0].fill, PathFill::DarkenLess));
    assert!(custom_geom.paths[0].commands.iter().any(|command| matches!(
        command,
        pptx2html_core::model::PathCommand::QuadBezTo { .. }
    )));
    assert!(custom_geom.paths[0].commands.iter().any(|command| matches!(
        command,
        pptx2html_core::model::PathCommand::CubicBezTo { .. }
    )));
    assert_eq!(
        handle_shape
            .effects
            .outer_shadow
            .as_ref()
            .map(|shadow| shadow.color.modifiers.len()),
        Some(1)
    );
    assert_eq!(
        handle_shape
            .effects
            .glow
            .as_ref()
            .map(|glow| glow.color.modifiers.len()),
        Some(1)
    );

    let fallback_shape = slide
        .shapes
        .iter()
        .find(|shape| shape.name == "Effect Fallback Shape")
        .expect("effect fallback shape");
    assert_eq!(
        fallback_shape
            .effects
            .outer_shadow
            .as_ref()
            .and_then(|shadow| shadow.color.to_css())
            .as_deref(),
        Some("#000000")
    );
    assert_eq!(
        fallback_shape
            .effects
            .glow
            .as_ref()
            .and_then(|glow| glow.color.to_css())
            .as_deref(),
        Some("#FFC000")
    );
}
