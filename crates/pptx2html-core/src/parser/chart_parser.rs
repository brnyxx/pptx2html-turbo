use std::collections::{BTreeMap, BTreeSet};

use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use super::xml_utils;
use crate::error::PptxResult;
use crate::model::{
    ChartBubbleSizeRepresents, ChartDataLabelPosition, ChartDataLabelSettings, ChartFallbackReason,
    ChartGrouping, ChartMarkerSpec, ChartOfPieType, ChartRadarStyle, ChartScatterStyle,
    ChartSeries, ChartSpec, ChartSplitType, ChartType,
};

const CLASSIC_CHART_NS: &[u8] = b"http://schemas.openxmlformats.org/drawingml/2006/chart";
const CHARTEX_NS: &[u8] = b"http://schemas.microsoft.com/office/drawing/2014/chartex";

pub(crate) struct ChartParseOutcome {
    pub(crate) direct_spec: Option<ChartSpec>,
    pub(crate) fallback_reason: Option<ChartFallbackReason>,
    pub(crate) qualified_name: Option<String>,
}

pub(crate) fn classify_and_parse(xml: &str) -> PptxResult<ChartParseOutcome> {
    let classification = classify(xml)?;
    if let Some(reason) = classification.reason {
        return Ok(ChartParseOutcome {
            direct_spec: None,
            fallback_reason: Some(reason),
            qualified_name: classification.qualified_name,
        });
    }

    let Some(spec) = parse_direct_chart(xml)? else {
        return Ok(ChartParseOutcome {
            direct_spec: None,
            fallback_reason: Some(ChartFallbackReason::NoSeries),
            qualified_name: classification.qualified_name,
        });
    };
    let fallback_reason = direct_compatibility_failure(&spec);
    Ok(ChartParseOutcome {
        direct_spec: fallback_reason.is_none().then_some(spec),
        fallback_reason,
        qualified_name: classification.qualified_name,
    })
}

struct Classification {
    reason: Option<ChartFallbackReason>,
    qualified_name: Option<String>,
}

fn classify(xml: &str) -> PptxResult<Classification> {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut families = Vec::new();
    let mut family_axis_ids = BTreeSet::new();
    let mut defined_axes = BTreeMap::new();
    let mut axis_crossings = BTreeMap::new();
    let mut in_family = false;
    let mut axis_kind: Option<String> = None;
    let mut axis_id: Option<String> = None;
    let mut qualified_name = None;
    let mut saw_chartex = false;

    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element) | Event::Empty(element))) => {
                let element_name = element.name();
                let local = xml_utils::local_name(element_name.as_ref());
                let classic = matches!(namespace, ResolveResult::Bound(ref value) if value.as_ref() == CLASSIC_CHART_NS);
                let chartex = matches!(namespace, ResolveResult::Bound(ref value) if value.as_ref() == CHARTEX_NS);
                if chartex {
                    saw_chartex = true;
                    qualified_name.get_or_insert_with(|| {
                        String::from_utf8_lossy(element.name().as_ref()).into_owned()
                    });
                }
                if !classic {
                    buffer.clear();
                    continue;
                }
                if is_chart_family(local) {
                    families.push(local.to_owned());
                    in_family = !element.is_empty();
                    qualified_name.get_or_insert_with(|| {
                        String::from_utf8_lossy(element.name().as_ref()).into_owned()
                    });
                } else if matches!(local, "catAx" | "dateAx" | "valAx" | "serAx") {
                    if !element.is_empty() {
                        axis_kind = Some(local.to_owned());
                    }
                } else if local == "axId"
                    && let Some(value) = xml_utils::attr_str(&element, "val")
                {
                    if in_family {
                        family_axis_ids.insert(value);
                    } else if let Some(kind) = axis_kind.as_ref() {
                        defined_axes.insert(value.clone(), kind.clone());
                        axis_id = Some(value);
                    }
                } else if local == "crossAx"
                    && let Some(id) = axis_id.as_ref()
                    && let Some(crossing) = xml_utils::attr_str(&element, "val")
                {
                    axis_crossings.insert(id.clone(), crossing);
                }
            }
            Ok((namespace, Event::End(element))) => {
                if !matches!(namespace, ResolveResult::Bound(ref value) if value.as_ref() == CLASSIC_CHART_NS)
                {
                    buffer.clear();
                    continue;
                }
                let element_name = element.name();
                let local = xml_utils::local_name(element_name.as_ref());
                if is_chart_family(local) {
                    in_family = false;
                } else if matches!(local, "catAx" | "dateAx" | "valAx" | "serAx") {
                    axis_kind = None;
                    axis_id = None;
                }
            }
            Ok((_, Event::Eof)) => break,
            Err(error) => return Err(crate::error::PptxError::Xml(error)),
            _ => {}
        }
        buffer.clear();
    }

    let reason = if saw_chartex {
        Some(ChartFallbackReason::ChartEx)
    } else if families.is_empty() {
        Some(ChartFallbackReason::UnsupportedFamily)
    } else if families.len() > 1 {
        Some(ChartFallbackReason::CombinationChart)
    } else if !is_direct_family(&families[0]) {
        Some(ChartFallbackReason::UnsupportedFamily)
    } else if family_requires_axes(&families[0])
        && !axes_are_compatible(
            &families[0],
            &family_axis_ids,
            &defined_axes,
            &axis_crossings,
        )
    {
        Some(ChartFallbackReason::IncompatibleAxes)
    } else {
        None
    };
    Ok(Classification {
        reason,
        qualified_name,
    })
}

