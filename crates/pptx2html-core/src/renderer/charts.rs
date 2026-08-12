use std::fmt::Write;

use base64::Engine;

use super::{
    ChartData, ChartDataLabelPosition, ChartGrouping, ChartScatterStyle, ChartType, RenderCtx,
    escape_html,
};

pub(super) fn render_chart(
    chart_data: &ChartData,
    ctx: &RenderCtx<'_>,
    w: f64,
    h: f64,
    html: &mut String,
) {
    if let Some(ref spec) = chart_data.direct_spec
        && let Some(first_series) = spec.series.first()
    {
        let category_count = first_series.categories.len();
        let all_series_compatible = match spec.chart_type {
            ChartType::Scatter => {
                let point_count = first_series.x_values.len();
                point_count > 0
                    && spec.series.iter().all(|series| {
                        series.x_values.len() == point_count && series.values.len() == point_count
                    })
            }
            ChartType::Bubble => {
                let point_count = first_series.x_values.len();
                point_count > 0
                    && spec.series.iter().all(|series| {
                        series.x_values.len() == point_count
                            && series.values.len() == point_count
                            && series.bubble_sizes.len() == point_count
                    })
            }
            _ => {
                category_count > 0
                    && spec.series.iter().all(|series| {
                        series.categories.len() == category_count
                            && series.values.len() == category_count
                            && series.categories == first_series.categories
                    })
            }
        };
        let direct_chart_supported = all_series_compatible
            && match spec.chart_type {
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
                            Some(crate::model::ChartBubbleSizeRepresents::Width)
                        )
                }
                ChartType::OfPie => {
                    spec.series.len() == 1
                        && spec.data_labels.is_none()
                        && matches!(spec.of_pie_type, Some(crate::model::ChartOfPieType::Pie))
                        && matches!(spec.split_type, Some(crate::model::ChartSplitType::Pos))
                        && spec.split_pos.is_some_and(|value| value >= 1.0)
                }
                ChartType::Radar => !spec.series.is_empty() && spec.data_labels.is_none(),
                ChartType::Pie | ChartType::Doughnut => spec.series.len() == 1,
                _ => true,
            };

        if direct_chart_supported {
            let palette = [
                "#4472C4", "#ED7D31", "#A5A5A5", "#FFC000", "#5B9BD5", "#70AD47",
            ];
            let max_value = spec
                .series
                .iter()
                .flat_map(|series| series.values.iter().copied())
                .fold(0.0_f64, f64::max)
                .max(1.0);
            let chart_height = (h - 52.0).max(60.0);
            let series_count = spec.series.len().max(1) as f64;
            let gap_width = spec.gap_width.unwrap_or(150).clamp(0, 500);
            let overlap = spec.overlap.unwrap_or(0).clamp(-100, 100);
            let overlap_ratio = (overlap as f64 + 100.0) / 200.0;
            let build_bar_data_label =
                |series_name: Option<&str>,
                 category: Option<&str>,
                 value: f64,
                 percent: Option<f64>| {
                    spec.data_labels.as_ref().and_then(|labels| {
                        let mut parts = Vec::new();
                        if labels.show_series_name
                            && let Some(series_name) = series_name
                        {
                            parts.push(escape_html(series_name));
                        }
                        if labels.show_category_name
                            && let Some(category) = category
                        {
                            parts.push(escape_html(category));
                        }
                        if labels.show_value {
                            parts.push(format!("{value}"));
                        }
                        if labels.show_percent
                            && let Some(percent) = percent
                        {
                            parts.push(format!("{:.0}%", percent * 100.0));
                        }
                        if parts.is_empty() {
                            None
                        } else {
                            Some(parts.join(": "))
                        }
                    })
                };
            let build_point_data_label =
                |series_name: Option<&str>, category: Option<&str>, value: f64| {
                    spec.data_labels.as_ref().and_then(|labels| {
                        let mut parts = Vec::new();
                        if labels.show_series_name
                            && let Some(series_name) = series_name
                        {
                            parts.push(escape_html(series_name));
                        }
                        if labels.show_category_name
                            && let Some(category) = category
                        {
                            parts.push(escape_html(category));
                        }
                        if labels.show_value {
                            parts.push(format!("{value}"));
                        }
                        if parts.is_empty() {
                            None
                        } else {
                            Some(parts.join(": "))
                        }
                    })
                };
            let resolve_bar_label_position = || {
                spec.data_labels
                    .as_ref()
                    .and_then(|labels| labels.position)
                    .unwrap_or(ChartDataLabelPosition::OutEnd)
            };

            let _ = writeln!(html, "<div class=\"chart-direct\">");
            html.push_str("<div class=\"chart-legend\">\n");
            if matches!(spec.chart_type, ChartType::Pie | ChartType::Doughnut) {
                for (idx, category) in first_series.categories.iter().enumerate() {
                    let color = palette[idx % palette.len()];
                    let _ = writeln!(
                        html,
                        "<span class=\"chart-legend-item\"><span class=\"chart-legend-swatch\" style=\"background:{color}\"></span>{}</span>",
                        escape_html(category)
                    );
                }
            } else {
                for (series_idx, series) in spec.series.iter().enumerate() {
                    let color = palette[series_idx % palette.len()];
                    let label = series.name.as_deref().unwrap_or("Series");
                    let _ = writeln!(
                        html,
                        "<span class=\"chart-legend-item\"><span class=\"chart-legend-swatch\" style=\"background:{color}\"></span>{}</span>",
                        escape_html(label)
                    );
                }
            }
            html.push_str("</div>\n");
            html.push_str("<div class=\"chart-plot-area\">\n");
            if let Some(title) = spec.value_axis_title.as_deref() {
                let _ = writeln!(
                    html,
                    "<div class=\"chart-axis-title chart-axis-title-y\">{}</div>",
                    escape_html(title)
                );
            }
            html.push_str("<div class=\"chart-plot-main\">\n");
            let _ = writeln!(
                html,
                "<svg viewBox=\"0 0 {w:.1} {chart_height:.1}\" class=\"chart-svg\" preserveAspectRatio=\"none\">"
            );
            let grouping_attr = match spec.grouping {
                ChartGrouping::Clustered => "clustered",
                ChartGrouping::Stacked => "stacked",
                ChartGrouping::PercentStacked => "percent-stacked",
                ChartGrouping::Standard => "standard",
            };
            let hole_attr = spec
                .hole_size
                .map(|hole_size| format!(" data-chart-hole-size=\"{hole_size}\""))
                .unwrap_or_default();
            let _ = writeln!(
                html,
                "<g data-chart-grouping=\"{grouping_attr}\" data-chart-gap-width=\"{gap_width}\" data-chart-overlap=\"{overlap}\"{hole_attr}>"
            );
            match spec.chart_type {
                ChartType::Column => {
                    let slot_width = (w / category_count as f64).max(24.0);
                    let group_width = (slot_width * (100.0 / (100.0 + gap_width as f64))).max(8.0);
                    let outer_gap = ((slot_width - group_width) / 2.0).max(2.0);
                    match spec.grouping {
                        ChartGrouping::Stacked | ChartGrouping::PercentStacked => {
                            let bar_width = group_width.max(8.0);
                            let mut category_totals = vec![0.0; category_count];
                            if matches!(spec.grouping, ChartGrouping::PercentStacked) {
                                for (idx, total) in
                                    category_totals.iter_mut().enumerate().take(category_count)
                                {
                                    *total = spec
                                        .series
                                        .iter()
                                        .map(|s| s.values[idx].max(0.0))
                                        .sum::<f64>()
                                        .max(1.0);
                                }
                            }
                            let mut accumulated = vec![0.0; category_count];
                            for (series_idx, series) in spec.series.iter().enumerate() {
                                let color = palette[series_idx % palette.len()];
                                for (idx, value) in series.values.iter().enumerate() {
                                    let normalized =
                                        if matches!(spec.grouping, ChartGrouping::PercentStacked) {
                                            value.max(0.0) / category_totals[idx]
                                        } else {
                                            *value
                                        };
                                    let bar_height = if normalized <= 0.0 {
                                        0.0
                                    } else if matches!(spec.grouping, ChartGrouping::PercentStacked)
                                    {
                                        normalized * (chart_height - 8.0)
                                    } else {
                                        (normalized / max_value) * (chart_height - 8.0)
                                    };
                                    let x = idx as f64 * slot_width + outer_gap;
                                    let y = chart_height - accumulated[idx] - bar_height;
                                    accumulated[idx] += bar_height;
                                    let _ = writeln!(
                                        html,
                                        "<rect class=\"chart-bar-stacked\" style=\"fill:{color}\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{bar_width:.1}\" height=\"{bar_height:.1}\" rx=\"2\" />"
                                    );
                                    if let Some(label_text) = build_bar_data_label(
                                        series.name.as_deref(),
                                        first_series.categories.get(idx).map(|s| s.as_str()),
                                        *value,
                                        matches!(spec.grouping, ChartGrouping::PercentStacked)
                                            .then_some(normalized),
                                    ) && *value > 0.0
                                    {
                                        let label_position = resolve_bar_label_position();
                                        let label_x = x + bar_width / 2.0;
                                        let label_y = match label_position {
                                            ChartDataLabelPosition::Center => y + bar_height / 2.0,
                                            ChartDataLabelPosition::InEnd => {
                                                (y + 12.0).min(y + bar_height - 6.0)
                                            }
                                            ChartDataLabelPosition::OutEnd => (y - 6.0).max(10.0),
                                        };
                                        let label_position_attr = match label_position {
                                            ChartDataLabelPosition::Center => "ctr",
                                            ChartDataLabelPosition::InEnd => "inEnd",
                                            ChartDataLabelPosition::OutEnd => "outEnd",
                                        };
                                        let _ = writeln!(
                                            html,
                                            "<text class=\"chart-data-label\" data-label-position=\"{label_position_attr}\" x=\"{label_x:.1}\" y=\"{label_y:.1}\">{}</text>",
                                            label_text
                                        );
                                    }
                                }
                            }
                        }
                        _ => {
                            let clustered_divisor =
                                (series_count - (series_count - 1.0) * overlap_ratio).max(1.0);
                            let bar_width = (group_width / clustered_divisor).max(4.0);
                            let step = if series_count > 1.0 {
                                bar_width * (1.0 - overlap_ratio)
                            } else {
                                0.0
                            };
                            for (series_idx, series) in spec.series.iter().enumerate() {
                                let color = palette[series_idx % palette.len()];
                                for (idx, value) in series.values.iter().enumerate() {
                                    let bar_height = if *value <= 0.0 {
                                        0.0
                                    } else {
                                        (*value / max_value) * (chart_height - 8.0)
                                    };
                                    let x = idx as f64 * slot_width
                                        + outer_gap
                                        + series_idx as f64 * step;
                                    let y = chart_height - bar_height;
                                    let _ = writeln!(
                                        html,
                                        "<rect class=\"chart-bar\" style=\"fill:{color}\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{bar_width:.1}\" height=\"{bar_height:.1}\" rx=\"2\" />"
                                    );
                                    if let Some(label_text) = build_bar_data_label(
                                        series.name.as_deref(),
                                        first_series.categories.get(idx).map(|s| s.as_str()),
                                        *value,
                                        None,
                                    ) && *value > 0.0
                                    {
                                        let label_position = resolve_bar_label_position();
                                        let label_x = x + bar_width / 2.0;
                                        let label_y = match label_position {
                                            ChartDataLabelPosition::Center => y + bar_height / 2.0,
                                            ChartDataLabelPosition::InEnd => {
                                                (y + 12.0).min(y + bar_height - 6.0)
                                            }
                                            ChartDataLabelPosition::OutEnd => (y - 6.0).max(10.0),
                                        };
                                        let label_position_attr = match label_position {
                                            ChartDataLabelPosition::Center => "ctr",
                                            ChartDataLabelPosition::InEnd => "inEnd",
                                            ChartDataLabelPosition::OutEnd => "outEnd",
                                        };
                                        let _ = writeln!(
                                            html,
                                            "<text class=\"chart-data-label\" data-label-position=\"{label_position_attr}\" x=\"{label_x:.1}\" y=\"{label_y:.1}\">{}</text>",
                                            label_text
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                ChartType::Bar => {
                    let slot_height = (chart_height / category_count as f64).max(24.0);
                    let group_height =
                        (slot_height * (100.0 / (100.0 + gap_width as f64))).max(8.0);
                    let outer_gap = ((slot_height - group_height) / 2.0).max(2.0);
                    match spec.grouping {
                        ChartGrouping::Stacked | ChartGrouping::PercentStacked => {
                            let bar_height = group_height.max(8.0);
                            let mut category_totals = vec![0.0; category_count];
                            if matches!(spec.grouping, ChartGrouping::PercentStacked) {
                                for (idx, total) in
                                    category_totals.iter_mut().enumerate().take(category_count)
                                {
                                    *total = spec
                                        .series
                                        .iter()
                                        .map(|s| s.values[idx].max(0.0))
                                        .sum::<f64>()
                                        .max(1.0);
                                }
                            }
                            let mut accumulated = vec![0.0; category_count];
                            for (series_idx, series) in spec.series.iter().enumerate() {
                                let color = palette[series_idx % palette.len()];
                                for (idx, value) in series.values.iter().enumerate() {
                                    let normalized =
                                        if matches!(spec.grouping, ChartGrouping::PercentStacked) {
                                            value.max(0.0) / category_totals[idx]
                                        } else {
                                            *value
                                        };
                                    let width = if normalized <= 0.0 {
                                        0.0
                                    } else if matches!(spec.grouping, ChartGrouping::PercentStacked)
                                    {
                                        normalized * (w - 8.0)
                                    } else {
                                        (normalized / max_value) * (w - 8.0)
                                    };
                                    let x = accumulated[idx];
                                    let y = idx as f64 * slot_height + outer_gap;
                                    accumulated[idx] += width;
                                    let _ = writeln!(
                                        html,
                                        "<rect class=\"chart-bar-horizontal\" style=\"fill:{color}\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{width:.1}\" height=\"{bar_height:.1}\" rx=\"2\" />"
                                    );
                                    if let Some(label_text) = build_bar_data_label(
                                        series.name.as_deref(),
                                        first_series.categories.get(idx).map(|s| s.as_str()),
                                        *value,
                                        matches!(spec.grouping, ChartGrouping::PercentStacked)
                                            .then_some(normalized),
                                    ) && *value > 0.0
                                    {
                                        let label_position = resolve_bar_label_position();
                                        let label_x = match label_position {
                                            ChartDataLabelPosition::Center => x + width / 2.0,
                                            ChartDataLabelPosition::InEnd => {
                                                (x + width - 10.0).max(x + 6.0)
                                            }
                                            ChartDataLabelPosition::OutEnd => {
                                                (x + width + 10.0).min(w - 6.0)
                                            }
                                        };
                                        let label_y = y + bar_height / 2.0;
                                        let label_position_attr = match label_position {
                                            ChartDataLabelPosition::Center => "ctr",
                                            ChartDataLabelPosition::InEnd => "inEnd",
                                            ChartDataLabelPosition::OutEnd => "outEnd",
                                        };
                                        let _ = writeln!(
                                            html,
                                            "<text class=\"chart-data-label\" data-label-position=\"{label_position_attr}\" x=\"{label_x:.1}\" y=\"{label_y:.1}\">{}</text>",
                                            label_text
                                        );
                                    }
                                }
                            }
                        }
                        _ => {
                            let clustered_divisor =
                                (series_count - (series_count - 1.0) * overlap_ratio).max(1.0);
                            let bar_height = (group_height / clustered_divisor).max(4.0);
                            let step = if series_count > 1.0 {
                                bar_height * (1.0 - overlap_ratio)
                            } else {
                                0.0
                            };
                            for (series_idx, series) in spec.series.iter().enumerate() {
                                let color = palette[series_idx % palette.len()];
                                for (idx, value) in series.values.iter().enumerate() {
                                    let width = if *value <= 0.0 {
                                        0.0
                                    } else {
                                        (*value / max_value) * (w - 8.0)
                                    };
                                    let y = idx as f64 * slot_height
                                        + outer_gap
                                        + series_idx as f64 * step;
                                    let _ = writeln!(
                                        html,
                                        "<rect class=\"chart-bar-horizontal\" style=\"fill:{color}\" x=\"0.0\" y=\"{y:.1}\" width=\"{width:.1}\" height=\"{bar_height:.1}\" rx=\"2\" />"
                                    );
                                    if let Some(label_text) = build_bar_data_label(
                                        series.name.as_deref(),
                                        first_series.categories.get(idx).map(|s| s.as_str()),
                                        *value,
                                        None,
                                    ) && *value > 0.0
                                    {
                                        let label_position = resolve_bar_label_position();
                                        let label_x = match label_position {
                                            ChartDataLabelPosition::Center => width / 2.0,
                                            ChartDataLabelPosition::InEnd => {
                                                (width - 10.0).max(6.0)
                                            }
                                            ChartDataLabelPosition::OutEnd => {
                                                (width + 10.0).min(w - 6.0)
                                            }
                                        };
                                        let label_y = y + bar_height / 2.0;
                                        let label_position_attr = match label_position {
                                            ChartDataLabelPosition::Center => "ctr",
                                            ChartDataLabelPosition::InEnd => "inEnd",
                                            ChartDataLabelPosition::OutEnd => "outEnd",
                                        };
                                        let _ = writeln!(
                                            html,
                                            "<text class=\"chart-data-label\" data-label-position=\"{label_position_attr}\" x=\"{label_x:.1}\" y=\"{label_y:.1}\">{}</text>",
                                            label_text
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                ChartType::Radar => {
                    let center_x = w / 2.0;
                    let center_y = chart_height / 2.0;
                    let radius = (chart_height.min(w) / 2.0 - 18.0).max(24.0);
                    let radar_style = spec.radar_style.unwrap_or_default();
                    let filled = matches!(radar_style, crate::model::ChartRadarStyle::Filled);

                    for ring in [0.25_f64, 0.5, 0.75, 1.0] {
                        let ring_points = (0..category_count)
                            .map(|idx| {
                                let angle = -std::f64::consts::FRAC_PI_2
                                    + idx as f64 * std::f64::consts::TAU / category_count as f64;
                                let x = center_x + radius * ring * angle.cos();
                                let y = center_y + radius * ring * angle.sin();
                                format!("{x:.1},{y:.1}")
                            })
                            .collect::<Vec<_>>()
                            .join(" ");
                        let _ = writeln!(
                            html,
                            "<polygon class=\"chart-radar-grid\" points=\"{ring_points}\" fill=\"none\" stroke=\"#ddd\" stroke-width=\"1\" />"
                        );
                    }

                    for idx in 0..category_count {
                        let angle = -std::f64::consts::FRAC_PI_2
                            + idx as f64 * std::f64::consts::TAU / category_count as f64;
                        let x = center_x + radius * angle.cos();
                        let y = center_y + radius * angle.sin();
                        let _ = writeln!(
                            html,
                            "<line class=\"chart-radar-spoke\" x1=\"{center_x:.1}\" y1=\"{center_y:.1}\" x2=\"{x:.1}\" y2=\"{y:.1}\" stroke=\"#e2e2e2\" stroke-width=\"1\" />"
                        );
                    }

                    for (series_idx, series) in spec.series.iter().enumerate() {
                        let color = palette[series_idx % palette.len()];
                        let marker_symbol = series
                            .marker
                            .as_ref()
                            .and_then(|marker| marker.symbol.as_deref())
                            .unwrap_or("circle");
                        let marker_radius = series
                            .marker
                            .as_ref()
                            .and_then(|marker| marker.size)
                            .map(|size| (size as f64 / 2.0).clamp(2.0, 18.0))
                            .unwrap_or(3.0);
                        let render_markers =
                            matches!(radar_style, crate::model::ChartRadarStyle::Marker)
                                && marker_symbol != "none";
                        let points = series
                            .values
                            .iter()
                            .enumerate()
                            .map(|(idx, value)| {
                                let angle = -std::f64::consts::FRAC_PI_2
                                    + idx as f64 * std::f64::consts::TAU / category_count as f64;
                                let scaled_radius = if *value <= 0.0 {
                                    0.0
                                } else {
                                    (*value / max_value) * radius
                                };
                                let x = center_x + scaled_radius * angle.cos();
                                let y = center_y + scaled_radius * angle.sin();
                                (x, y)
                            })
                            .collect::<Vec<_>>();
                        let polygon_points = points
                            .iter()
                            .map(|(x, y)| format!("{x:.1},{y:.1}"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        if filled {
                            let _ = writeln!(
                                html,
                                "<polygon class=\"chart-radar-fill\" style=\"fill:{color};opacity:0.30\" points=\"{polygon_points}\" />"
                            );
                        }
                        let _ = writeln!(
                            html,
                            "<polygon class=\"chart-radar-line\" style=\"fill:none;stroke:{color};stroke-width:2\" points=\"{polygon_points}\" />"
                        );
                        if render_markers {
                            for (x, y) in &points {
                                let _ = writeln!(
                                    html,
                                    "<circle class=\"chart-point\" data-marker-symbol=\"{}\" style=\"fill:{color}\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{marker_radius:.1}\" />",
                                    escape_html(marker_symbol)
                                );
                            }
                        }
                    }
                }
                ChartType::Line => {
                    let left_pad = 8.0;
                    let right_pad = 8.0;
                    let usable_width = (w - left_pad - right_pad).max(1.0);
                    let point_label_position = spec
                        .data_labels
                        .as_ref()
                        .and_then(|labels| labels.position)
                        .unwrap_or(ChartDataLabelPosition::OutEnd);
                    let point_count = first_series.values.len().max(1);
                    let step_x = if point_count > 1 {
                        usable_width / (point_count as f64 - 1.0)
                    } else {
                        0.0
                    };
                    for (series_idx, series) in spec.series.iter().enumerate() {
                        let color = palette[series_idx % palette.len()];
                        let mut points = Vec::new();
                        let marker_symbol = series
                            .marker
                            .as_ref()
                            .and_then(|marker| marker.symbol.as_deref())
                            .unwrap_or("circle");
                        let marker_radius = series
                            .marker
                            .as_ref()
                            .and_then(|marker| marker.size)
                            .map(|size| (size as f64 / 2.0).clamp(2.0, 18.0))
                            .unwrap_or(3.0);
                        let render_value_labels = spec
                            .data_labels
                            .as_ref()
                            .map(|labels| labels.show_value)
                            .unwrap_or(false);
                        for (idx, value) in series.values.iter().enumerate() {
                            let x = left_pad + idx as f64 * step_x;
                            let y = chart_height
                                - if *value <= 0.0 {
                                    0.0
                                } else {
                                    (*value / max_value) * (chart_height - 8.0)
                                };
                            points.push((x, y));
                        }
                        let polyline_points = points
                            .iter()
                            .map(|(x, y)| format!("{x:.1},{y:.1}"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let _ = writeln!(
                            html,
                            "<polyline class=\"chart-line\" style=\"stroke:{color}\" points=\"{polyline_points}\" />"
                        );
                        if marker_symbol != "none" {
                            for (idx, ((x, y), value)) in
                                points.iter().copied().zip(series.values.iter()).enumerate()
                            {
                                let _ = writeln!(
                                    html,
                                    "<circle class=\"chart-point\" data-marker-symbol=\"{}\" style=\"fill:{color}\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{marker_radius:.1}\" />",
                                    escape_html(marker_symbol)
                                );
                                if render_value_labels && *value > 0.0 {
                                    let label_y = match point_label_position {
                                        ChartDataLabelPosition::Center => y,
                                        ChartDataLabelPosition::InEnd => {
                                            (y + 10.0).min(chart_height - 6.0)
                                        }
                                        ChartDataLabelPosition::OutEnd => (y - 10.0).max(10.0),
                                    };
                                    let label_text = build_point_data_label(
                                        series.name.as_deref(),
                                        series.categories.get(idx).map(|s| s.as_str()),
                                        *value,
                                    )
                                    .unwrap_or_else(|| value.to_string());
                                    let label_position_attr = match point_label_position {
                                        ChartDataLabelPosition::Center => "ctr",
                                        ChartDataLabelPosition::InEnd => "inEnd",
                                        ChartDataLabelPosition::OutEnd => "outEnd",
                                    };
                                    let _ = writeln!(
                                        html,
                                        "<text class=\"chart-data-label\" data-label-position=\"{label_position_attr}\" x=\"{x:.1}\" y=\"{label_y:.1}\">{}</text>",
                                        label_text
                                    );
                                }
                            }
                        } else if render_value_labels {
                            for (idx, ((x, y), value)) in
                                points.iter().copied().zip(series.values.iter()).enumerate()
                            {
                                if *value > 0.0 {
                                    let label_y = match point_label_position {
                                        ChartDataLabelPosition::Center => y,
                                        ChartDataLabelPosition::InEnd => {
                                            (y + 10.0).min(chart_height - 6.0)
                                        }
                                        ChartDataLabelPosition::OutEnd => (y - 10.0).max(10.0),
                                    };
                                    let label_text = build_point_data_label(
                                        series.name.as_deref(),
                                        series.categories.get(idx).map(|s| s.as_str()),
                                        *value,
                                    )
                                    .unwrap_or_else(|| value.to_string());
                                    let label_position_attr = match point_label_position {
                                        ChartDataLabelPosition::Center => "ctr",
                                        ChartDataLabelPosition::InEnd => "inEnd",
                                        ChartDataLabelPosition::OutEnd => "outEnd",
                                    };
                                    let _ = writeln!(
                                        html,
                                        "<text class=\"chart-data-label\" data-label-position=\"{label_position_attr}\" x=\"{x:.1}\" y=\"{label_y:.1}\">{}</text>",
                                        label_text
                                    );
                                }
                            }
                        }
                    }
                }
                ChartType::Scatter => {
                    let left_pad = 8.0;
                    let right_pad = 8.0;
                    let top_pad = 8.0;
                    let bottom_pad = 8.0;
                    let scatter_label_position = spec
                        .data_labels
                        .as_ref()
                        .and_then(|labels| labels.position)
                        .unwrap_or(ChartDataLabelPosition::OutEnd);
                    let scatter_style = spec.scatter_style.unwrap_or_default();
                    let render_line = matches!(
                        scatter_style,
                        ChartScatterStyle::Line
                            | ChartScatterStyle::LineMarker
                            | ChartScatterStyle::Smooth
                            | ChartScatterStyle::SmoothMarker
                    );
                    let all_x_values = spec
                        .series
                        .iter()
                        .flat_map(|series| series.x_values.iter().copied());
                    let all_y_values = spec
                        .series
                        .iter()
                        .flat_map(|series| series.values.iter().copied());
                    let min_x = all_x_values.clone().fold(f64::INFINITY, f64::min);
                    let max_x = all_x_values.fold(f64::NEG_INFINITY, f64::max);
                    let min_y = all_y_values.clone().fold(f64::INFINITY, f64::min);
                    let max_y = all_y_values.fold(f64::NEG_INFINITY, f64::max);
                    let x_span = if min_x.is_finite() && max_x.is_finite() {
                        (max_x - min_x).abs().max(1.0)
                    } else {
                        1.0
                    };
                    let y_span = if min_y.is_finite() && max_y.is_finite() {
                        (max_y - min_y).abs().max(1.0)
                    } else {
                        1.0
                    };
                    let usable_width = (w - left_pad - right_pad).max(1.0);
                    let usable_height = (chart_height - top_pad - bottom_pad).max(1.0);

                    for (series_idx, series) in spec.series.iter().enumerate() {
                        let color = palette[series_idx % palette.len()];
                        let marker_symbol = series
                            .marker
                            .as_ref()
                            .and_then(|marker| marker.symbol.as_deref())
                            .unwrap_or("circle");
                        let marker_radius = series
                            .marker
                            .as_ref()
                            .and_then(|marker| marker.size)
                            .map(|size| (size as f64 / 2.0).clamp(2.0, 18.0))
                            .unwrap_or(3.0);
                        let render_value_labels = spec
                            .data_labels
                            .as_ref()
                            .map(|labels| labels.show_value)
                            .unwrap_or(false);
                        let render_markers = marker_symbol != "none"
                            && matches!(
                                scatter_style,
                                ChartScatterStyle::Marker
                                    | ChartScatterStyle::LineMarker
                                    | ChartScatterStyle::SmoothMarker
                            );
                        let mut points = Vec::new();
                        for (x_value, y_value) in series.x_values.iter().zip(series.values.iter()) {
                            let x = left_pad + ((*x_value - min_x) / x_span) * usable_width;
                            let y = chart_height
                                - bottom_pad
                                - ((*y_value - min_y) / y_span) * usable_height;
                            points.push((x, y));
                        }
                        if render_line {
                            let polyline_points = points
                                .iter()
                                .map(|(x, y)| format!("{x:.1},{y:.1}"))
                                .collect::<Vec<_>>()
                                .join(" ");
                            let _ = writeln!(
                                html,
                                "<polyline class=\"chart-line\" style=\"stroke:{color}\" points=\"{polyline_points}\" />"
                            );
                        }
                        if render_markers {
                            for (idx, ((x, y), value)) in
                                points.iter().copied().zip(series.values.iter()).enumerate()
                            {
                                let _ = writeln!(
                                    html,
                                    "<circle class=\"chart-point\" data-marker-symbol=\"{}\" style=\"fill:{color}\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{marker_radius:.1}\" />",
                                    escape_html(marker_symbol)
                                );
                                if render_value_labels && *value > 0.0 {
                                    let label_y = match scatter_label_position {
                                        ChartDataLabelPosition::Center => y,
                                        ChartDataLabelPosition::InEnd => {
                                            (y + 10.0).min(chart_height - 6.0)
                                        }
                                        ChartDataLabelPosition::OutEnd => (y - 10.0).max(10.0),
                                    };
                                    let category_text =
                                        series.x_values.get(idx).map(|value| value.to_string());
                                    let label_text = build_point_data_label(
                                        series.name.as_deref(),
                                        category_text.as_deref(),
                                        *value,
                                    )
                                    .unwrap_or_else(|| value.to_string());
                                    let label_position_attr = match scatter_label_position {
                                        ChartDataLabelPosition::Center => "ctr",
                                        ChartDataLabelPosition::InEnd => "inEnd",
                                        ChartDataLabelPosition::OutEnd => "outEnd",
                                    };
                                    let _ = writeln!(
                                        html,
                                        "<text class=\"chart-data-label\" data-label-position=\"{label_position_attr}\" x=\"{x:.1}\" y=\"{label_y:.1}\">{}</text>",
                                        label_text
                                    );
                                }
                            }
                        } else if render_value_labels {
                            for (idx, ((x, y), value)) in
                                points.iter().copied().zip(series.values.iter()).enumerate()
                            {
                                if *value > 0.0 {
                                    let label_y = match scatter_label_position {
                                        ChartDataLabelPosition::Center => y,
                                        ChartDataLabelPosition::InEnd => {
                                            (y + 10.0).min(chart_height - 6.0)
                                        }
                                        ChartDataLabelPosition::OutEnd => (y - 10.0).max(10.0),
                                    };
                                    let category_text =
                                        series.x_values.get(idx).map(|value| value.to_string());
                                    let label_text = build_point_data_label(
                                        series.name.as_deref(),
                                        category_text.as_deref(),
                                        *value,
                                    )
                                    .unwrap_or_else(|| value.to_string());
                                    let label_position_attr = match scatter_label_position {
                                        ChartDataLabelPosition::Center => "ctr",
                                        ChartDataLabelPosition::InEnd => "inEnd",
                                        ChartDataLabelPosition::OutEnd => "outEnd",
                                    };
                                    let _ = writeln!(
                                        html,
                                        "<text class=\"chart-data-label\" data-label-position=\"{label_position_attr}\" x=\"{x:.1}\" y=\"{label_y:.1}\">{}</text>",
                                        label_text
                                    );
                                }
                            }
                        }
                    }
                }
                ChartType::Bubble => {
                    let left_pad = 8.0;
                    let right_pad = 8.0;
                    let top_pad = 8.0;
                    let bottom_pad = 8.0;
                    let all_x_values = spec
                        .series
                        .iter()
                        .flat_map(|series| series.x_values.iter().copied());
                    let all_y_values = spec
                        .series
                        .iter()
                        .flat_map(|series| series.values.iter().copied());
                    let all_bubble_sizes = spec
                        .series
                        .iter()
                        .flat_map(|series| series.bubble_sizes.iter().copied());
                    let min_x = all_x_values.clone().fold(f64::INFINITY, f64::min);
                    let max_x = all_x_values.fold(f64::NEG_INFINITY, f64::max);
                    let min_y = all_y_values.clone().fold(f64::INFINITY, f64::min);
                    let max_y = all_y_values.fold(f64::NEG_INFINITY, f64::max);
                    let max_bubble = all_bubble_sizes.fold(0.0_f64, f64::max).max(1.0);
                    let bubble_scale = (spec.bubble_scale.unwrap_or(100.0) / 100.0).clamp(0.0, 3.0);
                    let x_span = if min_x.is_finite() && max_x.is_finite() {
                        (max_x - min_x).abs().max(1.0)
                    } else {
                        1.0
                    };
                    let y_span = if min_y.is_finite() && max_y.is_finite() {
                        (max_y - min_y).abs().max(1.0)
                    } else {
                        1.0
                    };
                    let usable_width = (w - left_pad - right_pad).max(1.0);
                    let usable_height = (chart_height - top_pad - bottom_pad).max(1.0);

                    for (series_idx, series) in spec.series.iter().enumerate() {
                        let color = palette[series_idx % palette.len()];
                        for ((x_value, y_value), bubble_size) in series
                            .x_values
                            .iter()
                            .zip(series.values.iter())
                            .zip(series.bubble_sizes.iter())
                        {
                            let x = left_pad + ((*x_value - min_x) / x_span) * usable_width;
                            let y = chart_height
                                - bottom_pad
                                - ((*y_value - min_y) / y_span) * usable_height;
                            let radius = ((4.0 + (*bubble_size / max_bubble) * 14.0)
                                * bubble_scale)
                                .clamp(4.0, 24.0);
                            let _ = writeln!(
                                html,
                                "<circle class=\"chart-bubble\" style=\"fill:{color};opacity:0.45;stroke:{color};stroke-width:1\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{radius:.1}\" />"
                            );
                        }
                    }
                }
                ChartType::Area => {
                    let left_pad = 8.0;
                    let right_pad = 8.0;
                    let usable_width = (w - left_pad - right_pad).max(1.0);
                    let point_label_position = spec
                        .data_labels
                        .as_ref()
                        .and_then(|labels| labels.position)
                        .unwrap_or(ChartDataLabelPosition::OutEnd);
                    let step_x = if category_count > 1 {
                        usable_width / (category_count as f64 - 1.0)
                    } else {
                        0.0
                    };
                    let render_value_labels = spec
                        .data_labels
                        .as_ref()
                        .map(|labels| labels.show_value)
                        .unwrap_or(false);
                    for (series_idx, series) in spec.series.iter().enumerate() {
                        let color = palette[series_idx % palette.len()];
                        let mut points = Vec::new();
                        for (idx, value) in series.values.iter().enumerate() {
                            let x = left_pad + idx as f64 * step_x;
                            let y = chart_height
                                - if *value <= 0.0 {
                                    0.0
                                } else {
                                    (*value / max_value) * (chart_height - 8.0)
                                };
                            points.push((x, y));
                        }
                        if let (Some((first_x, _)), Some((last_x, _))) =
                            (points.first().copied(), points.last().copied())
                        {
                            let area_points = points
                                .iter()
                                .map(|(x, y)| format!("{x:.1},{y:.1}"))
                                .chain([
                                    format!("{last_x:.1},{chart_height:.1}"),
                                    format!("{first_x:.1},{chart_height:.1}"),
                                ])
                                .collect::<Vec<_>>()
                                .join(" ");
                            let line_points = points
                                .iter()
                                .map(|(x, y)| format!("{x:.1},{y:.1}"))
                                .collect::<Vec<_>>()
                                .join(" ");
                            let _ = writeln!(
                                html,
                                "<polygon class=\"chart-area\" style=\"fill:{color}\" points=\"{area_points}\" />"
                            );
                            let _ = writeln!(
                                html,
                                "<polyline class=\"chart-line\" style=\"stroke:{color}\" points=\"{line_points}\" />"
                            );
                            if render_value_labels {
                                for (idx, ((x, y), value)) in
                                    points.iter().copied().zip(series.values.iter()).enumerate()
                                {
                                    if *value > 0.0 {
                                        let label_y = match point_label_position {
                                            ChartDataLabelPosition::Center => y,
                                            ChartDataLabelPosition::InEnd => {
                                                (y + 10.0).min(chart_height - 6.0)
                                            }
                                            ChartDataLabelPosition::OutEnd => (y - 10.0).max(10.0),
                                        };
                                        let label_text = build_point_data_label(
                                            series.name.as_deref(),
                                            series.categories.get(idx).map(|s| s.as_str()),
                                            *value,
                                        )
                                        .unwrap_or_else(|| value.to_string());
                                        let label_position_attr = match point_label_position {
                                            ChartDataLabelPosition::Center => "ctr",
                                            ChartDataLabelPosition::InEnd => "inEnd",
                                            ChartDataLabelPosition::OutEnd => "outEnd",
                                        };
                                        let _ = writeln!(
                                            html,
                                            "<text class=\"chart-data-label\" data-label-position=\"{label_position_attr}\" x=\"{x:.1}\" y=\"{label_y:.1}\">{}</text>",
                                            label_text
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                ChartType::OfPie => {
                    let values = &first_series.values;
                    let split_count = spec
                        .split_pos
                        .map(|value| value.round() as usize)
                        .unwrap_or(0)
                        .min(values.len().saturating_sub(1));
                    let primary_len = values.len().saturating_sub(split_count);
                    let (primary_values, secondary_values) = values.split_at(primary_len);
                    let (primary_categories, secondary_categories) = first_series
                        .categories
                        .split_at(primary_len.min(first_series.categories.len()));
                    let primary_radius = (chart_height.min(w * 0.58) / 2.0 - 10.0).max(12.0);
                    let secondary_radius = (primary_radius
                        * (spec.second_pie_size.unwrap_or(75) as f64 / 100.0))
                        .clamp(10.0, primary_radius);
                    let primary_center_x = w * 0.33;
                    let secondary_center_x = w * 0.77;
                    let center_y = chart_height / 2.0;

                    let render_cluster =
                        |html: &mut String,
                         class_name: &str,
                         center_x: f64,
                         center_y: f64,
                         radius: f64,
                         values: &[f64],
                         color_offset: usize| {
                            let total = values.iter().copied().filter(|v| *v > 0.0).sum::<f64>();
                            if total <= 0.0 {
                                return;
                            }
                            let _ = writeln!(html, "<g class=\"{class_name}\">");
                            let mut start_angle = -std::f64::consts::FRAC_PI_2;
                            for (idx, value) in values.iter().enumerate() {
                                if *value <= 0.0 {
                                    continue;
                                }
                                let color = palette[(color_offset + idx) % palette.len()];
                                let sweep = (*value / total) * std::f64::consts::TAU;
                                let end_angle = start_angle + sweep;
                                let x1 = center_x + radius * start_angle.cos();
                                let y1 = center_y + radius * start_angle.sin();
                                let x2 = center_x + radius * end_angle.cos();
                                let y2 = center_y + radius * end_angle.sin();
                                let large_arc = if sweep > std::f64::consts::PI { 1 } else { 0 };
                                let path = format!(
                                    "M {center_x:.1} {center_y:.1} L {x1:.1} {y1:.1} A {radius:.1} {radius:.1} 0 {large_arc} 1 {x2:.1} {y2:.1} Z"
                                );
                                let _ = writeln!(
                                    html,
                                    "<path class=\"chart-pie-slice\" style=\"fill:{color}\" d=\"{path}\" />"
                                );
                                start_angle = end_angle;
                            }
                            let _ = writeln!(html, "</g>");
                        };

                    render_cluster(
                        html,
                        "chart-of-pie-primary",
                        primary_center_x,
                        center_y,
                        primary_radius,
                        primary_values,
                        0,
                    );
                    render_cluster(
                        html,
                        "chart-of-pie-secondary",
                        secondary_center_x,
                        center_y,
                        secondary_radius,
                        secondary_values,
                        primary_len,
                    );

                    let mut label_y = chart_height - 10.0;
                    for category in primary_categories.iter().chain(secondary_categories.iter()) {
                        let _ = writeln!(
                            html,
                            "<text class=\"chart-data-label\" x=\"{:.1}\" y=\"{label_y:.1}\">{}</text>",
                            w / 2.0,
                            escape_html(category)
                        );
                        label_y -= 12.0;
                    }
                }
                ChartType::Pie | ChartType::Doughnut => {
                    let radius = (chart_height.min(w) / 2.0 - 8.0).max(12.0);
                    let center_x = w / 2.0;
                    let center_y = chart_height / 2.0;
                    let pie_label_position = spec
                        .data_labels
                        .as_ref()
                        .and_then(|labels| labels.position)
                        .unwrap_or(ChartDataLabelPosition::Center);
                    let hole_ratio = if matches!(spec.chart_type, ChartType::Doughnut) {
                        spec.hole_size.unwrap_or(50) as f64 / 100.0
                    } else {
                        0.0
                    };
                    let inner_radius = radius * hole_ratio;
                    let values = &first_series.values;
                    let total = values.iter().copied().filter(|v| *v > 0.0).sum::<f64>();
                    if total > 0.0 {
                        let mut start_angle = -std::f64::consts::FRAC_PI_2;
                        for (idx, value) in values.iter().enumerate() {
                            if *value <= 0.0 {
                                continue;
                            }
                            let color = palette[idx % palette.len()];
                            let sweep = (*value / total) * std::f64::consts::TAU;
                            let end_angle = start_angle + sweep;
                            let x1 = center_x + radius * start_angle.cos();
                            let y1 = center_y + radius * start_angle.sin();
                            let x2 = center_x + radius * end_angle.cos();
                            let y2 = center_y + radius * end_angle.sin();
                            let large_arc = if sweep > std::f64::consts::PI { 1 } else { 0 };
                            let path = if inner_radius > 0.0 {
                                let ix2 = center_x + inner_radius * end_angle.cos();
                                let iy2 = center_y + inner_radius * end_angle.sin();
                                let ix1 = center_x + inner_radius * start_angle.cos();
                                let iy1 = center_y + inner_radius * start_angle.sin();
                                format!(
                                    "M {x1:.1} {y1:.1} A {radius:.1} {radius:.1} 0 {large_arc} 1 {x2:.1} {y2:.1} L {ix2:.1} {iy2:.1} A {inner_radius:.1} {inner_radius:.1} 0 {large_arc} 0 {ix1:.1} {iy1:.1} Z"
                                )
                            } else {
                                format!(
                                    "M {center_x:.1} {center_y:.1} L {x1:.1} {y1:.1} A {radius:.1} {radius:.1} 0 {large_arc} 1 {x2:.1} {y2:.1} Z"
                                )
                            };
                            let _ = writeln!(
                                html,
                                "<path class=\"chart-pie-slice\" style=\"fill:{color}\" d=\"{path}\" />"
                            );
                            if let Some(data_labels) = spec.data_labels.as_ref() {
                                let mut label_parts = Vec::new();
                                if data_labels.show_category_name
                                    && let Some(category) = first_series.categories.get(idx)
                                {
                                    label_parts.push(escape_html(category));
                                }
                                if data_labels.show_value {
                                    label_parts.push(format!("{}", value));
                                }
                                if data_labels.show_percent {
                                    label_parts.push(format!("{:.0}%", (*value / total) * 100.0));
                                }
                                if !label_parts.is_empty() {
                                    let mid_angle = start_angle + sweep / 2.0;
                                    let label_radius = match pie_label_position {
                                        ChartDataLabelPosition::OutEnd => radius + 16.0,
                                        ChartDataLabelPosition::Center
                                        | ChartDataLabelPosition::InEnd => {
                                            if inner_radius > 0.0 {
                                                inner_radius + (radius - inner_radius) * 0.5
                                            } else {
                                                radius * 0.62
                                            }
                                        }
                                    };
                                    let label_x = center_x + label_radius * mid_angle.cos();
                                    let label_y = center_y + label_radius * mid_angle.sin();
                                    let label_text = label_parts.join(": ");
                                    let label_position_attr = match pie_label_position {
                                        ChartDataLabelPosition::Center => "ctr",
                                        ChartDataLabelPosition::InEnd => "inEnd",
                                        ChartDataLabelPosition::OutEnd => "outEnd",
                                    };
                                    let _ = writeln!(
                                        html,
                                        "<text class=\"chart-data-label\" data-label-position=\"{label_position_attr}\" x=\"{label_x:.1}\" y=\"{label_y:.1}\">{}</text>",
                                        label_text
                                    );
                                }
                            }
                            start_angle = end_angle;
                        }
                    }
                }
            }
            html.push_str("</svg>\n<div class=\"chart-axis-labels\">");
            if !matches!(spec.chart_type, ChartType::Scatter) {
                for category in &first_series.categories {
                    let _ = writeln!(html, "<span>{}</span>", escape_html(category));
                }
            }
            html.push_str("</div>\n");
            if let Some(title) = spec.category_axis_title.as_deref() {
                let _ = writeln!(
                    html,
                    "<div class=\"chart-axis-title chart-axis-title-x\">{}</div>",
                    escape_html(title)
                );
            }
            html.push_str("</div>\n</div>\n</div>\n");
            return;
        }
    }

    if let Some(ref img_data) = chart_data.preview_image
        && !img_data.is_empty()
    {
        let mime = chart_data.preview_mime.as_deref().unwrap_or("image/png");
        let src = if ctx.embed_images {
            let b64 = base64::engine::general_purpose::STANDARD.encode(img_data);
            format!("data:{mime};base64,{b64}")
        } else {
            ctx.register_external_asset("chart", mime, img_data)
        };
        let _ = writeln!(
            html,
            "<img class=\"shape-image\" src=\"{src}\" alt=\"Chart\">"
        );
    } else {
        html.push_str(
                    "<div class=\"chart-placeholder\">\
                     <svg width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\">\
                     <rect x=\"3\" y=\"12\" width=\"4\" height=\"9\"/><rect x=\"10\" y=\"7\" width=\"4\" height=\"14\"/>\
                     <rect x=\"17\" y=\"3\" width=\"4\" height=\"18\"/></svg>\
                     <span style=\"margin-left:8px\">Chart</span></div>\n"
                );
    }
    html.push_str("</div>\n");
}
