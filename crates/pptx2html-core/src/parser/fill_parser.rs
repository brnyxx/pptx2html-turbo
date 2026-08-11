use std::collections::HashMap;
use std::io::{Read, Seek};

use quick_xml::events::BytesStart;
use zip::ZipArchive;

use super::graphic_frame_parser::{mime_from_extension, resolve_relationship_path};
use super::slide_parser::ShapeBuilder;
#[cfg(test)]
use super::table_parser::TableCellBuilder;
use super::table_parser::{TableSaxState, assign_cell_color};
#[cfg(test)]
use super::text_parser::ParagraphBuilder;
use super::text_parser::{RunBuilder, TextSaxState};
use super::xml_utils;
use crate::model::*;

pub(crate) struct FillSaxState {
    pub(crate) current_color: Option<Color>,
    pub(crate) in_line: bool,
    pub(crate) in_gradient: bool,
    pub(crate) gradient_stops: Vec<GradientStop>,
    pub(crate) gradient_angle: f64,
    pub(crate) gradient_type: GradientType,
    pub(crate) gradient_stop_position: f64,
    pub(crate) in_text_effect_list: bool,
    pub(crate) in_text_outer_shadow: bool,
    pub(crate) text_shadow_blur: f64,
    pub(crate) text_shadow_distance: f64,
    pub(crate) text_shadow_direction: f64,
    pub(crate) in_highlight: bool,
    pub(crate) in_shape_effect_list: bool,
    pub(crate) in_shape_outer_shadow: bool,
    pub(crate) shape_shadow_blur: f64,
    pub(crate) shape_shadow_distance: f64,
    pub(crate) shape_shadow_direction: f64,
    pub(crate) shape_shadow_alpha: f64,
    pub(crate) in_shape_glow: bool,
    pub(crate) shape_glow_radius: f64,
    pub(crate) shape_glow_alpha: f64,
    pub(crate) shape_effect_color: Option<Color>,
    in_style: bool,
    style_builder: Option<ShapeStyleRef>,
    style_current_ref: Option<String>,
    style_index: Option<String>,
    in_background: bool,
    in_background_image: bool,
    background_image_rel_id: Option<String>,
    background_solid_color: Option<Color>,
    in_background_gradient: bool,
    background_gradient_stops: Vec<GradientStop>,
    background_gradient_angle: f64,
    background_gradient_type: GradientType,
    background_stop_position: f64,
}

pub(crate) struct ColorTargets<'a> {
    pub(crate) depth: &'a [String],
    pub(crate) in_shape_properties: bool,
    pub(crate) shape: &'a mut Option<ShapeBuilder>,
    pub(crate) text: &'a mut TextSaxState,
    pub(crate) table: &'a mut TableSaxState,
}

pub(crate) struct FillEndTargets<'a, R: Read + Seek> {
    pub(crate) shape: &'a mut Option<ShapeBuilder>,
    pub(crate) text: &'a mut TextSaxState,
    pub(crate) table: &'a mut TableSaxState,
    pub(crate) slide: &'a mut Slide,
    pub(crate) relationships: &'a HashMap<String, String>,
    pub(crate) archive: &'a mut ZipArchive<R>,
}

impl Default for FillSaxState {
    fn default() -> Self {
        Self {
            current_color: None,
            in_line: false,
            in_gradient: false,
            gradient_stops: Vec::new(),
            gradient_angle: 0.0,
            gradient_type: GradientType::Linear,
            gradient_stop_position: 0.0,
            in_text_effect_list: false,
            in_text_outer_shadow: false,
            text_shadow_blur: 0.0,
            text_shadow_distance: 0.0,
            text_shadow_direction: 0.0,
            in_highlight: false,
            in_shape_effect_list: false,
            in_shape_outer_shadow: false,
            shape_shadow_blur: 0.0,
            shape_shadow_distance: 0.0,
            shape_shadow_direction: 0.0,
            shape_shadow_alpha: 1.0,
            in_shape_glow: false,
            shape_glow_radius: 0.0,
            shape_glow_alpha: 1.0,
            shape_effect_color: None,
            in_style: false,
            style_builder: None,
            style_current_ref: None,
            style_index: None,
            in_background: false,
            in_background_image: false,
            background_image_rel_id: None,
            background_solid_color: None,
            in_background_gradient: false,
            background_gradient_stops: Vec::new(),
            background_gradient_angle: 0.0,
            background_gradient_type: GradientType::Linear,
            background_stop_position: 0.0,
        }
    }
}