fn is_chart_family(local: &str) -> bool {
    matches!(
        local,
        "areaChart"
            | "area3DChart"
            | "barChart"
            | "bar3DChart"
            | "bubbleChart"
            | "doughnutChart"
            | "lineChart"
            | "line3DChart"
            | "ofPieChart"
            | "pieChart"
            | "pie3DChart"
            | "radarChart"
            | "scatterChart"
            | "stockChart"
            | "surfaceChart"
            | "surface3DChart"
    )
}

fn is_direct_family(local: &str) -> bool {
    matches!(
        local,
        "areaChart"
            | "area3DChart"
            | "barChart"
            | "bar3DChart"
            | "bubbleChart"
            | "doughnutChart"
            | "lineChart"
            | "line3DChart"
            | "ofPieChart"
            | "pieChart"
            | "pie3DChart"
            | "radarChart"
            | "scatterChart"
    )
}

fn family_requires_axes(local: &str) -> bool {
    matches!(
        local,
        "areaChart"
            | "area3DChart"
            | "barChart"
            | "bar3DChart"
            | "bubbleChart"
            | "lineChart"
            | "line3DChart"
            | "scatterChart"
    )
}

fn axes_are_compatible(
    family: &str,
    references: &BTreeSet<String>,
    definitions: &BTreeMap<String, String>,
    crossings: &BTreeMap<String, String>,
) -> bool {
    if references.len() != 2
        || !references.iter().all(|id| definitions.contains_key(id))
        || !references.iter().all(|id| {
            crossings
                .get(id)
                .is_some_and(|crossing| crossing != id && references.contains(crossing))
        })
    {
        return false;
    }
    let kinds = references
        .iter()
        .filter_map(|id| definitions.get(id).map(String::as_str))
        .collect::<Vec<_>>();
    if matches!(family, "scatterChart" | "bubbleChart") {
        kinds.iter().all(|kind| *kind == "valAx")
    } else {
        kinds.iter().filter(|kind| **kind == "valAx").count() == 1
            && kinds
                .iter()
                .filter(|kind| matches!(**kind, "catAx" | "dateAx"))
                .count()
                == 1
    }
}

