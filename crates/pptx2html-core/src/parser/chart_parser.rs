use std::collections::{BTreeMap, BTreeSet};

use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use super::xml_utils;
use crate::error::PptxResult;
use crate::model::{
    ChartBubbleSizeRepresents, ChartDataLabelPosition, ChartDataLabelSettings, ChartGrouping,
    ChartLegendPosition, ChartMarkerSpec, ChartOfPieType, ChartRadarStyle, ChartScatterStyle,
    ChartSeries, ChartSpec, ChartSplitType, ChartTickMark, ChartType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChartFallbackReason {
    MissingRelationshipId,
    MissingRelationship,
    MissingPart,
    InvalidXml,
    InvalidAncestry,
    ChartEx,
    UnsupportedFamily,
    CombinationChart,
    IncompatibleAxes,
    IncompatibleSeries,
    InvalidCache,
    UnsupportedVariant,
    NoSeries,
}

impl ChartFallbackReason {
    pub(crate) fn is_structure_unsupported(self) -> bool {
        matches!(
            self,
            Self::CombinationChart | Self::IncompatibleAxes | Self::IncompatibleSeries
        )
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::MissingRelationshipId => "missing-relationship-id",
            Self::MissingRelationship => "missing-relationship",
            Self::MissingPart => "missing-chart-part",
            Self::InvalidXml => "invalid-chart-xml",
            Self::InvalidAncestry => "invalid-chart-ancestry",
            Self::ChartEx => "chartex",
            Self::UnsupportedFamily => "unsupported-family",
            Self::CombinationChart => "combination-chart",
            Self::IncompatibleAxes => "incompatible-axes",
            Self::IncompatibleSeries => "incompatible-series",
            Self::InvalidCache => "invalid-cache",
            Self::UnsupportedVariant => "unsupported-variant",
            Self::NoSeries => "no-series-data",
        }
    }
}

const CLASSIC_CHART_NS: &[u8] = b"http://schemas.openxmlformats.org/drawingml/2006/chart";
const CHARTEX_NS: &[u8] = b"http://schemas.microsoft.com/office/drawing/2014/chartex";
const DRAWING_NS: &[u8] = b"http://schemas.openxmlformats.org/drawingml/2006/main";

pub(crate) struct ChartParseOutcome {
    pub(crate) direct_spec: Option<ChartSpec>,
    pub(crate) fallback_reason: Option<ChartFallbackReason>,
    pub(crate) qualified_name: Option<String>,
    pub(crate) series_summary: String,
    pub(crate) element_inventory: String,
}

pub(crate) fn classify_and_parse(xml: &str) -> PptxResult<ChartParseOutcome> {
    let semantic_xml = classic_only_xml(xml)?;
    let classification = classify(xml, &semantic_xml)?;
    let series_summary = if classification.reason == Some(ChartFallbackReason::ChartEx) {
        summarize_chartex_series(xml)?
    } else {
        summarize_series(&semantic_xml)?
    };
    let element_inventory = classification.element_inventory.clone();
    if let Some(reason) = classification.reason {
        return Ok(ChartParseOutcome {
            direct_spec: None,
            fallback_reason: Some(reason),
            qualified_name: classification.qualified_name,
            series_summary,
            element_inventory,
        });
    }
    if !validate_caches(&semantic_xml)? {
        return Ok(ChartParseOutcome {
            direct_spec: None,
            fallback_reason: Some(ChartFallbackReason::InvalidCache),
            qualified_name: classification.qualified_name,
            series_summary,
            element_inventory,
        });
    }
    let Some(spec) = parse_direct_chart(&semantic_xml)? else {
        return Ok(ChartParseOutcome {
            direct_spec: None,
            fallback_reason: Some(ChartFallbackReason::NoSeries),
            qualified_name: classification.qualified_name,
            series_summary,
            element_inventory,
        });
    };
    let fallback_reason = direct_compatibility_failure(&spec);
    Ok(ChartParseOutcome {
        direct_spec: fallback_reason.is_none().then_some(spec),
        fallback_reason,
        qualified_name: classification.qualified_name,
        series_summary,
        element_inventory,
    })
}

struct Classification {
    reason: Option<ChartFallbackReason>,
    qualified_name: Option<String>,
    element_inventory: String,
}