impl FillSaxState {
    pub(crate) fn route_color(&mut self, color: Color, targets: ColorTargets<'_>) {
        if self.in_highlight {
            if targets.table.in_run_properties {
                if let Some(run) = targets.table.run.as_mut() {
                    run.highlight = Some(color);
                }
            } else if targets.text.in_run_properties
                && let Some(run) = targets.text.run.as_mut()
            {
                run.highlight = Some(color);
            }
        } else if self.in_text_outer_shadow {
            self.current_color = Some(color);
        } else if self.in_shape_outer_shadow || self.in_shape_glow {
            self.shape_effect_color = Some(color);
        } else if targets.text.in_default_run_properties || targets.table.in_default_run_properties
        {
            self.current_color = Some(color);
        } else if targets.table.in_bullet_color {
            if let Some(paragraph) = targets.table.paragraph.as_mut() {
                paragraph.bu_color = Some(color);
            }
        } else if targets.text.in_list_default_run_properties {
            if let Some(defaults) = targets.text.run_defaults.as_mut() {
                defaults.color = Some(color);
            }
        } else if targets.table.in_properties {
            assign_cell_color(color, &targets.table.border_side, &mut targets.table.cell);
        } else if targets.table.in_run_properties {
            if let Some(run) = targets.table.run.as_mut() {
                run.color = color;
            }
        } else if targets.text.in_bullet_color {
            if let Some(paragraph) = targets.text.paragraph.as_mut() {
                paragraph.bu_color = Some(color);
            }
        } else if self.in_style && self.style_current_ref.is_some() {
            assign_style_ref_color(
                self.style_current_ref.as_deref().unwrap_or(""),
                self.style_index.as_deref().unwrap_or("0"),
                color,
                &mut self.style_builder,
            );
        } else if self.in_background && !self.in_background_image {
            assign_background_color_target(
                color,
                targets.depth,
                self.in_background_gradient,
                self.background_stop_position,
                &mut self.background_gradient_stops,
                &mut self.background_solid_color,
            );
        } else {
            assign_color(
                color,
                targets.depth,
                targets.in_shape_properties,
                self.in_line,
                targets.text.in_run_properties,
                self.in_gradient,
                self.gradient_stop_position,
                targets.shape,
                &mut targets.text.run,
                &mut self.gradient_stops,
            );
        }
    }

    pub(crate) fn take_completed_color(&mut self, local: &str) -> Option<Color> {
        matches!(local, "srgbClr" | "schemeClr" | "prstClr" | "sysClr")
            .then(|| self.current_color.take())
            .flatten()
    }

    pub(crate) fn parse_empty_color(&self, local: &str, element: &BytesStart<'_>) -> Option<Color> {
        if local == "sysClr" {
            return xml_utils::attr_str(element, "lastClr")
                .map(Color::rgb)
                .or_else(|| xml_utils::attr_str(element, "val").map(Color::system));
        }
        parse_color(local, element)
    }

