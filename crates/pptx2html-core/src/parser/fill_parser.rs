use quick_xml::events::BytesStart;

use super::slide_parser::ShapeBuilder;
use super::table_parser::{TableCellBuilder, assign_cell_color};
use super::text_parser::{ParagraphBuilder, RunBuilder};
use super::xml_utils;
use crate::model::*;

pub(crate) fn assign_style_ref_color(
    ref_kind: &str,
    idx: &str,
    color: Color,
    builder: &mut Option<ShapeStyleRef>,
) {
    let Some(builder) = builder.as_mut() else {
        return;
    };
    match ref_kind {
        "fillRef" => builder.fill_ref = Some(style_ref(idx, color)),
        "lnRef" => builder.ln_ref = Some(style_ref(idx, color)),
        "effectRef" => builder.effect_ref = Some(style_ref(idx, color)),
        "fontRef" => {
            builder.font_ref = Some(FontRef {
                idx: idx.to_owned(),
                color,
            });
        }
        _ => {}
    }
}

pub(crate) fn ensure_style_ref(ref_kind: &str, idx: &str, builder: &mut Option<ShapeStyleRef>) {
    let Some(builder) = builder.as_mut() else {
        return;
    };
    match ref_kind {
        "fillRef" if builder.fill_ref.is_none() => {
            builder.fill_ref = Some(style_ref(idx, Color::none()));
        }
        "lnRef" if builder.ln_ref.is_none() => {
            builder.ln_ref = Some(style_ref(idx, Color::none()));
        }
        "effectRef" if builder.effect_ref.is_none() => {
            builder.effect_ref = Some(style_ref(idx, Color::none()));
        }
        "fontRef" if builder.font_ref.is_none() => {
            builder.font_ref = Some(FontRef {
                idx: idx.to_owned(),
                color: Color::none(),
            });
        }
        _ => {}
    }
}

pub(crate) fn assign_style_ref_no_color(
    ref_kind: &str,
    idx: &str,
    builder: &mut Option<ShapeStyleRef>,
) {
    let Some(builder) = builder.as_mut() else {
        return;
    };
    match ref_kind {
        "fillRef" => builder.fill_ref = Some(style_ref(idx, Color::none())),
        "lnRef" => builder.ln_ref = Some(style_ref(idx, Color::none())),
        "effectRef" => builder.effect_ref = Some(style_ref(idx, Color::none())),
        "fontRef" => {
            builder.font_ref = Some(FontRef {
                idx: idx.to_owned(),
                color: Color::none(),
            });
        }
        _ => {}
    }
}

fn style_ref(idx: &str, color: Color) -> StyleRef {
    StyleRef {
        idx: idx.parse::<u32>().unwrap_or(0),
        color,
    }
}

pub(crate) fn parse_line_end(element: &BytesStart<'_>) -> Option<LineEnd> {
    let end_type = match xml_utils::attr_str(element, "type").as_deref() {
        Some("arrow") => LineEndType::Arrow,
        Some("triangle") => LineEndType::Triangle,
        Some("stealth") => LineEndType::Stealth,
        Some("diamond") => LineEndType::Diamond,
        Some("oval") => LineEndType::Oval,
        Some("none") | None | Some(_) => return None,
    };
    let width = parse_line_end_size(xml_utils::attr_str(element, "w").as_deref());
    let length = parse_line_end_size(xml_utils::attr_str(element, "len").as_deref());
    Some(LineEnd {
        end_type,
        width,
        length,
    })
}

