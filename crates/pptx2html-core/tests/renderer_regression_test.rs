use pptx2html_core::ConversionOptions;
use pptx2html_core::model::presentation::{ColorScheme, Presentation, Theme};
use pptx2html_core::model::{
    ChartBubbleSizeRepresents, ChartData, ChartDataLabelPosition, ChartDataLabelSettings,
    ChartMarkerSpec, ChartOfPieType, ChartScatterStyle, ChartSeries, ChartSpec, ChartSplitType,
    ChartType, Emu, Fill, ImageFill, Shape, ShapeType, Size, Slide,
};
use pptx2html_core::renderer::HtmlRenderer;

fn chart_shape(spec: ChartSpec) -> Shape {
    Shape {
        shape_type: ShapeType::Chart(ChartData {
            rel_id: "rIdChart".to_string(),
            preview_image: None,
            preview_mime: None,
            direct_spec: Some(spec),
            ..Default::default()
        }),
        size: Size {
            width: Emu(1_828_800),
            height: Emu(914_400),
        },
        ..Default::default()
    }
}

#[test]
fn renderer_handles_chart_edge_case_branches_via_public_html_renderer() {
    let mut presentation = Presentation {
        slide_size: Size {
            width: Emu(9_144_000),
            height: Emu(6_858_000),
        },
        ..Default::default()
    };
    presentation.themes.push(Theme {
        name: "Theme".to_string(),
        color_scheme: ColorScheme {
            accent1: "4472C4".to_string(),
            accent2: "ED7D31".to_string(),
            accent3: "A5A5A5".to_string(),
            accent4: "FFC000".to_string(),
            accent5: "5B9BD5".to_string(),
            accent6: "70AD47".to_string(),
            ..Default::default()
        },
        ..Default::default()
    });

    presentation.slides.push(Slide {
        shapes: vec![
            chart_shape(ChartSpec {
                chart_type: ChartType::Line,
                data_labels: Some(ChartDataLabelSettings {
                    show_value: true,
                    position: Some(ChartDataLabelPosition::Center),
                    ..Default::default()
                }),
                series: vec![ChartSeries {
                    name: Some("Line".to_string()),
                    categories: vec!["Q1".to_string()],
                    values: vec![5.0],
                    marker: Some(ChartMarkerSpec {
                        symbol: Some("none".to_string()),
                        size: Some(6),
                    }),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            chart_shape(ChartSpec {
                chart_type: ChartType::Scatter,
                scatter_style: Some(ChartScatterStyle::Line),
                data_labels: Some(ChartDataLabelSettings {
                    show_value: true,
                    position: Some(ChartDataLabelPosition::Center),
                    ..Default::default()
                }),
                series: vec![ChartSeries {
                    name: Some("Scatter".to_string()),
                    x_values: vec![1.0],
                    values: vec![2.0],
                    marker: Some(ChartMarkerSpec {
                        symbol: Some("none".to_string()),
                        size: Some(8),
                    }),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            chart_shape(ChartSpec {
                chart_type: ChartType::Bubble,
                bubble_scale: Some(150.0),
                bubble_size_represents: Some(ChartBubbleSizeRepresents::Area),
                series: vec![ChartSeries {
                    name: Some("Bubble".to_string()),
                    x_values: vec![f64::NAN],
                    values: vec![f64::NAN],
                    bubble_sizes: vec![4.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            chart_shape(ChartSpec {
                chart_type: ChartType::Area,
                data_labels: Some(ChartDataLabelSettings {
                    show_value: true,
                    position: Some(ChartDataLabelPosition::Center),
                    ..Default::default()
                }),
                series: vec![ChartSeries {
                    name: Some("Area".to_string()),
                    categories: vec!["Only".to_string()],
                    values: vec![1.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            chart_shape(ChartSpec {
                chart_type: ChartType::OfPie,
                of_pie_type: Some(ChartOfPieType::Pie),
                split_type: Some(ChartSplitType::Pos),
                split_pos: Some(1.0),
                second_pie_size: Some(75),
                series: vec![ChartSeries {
                    name: Some("Split".to_string()),
                    categories: vec!["A".to_string(), "B".to_string()],
                    values: vec![5.0, 0.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
        ],
        ..Default::default()
    });

    let html = HtmlRenderer::render(&presentation).expect("render should succeed");

    assert!(html.contains("data-label-position=\"ctr\""));
    assert!(html.contains("chart-bubble"));
    assert!(html.contains("chart-of-pie-primary"));
    assert!(html.contains("chart-area"));
    assert!(html.contains("chart-line"));
}

#[test]
fn renderer_assigns_expected_external_asset_extensions_via_public_wrappers() {
    let mut presentation = Presentation {
        slide_size: Size {
            width: Emu(9_144_000),
            height: Emu(6_858_000),
        },
        ..Default::default()
    };
    presentation.slides.push(Slide {
        shapes: vec![
            Shape {
                shape_type: ShapeType::Rectangle,
                fill: Fill::Image(ImageFill {
                    rel_id: "rIdJpg".to_string(),
                    content_type: "image/jpeg".to_string(),
                    data: vec![1, 2, 3],
                }),
                size: Size {
                    width: Emu(457_200),
                    height: Emu(228_600),
                },
                ..Default::default()
            },
            Shape {
                shape_type: ShapeType::Rectangle,
                fill: Fill::Image(ImageFill {
                    rel_id: "rIdGif".to_string(),
                    content_type: "image/gif".to_string(),
                    data: vec![1, 2, 3],
                }),
                size: Size {
                    width: Emu(457_200),
                    height: Emu(228_600),
                },
                ..Default::default()
            },
            Shape {
                shape_type: ShapeType::Rectangle,
                fill: Fill::Image(ImageFill {
                    rel_id: "rIdSvg".to_string(),
                    content_type: "image/svg+xml".to_string(),
                    data: vec![1, 2, 3],
                }),
                size: Size {
                    width: Emu(457_200),
                    height: Emu(228_600),
                },
                ..Default::default()
            },
            Shape {
                shape_type: ShapeType::Rectangle,
                fill: Fill::Image(ImageFill {
                    rel_id: "rIdWebp".to_string(),
                    content_type: "image/webp".to_string(),
                    data: vec![1, 2, 3],
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

    let html = HtmlRenderer::render_with_options(
        &presentation,
        &ConversionOptions {
            embed_images: false,
            ..Default::default()
        },
    )
    .expect("render_with_options should succeed");
    assert!(html.contains("images/slide-1/background-0.jpg"));

    let result = HtmlRenderer::render_with_options_metadata(
        &presentation,
        &ConversionOptions {
            embed_images: false,
            ..Default::default()
        },
    )
    .expect("render_with_options_metadata should succeed");

    let paths = result
        .external_assets
        .iter()
        .map(|asset| asset.relative_path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.iter().any(|path| path.ends_with(".jpg")));
    assert!(paths.iter().any(|path| path.ends_with(".gif")));
    assert!(paths.iter().any(|path| path.ends_with(".svg")));
    assert!(paths.iter().any(|path| path.ends_with(".webp")));
}