    pub(crate) fn handle_empty(
        &mut self,
        local: &str,
        element: &BytesStart<'_>,
        in_shape_properties: bool,
        shape: &mut Option<ShapeBuilder>,
    ) -> bool {
        match local {
            "lin" if self.in_background_gradient => {
                self.background_gradient_angle = xml_utils::attr_str(element, "ang")
                    .and_then(|value| value.parse::<f64>().ok())
                    .map(|value| value / 60_000.0)
                    .unwrap_or(0.0);
                self.background_gradient_type = GradientType::Linear;
            }
            "path" if self.in_background_gradient => {
                if let Some(value) = xml_utils::attr_str(element, "path") {
                    self.background_gradient_type = GradientType::from_path_attr(&value);
                }
            }
            "lnRef" | "fillRef" | "effectRef" | "fontRef" if self.in_style => {
                let index = xml_utils::attr_str(element, "idx").unwrap_or_default();
                assign_style_ref_no_color(local, &index, &mut self.style_builder);
            }
            "blip" if self.in_background_image => {
                self.background_image_rel_id = relationship_embed_id(element);
            }
            "noFill" if self.in_line => {
                if let Some(shape) = shape.as_mut() {
                    shape.border_style = BorderStyle::None;
                    shape.border_width = 0.0;
                    shape.border_no_fill = true;
                }
            }
            "noFill" if in_shape_properties => {
                if let Some(shape) = shape.as_mut() {
                    shape.fill = Fill::NoFill;
                }
            }
            "prstDash" if self.in_line => {
                let Some(shape) = shape.as_mut() else {
                    return true;
                };
                let Some(value) = xml_utils::attr_str(element, "val") else {
                    return true;
                };
                shape.border_style = match value.as_str() {
                    "solid" => BorderStyle::Solid,
                    "dash" | "lgDash" | "sysDash" => BorderStyle::Dashed,
                    "dot" | "sysDot" | "lgDashDot" | "lgDashDotDot" | "sysDashDot"
                    | "sysDashDotDot" => BorderStyle::Dotted,
                    _ => BorderStyle::Solid,
                };
                shape.dash_style = match value.as_str() {
                    "dash" => DashStyle::Dash,
                    "dot" => DashStyle::Dot,
                    "dashDot" => DashStyle::DashDot,
                    "lgDash" => DashStyle::LongDash,
                    "lgDashDot" => DashStyle::LongDashDot,
                    "lgDashDotDot" => DashStyle::LongDashDotDot,
                    "sysDash" => DashStyle::SystemDash,
                    "sysDot" => DashStyle::SystemDot,
                    "sysDashDot" => DashStyle::SystemDashDot,
                    "sysDashDotDot" => DashStyle::SystemDashDotDot,
                    _ => DashStyle::Solid,
                };
            }
            "round" if self.in_line => {
                if let Some(shape) = shape.as_mut() {
                    shape.border_join = LineJoin::Round;
                }
            }
            "bevel" if self.in_line => {
                if let Some(shape) = shape.as_mut() {
                    shape.border_join = LineJoin::Bevel;
                }
            }
            "miter" if self.in_line => {
                if let Some(shape) = shape.as_mut() {
                    shape.border_join = LineJoin::Miter;
                    shape.miter_limit = xml_utils::attr_str(element, "lim")
                        .and_then(|value| value.parse::<f64>().ok())
                        .map(|value| value / 100_000.0);
                }
            }
            "headEnd" if self.in_line => {
                if let Some(shape) = shape.as_mut() {
                    shape.head_end = parse_line_end(element);
                }
            }
            "tailEnd" if self.in_line => {
                if let Some(shape) = shape.as_mut() {
                    shape.tail_end = parse_line_end(element);
                }
            }
            "lin" if self.in_gradient => {
                self.gradient_angle = xml_utils::attr_str(element, "ang")
                    .and_then(|value| value.parse::<f64>().ok())
                    .map(|value| value / 60_000.0)
                    .unwrap_or(0.0);
                self.gradient_type = GradientType::Linear;
            }
            "path" if self.in_gradient => {
                if let Some(value) = xml_utils::attr_str(element, "path") {
                    self.gradient_type = GradientType::from_path_attr(&value);
                }
            }
            "tint" | "shade" | "alpha" | "lumMod" | "lumOff" | "satMod" | "satOff" | "hueMod"
            | "hueOff" | "comp" | "inv" | "gray" => {
                let value =
                    xml_utils::attr_str(element, "val").and_then(|value| value.parse::<i32>().ok());
                if let Some(modifier) = ColorModifier::from_ooxml(local, value) {
                    if self.in_shape_outer_shadow || self.in_shape_glow {
                        if let Some(color) = self.shape_effect_color.as_mut() {
                            color.modifiers.push(modifier);
                        }
                    } else if let Some(color) = self.current_color.as_mut() {
                        color.modifiers.push(modifier);
                    }
                }
            }
            _ => return false,
        }
        true
    }
    pub(crate) fn handle_start(
        &mut self,
        local: &str,
        element: &BytesStart<'_>,
        in_shape_properties: bool,
        shape: &mut Option<ShapeBuilder>,
        text: &TextSaxState,
        table: &TableSaxState,
    ) -> bool {
        match local {
            "bgPr" => self.in_background = true,
            "style" if shape.is_some() && !in_shape_properties => {
                self.in_style = true;
                self.style_builder = Some(ShapeStyleRef::default());
            }
            "lnRef" | "fillRef" | "effectRef" | "fontRef" if self.in_style => {
                self.style_current_ref = Some(local.to_owned());
                self.style_index = xml_utils::attr_str(element, "idx");
            }
            "gradFill" if self.in_background => {
                self.in_background_gradient = true;
                self.background_gradient_stops.clear();
                self.background_gradient_angle = 0.0;
                self.background_gradient_type = GradientType::Linear;
            }
            "gs" if self.in_background_gradient => {
                self.background_stop_position = xml_utils::attr_str(element, "pos")
                    .and_then(|value| value.parse::<f64>().ok())
                    .map(|value| value / 100_000.0)
                    .unwrap_or(0.0);
            }
            "lin" if self.in_background_gradient => {
                self.background_gradient_angle = xml_utils::attr_str(element, "ang")
                    .and_then(|value| value.parse::<f64>().ok())
                    .map(|value| value / 60_000.0)
                    .unwrap_or(0.0);
                self.background_gradient_type = GradientType::Linear;
            }
            "path" if self.in_background_gradient => {
                if let Some(value) = xml_utils::attr_str(element, "path") {
                    self.background_gradient_type = GradientType::from_path_attr(&value);
                }
            }
            "blipFill" if self.in_background => self.in_background_image = true,
            "blip" if self.in_background_image => {
                self.background_image_rel_id = relationship_embed_id(element);
            }
            "ln" if in_shape_properties => {
                self.in_line = true;
                let shape = shape.as_mut().expect("shape builder for line");
                shape.border_width = xml_utils::attr_str(element, "w")
                    .map(|value| Emu::parse_emu(&value).to_pt())
                    .unwrap_or(0.0);
                shape.border_cap = match xml_utils::attr_str(element, "cap").as_deref() {
                    Some("rnd") => LineCap::Round,
                    Some("flat") => LineCap::Flat,
                    _ => LineCap::Square,
                };
                shape.border_compound = match xml_utils::attr_str(element, "cmpd").as_deref() {
                    Some("dbl") => CompoundLine::Double,
                    Some("thickThin") => CompoundLine::ThickThin,
                    Some("thinThick") => CompoundLine::ThinThick,
                    Some("tri") => CompoundLine::Triple,
                    _ => CompoundLine::Single,
                };
                shape.border_alignment = match xml_utils::attr_str(element, "algn").as_deref() {
                    Some("in") => LineAlignment::Inset,
                    _ => LineAlignment::Center,
                };
            }
            "gradFill" if in_shape_properties && !self.in_line => {
                self.in_gradient = true;
                self.gradient_stops.clear();
                self.gradient_angle = 0.0;
                self.gradient_type = GradientType::Linear;
            }
            "gs" if self.in_gradient => {
                self.gradient_stop_position = xml_utils::attr_str(element, "pos")
                    .and_then(|value| value.parse::<f64>().ok())
                    .map(|value| value / 100_000.0)
                    .unwrap_or(0.0);
            }
            "lin" if self.in_gradient => {
                self.gradient_angle = xml_utils::attr_str(element, "ang")
                    .and_then(|value| value.parse::<f64>().ok())
                    .map(|value| value / 60_000.0)
                    .unwrap_or(0.0);
            }
            "path" if self.in_gradient => {
                if let Some(value) = xml_utils::attr_str(element, "path") {
                    self.gradient_type = GradientType::from_path_attr(&value);
                }
            }
            "srgbClr" | "schemeClr" | "prstClr" | "sysClr" => {
                let color = parse_color(local, element);
                if self.in_shape_outer_shadow || self.in_shape_glow {
                    self.shape_effect_color = color;
                } else {
                    self.current_color = color;
                }
            }
            "effectLst" if text.in_run_properties || table.in_run_properties => {
                self.in_text_effect_list = true;
            }
            "outerShdw" if self.in_text_effect_list => {
                self.in_text_outer_shadow = true;
                (
                    self.text_shadow_blur,
                    self.text_shadow_distance,
                    self.text_shadow_direction,
                ) = parse_outer_shadow(element);
            }
            "highlight" if text.in_run_properties || table.in_run_properties => {
                self.in_highlight = true;
                self.current_color = None;
            }
            "effectLst" if in_shape_properties && shape.is_some() => {
                self.in_shape_effect_list = true;
            }
            "outerShdw" if self.in_shape_effect_list => {
                self.in_shape_outer_shadow = true;
                (
                    self.shape_shadow_blur,
                    self.shape_shadow_distance,
                    self.shape_shadow_direction,
                ) = parse_outer_shadow(element);
                self.shape_shadow_alpha = 1.0;
            }
            "glow" if self.in_shape_effect_list => {
                self.in_shape_glow = true;
                self.shape_glow_radius = parse_glow_radius(element);
                self.shape_glow_alpha = 1.0;
            }
            _ => return false,
        }
        true
    }