fn classify(raw_xml: &str, semantic_xml: &str) -> PptxResult<Classification> {
    let ChartInventory {
        mut qualified_name,
        chartex_root_name,
        element_counts,
        saw_chartex,
    } = chart_inventory(raw_xml)?;
    let mut reader = NsReader::from_str(semantic_xml);
    let mut buffer = Vec::new();
    let mut families = Vec::new();
    let mut family_axis_ids = BTreeSet::new();
    let mut defined_axes = BTreeMap::new();
    let mut axis_crossings = BTreeMap::new();
    let mut family_depth = None;
    let mut axis_kind: Option<(usize, String)> = None;
    let mut axis_id: Option<String> = None;
    let mut depth = 0_usize;

    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, event @ (Event::Start(_) | Event::Empty(_)))) => {
                let has_end = matches!(event, Event::Start(_));
                let element = match &event {
                    Event::Start(element) | Event::Empty(element) => element,
                    _ => unreachable!(),
                };
                let element_name = element.name();
                let local = xml_utils::local_name(element_name.as_ref());
                let classic = matches!(namespace, ResolveResult::Bound(ref value) if value.as_ref() == CLASSIC_CHART_NS);
                if !classic {
                    buffer.clear();
                    continue;
                }
                if is_chart_family(local) {
                    families.push(local.to_owned());
                    if has_end {
                        family_depth = Some(depth);
                    }
                    qualified_name.get_or_insert_with(|| {
                        String::from_utf8_lossy(element.name().as_ref()).into_owned()
                    });
                } else if matches!(local, "catAx" | "dateAx" | "valAx" | "serAx") {
                    if has_end {
                        axis_kind = Some((depth, local.to_owned()));
                    }
                } else if local == "axId"
                    && let Some(value) = xml_utils::attr_str(element, "val")
                {
                    if family_depth.is_some() {
                        family_axis_ids.insert(value);
                    } else if let Some((_, kind)) = axis_kind.as_ref() {
                        defined_axes.insert(value.clone(), kind.clone());
                        axis_id = Some(value);
                    }
                } else if local == "crossAx"
                    && let Some(id) = axis_id.as_ref()
                    && let Some(crossing) = xml_utils::attr_str(element, "val")
                {
                    axis_crossings.insert(id.clone(), crossing);
                }
                if has_end {
                    depth += 1;
                }
            }
            Ok((namespace, Event::End(element))) => {
                depth = depth.saturating_sub(1);
                if !matches!(namespace, ResolveResult::Bound(ref value) if value.as_ref() == CLASSIC_CHART_NS)
                {
                    buffer.clear();
                    continue;
                }
                let element_name = element.name();
                let local = xml_utils::local_name(element_name.as_ref());
                if is_chart_family(local) && family_depth == Some(depth) {
                    family_depth = None;
                } else if matches!(local, "catAx" | "dateAx" | "valAx" | "serAx")
                    && axis_kind
                        .as_ref()
                        .is_some_and(|(axis_depth, _)| *axis_depth == depth)
                {
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

    if saw_chartex && qualified_name.is_none() {
        qualified_name = chartex_root_name;
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
    let element_inventory = element_counts
        .into_iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect::<Vec<_>>()
        .join(",");
    Ok(Classification {
        reason,
        qualified_name,
        element_inventory,
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

fn summarize_series(xml: &str) -> PptxResult<String> {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut series = 0_usize;
    let mut points = 0_usize;
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((
                ResolveResult::Bound(namespace),
                Event::Start(element) | Event::Empty(element),
            )) if namespace.as_ref() == CLASSIC_CHART_NS => {
                match xml_utils::local_name(element.name().as_ref()) {
                    "ser" => series += 1,
                    "pt" => points += 1,
                    _ => {}
                }
            }
            Ok((_, Event::Eof)) => break,
            Err(error) => return Err(crate::error::PptxError::Xml(error)),
            _ => {}
        }
        buffer.clear();
    }
    Ok(format!("series={series},cache_points={points}"))
}

fn summarize_chartex_series(xml: &str) -> PptxResult<String> {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut path: Vec<(PartNamespace, String)> = Vec::new();
    let mut series = 0_usize;
    let mut points = 0_usize;
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element))) => {
                let namespace = part_namespace(namespace);
                let element_name = element.name();
                let local = xml_utils::local_name(element_name.as_ref());
                count_chartex_semantics(&path, namespace, local, &mut series, &mut points);
                path.push((namespace, local.to_owned()));
            }
            Ok((namespace, Event::Empty(element))) => {
                let namespace = part_namespace(namespace);
                let element_name = element.name();
                let local = xml_utils::local_name(element_name.as_ref());
                count_chartex_semantics(&path, namespace, local, &mut series, &mut points);
            }
            Ok((_, Event::End(_))) => {
                path.pop();
            }
            Ok((_, Event::Eof)) => break,
            Err(error) => return Err(crate::error::PptxError::Xml(error)),
            _ => {}
        }
        buffer.clear();
    }
    Ok(format!("series={series},cache_points={points}"))
}