fn direct_compatibility_failure(spec: &ChartSpec) -> Option<ChartFallbackReason> {
    let first = spec.series.first()?;
    let compatible = match spec.chart_type {
        ChartType::Scatter => {
            let count = first.x_values.len();
            count > 0
                && spec
                    .series
                    .iter()
                    .all(|series| series.x_values.len() == count && series.values.len() == count)
        }
        ChartType::Bubble => {
            let count = first.x_values.len();
            count > 0
                && spec.series.iter().all(|series| {
                    series.x_values.len() == count
                        && series.values.len() == count
                        && series.bubble_sizes.len() == count
                })
        }
        _ => {
            let count = first.categories.len();
            count > 0
                && spec.series.iter().all(|series| {
                    series.categories.len() == count
                        && series.values.len() == count
                        && series.categories == first.categories
                })
        }
    };
    if !compatible {
        return Some(ChartFallbackReason::IncompatibleSeries);
    }
    let supported_variant = match spec.chart_type {
        ChartType::Area => !matches!(
            spec.grouping,
            ChartGrouping::Stacked | ChartGrouping::PercentStacked
        ),
        ChartType::Bubble => {
            spec.series.len() == 1
                && spec.data_labels.is_none()
                && spec
                    .series
                    .iter()
                    .all(|series| series.bubble_sizes.iter().all(|size| *size >= 0.0))
                && !matches!(
                    spec.bubble_size_represents,
                    Some(ChartBubbleSizeRepresents::Width)
                )
        }
        ChartType::OfPie => {
            spec.series.len() == 1
                && spec.data_labels.is_none()
                && matches!(spec.of_pie_type, Some(ChartOfPieType::Pie))
                && matches!(spec.split_type, Some(ChartSplitType::Pos))
                && spec.split_pos.is_some_and(|value| value >= 1.0)
        }
        ChartType::Radar => spec.data_labels.is_none(),
        ChartType::Pie | ChartType::Doughnut => spec.series.len() == 1,
        _ => true,
    };
    (!supported_variant).then_some(ChartFallbackReason::UnsupportedVariant)
}

#[derive(Default)]
struct SeriesBuilder {
    name: Option<String>,
    categories: Vec<String>,
    x_values: Vec<f64>,
    values: Vec<f64>,
    bubble_sizes: Vec<f64>,
    marker: Option<ChartMarkerSpec>,
}