    pub(crate) fn handle_end<R: Read + Seek>(
        &mut self,
        local: &str,
        targets: FillEndTargets<'_, R>,
    ) -> bool {
        match local {
            "blipFill" if self.in_background_image => self.in_background_image = false,
            "bgPr" if self.in_background => {
                self.in_background = false;
                self.finish_background(targets.slide, targets.relationships, targets.archive);
            }
            "lnRef" | "fillRef" | "effectRef" | "fontRef"
                if self.in_style && self.style_current_ref.is_some() =>
            {
                if let Some(index) = self.style_index.take() {
                    ensure_style_ref(local, &index, &mut self.style_builder);
                }
                self.style_current_ref = None;
            }
            "style" if self.in_style => {
                self.in_style = false;
                if let Some(shape) = targets.shape.as_mut() {
                    shape.style_ref = self.style_builder.take();
                }
            }
            "gradFill" if self.in_background_gradient => {
                self.in_background_gradient = false;
            }
            "effectLst" if self.in_text_effect_list => self.in_text_effect_list = false,
            "effectLst" if self.in_shape_effect_list => self.in_shape_effect_list = false,
            "outerShdw" if self.in_text_outer_shadow => {
                self.in_text_outer_shadow = false;
                if let Some(color) = self.current_color.take() {
                    let shadow = TextShadow {
                        color,
                        blur_rad: self.text_shadow_blur,
                        dist: self.text_shadow_distance,
                        dir: self.text_shadow_direction,
                    };
                    if targets.table.in_run_properties {
                        if let Some(run) = targets.table.run.as_mut() {
                            run.shadow = Some(shadow);
                        }
                    } else if targets.text.in_run_properties
                        && let Some(run) = targets.text.run.as_mut()
                    {
                        run.shadow = Some(shadow);
                    }
                }
            }
            "outerShdw" if self.in_shape_outer_shadow => {
                self.in_shape_outer_shadow = false;
                finish_outer_shadow(
                    targets.shape,
                    &mut self.shape_effect_color,
                    self.shape_shadow_blur,
                    self.shape_shadow_distance,
                    self.shape_shadow_direction,
                    self.shape_shadow_alpha,
                );
            }
            "glow" if self.in_shape_glow => {
                self.in_shape_glow = false;
                finish_glow(
                    targets.shape,
                    &mut self.shape_effect_color,
                    self.shape_glow_radius,
                    self.shape_glow_alpha,
                );
            }
            "highlight" if self.in_highlight => {
                self.in_highlight = false;
                if let Some(color) = self.current_color.take() {
                    if targets.table.in_run_properties {
                        if let Some(run) = targets.table.run.as_mut() {
                            run.highlight = Some(color);
                        }
                    } else if targets.text.in_run_properties
                        && let Some(run) = targets.text.run.as_mut()
                    {
                        run.highlight = Some(color);
                    }
                }
            }
            "gradFill" if self.in_gradient => {
                self.in_gradient = false;
                if let Some(shape) = targets.shape.as_mut() {
                    shape.fill = Fill::Gradient(GradientFill {
                        gradient_type: std::mem::take(&mut self.gradient_type),
                        stops: std::mem::take(&mut self.gradient_stops),
                        angle: self.gradient_angle,
                    });
                }
            }
            "ln" if self.in_line => self.in_line = false,
            _ => return false,
        }
        true
    }