fn count_chartex_semantics(
    path: &[(PartNamespace, String)],
    namespace: PartNamespace,
    local: &str,
    series: &mut usize,
    points: &mut usize,
) {
    let exact_root = matches!(
        path,
        [
            (PartNamespace::ChartEx, chart_space),
            (PartNamespace::ChartEx, chart),
            (PartNamespace::ChartEx, plot_area),
            ..
        ] if chart_space == "chartSpace" && chart == "chart" && plot_area == "plotArea"
    );
    let official_descendants = path
        .iter()
        .skip(3)
        .all(|(namespace, _)| matches!(namespace, PartNamespace::ChartEx | PartNamespace::Drawing));
    if namespace == PartNamespace::ChartEx && exact_root && official_descendants {
        match local {
            "series" => *series += 1,
            "dataPt" => *points += 1,
            _ => {}
        }
    }
}

#[derive(Default)]
struct CacheValidation {
    numeric: bool,
    point_count: Option<usize>,
    indices: Vec<usize>,
    in_point: bool,
    in_value: bool,
    point_has_value: bool,
    valid: bool,
}

fn validate_caches(xml: &str) -> PptxResult<bool> {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut cache: Option<CacheValidation> = None;
    let mut saw_cache = false;
    let mut all_valid = true;
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((
                ResolveResult::Bound(namespace),
                Event::Start(element) | Event::Empty(element),
            )) if namespace.as_ref() == CLASSIC_CHART_NS => {
                let element_name = element.name();
                let local = xml_utils::local_name(element_name.as_ref());
                if matches!(local, "numLit" | "numCache" | "strLit" | "strCache") {
                    saw_cache = true;
                    cache = Some(CacheValidation {
                        numeric: matches!(local, "numLit" | "numCache"),
                        valid: true,
                        ..Default::default()
                    });
                } else if local == "ptCount"
                    && let Some(current) = cache.as_mut()
                {
                    current.point_count = xml_utils::attr_str(&element, "val")
                        .and_then(|value| value.parse::<usize>().ok());
                    current.valid &= current.point_count.is_some();
                } else if local == "pt"
                    && let Some(current) = cache.as_mut()
                {
                    let index = xml_utils::attr_str(&element, "idx")
                        .and_then(|value| value.parse::<usize>().ok());
                    current.valid &= index.is_some();
                    if let Some(index) = index {
                        current.indices.push(index);
                    }
                    current.in_point = !element.is_empty();
                    current.point_has_value = false;
                } else if local == "v"
                    && let Some(current) = cache.as_mut()
                    && current.in_point
                {
                    current.in_value = !element.is_empty();
                }
            }
            Ok((_, Event::Text(text))) => {
                if let Some(current) = cache.as_mut()
                    && current.in_value
                {
                    let value = text.unescape().unwrap_or_default();
                    current.point_has_value = true;
                    if current.numeric {
                        current.valid &= value.parse::<f64>().is_ok_and(f64::is_finite);
                    }
                }
            }
            Ok((ResolveResult::Bound(namespace), Event::End(element)))
                if namespace.as_ref() == CLASSIC_CHART_NS =>
            {
                match xml_utils::local_name(element.name().as_ref()) {
                    "v" => {
                        if let Some(current) = cache.as_mut() {
                            current.in_value = false;
                        }
                    }
                    "pt" => {
                        if let Some(current) = cache.as_mut() {
                            current.valid &= current.point_has_value;
                            current.in_point = false;
                        }
                    }
                    "numLit" | "numCache" | "strLit" | "strCache" => {
                        if let Some(current) = cache.take() {
                            let count = current.point_count.unwrap_or(usize::MAX);
                            let sequential =
                                current.indices.iter().copied().eq(0..current.indices.len());
                            all_valid &=
                                current.valid && count == current.indices.len() && sequential;
                        }
                    }
                    _ => {}
                }
            }
            Ok((_, Event::Eof)) => break,
            Err(error) => return Err(crate::error::PptxError::Xml(error)),
            _ => {}
        }
        buffer.clear();
    }
    Ok(saw_cache && cache.is_none() && all_valid)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartNamespace {
    Classic,
    ChartEx,
    Drawing,
    Other,
}