fn parse_line_end_size(value: Option<&str>) -> LineEndSize {
    match value {
        Some("sm") => LineEndSize::Small,
        Some("lg") => LineEndSize::Large,
        _ => LineEndSize::Medium,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn assign_color(
    color: Color,
    depth: &[String],
    in_shape_properties: bool,
    in_line: bool,
    in_run_properties: bool,
    in_gradient: bool,
    stop_position: f64,
    shape: &mut Option<ShapeBuilder>,
    run: &mut Option<RunBuilder>,
    stops: &mut Vec<GradientStop>,
) {
    if in_run_properties || contains(depth, "rPr") {
        if let Some(run) = run.as_mut() {
            run.color = color;
        }
        return;
    }
    if in_gradient && contains(depth, "gs") {
        stops.push(GradientStop {
            position: stop_position,
            color,
        });
        return;
    }
    if in_line {
        if let Some(shape) = shape.as_mut() {
            shape.border_color = color;
            if matches!(shape.border_style, BorderStyle::None) {
                shape.border_style = BorderStyle::Solid;
            }
        }
        return;
    }
    if in_shape_properties && let Some(shape) = shape.as_mut() {
        shape.fill = Fill::Solid(SolidFill { color });
    }
}

fn contains(depth: &[String], tag: &str) -> bool {
    depth.iter().any(|item| item == tag)
}

pub(crate) fn assign_background_color_target(
    color: Color,
    depth: &[String],
    in_gradient: bool,
    stop_position: f64,
    stops: &mut Vec<GradientStop>,
    solid_color: &mut Option<Color>,
) {
    if in_gradient && contains(depth, "gs") {
        stops.push(GradientStop {
            position: stop_position,
            color,
        });
    } else {
        *solid_color = Some(color);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_color(
    color: Color,
    depth: &[String],
    in_shape_properties: bool,
    in_line: bool,
    in_run_properties: bool,
    in_gradient: bool,
    stop_position: f64,
    in_shape_effect: bool,
    shape_effect_color: &mut Option<Color>,
    in_highlight: bool,
    in_cell_run_properties: bool,
    cell_run: &mut Option<RunBuilder>,
    shape_run: &mut Option<RunBuilder>,
    in_outer_shadow: bool,
    current_color: &mut Option<Color>,
    in_cell_bullet_color: bool,
    cell_paragraph: &mut Option<ParagraphBuilder>,
    in_shape_run_defaults: bool,
    shape_run_defaults: &mut Option<RunDefaults>,
    in_cell_properties: bool,
    cell_border_side: &Option<String>,
    cell: &mut Option<TableCellBuilder>,
    in_bullet_color: bool,
    paragraph: &mut Option<ParagraphBuilder>,
    in_style: bool,
    style_kind: Option<&str>,
    style_index: Option<&str>,
    style: &mut Option<ShapeStyleRef>,
    in_background: bool,
    in_background_image: bool,
    in_background_gradient: bool,
    background_stop_position: f64,
    background_stops: &mut Vec<GradientStop>,
    background_solid_color: &mut Option<Color>,
    shape: &mut Option<ShapeBuilder>,
    stops: &mut Vec<GradientStop>,
) {
    if in_shape_effect {
        *shape_effect_color = Some(color);
    } else if in_highlight {
        if in_cell_run_properties {
            if let Some(run) = cell_run.as_mut() {
                run.highlight = Some(color);
            }
        } else if in_run_properties && let Some(run) = shape_run.as_mut() {
            run.highlight = Some(color);
        }
    } else if in_outer_shadow {
        *current_color = Some(color);
    } else if in_cell_bullet_color {
        if let Some(paragraph) = cell_paragraph.as_mut() {
            paragraph.bu_color = Some(color);
        }
    } else if in_shape_run_defaults {
        if let Some(defaults) = shape_run_defaults.as_mut() {
            defaults.color = Some(color);
        }
    } else if in_cell_properties {
        assign_cell_color(color, cell_border_side, cell);
    } else if in_cell_run_properties {
        if let Some(run) = cell_run.as_mut() {
            run.color = color;
        }
    } else if in_bullet_color {
        if let Some(paragraph) = paragraph.as_mut() {
            paragraph.bu_color = Some(color);
        }
    } else if in_style && style_kind.is_some() {
        assign_style_ref_color(
            style_kind.unwrap_or(""),
            style_index.unwrap_or("0"),
            color,
            style,
        );
    } else if in_background && !in_background_image {
        assign_background_color_target(
            color,
            depth,
            in_background_gradient,
            background_stop_position,
            background_stops,
            background_solid_color,
        );
    } else {
        assign_color(
            color,
            depth,
            in_shape_properties,
            in_line,
            in_run_properties,
            in_gradient,
            stop_position,
            shape,
            shape_run,
            stops,
        );
    }
}

pub(crate) fn parse_outer_shadow(element: &BytesStart<'_>) -> (f64, f64, f64) {
    (
        parse_effect_emu(element, "blurRad"),
        parse_effect_emu(element, "dist"),
        xml_utils::attr_str(element, "dir")
            .and_then(|value| value.parse::<f64>().ok())
            .map(|value| value / 60_000.0)
            .unwrap_or(0.0),
    )
}

pub(crate) fn parse_glow_radius(element: &BytesStart<'_>) -> f64 {
    parse_effect_emu(element, "rad")
}

fn parse_effect_emu(element: &BytesStart<'_>, attribute: &str) -> f64 {
    xml_utils::attr_str(element, attribute)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| Emu(value as i64).to_pt())
        .unwrap_or(0.0)
}

pub(crate) fn finish_outer_shadow(
    shape: &mut Option<ShapeBuilder>,
    color: &mut Option<Color>,
    blur_radius: f64,
    distance: f64,
    direction: f64,
    alpha: f64,
) {
    if let Some(shape) = shape.as_mut() {
        shape.shape_outer_shadow = Some(OuterShadow {
            blur_radius,
            distance,
            direction,
            color: color.take().unwrap_or_else(|| Color::rgb("000000")),
            alpha,
        });
    }
}

pub(crate) fn finish_glow(
    shape: &mut Option<ShapeBuilder>,
    color: &mut Option<Color>,
    radius: f64,
    alpha: f64,
) {
    if let Some(shape) = shape.as_mut() {
        shape.shape_glow = Some(GlowEffect {
            radius,
            color: color.take().unwrap_or_else(|| Color::rgb("FFC000")),
            alpha,
        });
    }
}