    fn finish_background<R: Read + Seek>(
        &mut self,
        slide: &mut Slide,
        relationships: &HashMap<String, String>,
        archive: &mut ZipArchive<R>,
    ) {
        if let Some(rel_id) = self.background_image_rel_id.take() {
            let Some(target) = relationships.get(&rel_id) else {
                return;
            };
            let path = resolve_relationship_path("ppt/slides", target);
            let Ok(mut entry) = archive.by_name(&path) else {
                return;
            };
            let mut data = Vec::new();
            let _ = entry.read_to_end(&mut data);
            if !data.is_empty() {
                slide.background = Some(Fill::Image(ImageFill {
                    rel_id,
                    data,
                    content_type: mime_from_extension(&path),
                }));
            }
        } else if let Some(color) = self.background_solid_color.take() {
            slide.background = Some(Fill::Solid(SolidFill { color }));
        } else if !self.background_gradient_stops.is_empty() {
            slide.background = Some(Fill::Gradient(GradientFill {
                gradient_type: std::mem::take(&mut self.background_gradient_type),
                stops: std::mem::take(&mut self.background_gradient_stops),
                angle: self.background_gradient_angle,
            }));
        }
    }
}

fn relationship_embed_id(element: &BytesStart<'_>) -> Option<String> {
    element.attributes().flatten().find_map(|attribute| {
        let key = std::str::from_utf8(attribute.key.as_ref()).unwrap_or("");
        key.ends_with("embed")
            .then(|| String::from_utf8_lossy(&attribute.value).to_string())
    })
}

fn parse_color(local: &str, element: &BytesStart<'_>) -> Option<Color> {
    match local {
        "srgbClr" => xml_utils::attr_str(element, "val").map(Color::rgb),
        "schemeClr" => xml_utils::attr_str(element, "val").map(Color::theme),
        "prstClr" => xml_utils::attr_str(element, "val").map(Color::preset),
        "sysClr" => xml_utils::attr_str(element, "val")
            .map(Color::system)
            .or_else(|| xml_utils::attr_str(element, "lastClr").map(Color::rgb)),
        _ => None,
    }
}

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
#[cfg(test)]
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