fn parse_direct_chart(xml: &str) -> PptxResult<Option<ChartSpec>> {
    let mut reader = Reader::from_str(xml);
    let mut in_bar_chart = false;
    let mut in_line_chart = false;
    let mut in_scatter_chart = false;
    let mut in_bubble_chart = false;
    let mut in_area_chart = false;
    let mut in_radar_chart = false;
    let mut in_of_pie_chart = false;
    let mut in_pie_chart = false;
    let mut in_doughnut_chart = false;
    let mut chart_type = ChartType::Column;
    let mut grouping = ChartGrouping::Clustered;
    let mut of_pie_type = None;
    let mut split_type = None;
    let mut split_pos = None;
    let mut second_pie_size = None;
    let mut scatter_style = None;
    let mut bubble_scale = None;
    let mut bubble_size_represents = None;
    let mut show_neg_bubbles = None;
    let mut radar_style = None;
    let mut gap_width = None;
    let mut overlap = None;
    let mut hole_size = None;
    let mut current_series: Option<SeriesBuilder> = None;
    let mut series = Vec::new();
    let mut category_axis_title = String::new();
    let mut value_axis_title = String::new();
    let mut data_labels = ChartDataLabelSettings::default();
    let mut saw_dlbls = false;
    let mut in_tx = false;
    let mut in_cat = false;
    let mut in_val = false;
    let mut in_x_val = false;
    let mut in_y_val = false;
    let mut in_bubble_size = false;
    let mut in_pt = false;
    let mut in_v = false;
    let mut in_marker = false;
    let mut in_cat_ax = false;
    let mut in_val_ax = false;
    let mut in_title = false;
    let mut in_title_text = false;
    let mut in_dlbls = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = xml_utils::local_name(e.name().as_ref()).to_string();
                match local.as_str() {
                    "barChart" | "bar3DChart" => in_bar_chart = true,
                    "lineChart" | "line3DChart" => {
                        in_line_chart = true;
                        chart_type = ChartType::Line;
                    }
                    "scatterChart" => {
                        in_scatter_chart = true;
                        chart_type = ChartType::Scatter;
                        grouping = ChartGrouping::Standard;
                        scatter_style = Some(ChartScatterStyle::Marker);
                    }
                    "bubbleChart" => {
                        in_bubble_chart = true;
                        chart_type = ChartType::Bubble;
                        grouping = ChartGrouping::Standard;
                    }
                    "bubbleScale" if in_bubble_chart => {
                        bubble_scale = xml_utils::attr_str(e, "val").and_then(|val| {
                            let normalized = val.trim_end_matches('%');
                            normalized
                                .parse::<f64>()
                                .ok()
                                .map(|parsed| parsed.clamp(0.0, 300.0))
                        });
                    }
                    "sizeRepresents" if in_bubble_chart => {
                        bubble_size_represents =
                            Some(match xml_utils::attr_str(e, "val").as_deref() {
                                Some("w") => ChartBubbleSizeRepresents::Width,
                                _ => ChartBubbleSizeRepresents::Area,
                            });
                    }
                    "showNegBubbles" if in_bubble_chart => {
                        show_neg_bubbles = xml_utils::attr_str(e, "val")
                            .map(|val| matches!(val.as_str(), "1" | "true"));
                    }
                    "scatterStyle" if in_scatter_chart => {
                        scatter_style = Some(match xml_utils::attr_str(e, "val").as_deref() {
                            Some("none") => ChartScatterStyle::None,
                            Some("line") => ChartScatterStyle::Line,
                            Some("lineMarker") => ChartScatterStyle::LineMarker,
                            Some("smooth") => ChartScatterStyle::Smooth,
                            Some("smoothMarker") => ChartScatterStyle::SmoothMarker,
                            _ => ChartScatterStyle::Marker,
                        });
                    }
                    "areaChart" | "area3DChart" => {
                        in_area_chart = true;
                        chart_type = ChartType::Area;
                        grouping = ChartGrouping::Standard;
                    }
                    "radarChart" => {
                        in_radar_chart = true;
                        chart_type = ChartType::Radar;
                        grouping = ChartGrouping::Standard;
                    }
                    "radarStyle" if in_radar_chart => {
                        radar_style = Some(match xml_utils::attr_str(e, "val").as_deref() {
                            Some("marker") => ChartRadarStyle::Marker,
                            Some("filled") => ChartRadarStyle::Filled,
                            _ => ChartRadarStyle::Standard,
                        });
                    }
                    "ofPieChart" => {
                        in_of_pie_chart = true;
                        chart_type = ChartType::OfPie;
                        grouping = ChartGrouping::Standard;
                    }
                    "ofPieType" if in_of_pie_chart => {
                        of_pie_type = Some(match xml_utils::attr_str(e, "val").as_deref() {
                            Some("bar") => ChartOfPieType::Bar,
                            _ => ChartOfPieType::Pie,
                        });
                    }
                    "splitType" if in_of_pie_chart => {
                        split_type = Some(match xml_utils::attr_str(e, "val").as_deref() {
                            Some("pos") => ChartSplitType::Pos,
                            Some("percent") => ChartSplitType::Percent,
                            Some("val") => ChartSplitType::Value,
                            Some("cust") => ChartSplitType::Custom,
                            _ => ChartSplitType::Auto,
                        });
                    }
                    "splitPos" if in_of_pie_chart => {
                        split_pos =
                            xml_utils::attr_str(e, "val").and_then(|val| val.parse::<f64>().ok());
                    }
                    "secondPieSize" if in_of_pie_chart => {
                        second_pie_size = xml_utils::attr_str(e, "val")
                            .and_then(|val| val.parse::<i32>().ok())
                            .map(|val| val.clamp(5, 200));
                    }
                    "pieChart" | "pie3DChart" => {
                        in_pie_chart = true;
                        chart_type = ChartType::Pie;
                    }
                    "doughnutChart" => {
                        in_doughnut_chart = true;
                        chart_type = ChartType::Doughnut;
                    }
                    "barDir" if in_bar_chart => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            chart_type = if val == "bar" {
                                ChartType::Bar
                            } else {
                                ChartType::Column
                            };
                        }
                    }
                    "grouping" if in_bar_chart || in_line_chart || in_area_chart => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            grouping = match val.as_str() {
                                "stacked" => ChartGrouping::Stacked,
                                "percentStacked" => ChartGrouping::PercentStacked,
                                "standard" => ChartGrouping::Standard,
                                _ => ChartGrouping::Clustered,
                            };
                        }
                    }
                    "gapWidth" if in_bar_chart || in_of_pie_chart => {
                        gap_width = xml_utils::attr_str(e, "val")
                            .and_then(|val| val.parse::<i32>().ok())
                            .map(|val| val.clamp(0, 500));
                    }
                    "overlap" if in_bar_chart => {
                        overlap = xml_utils::attr_str(e, "val")
                            .and_then(|val| val.parse::<i32>().ok())
                            .map(|val| val.clamp(-100, 100));
                    }
                    "holeSize" if in_doughnut_chart => {
                        hole_size = xml_utils::attr_str(e, "val")
                            .and_then(|val| val.parse::<i32>().ok())
                            .map(|val| val.clamp(10, 90));
                    }
                    "dLbls"
                        if in_bar_chart
                            || in_line_chart
                            || in_scatter_chart
                            || in_bubble_chart
                            || in_area_chart
                            || in_radar_chart
                            || in_of_pie_chart
                            || in_pie_chart
                            || in_doughnut_chart =>
                    {
                        in_dlbls = true;
                        saw_dlbls = true;
                    }
                    "showVal" if in_dlbls => {
                        data_labels.show_value = xml_utils::attr_str(e, "val")
                            .map(|val| val != "0")
                            .unwrap_or(true);
                    }
                    "showCatName" if in_dlbls => {
                        data_labels.show_category_name = xml_utils::attr_str(e, "val")
                            .map(|val| val != "0")
                            .unwrap_or(true);
                    }
                    "showSerName" if in_dlbls => {
                        data_labels.show_series_name = xml_utils::attr_str(e, "val")
                            .map(|val| val != "0")
                            .unwrap_or(true);
                    }
                    "showPercent" if in_dlbls => {
                        data_labels.show_percent = xml_utils::attr_str(e, "val")
                            .map(|val| val != "0")
                            .unwrap_or(true);
                    }
                    "dLblPos" if in_dlbls => {
                        data_labels.position = match xml_utils::attr_str(e, "val").as_deref() {
                            Some("ctr") => Some(ChartDataLabelPosition::Center),
                            Some("inEnd") => Some(ChartDataLabelPosition::InEnd),
                            Some("outEnd") => Some(ChartDataLabelPosition::OutEnd),
                            _ => None,
                        };
                    }
                    "catAx" => in_cat_ax = true,
                    "valAx" => in_val_ax = true,
                    "title" if in_cat_ax || in_val_ax => in_title = true,
                    "t" if in_title => in_title_text = true,
                    "ser"
                        if in_bar_chart
                            || in_line_chart
                            || in_scatter_chart
                            || in_bubble_chart
                            || in_area_chart
                            || in_radar_chart
                            || in_of_pie_chart
                            || in_pie_chart
                            || in_doughnut_chart =>
                    {
                        current_series = Some(SeriesBuilder::default())
                    }
                    "marker"
                        if current_series.is_some()
                            && (in_line_chart || in_scatter_chart || in_radar_chart) =>
                    {
                        in_marker = true
                    }
                    "symbol" if current_series.is_some() && in_marker => {
                        if let Some(symbol) = xml_utils::attr_str(e, "val")
                            && let Some(series_builder) = current_series.as_mut()
                        {
                            let marker = series_builder
                                .marker
                                .get_or_insert_with(ChartMarkerSpec::default);
                            marker.symbol = Some(symbol);
                        }
                    }
                    "size" if current_series.is_some() && in_marker => {
                        if let Some(size) = xml_utils::attr_str(e, "val")
                            .and_then(|val| val.parse::<i32>().ok())
                            .map(|val| val.clamp(2, 72))
                            && let Some(series_builder) = current_series.as_mut()
                        {
                            let marker = series_builder
                                .marker
                                .get_or_insert_with(ChartMarkerSpec::default);
                            marker.size = Some(size);
                        }
                    }
                    "tx" if current_series.is_some() => in_tx = true,
                    "cat" if current_series.is_some() => in_cat = true,
                    "val" if current_series.is_some() => in_val = true,
                    "xVal" if current_series.is_some() && (in_scatter_chart || in_bubble_chart) => {
                        in_x_val = true
                    }
                    "yVal" if current_series.is_some() && (in_scatter_chart || in_bubble_chart) => {
                        in_y_val = true
                    }
                    "bubbleSize" if current_series.is_some() && in_bubble_chart => {
                        in_bubble_size = true
                    }
                    "pt" if current_series.is_some() => in_pt = true,
                    "v" if current_series.is_some() => in_v = true,
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if current_series.is_some() && in_v => {
                let text = e.unescape().unwrap_or_default().to_string();
                if let Some(series_builder) = current_series.as_mut() {
                    if in_tx && !in_cat && !in_val {
                        if !text.trim().is_empty() {
                            series_builder.name = Some(text);
                        }
                    } else if in_pt
                        && in_x_val
                        && let Ok(value) = text.parse::<f64>()
                    {
                        series_builder.x_values.push(value);
                    } else if in_pt
                        && in_y_val
                        && let Ok(value) = text.parse::<f64>()
                    {
                        series_builder.values.push(value);
                    } else if in_pt
                        && in_bubble_size
                        && let Ok(value) = text.parse::<f64>()
                    {
                        series_builder.bubble_sizes.push(value);
                    } else if in_pt && in_cat {
                        series_builder.categories.push(text);
                    } else if in_pt
                        && in_val
                        && let Ok(value) = text.parse::<f64>()
                    {
                        series_builder.values.push(value);
                    }
                }
            }
            Ok(Event::Text(ref e)) if in_title_text => {
                let text = e.unescape().unwrap_or_default().to_string();
                if !text.trim().is_empty() {
                    if in_cat_ax {
                        category_axis_title.push_str(&text);
                    } else if in_val_ax {
                        value_axis_title.push_str(&text);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let local = xml_utils::local_name(e.name().as_ref()).to_string();
                match local.as_str() {
                    "v" => in_v = false,
                    "t" => in_title_text = false,
                    "pt" => in_pt = false,
                    "tx" => in_tx = false,
                    "cat" => in_cat = false,
                    "val" => in_val = false,
                    "xVal" => in_x_val = false,
                    "yVal" => in_y_val = false,
                    "title" => in_title = false,
                    "dLbls" => in_dlbls = false,
                    "catAx" => in_cat_ax = false,
                    "valAx" => in_val_ax = false,
                    "ser" => {
                        if let Some(series_builder) = current_series.take()
                            && ((!series_builder.categories.is_empty()
                                && !series_builder.values.is_empty())
                                || (!series_builder.x_values.is_empty()
                                    && !series_builder.values.is_empty()))
                        {
                            series.push(ChartSeries {
                                name: series_builder.name,
                                categories: series_builder.categories,
                                x_values: series_builder.x_values,
                                values: series_builder.values,
                                bubble_sizes: series_builder.bubble_sizes,
                                marker: series_builder.marker,
                            });
                        }
                    }
                    "marker" => in_marker = false,
                    "barChart" | "bar3DChart" => in_bar_chart = false,
                    "lineChart" | "line3DChart" => in_line_chart = false,
                    "scatterChart" => in_scatter_chart = false,
                    "bubbleChart" => in_bubble_chart = false,
                    "areaChart" | "area3DChart" => in_area_chart = false,
                    "radarChart" => in_radar_chart = false,
                    "ofPieChart" => in_of_pie_chart = false,
                    "pieChart" | "pie3DChart" => in_pie_chart = false,
                    "doughnutChart" => in_doughnut_chart = false,
                    "bubbleSize" => in_bubble_size = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(crate::error::PptxError::Xml(e)),
            _ => {}
        }
    }

    if series.is_empty() {
        Ok(None)
    } else {
        Ok(Some(ChartSpec {
            chart_type,
            grouping,
            of_pie_type,
            split_type,
            split_pos,
            second_pie_size,
            scatter_style,
            bubble_scale,
            bubble_size_represents,
            show_neg_bubbles,
            radar_style,
            gap_width,
            overlap,
            hole_size,
            category_axis_title: (!category_axis_title.trim().is_empty())
                .then_some(category_axis_title),
            value_axis_title: (!value_axis_title.trim().is_empty()).then_some(value_axis_title),
            data_labels: saw_dlbls.then_some(data_labels),
            series,
        }))
    }
}