fn part_namespace(namespace: ResolveResult<'_>) -> PartNamespace {
    let ResolveResult::Bound(namespace) = namespace else {
        return PartNamespace::Other;
    };
    match namespace.as_ref() {
        CLASSIC_CHART_NS => PartNamespace::Classic,
        CHARTEX_NS => PartNamespace::ChartEx,
        DRAWING_NS => PartNamespace::Drawing,
        _ => PartNamespace::Other,
    }
}

struct ChartInventory {
    qualified_name: Option<String>,
    chartex_root_name: Option<String>,
    element_counts: BTreeMap<String, usize>,
    saw_chartex: bool,
}

fn chart_inventory(xml: &str) -> PptxResult<ChartInventory> {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut qualified_name = None;
    let mut chartex_root_name = None;
    let mut element_counts = BTreeMap::<String, usize>::new();
    let mut saw_chartex = false;
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element) | Event::Empty(element))) => {
                let namespace = part_namespace(namespace);
                let element_name = element.name();
                let local = xml_utils::local_name(element_name.as_ref());
                if matches!(namespace, PartNamespace::Classic | PartNamespace::ChartEx) {
                    let prefix = if namespace == PartNamespace::Classic {
                        "c"
                    } else {
                        "cx"
                    };
                    *element_counts
                        .entry(format!("{prefix}:{local}"))
                        .or_default() += 1;
                }
                if namespace == PartNamespace::ChartEx {
                    saw_chartex = true;
                    let candidate = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                    chartex_root_name.get_or_insert_with(|| candidate.clone());
                    if !matches!(local, "chartSpace" | "chart" | "plotArea") {
                        qualified_name.get_or_insert(candidate);
                    }
                }
            }
            Ok((_, Event::Eof)) => break,
            Err(error) => return Err(crate::error::PptxError::Xml(error)),
            _ => {}
        }
        buffer.clear();
    }
    Ok(ChartInventory {
        qualified_name,
        chartex_root_name,
        element_counts,
        saw_chartex,
    })
}

fn exact_classic_scaffold(path: &[(PartNamespace, String)]) -> bool {
    matches!(
        path,
        [
            (PartNamespace::Classic, chart_space),
            (PartNamespace::Classic, chart),
            (PartNamespace::Classic, plot_area),
        ] if chart_space == "chartSpace" && chart == "chart" && plot_area == "plotArea"
    )
}

fn include_semantic_element(
    path: &[(PartNamespace, String)],
    namespace: PartNamespace,
    local: &str,
    parent_included: bool,
) -> bool {
    match path {
        [] => namespace == PartNamespace::Classic && local == "chartSpace",
        [(PartNamespace::Classic, root)] => {
            parent_included
                && root == "chartSpace"
                && namespace == PartNamespace::Classic
                && matches!(local, "chart" | "txPr")
        }
        _ if path.len() >= 2 && path[0].1 == "chartSpace" && path[1].1 == "txPr" => {
            parent_included && matches!(namespace, PartNamespace::Classic | PartNamespace::Drawing)
        }
        [
            (PartNamespace::Classic, root),
            (PartNamespace::Classic, chart),
        ] => {
            parent_included
                && root == "chartSpace"
                && chart == "chart"
                && namespace == PartNamespace::Classic
                && matches!(local, "plotArea" | "legend" | "txPr")
        }
        _ if path.len() == 3 && exact_classic_scaffold(path) => {
            parent_included
                && namespace == PartNamespace::Classic
                && (is_chart_family(local)
                    || matches!(local, "catAx" | "dateAx" | "valAx" | "serAx"))
        }
        _ if path.len() >= 3
            && path[0].1 == "chartSpace"
            && path[1].1 == "chart"
            && matches!(path[2].1.as_str(), "legend" | "txPr") =>
        {
            parent_included && matches!(namespace, PartNamespace::Classic | PartNamespace::Drawing)
        }
        _ if path.len() > 3 && exact_classic_scaffold(&path[..3]) => {
            parent_included
                && matches!(namespace, PartNamespace::Classic | PartNamespace::Drawing)
                && !is_chart_family(local)
                && !matches!(local, "catAx" | "dateAx" | "valAx" | "serAx")
        }
        _ => false,
    }
}

