use pptx2html_core::ConversionOptions;
use pptx2html_core::model::{
    Bullet, BulletChar, ChartData, ChartSeries, ChartSpec, ChartType, Color, Emu, Fill,
    OuterShadow, Presentation, Shape, ShapeEffects, ShapeType, Size, Slide, SolidFill, TableCell,
    TableData, TableRow, TextBody, TextParagraph, TextRun, UnresolvedType, UnsupportedData,
};
use pptx2html_core::renderer::HtmlRenderer;

fn render_shapes(shapes: Vec<Shape>) -> String {
    let presentation = Presentation {
        slide_size: Size {
            width: Emu(9_144_000),
            height: Emu(6_858_000),
        },
        slides: vec![Slide {
            shapes,
            ..Default::default()
        }],
        ..Default::default()
    };
    HtmlRenderer::render(&presentation).expect("renderer seam fixture should render")
}

fn text_body(text: &str) -> TextBody {
    TextBody {
        paragraphs: vec![TextParagraph {
            runs: vec![TextRun {
                text: text.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn sized_shape(shape_type: ShapeType) -> Shape {
    Shape {
        shape_type,
        size: Size {
            width: Emu(1_828_800),
            height: Emu(914_400),
        },
        ..Default::default()
    }
}

#[test]
fn fills_and_effects_keep_css_contract() {
    let mut shape = sized_shape(ShapeType::Rectangle);
    shape.fill = Fill::Solid(SolidFill {
        color: Color::rgb("336699"),
    });
    shape.effects = ShapeEffects {
        outer_shadow: Some(OuterShadow {
            blur_radius: 2.0,
            distance: 4.0,
            direction: 90.0,
            color: Color::rgb("112233"),
            alpha: 1.0,
        }),
        glow: None,
    };

    let html = render_shapes(vec![shape]);

    assert!(html.contains("background-color: #336699"));
    assert!(html.contains("box-shadow: 0.0pt 4.0pt 2.0pt #112233"));
}

#[test]
fn bullets_and_text_keep_markup_contract() {
    let mut shape = sized_shape(ShapeType::TextBox);
    shape.text_body = Some(TextBody {
        paragraphs: vec![TextParagraph {
            bullet: Some(Bullet::Char(BulletChar {
                char: "*".to_string(),
                font: Some("Arial".to_string()),
                size_pct: Some(1.25),
                color: Some(Color::rgb("AA0000")),
            })),
            runs: vec![TextRun {
                text: "Seam text".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    });

    let html = render_shapes(vec![shape]);

    assert!(html.contains(
        "<span class=\"bullet\" style=\"font-family: 'Arial'; color: #AA0000; font-size: 125%; \">* </span>"
    ));
    assert!(html.contains(">Seam text</span>"));
}

#[test]
fn tables_keep_structure_and_cell_style_contract() {
    let cell = TableCell {
        text_body: Some(text_body("Cell contract")),
        fill: Fill::Solid(SolidFill {
            color: Color::rgb("DDEEFF"),
        }),
        col_span: 2,
        ..Default::default()
    };
    let table = TableData {
        rows: vec![TableRow {
            height: 36.0,
            cells: vec![cell],
        }],
        col_widths: vec![2.0, 1.0],
        first_row: true,
        ..Default::default()
    };

    let html = render_shapes(vec![sized_shape(ShapeType::Table(table))]);

    assert!(html.contains("<col style=\"width:66.7%\"/>"));
    assert!(html.contains("<td colspan=\"2\" style=\"background-color: #DDEEFF"));
    assert!(html.contains(">Cell contract</span>"));
}

#[test]
fn charts_keep_direct_svg_contract() {
    let chart = ChartData {
        rel_id: "rIdChart".to_string(),
        direct_spec: Some(ChartSpec {
            chart_type: ChartType::Column,
            series: vec![ChartSeries {
                name: Some("Revenue".to_string()),
                categories: vec!["Q1".to_string(), "Q2".to_string()],
                values: vec![10.0, 20.0],
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    };

    let html = render_shapes(vec![sized_shape(ShapeType::Chart(chart))]);

    assert!(html.contains("<div class=\"chart-direct\">"));
    assert!(html.contains("<rect class=\"chart-bar\""));
    assert!(html.contains(
        "<span class=\"chart-legend-item\"><span class=\"chart-legend-swatch\" style=\"background:#4472C4\"></span>Revenue</span>"
    ));
}

#[test]
fn actions_and_hyperlinks_keep_anchor_wrapper_contract() {
    let mut shape = sized_shape(ShapeType::TextBox);
    shape.text_body = Some(TextBody {
        paragraphs: vec![TextParagraph {
            runs: vec![TextRun {
                text: "Open target".to_string(),
                hyperlink: Some("https://example.test/path?a=1&b=2".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    });

    let html = render_shapes(vec![shape]);

    assert!(
        html.contains("<a class=\"run\" href=\"https://example.test/path?a=1&amp;b=2\" style=\"")
    );
    assert!(html.contains(">Open target</span></a>"));
}

#[test]
fn smartart_and_ole_keep_placeholder_and_metadata_contract() {
    let shapes = vec![
        sized_shape(ShapeType::Unsupported(UnsupportedData {
            label: "SmartArt".to_string(),
            element_type: UnresolvedType::SmartArt,
            raw_xml: Some("<dgm:relIds r:dm=\"rId1\"/>".to_string()),
            custom_geometry: None,
        })),
        sized_shape(ShapeType::Unsupported(UnsupportedData {
            label: "OLE Object".to_string(),
            element_type: UnresolvedType::OleObject,
            raw_xml: Some("<p:oleObj progId=\"Excel.Sheet.12\"/>".to_string()),
            custom_geometry: None,
        })),
    ];
    let presentation = Presentation {
        slide_size: Size {
            width: Emu(9_144_000),
            height: Emu(6_858_000),
        },
        slides: vec![Slide {
            shapes,
            ..Default::default()
        }],
        ..Default::default()
    };

    let result =
        HtmlRenderer::render_with_options_metadata(&presentation, &ConversionOptions::default())
            .expect("unsupported renderer seam fixture should render");

    assert!(result.html.contains(
        "id=\"unresolved-s0-e0\" data-type=\"smartart\" data-slide=\"0\"><span>[SmartArt]</span>"
    ));
    assert!(result.html.contains(
        "id=\"unresolved-s0-e1\" data-type=\"ole\" data-slide=\"0\"><span>[OLE Object]</span>"
    ));
    assert_eq!(result.unresolved_elements.len(), 2);
    assert_eq!(result.unresolved_elements[0].slide_index, 0);
    assert_eq!(
        result.unresolved_elements[0].raw_xml.as_deref(),
        Some("<dgm:relIds r:dm=\"rId1\"/>")
    );
    assert_eq!(
        result.unresolved_elements[1].raw_xml.as_deref(),
        Some("<p:oleObj progId=\"Excel.Sheet.12\"/>")
    );
}