fn classic_only_xml(xml: &str) -> PptxResult<String> {
    let mut reader = NsReader::from_str(xml);
    let mut writer = quick_xml::Writer::new(Vec::new());
    let mut buffer = Vec::new();
    let mut path: Vec<(PartNamespace, String)> = Vec::new();
    let mut included = Vec::new();
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element))) => {
                let namespace = part_namespace(namespace);
                let element_name = element.name();
                let local = xml_utils::local_name(element_name.as_ref()).to_owned();
                let keep = include_semantic_element(
                    &path,
                    namespace,
                    &local,
                    included.last().copied().unwrap_or(true),
                );
                if keep {
                    writer.write_event(Event::Start(element.into_owned()))?;
                }
                path.push((namespace, local));
                included.push(keep);
            }
            Ok((namespace, Event::Empty(element))) => {
                let namespace = part_namespace(namespace);
                let element_name = element.name();
                let local = xml_utils::local_name(element_name.as_ref());
                if include_semantic_element(
                    &path,
                    namespace,
                    local,
                    included.last().copied().unwrap_or(true),
                ) {
                    writer.write_event(Event::Empty(element.into_owned()))?;
                }
            }
            Ok((_, event @ Event::End(_))) => {
                path.pop();
                if included.pop().unwrap_or(false) {
                    writer.write_event(event.into_owned())?;
                }
            }
            Ok((_, event @ (Event::Text(_) | Event::CData(_))))
                if included.last().copied().unwrap_or(false) =>
            {
                writer.write_event(event.into_owned())?;
            }
            Ok((_, Event::Eof)) => break,
            Err(error) => return Err(crate::error::PptxError::Xml(error)),
            _ => {}
        }
        buffer.clear();
    }
    String::from_utf8(writer.into_inner())
        .map_err(|error| crate::error::PptxError::UnsupportedFormat(error.to_string()))
}

fn parse_direct_chart(xml: &str) -> PptxResult<Option<ChartSpec>> {
    let classic_xml = classic_only_xml(xml)?;
    let mut reader = Reader::from_str(&classic_xml);
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
    let mut value_axis_min = None;
    let mut value_axis_max = None;
    let mut value_axis_major_unit = None;
    let mut value_axis_major_gridlines = false;
    let mut value_axis_visible = true;
    let mut category_axis_major_tick_mark = ChartTickMark::None;
    let mut value_axis_major_tick_mark = ChartTickMark::None;
    let mut legend_position = None;
    let mut text_size_pt = None;
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
                    "min" if in_val_ax => {
                        value_axis_min =
                            xml_utils::attr_str(e, "val").and_then(|val| val.parse().ok());
                    }
                    "max" if in_val_ax => {
                        value_axis_max =
                            xml_utils::attr_str(e, "val").and_then(|val| val.parse().ok());
                    }
                    "majorUnit" if in_val_ax => {
                        value_axis_major_unit =
                            xml_utils::attr_str(e, "val").and_then(|val| val.parse().ok());
                    }
                    "majorGridlines" if in_val_ax => value_axis_major_gridlines = true,
                    "majorTickMark" if in_cat_ax => {
                        category_axis_major_tick_mark =
                            parse_chart_tick_mark(xml_utils::attr_str(e, "val").as_deref());
                    }
                    "majorTickMark" if in_val_ax => {
                        value_axis_major_tick_mark =
                            parse_chart_tick_mark(xml_utils::attr_str(e, "val").as_deref());
                    }
                    "delete" if in_val_ax => {
                        value_axis_visible = xml_utils::attr_str(e, "val")
                            .map(|val| !matches!(val.as_str(), "1" | "true"))
                            .unwrap_or(false);
                    }
                    "legendPos" => {
                        legend_position = Some(match xml_utils::attr_str(e, "val").as_deref() {
                            Some("l") => ChartLegendPosition::Left,
                            Some("t") => ChartLegendPosition::Top,
                            Some("b") => ChartLegendPosition::Bottom,
                            _ => ChartLegendPosition::Right,
                        });
                    }
                    "defRPr" => {
                        text_size_pt = xml_utils::attr_str(e, "sz")
                            .and_then(|value| value.parse::<f64>().ok())
                            .map(|value| value / 100.0);
                    }
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
            value_axis_min,
            value_axis_max,
            value_axis_major_unit,
            value_axis_major_gridlines,
            value_axis_visible,
            category_axis_major_tick_mark,
            value_axis_major_tick_mark,
            legend_position,
            text_size_pt,
            data_labels: saw_dlbls.then_some(data_labels),
            series,
        }))
    }
}

fn parse_chart_tick_mark(value: Option<&str>) -> ChartTickMark {
    match value {
        Some("in") => ChartTickMark::Inside,
        Some("out") => ChartTickMark::Outside,
        Some("cross") => ChartTickMark::Cross,
        _ => ChartTickMark::None,
    }
}
