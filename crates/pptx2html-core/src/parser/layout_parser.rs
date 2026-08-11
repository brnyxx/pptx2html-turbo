use std::collections::HashMap;
use std::io::{Read, Seek};

use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

use super::master_parser::{
    is_lvl_ppr, parse_def_rpr_attrs, parse_lvl_index, parse_lvl_ppr_attrs, parse_placeholder_attrs,
};
use super::pattern_fill_parser::PatternBuilder;
use super::slide_parser::{parse_autofit_ratio, parse_guide_formula_value, parse_line_end};
use super::xml_utils;
use crate::error::{PptxError, PptxResult};
use crate::model::*;

/// Parse slideLayout XML into SlideLayout
pub fn parse_slide_layout<R: Read + Seek>(
    xml: &str,
    rels: &HashMap<String, String>,
    archive: &mut ZipArchive<R>,
) -> PptxResult<SlideLayout> {
    let mut reader = Reader::from_str(xml);
    let mut layout = SlideLayout::default();

    let mut depth: Vec<String> = Vec::new();
    let mut in_sp_tree = false;
    let mut current_shape: Option<LayoutShapeBuilder> = None;
    let mut in_nv_pr = false;
    let mut in_tx_body = false;
    let mut in_lst_style = false;
    let mut current_lvl: Option<usize> = None;
    let mut current_para_defaults: Option<ParagraphDefaults> = None;
    let mut current_run_defaults: Option<RunDefaults> = None;
    let mut in_def_rpr = false;
    let mut current_color: Option<Color> = None;
    let mut in_ln_spc = false;
    let mut in_spc_bef = false;
    let mut in_spc_aft = false;
    let mut in_ln = false;
    let mut in_av_lst = false;

    // Background parsing state
    let mut in_bg_pr = false;
    let mut in_bg_blip_fill = false;
    let mut bg_blip_rel_id: Option<String> = None;
    let mut bg_solid_color: Option<Color> = None;
    let mut in_bg_grad_fill = false;
    let mut bg_grad_stops: Vec<GradientStop> = Vec::new();
    let mut bg_grad_angle: f64 = 0.0;
    let mut bg_grad_type = GradientType::Linear;
    let mut bg_gs_pos: f64 = 0.0;
    let mut bg_pattern: Option<PatternBuilder> = None;
    let mut shape_pattern: Option<PatternBuilder> = None;

    // Parse root element attributes
    // We handle them in the first Start event for sldLayout

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = xml_utils::local_name(e.name().as_ref()).to_string();
                depth.push(local.clone());

                if local == "pattFill" && in_bg_pr {
                    let mut pattern = PatternBuilder::default();
                    pattern.begin(e);
                    bg_pattern = Some(pattern);
                    continue;
                }
                if local == "pattFill" && current_shape.is_some() {
                    let mut pattern = PatternBuilder::default();
                    pattern.begin(e);
                    shape_pattern = Some(pattern);
                    continue;
                }
                if matches!(local.as_str(), "fgClr" | "bgClr")
                    && let Some(pattern) = bg_pattern.as_mut().or(shape_pattern.as_mut())
                {
                    pattern.start_color_role(&local);
                    continue;
                }
                if let Some(pattern) = bg_pattern.as_mut().or(shape_pattern.as_mut())
                    && (pattern.parse_color(&local, e) || pattern.append_modifier(&local, e))
                {
                    continue;
                }

                match local.as_str() {
                    "sldLayout" => {
                        // Root element attributes
                        if let Some(t) = xml_utils::attr_str(e, "type") {
                            layout.layout_type = Some(t);
                        }
                        if let Some(show) = xml_utils::attr_str(e, "showMasterSp") {
                            layout.show_master_sp = show != "0" && show != "false";
                        }
                    }
                    // Background properties
                    "bgPr" => in_bg_pr = true,
                    "blipFill" if in_bg_pr => in_bg_blip_fill = true,
                    "blip" if in_bg_blip_fill => {
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            if key.ends_with("embed") {
                                bg_blip_rel_id =
                                    Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                    "gradFill" if in_bg_pr => {
                        in_bg_grad_fill = true;
                        bg_grad_stops.clear();
                        bg_grad_angle = 0.0;
                        bg_grad_type = GradientType::Linear;
                    }
                    "path" if in_bg_grad_fill => {
                        if let Some(val) = xml_utils::attr_str(e, "path") {
                            bg_grad_type = GradientType::from_path_attr(&val);
                        }
                    }
                    "gs" if in_bg_grad_fill => {
                        bg_gs_pos = xml_utils::attr_str(e, "pos")
                            .and_then(|v| v.parse::<f64>().ok())
                            .map(|v| v / 100_000.0)
                            .unwrap_or(0.0);
                    }
                    "srgbClr" if in_bg_pr && !in_bg_blip_fill => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            let color = Color::rgb(val);
                            if in_bg_grad_fill {
                                bg_grad_stops.push(GradientStop {
                                    position: bg_gs_pos,
                                    color,
                                });
                            } else {
                                bg_solid_color = Some(color);
                            }
                        }
                    }
                    "schemeClr" if in_bg_pr && !in_bg_blip_fill => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            let color = Color::theme(val);
                            if in_bg_grad_fill {
                                bg_grad_stops.push(GradientStop {
                                    position: bg_gs_pos,
                                    color,
                                });
                            } else {
                                bg_solid_color = Some(color);
                            }
                        }
                    }
                    "spTree" => in_sp_tree = true,
                    "sp" if in_sp_tree => {
                        current_shape = Some(LayoutShapeBuilder::default());
                    }
                    "txBody" if current_shape.is_some() => {
                        in_tx_body = true;
                    }
                    "bodyPr" if current_shape.is_some() && in_tx_body => {
                        if let Some(sb) = current_shape.as_mut() {
                            if let Some(anchor) = xml_utils::attr_str(e, "anchor") {
                                sb.text_vertical_align = VerticalAlign::from_ooxml(&anchor);
                                sb.text_vertical_align_explicit = true;
                            }
                            if let Some(anchor_ctr) = xml_utils::attr_str(e, "anchorCtr") {
                                sb.text_anchor_center = anchor_ctr == "1" || anchor_ctr == "true";
                            }
                            if let Some(rot) = xml_utils::attr_str(e, "rot") {
                                sb.text_rotation_deg = rot.parse::<f64>().unwrap_or(0.0) / 60_000.0;
                            }
                            if let Some(vert) = xml_utils::attr_str(e, "vert") {
                                sb.vertical_text_explicit = true;
                                sb.vertical_text = if vert == "horz" { None } else { Some(vert) };
                            }
                            if let Some(v) = xml_utils::attr_str(e, "lIns") {
                                sb.text_margins.left = Emu::parse_emu(&v).to_pt();
                                sb.text_margin_left_explicit = true;
                            }
                            if let Some(v) = xml_utils::attr_str(e, "tIns") {
                                sb.text_margins.top = Emu::parse_emu(&v).to_pt();
                                sb.text_margin_top_explicit = true;
                            }
                            if let Some(v) = xml_utils::attr_str(e, "rIns") {
                                sb.text_margins.right = Emu::parse_emu(&v).to_pt();
                                sb.text_margin_right_explicit = true;
                            }
                            if let Some(v) = xml_utils::attr_str(e, "bIns") {
                                sb.text_margins.bottom = Emu::parse_emu(&v).to_pt();
                                sb.text_margin_bottom_explicit = true;
                            }
                            if let Some(wrap) = xml_utils::attr_str(e, "wrap") {
                                sb.text_word_wrap = wrap != "none";
                                sb.text_word_wrap_explicit = true;
                            }
                        }
                    }
                    "normAutofit" if current_shape.is_some() && in_tx_body => {
                        if let Some(sb) = current_shape.as_mut() {
                            let font_scale = parse_autofit_ratio(e, "fontScale");
                            let line_spacing_reduction = parse_autofit_ratio(e, "lnSpcReduction");
                            sb.text_auto_fit = AutoFit::Normal {
                                font_scale,
                                line_spacing_reduction,
                            };
                        }
                    }
                    "noAutofit" if current_shape.is_some() && in_tx_body => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.text_auto_fit = AutoFit::NoAutoFit;
                        }
                    }
                    "spAutoFit" if current_shape.is_some() && in_tx_body => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.text_auto_fit = AutoFit::Shrink;
                        }
                    }
                    "lstStyle" if in_tx_body => {
                        in_lst_style = true;
                    }
                    s if in_lst_style && is_lvl_ppr(s) => {
                        let lvl = parse_lvl_index(s);
                        current_lvl = Some(lvl);
                        let mut pd = ParagraphDefaults::default();
                        parse_lvl_ppr_attrs(e, &mut pd);
                        current_para_defaults = Some(pd);
                    }
                    "defRPr" if in_lst_style && current_lvl.is_some() => {
                        in_def_rpr = true;
                        let mut rd = RunDefaults::default();
                        parse_def_rpr_attrs(e, &mut rd);
                        current_run_defaults = Some(rd);
                    }
                    "lnSpc" if in_lst_style && current_lvl.is_some() && !in_def_rpr => {
                        in_ln_spc = true;
                    }
                    "spcBef" if in_lst_style && current_lvl.is_some() && !in_def_rpr => {
                        in_spc_bef = true;
                    }
                    "spcAft" if in_lst_style && current_lvl.is_some() && !in_def_rpr => {
                        in_spc_aft = true;
                    }
                    "srgbClr" if in_def_rpr => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            current_color = Some(Color::rgb(val));
                        }
                    }
                    "schemeClr" if in_def_rpr => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            current_color = Some(Color::theme(val));
                        }
                    }
                    "nvPr" if current_shape.is_some() => {
                        in_nv_pr = true;
                    }
                    "ln" if current_shape.is_some() && depth.iter().any(|d| d == "spPr") => {
                        in_ln = true;
                        if let Some(sb) = current_shape.as_mut() {
                            sb.border.width = xml_utils::attr_str(e, "w")
                                .map(|w| Emu::parse_emu(&w).to_pt())
                                .unwrap_or(0.0);
                            sb.border.cap = match xml_utils::attr_str(e, "cap").as_deref() {
                                Some("rnd") => LineCap::Round,
                                Some("flat") => LineCap::Flat,
                                _ => LineCap::Square,
                            };
                            sb.border.compound = match xml_utils::attr_str(e, "cmpd").as_deref() {
                                Some("dbl") => CompoundLine::Double,
                                Some("thickThin") => CompoundLine::ThickThin,
                                Some("thinThick") => CompoundLine::ThinThick,
                                Some("tri") => CompoundLine::Triple,
                                _ => CompoundLine::Single,
                            };
                            sb.border.alignment = match xml_utils::attr_str(e, "algn").as_deref() {
                                Some("in") => LineAlignment::Inset,
                                _ => LineAlignment::Center,
                            };
                            sb.border.join = LineJoin::Miter;
                            sb.border.miter_limit = None;
                            sb.border.no_fill = false;
                        }
                    }
                    "xfrm" if current_shape.is_some() => {
                        if let Some(sb) = current_shape.as_mut() {
                            apply_layout_shape_transform(sb, e);
                        }
                    }
                    "prstGeom" if current_shape.is_some() => {
                        if let Some(sb) = current_shape.as_mut() {
                            set_layout_preset_geometry(sb, e);
                        }
                    }
                    "avLst" if current_shape.is_some() => {
                        in_av_lst = true;
                    }
                    "gd" if in_av_lst => {
                        if let Some(sb) = current_shape.as_mut() {
                            store_layout_adjust_value(sb, e);
                        }
                    }
                    "clrMapOvr" => {
                        // Will check child elements
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let local = xml_utils::local_name(e.name().as_ref()).to_string();

                if local == "pattFill" {
                    let mut pattern = PatternBuilder::default();
                    pattern.begin(e);
                    if in_bg_pr {
                        layout.background = Some(Fill::Pattern(pattern.finish()));
                        continue;
                    }
                    if let Some(shape) = current_shape.as_mut() {
                        shape.fill = Fill::Pattern(pattern.finish());
                        continue;
                    }
                }

                if let Some(pattern) = bg_pattern.as_mut().or(shape_pattern.as_mut())
                    && (pattern.parse_color(&local, e) || pattern.append_modifier(&local, e))
                {
                    continue;
                }

                match local.as_str() {
                    // Background blip (Empty variant)
                    "blip" if in_bg_blip_fill => {
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            if key.ends_with("embed") {
                                bg_blip_rel_id =
                                    Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                    "lin" if in_bg_grad_fill => {
                        if let Some(ang) = xml_utils::attr_str(e, "ang") {
                            bg_grad_angle = ang.parse::<f64>().unwrap_or(0.0) / 60_000.0;
                        }
                        bg_grad_type = GradientType::Linear;
                    }
                    "bodyPr" if current_shape.is_some() && in_tx_body => {
                        if let Some(sb) = current_shape.as_mut() {
                            if let Some(anchor) = xml_utils::attr_str(e, "anchor") {
                                sb.text_vertical_align = VerticalAlign::from_ooxml(&anchor);
                                sb.text_vertical_align_explicit = true;
                            }
                            if let Some(anchor_ctr) = xml_utils::attr_str(e, "anchorCtr") {
                                sb.text_anchor_center = anchor_ctr == "1" || anchor_ctr == "true";
                            }
                            if let Some(rot) = xml_utils::attr_str(e, "rot") {
                                sb.text_rotation_deg = rot.parse::<f64>().unwrap_or(0.0) / 60_000.0;
                            }
                            if let Some(vert) = xml_utils::attr_str(e, "vert") {
                                sb.vertical_text_explicit = true;
                                sb.vertical_text = if vert == "horz" { None } else { Some(vert) };
                            }
                            if let Some(v) = xml_utils::attr_str(e, "lIns") {
                                sb.text_margins.left = Emu::parse_emu(&v).to_pt();
                                sb.text_margin_left_explicit = true;
                            }
                            if let Some(v) = xml_utils::attr_str(e, "tIns") {
                                sb.text_margins.top = Emu::parse_emu(&v).to_pt();
                                sb.text_margin_top_explicit = true;
                            }
                            if let Some(v) = xml_utils::attr_str(e, "rIns") {
                                sb.text_margins.right = Emu::parse_emu(&v).to_pt();
                                sb.text_margin_right_explicit = true;
                            }
                            if let Some(v) = xml_utils::attr_str(e, "bIns") {
                                sb.text_margins.bottom = Emu::parse_emu(&v).to_pt();
                                sb.text_margin_bottom_explicit = true;
                            }
                            if let Some(wrap) = xml_utils::attr_str(e, "wrap") {
                                sb.text_word_wrap = wrap != "none";
                                sb.text_word_wrap_explicit = true;
                            }
                        }
                    }
                    "path" if in_bg_grad_fill => {
                        if let Some(val) = xml_utils::attr_str(e, "path") {
                            bg_grad_type = GradientType::from_path_attr(&val);
                        }
                    }
                    "normAutofit" if current_shape.is_some() && in_tx_body => {
                        if let Some(sb) = current_shape.as_mut() {
                            let font_scale = parse_autofit_ratio(e, "fontScale");
                            let line_spacing_reduction = parse_autofit_ratio(e, "lnSpcReduction");
                            sb.text_auto_fit = AutoFit::Normal {
                                font_scale,
                                line_spacing_reduction,
                            };
                        }
                    }
                    "noAutofit" if current_shape.is_some() && in_tx_body => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.text_auto_fit = AutoFit::NoAutoFit;
                        }
                    }
                    "spAutoFit" if current_shape.is_some() && in_tx_body => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.text_auto_fit = AutoFit::Shrink;
                        }
                    }
                    "srgbClr" if in_bg_pr && !in_bg_blip_fill => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            let color = Color::rgb(val);
                            if in_bg_grad_fill && depth.iter().any(|d| d == "gs") {
                                bg_grad_stops.push(GradientStop {
                                    position: bg_gs_pos,
                                    color,
                                });
                            } else {
                                bg_solid_color = Some(color);
                            }
                        }
                    }
                    "schemeClr" if in_bg_pr && !in_bg_blip_fill => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            let color = Color::theme(val);
                            if in_bg_grad_fill && depth.iter().any(|d| d == "gs") {
                                bg_grad_stops.push(GradientStop {
                                    position: bg_gs_pos,
                                    color,
                                });
                            } else {
                                bg_solid_color = Some(color);
                            }
                        }
                    }
                    "srgbClr" if in_ln => {
                        if let Some(val) = xml_utils::attr_str(e, "val")
                            && let Some(sb) = current_shape.as_mut()
                        {
                            sb.border.color = Color::rgb(val);
                        }
                    }
                    "schemeClr" if in_ln => {
                        if let Some(val) = xml_utils::attr_str(e, "val")
                            && let Some(sb) = current_shape.as_mut()
                        {
                            sb.border.color = Color::theme(val);
                        }
                    }
                    // Placeholder in shape
                    "ph" if in_nv_pr && current_shape.is_some() => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.placeholder = Some(parse_placeholder_attrs(e));
                        }
                    }
                    "latin" if in_def_rpr => {
                        if let Some(rd) = current_run_defaults.as_mut()
                            && let Some(typeface) = xml_utils::attr_str(e, "typeface")
                        {
                            rd.font_latin = Some(typeface);
                        }
                    }
                    "ea" if in_def_rpr => {
                        if let Some(rd) = current_run_defaults.as_mut()
                            && let Some(typeface) = xml_utils::attr_str(e, "typeface")
                        {
                            rd.font_ea = Some(typeface);
                        }
                    }
                    "cs" if in_def_rpr => {
                        if let Some(rd) = current_run_defaults.as_mut()
                            && let Some(typeface) = xml_utils::attr_str(e, "typeface")
                        {
                            rd.font_cs = Some(typeface);
                        }
                    }
                    "srgbClr" if in_def_rpr => {
                        if let Some(val) = xml_utils::attr_str(e, "val")
                            && let Some(rd) = current_run_defaults.as_mut()
                        {
                            rd.color = Some(Color::rgb(val));
                        }
                    }
                    "schemeClr" if in_def_rpr => {
                        if let Some(val) = xml_utils::attr_str(e, "val")
                            && let Some(rd) = current_run_defaults.as_mut()
                        {
                            rd.color = Some(Color::theme(val));
                        }
                    }
                    "spcPct"
                        if in_lst_style
                            && current_lvl.is_some()
                            && (in_ln_spc || in_spc_bef || in_spc_aft) =>
                    {
                        if let Some(val_str) = xml_utils::attr_str(e, "val")
                            && let Ok(val) = val_str.parse::<f64>()
                        {
                            let spacing = SpacingValue::Percent(val / 100_000.0);
                            if let Some(pd) = current_para_defaults.as_mut() {
                                if in_ln_spc {
                                    pd.line_spacing = Some(spacing);
                                } else if in_spc_bef {
                                    pd.space_before = Some(spacing);
                                } else if in_spc_aft {
                                    pd.space_after = Some(spacing);
                                }
                            }
                        }
                    }
                    "spcPts"
                        if in_lst_style
                            && current_lvl.is_some()
                            && (in_ln_spc || in_spc_bef || in_spc_aft) =>
                    {
                        if let Some(val_str) = xml_utils::attr_str(e, "val")
                            && let Ok(val) = val_str.parse::<f64>()
                        {
                            let spacing = SpacingValue::Points(val / 100.0);
                            if let Some(pd) = current_para_defaults.as_mut() {
                                if in_ln_spc {
                                    pd.line_spacing = Some(spacing);
                                } else if in_spc_bef {
                                    pd.space_before = Some(spacing);
                                } else if in_spc_aft {
                                    pd.space_after = Some(spacing);
                                }
                            }
                        }
                    }
                    // ClrMapOverride with masterClrMapping (use master's ClrMap)
                    "masterClrMapping" => {
                        layout.clr_map_ovr = Some(ClrMapOverride::UseMaster);
                    }
                    // Position/size for shapes — only inside <a:xfrm>
                    "off" if current_shape.is_some() && depth.iter().any(|d| d == "xfrm") => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.position.x =
                                Emu::parse_emu(&xml_utils::attr_str(e, "x").unwrap_or_default());
                            sb.position.y =
                                Emu::parse_emu(&xml_utils::attr_str(e, "y").unwrap_or_default());
                        }
                    }
                    "ext" if current_shape.is_some() && depth.iter().any(|d| d == "xfrm") => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.size.width =
                                Emu::parse_emu(&xml_utils::attr_str(e, "cx").unwrap_or_default());
                            sb.size.height =
                                Emu::parse_emu(&xml_utils::attr_str(e, "cy").unwrap_or_default());
                        }
                    }
                    "xfrm" if current_shape.is_some() => {
                        if let Some(sb) = current_shape.as_mut() {
                            apply_layout_shape_transform(sb, e);
                        }
                    }
                    "prstGeom" if current_shape.is_some() => {
                        if let Some(sb) = current_shape.as_mut() {
                            set_layout_preset_geometry(sb, e);
                        }
                    }
                    "gd" if in_av_lst => {
                        if let Some(sb) = current_shape.as_mut() {
                            store_layout_adjust_value(sb, e);
                        }
                    }
                    // overrideClrMapping (override ClrMap)
                    "overrideClrMapping" => {
                        let mut clr_map = crate::model::presentation::ClrMap::default();
                        for attr in e.attributes().flatten() {
                            let key = xml_utils::local_name(attr.key.as_ref());
                            let val = String::from_utf8_lossy(&attr.value);
                            clr_map.set(key, &val);
                        }
                        layout.clr_map_ovr = Some(ClrMapOverride::Override(clr_map));
                    }
                    "noFill" if in_ln => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.border.style = BorderStyle::None;
                            sb.border.width = 0.0;
                            sb.border.no_fill = true;
                        }
                    }
                    "prstDash" if in_ln => {
                        if let Some(sb) = current_shape.as_mut()
                            && let Some(val) = xml_utils::attr_str(e, "val")
                        {
                            sb.border.style = match val.as_str() {
                                "solid" => BorderStyle::Solid,
                                "dash" | "lgDash" | "sysDash" => BorderStyle::Dashed,
                                "dot" | "sysDot" | "lgDashDot" | "lgDashDotDot" | "sysDashDot"
                                | "sysDashDotDot" => BorderStyle::Dotted,
                                _ => BorderStyle::Solid,
                            };
                            sb.border.dash_style = match val.as_str() {
                                "solid" => DashStyle::Solid,
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
                    }
                    "round" if in_ln => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.border.join = LineJoin::Round;
                        }
                    }
                    "bevel" if in_ln => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.border.join = LineJoin::Bevel;
                        }
                    }
                    "miter" if in_ln => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.border.join = LineJoin::Miter;
                            sb.border.miter_limit = xml_utils::attr_str(e, "lim")
                                .and_then(|v| v.parse::<f64>().ok())
                                .map(|v| v / 100_000.0);
                        }
                    }
                    "headEnd" if in_ln => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.border.head_end = parse_line_end(e);
                        }
                    }
                    "tailEnd" if in_ln => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.border.tail_end = parse_line_end(e);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let local = xml_utils::local_name(e.name().as_ref()).to_string();
                depth.pop();

                match local.as_str() {
                    "fgClr" | "bgClr" => {
                        if let Some(pattern) = bg_pattern.as_mut().or(shape_pattern.as_mut()) {
                            pattern.finish_color_role();
                        }
                    }
                    "pattFill" if bg_pattern.is_some() => {
                        layout.background = bg_pattern
                            .take()
                            .map(|mut pattern| Fill::Pattern(pattern.finish()));
                    }
                    "pattFill" if shape_pattern.is_some() => {
                        if let (Some(mut pattern), Some(shape)) =
                            (shape_pattern.take(), current_shape.as_mut())
                        {
                            shape.fill = Fill::Pattern(pattern.finish());
                        }
                    }
                    "blipFill" if in_bg_blip_fill => in_bg_blip_fill = false,
                    "gradFill" if in_bg_grad_fill => {
                        in_bg_grad_fill = false;
                    }
                    "bgPr" => {
                        in_bg_pr = false;
                        if let Some(rel_id) = bg_blip_rel_id.take() {
                            if let Some(target) = rels.get(&rel_id) {
                                let path = resolve_layout_rel_path("ppt/slideLayouts", target);
                                if let Ok(mut entry) = archive.by_name(&path) {
                                    let mut buf = Vec::new();
                                    let _ = std::io::Read::read_to_end(&mut entry, &mut buf);
                                    if !buf.is_empty() {
                                        let ct = layout_bg_mime(&path);
                                        layout.background = Some(Fill::Image(ImageFill {
                                            rel_id,
                                            data: buf,
                                            content_type: ct,
                                        }));
                                    }
                                }
                            }
                        } else if let Some(color) = bg_solid_color.take() {
                            layout.background = Some(Fill::Solid(SolidFill { color }));
                        } else if !bg_grad_stops.is_empty() {
                            layout.background = Some(Fill::Gradient(GradientFill {
                                gradient_type: std::mem::take(&mut bg_grad_type),
                                stops: std::mem::take(&mut bg_grad_stops),
                                angle: bg_grad_angle,
                            }));
                        }
                    }
                    "spTree" => in_sp_tree = false,
                    "nvPr" => in_nv_pr = false,
                    "txBody" => in_tx_body = false,
                    "lstStyle" => in_lst_style = false,
                    "defRPr" if in_def_rpr => {
                        if let Some(color) = current_color.take()
                            && let Some(rd) = current_run_defaults.as_mut()
                        {
                            rd.color = Some(color);
                        }
                        in_def_rpr = false;
                    }
                    "lnSpc" if in_ln_spc => in_ln_spc = false,
                    "spcBef" if in_spc_bef => in_spc_bef = false,
                    "spcAft" if in_spc_aft => in_spc_aft = false,
                    s if in_lst_style && is_lvl_ppr(&local) => {
                        if let Some(pd) = current_para_defaults.take() {
                            let mut pd = pd;
                            pd.def_run_props = current_run_defaults.take();
                            if let Some(sb) = current_shape.as_mut() {
                                store_shape_level_defaults(sb, s, pd);
                            }
                        }
                        current_lvl = None;
                    }
                    "ln" if in_ln => {
                        in_ln = false;
                        if let Some(sb) = current_shape.as_mut()
                            && !sb.border.no_fill
                            && sb.border.width > 0.0
                            && matches!(sb.border.style, BorderStyle::None)
                        {
                            sb.border.style = BorderStyle::Solid;
                        }
                    }
                    "avLst" if in_av_lst => {
                        in_av_lst = false;
                    }
                    "sp" if current_shape.is_some() => {
                        if let Some(sb) = current_shape.take() {
                            layout.shapes.push(sb.build());
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(PptxError::Xml(e)),
            _ => {}
        }
    }

    Ok(layout)
}

#[derive(Default)]
struct LayoutShapeBuilder {
    preset_geometry: Option<String>,
    adjust_values: HashMap<String, f64>,
    rotation: f64,
    flip_h: bool,
    flip_v: bool,
    position: Position,
    size: Size,
    placeholder: Option<PlaceholderInfo>,
    list_style: Option<ListStyle>,
    text_vertical_align: VerticalAlign,
    text_vertical_align_explicit: bool,
    text_anchor_center: bool,
    text_rotation_deg: f64,
    text_margins: TextMargins,
    text_margin_top_explicit: bool,
    text_margin_bottom_explicit: bool,
    text_margin_left_explicit: bool,
    text_margin_right_explicit: bool,
    text_word_wrap: bool,
    text_word_wrap_explicit: bool,
    text_auto_fit: AutoFit,
    vertical_text: Option<String>,
    vertical_text_explicit: bool,
    border: Border,
    fill: Fill,
}

impl LayoutShapeBuilder {
    fn build(self) -> Shape {
        let word_wrap = if self.text_word_wrap_explicit {
            self.text_word_wrap
        } else {
            true
        };
        let shape_type = match self.preset_geometry.as_deref() {
            Some("rect") | None => ShapeType::Rectangle,
            Some("roundRect") => ShapeType::RoundedRectangle,
            Some("ellipse") => ShapeType::Ellipse,
            Some("triangle") => ShapeType::Triangle,
            Some(other) => ShapeType::Custom(other.to_string()),
        };
        let adjust_values = (!self.adjust_values.is_empty()).then_some(self.adjust_values);
        Shape {
            fill: self.fill,
            shape_type,
            position: self.position,
            size: self.size,
            rotation: self.rotation,
            flip_h: self.flip_h,
            flip_v: self.flip_v,
            placeholder: self.placeholder,
            vertical_text: self.vertical_text,
            vertical_text_explicit: self.vertical_text_explicit,
            border: self.border,
            adjust_values,
            text_body: if self.list_style.is_some()
                || self.text_vertical_align_explicit
                || self.text_anchor_center
                || self.text_rotation_deg != 0.0
                || self.text_margin_top_explicit
                || self.text_margin_bottom_explicit
                || self.text_margin_left_explicit
                || self.text_margin_right_explicit
                || self.text_word_wrap_explicit
                || !matches!(self.text_auto_fit, AutoFit::None)
            {
                Some(TextBody {
                    list_style: self.list_style,
                    vertical_align: self.text_vertical_align,
                    vertical_align_explicit: self.text_vertical_align_explicit,
                    anchor_center: self.text_anchor_center,
                    text_rotation_deg: self.text_rotation_deg,
                    margin_top_explicit: self.text_margin_top_explicit,
                    margin_bottom_explicit: self.text_margin_bottom_explicit,
                    margin_left_explicit: self.text_margin_left_explicit,
                    margin_right_explicit: self.text_margin_right_explicit,
                    word_wrap,
                    word_wrap_explicit: self.text_word_wrap_explicit,
                    auto_fit: self.text_auto_fit,
                    margins: self.text_margins,
                    ..Default::default()
                })
            } else {
                None
            },
            ..Default::default()
        }
    }
}

fn apply_layout_shape_transform(
    shape: &mut LayoutShapeBuilder,
    e: &quick_xml::events::BytesStart<'_>,
) {
    if let Some(rot) = xml_utils::attr_str(e, "rot") {
        shape.rotation = rot.parse::<f64>().unwrap_or(0.0) / 60_000.0;
    }
    if let Some(flip_h) = xml_utils::attr_str(e, "flipH") {
        shape.flip_h = flip_h == "1" || flip_h == "true";
    }
    if let Some(flip_v) = xml_utils::attr_str(e, "flipV") {
        shape.flip_v = flip_v == "1" || flip_v == "true";
    }
}

fn set_layout_preset_geometry(
    shape: &mut LayoutShapeBuilder,
    e: &quick_xml::events::BytesStart<'_>,
) {
    if let Some(prst) = xml_utils::attr_str(e, "prst") {
        shape.preset_geometry = Some(prst);
    }
}

fn store_layout_adjust_value(
    shape: &mut LayoutShapeBuilder,
    e: &quick_xml::events::BytesStart<'_>,
) {
    if let (Some(name), Some(fmla)) = (
        xml_utils::attr_str(e, "name"),
        xml_utils::attr_str(e, "fmla"),
    ) && let Ok(value) = parse_guide_formula_value(&fmla, &HashMap::new())
    {
        shape.adjust_values.insert(name, value);
    }
}

fn store_shape_level_defaults(
    shape: &mut LayoutShapeBuilder,
    lvl_tag: &str,
    pd: ParagraphDefaults,
) {
    let lvl = parse_lvl_index(lvl_tag);
    let list_style = shape.list_style.get_or_insert_with(ListStyle::default);
    list_style.levels[lvl] = Some(pd);
}

fn resolve_layout_rel_path(base_dir: &str, target: &str) -> String {
    if !target.contains("../") {
        return format!("{base_dir}/{target}");
    }
    let mut parts: Vec<&str> = base_dir.split('/').collect();
    for segment in target.split('/') {
        if segment == ".." {
            parts.pop();
        } else if !segment.is_empty() && segment != "." {
            parts.push(segment);
        }
    }
    parts.join("/")
}

fn layout_bg_mime(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        _ => "image/png",
    }
    .to_string()
}

#[cfg(test)]
mod coverage_tests {
    use std::io::{Cursor, Write};

    use zip::write::SimpleFileOptions;
    use zip::{ZipArchive, ZipWriter};

    use super::*;

    #[test]
    fn parse_slide_layout_covers_background_overrides_and_shape_builders() {
        let xml = concat!(
            r#"<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="title" showMasterSp="0">"#,
            r#"<p:cSld><p:bg><p:bgPr><a:blipFill><a:blip r:embed="rIdBg"/></a:blipFill></p:bgPr></p:bg><p:spTree>"#,
            r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>"#,
            r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Layout Shape"/><p:cNvSpPr/><p:nvPr><p:ph type="title" idx="1"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm><a:ln w="12700" cap="rnd" cmpd="dbl" algn="in"><a:prstDash val="sysDashDot"/><a:schemeClr val="accent3"/><a:miter lim="200000"/><a:headEnd type="triangle" w="sm" len="lg"/><a:tailEnd type="oval" w="lg" len="sm"/></a:ln></p:spPr><p:txBody><a:bodyPr anchor="ctr" anchorCtr="1" rot="5400000" vert="vert" lIns="45720" tIns="91440" rIns="45720" bIns="91440" wrap="none"/><a:normAutofit fontScale="50000" lnSpcReduction="20000"/><a:lstStyle><a:lvl1pPr algn="ctr"><a:lnSpc><a:spcPct val="150000"/></a:lnSpc><a:spcBef><a:spcPts val="1200"/></a:spcBef><a:spcAft><a:spcPct val="30000"/></a:spcAft><a:defRPr sz="1800" b="1"><a:latin typeface="Calibri"/><a:ea typeface="Malgun Gothic"/><a:cs typeface="Mangal"/><a:srgbClr val="FF0000"/></a:defRPr></a:lvl1pPr></a:lstStyle></p:txBody></p:sp>"#,
            r#"</p:spTree></p:cSld><p:clrMapOvr><a:overrideClrMapping tx1="accent1" bg1="lt1"/></p:clrMapOvr></p:sldLayout>"#
        );

        let rels = HashMap::from([("rIdBg".to_string(), "../media/layout-bg.jpg".to_string())]);
        let mut archive = archive_with_entries(&[("ppt/media/layout-bg.jpg", b"jpgdata")]);
        let layout = parse_slide_layout(xml, &rels, &mut archive).expect("layout parses");

        assert_eq!(layout.layout_type.as_deref(), Some("title"));
        assert!(!layout.show_master_sp);
        assert!(matches!(layout.background, Some(Fill::Image(_))));
        assert_eq!(layout.shapes.len(), 1);
        assert!(matches!(
            layout.clr_map_ovr,
            Some(ClrMapOverride::Override(_))
        ));

        let shape = &layout.shapes[0];
        assert_eq!(shape.placeholder.as_ref().and_then(|p| p.idx), Some(1));
        let body = shape.text_body.as_ref().expect("text body");
        assert!(matches!(body.vertical_align, VerticalAlign::Middle));
        assert!(body.anchor_center);
        assert_eq!(body.text_rotation_deg, 90.0);
        assert_eq!(shape.vertical_text.as_deref(), Some("vert"));
        assert!(!body.word_wrap);
        assert!(matches!(
            body.auto_fit,
            AutoFit::Normal {
                font_scale: Some(0.5),
                line_spacing_reduction: Some(0.2)
            }
        ));
        assert!(matches!(shape.border.dash_style, DashStyle::SystemDashDot));
        assert!(matches!(shape.border.join, LineJoin::Miter));
        assert_eq!(shape.border.miter_limit, Some(2.0));
        assert!(matches!(
            shape.border.head_end.as_ref().map(|e| &e.end_type),
            Some(LineEndType::Triangle)
        ));
        assert!(matches!(
            shape.border.tail_end.as_ref().map(|e| &e.end_type),
            Some(LineEndType::Oval)
        ));
    }

    #[test]
    fn layout_helper_functions_cover_path_mime_and_shape_building() {
        assert_eq!(
            resolve_layout_rel_path("ppt/slideLayouts", "../media/image1.png"),
            "ppt/media/image1.png"
        );
        assert_eq!(
            resolve_layout_rel_path("ppt/slideLayouts", "media/image1.png"),
            "ppt/slideLayouts/media/image1.png"
        );
        assert_eq!(layout_bg_mime("bg.png"), "image/png");
        assert_eq!(layout_bg_mime("bg.jpg"), "image/jpeg");
        assert_eq!(layout_bg_mime("bg.gif"), "image/gif");
        assert_eq!(layout_bg_mime("bg.bmp"), "image/bmp");
        assert_eq!(layout_bg_mime("bg.svg"), "image/svg+xml");
        assert_eq!(layout_bg_mime("bg.unknown"), "image/png");

        let mut builder = LayoutShapeBuilder::default();
        store_shape_level_defaults(
            &mut builder,
            "lvl1pPr",
            ParagraphDefaults {
                alignment: Some(Alignment::Center),
                ..Default::default()
            },
        );
        let built = builder.build();
        assert!(built.text_body.is_some());
        assert!(
            built
                .text_body
                .as_ref()
                .and_then(|body| body.list_style.as_ref())
                .is_some()
        );
    }

    fn archive_with_entries(entries: &[(&str, &[u8])]) -> ZipArchive<Cursor<Vec<u8>>> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        for (path, data) in entries {
            zip.start_file(path, options).unwrap();
            zip.write_all(data).unwrap();
        }
        ZipArchive::new(Cursor::new(zip.finish().unwrap().into_inner())).unwrap()
    }
}

#[cfg(test)]
mod more_tests {
    use std::collections::HashMap;
    use std::io::{Cursor, Write};

    use zip::ZipArchive;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    #[test]
    fn parse_slide_layout_parses_background_image_override_mapping_and_shape_defaults() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             type="title" showMasterSp="0">
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
          <p:cNvPr id="2" name="Layout Placeholder"/>
          <p:cNvSpPr/>
          <p:nvPr><p:ph type="body" idx="4"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="12700" y="25400"/><a:ext cx="381000" cy="254000"/></a:xfrm>
          <a:ln w="12700" cap="rnd" cmpd="tri" algn="in">
            <a:prstDash val="sysDashDot"/>
            <a:bevel/>
            <a:schemeClr val="accent4"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr" anchorCtr="1" rot="5400000" vert="vert270" lIns="91440" tIns="45720" rIns="182880" bIns="22860" wrap="none"></a:bodyPr>
          <a:noAutofit/>
          <a:lstStyle>
            <a:lvl1pPr algn="ctr" marL="457200" indent="-228600">
              <a:lnSpc><a:spcPct val="90000"/></a:lnSpc>
              <a:spcBef><a:spcPts val="1200"/></a:spcBef>
              <a:spcAft><a:spcPct val="50000"/></a:spcAft>
              <a:defRPr sz="2400" spc="200" baseline="30000" cap="all" u="dbl" strike="sngStrike" b="1" i="1">
                <a:latin typeface="Aptos"/>
                <a:ea typeface="Yu Gothic"/>
                <a:cs typeface="Noto Sans Devanagari"/>
                <a:srgbClr val="112233"/>
              </a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr>
    <a:overrideClrMapping bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  </p:clrMapOvr>
</p:sldLayout>"#;

        let rels = HashMap::from([("rIdBg".to_string(), "../media/layout.png".to_string())]);
        let mut archive = archive_with_entries(&[("ppt/media/layout.png", b"png-data")]);
        let layout = parse_slide_layout(xml, &rels, &mut archive).expect("layout should parse");

        assert_eq!(layout.layout_type.as_deref(), Some("title"));
        assert!(!layout.show_master_sp);
        assert!(matches!(
            &layout.background,
            Some(Fill::Image(fill))
                if fill.rel_id == "rIdBg"
                    && fill.content_type == "image/png"
                    && fill.data == b"png-data"
        ));
        assert!(matches!(
            layout.clr_map_ovr.as_ref(),
            Some(ClrMapOverride::Override(_))
        ));
        assert_eq!(layout.shapes.len(), 1);
        let shape = &layout.shapes[0];
        assert!(matches!(
            shape
                .placeholder
                .as_ref()
                .and_then(|ph| ph.ph_type.as_ref()),
            Some(PlaceholderType::Body)
        ));
        let text_body = shape.text_body.as_ref().expect("layout text body");
        assert!(matches!(text_body.auto_fit, AutoFit::NoAutoFit));
        assert!(matches!(text_body.vertical_align, VerticalAlign::Middle));
        assert_eq!(shape.border.color.to_css().as_deref(), Some("#FFC000"));
        assert!(matches!(shape.border.join, LineJoin::Bevel));
    }

    #[test]
    fn parse_slide_layout_parses_gradient_background_master_mapping_and_autofit_variants() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs>
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
        <p:nvSpPr><p:cNvPr id="2" name="Normal Autofit"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></a:xfrm></p:spPr>
        <p:txBody>
          <a:bodyPr anchor="b" wrap="square"/>
          <a:normAutofit fontScale="80000" lnSpcReduction="25000"/>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="Shrink Autofit"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></a:xfrm><a:ln w="12700"><a:noFill/></a:ln></p:spPr>
        <p:txBody>
          <a:bodyPr anchor="t" vert="horz"/>
          <a:spAutoFit/>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sldLayout>"#;

        let mut archive = archive_with_entries(&[]);
        let layout = parse_slide_layout(xml, &HashMap::new(), &mut archive)
            .expect("layout gradient should parse");

        assert!(layout.background.is_some());
        assert!(matches!(
            layout.clr_map_ovr.as_ref(),
            Some(ClrMapOverride::UseMaster)
        ));
        assert_eq!(layout.shapes.len(), 2);
        assert!(matches!(
            layout.shapes[0].text_body.as_ref().map(|b| &b.auto_fit),
            Some(AutoFit::Normal {
                font_scale: Some(scale),
                line_spacing_reduction: Some(reduction),
            }) if (*scale - 0.8).abs() < 1e-6 && (*reduction - 0.25).abs() < 1e-6
        ));
        assert!(matches!(
            layout.shapes[1].text_body.as_ref().map(|b| &b.auto_fit),
            Some(AutoFit::Shrink)
        ));
        assert!(layout.shapes[1].border.no_fill);
    }

    fn archive_with_entries(entries: &[(&str, &[u8])]) -> ZipArchive<Cursor<Vec<u8>>> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        for (path, data) in entries {
            zip.start_file(path, options).unwrap();
            zip.write_all(data).unwrap();
        }
        ZipArchive::new(Cursor::new(zip.finish().unwrap().into_inner())).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::{Cursor, Write};

    use zip::ZipArchive;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    #[test]
    fn layout_helper_builders_and_path_resolution_cover_remaining_branches() {
        let mut builder = LayoutShapeBuilder {
            text_word_wrap_explicit: true,
            text_word_wrap: false,
            text_vertical_align_explicit: true,
            text_vertical_align: VerticalAlign::Middle,
            text_anchor_center: true,
            text_rotation_deg: 90.0,
            text_margin_left_explicit: true,
            text_auto_fit: AutoFit::Shrink,
            vertical_text: Some("vert270".to_string()),
            vertical_text_explicit: true,
            ..Default::default()
        };
        store_shape_level_defaults(&mut builder, "lvl1pPr", ParagraphDefaults::default());
        let shape = builder.build();
        let text_body = shape.text_body.expect("shape text body");
        assert!(matches!(text_body.vertical_align, VerticalAlign::Middle));
        assert!(text_body.anchor_center);
        assert!(!text_body.word_wrap);
        assert!(matches!(text_body.auto_fit, AutoFit::Shrink));
        assert_eq!(shape.vertical_text.as_deref(), Some("vert270"));

        assert_eq!(
            resolve_layout_rel_path("ppt/slideLayouts", "../media/layout.png"),
            "ppt/media/layout.png"
        );
        assert_eq!(
            resolve_layout_rel_path("ppt/slideLayouts", "media/layout.png"),
            "ppt/slideLayouts/media/layout.png"
        );
        assert_eq!(layout_bg_mime("bg.png"), "image/png");
        assert_eq!(layout_bg_mime("bg.jpg"), "image/jpeg");
        assert_eq!(layout_bg_mime("bg.gif"), "image/gif");
        assert_eq!(layout_bg_mime("bg.bmp"), "image/bmp");
        assert_eq!(layout_bg_mime("bg.svg"), "image/svg+xml");
        assert_eq!(layout_bg_mime("bg.unknown"), "image/png");
    }

    #[test]
    fn parse_slide_layout_handles_gradient_and_image_backgrounds_and_shape_defaults() {
        let gradient_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             type="title" showMasterSp="0">
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
    </p:spTree>
  </p:cSld>
</p:sldLayout>"#;

        let mut gradient_archive = empty_archive();
        let gradient_layout =
            parse_slide_layout(gradient_xml, &HashMap::new(), &mut gradient_archive)
                .expect("gradient layout parses");
        assert_eq!(gradient_layout.layout_type.as_deref(), Some("title"));
        assert!(!gradient_layout.show_master_sp);
        assert!(matches!(
            &gradient_layout.background,
            Some(Fill::Gradient(fill)) if fill.stops.len() == 2
        ));

        let image_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             type="body">
  <p:clrMapOvr><p:overrideClrMapping bg1="lt1" tx1="dk1"/></p:clrMapOvr>
  <p:cSld>
    <p:bg>
      <p:bgPr><a:blipFill><a:blip r:embed="rIdBg"/></a:blipFill></p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Layout Placeholder"/>
          <p:cNvSpPr/>
          <p:nvPr><p:ph type="body" idx="2"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="12700" y="25400"/>
            <a:ext cx="381000" cy="254000"/>
          </a:xfrm>
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
              <a:defRPr sz="1800">
                <a:latin typeface="Calibri"/>
                <a:srgbClr val="336699"/>
              </a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p/>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sldLayout>"#;

        let mut image_archive = archive_with_media("ppt/media/layout.png", b"layout-image");
        let rels = HashMap::from([("rIdBg".to_string(), "../media/layout.png".to_string())]);
        let image_layout =
            parse_slide_layout(image_xml, &rels, &mut image_archive).expect("image layout parses");

        assert!(matches!(
            &image_layout.background,
            Some(Fill::Image(ImageFill { content_type, data, .. }))
                if content_type == "image/png" && data == b"layout-image"
        ));
        assert!(matches!(
            image_layout.clr_map_ovr,
            Some(ClrMapOverride::Override(_))
        ));
        let shape = &image_layout.shapes[0];
        assert_eq!(shape.position.x.to_pt(), 1.0);
        assert_eq!(shape.position.y.to_pt(), 2.0);
        assert_eq!(shape.size.width.to_pt(), 30.0);
        assert_eq!(shape.size.height.to_pt(), 20.0);
        assert_eq!(shape.placeholder.as_ref().and_then(|ph| ph.idx), Some(2));
        assert!(matches!(
            shape
                .placeholder
                .as_ref()
                .and_then(|ph| ph.ph_type.as_ref()),
            Some(PlaceholderType::Body)
        ));
        assert!(matches!(shape.border.cap, LineCap::Round));
        assert!(matches!(shape.border.compound, CompoundLine::Double));
        assert!(matches!(shape.border.alignment, LineAlignment::Inset));
        assert!(matches!(shape.border.join, LineJoin::Miter));
        assert_eq!(shape.border.miter_limit, Some(2.0));
        assert!(matches!(shape.border.dash_style, DashStyle::LongDashDot));
        assert_eq!(shape.border.color.to_css().as_deref(), Some("#A5A5A5"));
        let text_body = shape.text_body.as_ref().expect("text body");
        assert!(matches!(text_body.vertical_align, VerticalAlign::Middle));
        assert!(text_body.anchor_center);
        assert!(!text_body.word_wrap);
        assert_eq!(shape.vertical_text.as_deref(), Some("vert270"));

        let mut invalid_archive = empty_archive();
        assert!(
            parse_slide_layout(
                "<p:sldLayout xmlns:p=\"p\"><p:cSld><",
                &HashMap::new(),
                &mut invalid_archive,
            )
            .is_err()
        );
    }

    #[test]
    fn master_color_mapping_override_is_supported() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:clrMapOvr><p:masterClrMapping/></p:clrMapOvr>
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sldLayout>"#;
        let mut archive = empty_archive();
        let layout =
            parse_slide_layout(xml, &HashMap::new(), &mut archive).expect("layout should parse");
        assert!(matches!(
            layout.clr_map_ovr,
            Some(ClrMapOverride::UseMaster)
        ));
    }

    #[test]
    fn parse_slide_layout_preserves_preset_geometry_adjustments_and_transform() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Layout Chevron"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm rot="1800000" flipH="1" flipV="true">
          <a:off x="12700" y="25400"/>
          <a:ext cx="1828800" cy="914400"/>
        </a:xfrm>
        <a:prstGeom prst="chevron">
          <a:avLst>
            <a:gd name="adj" fmla="val 20000"/>
          </a:avLst>
        </a:prstGeom>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

        let mut archive = empty_archive();
        let layout =
            parse_slide_layout(xml, &HashMap::new(), &mut archive).expect("layout should parse");
        let shape = &layout.shapes[0];

        assert!(matches!(
            shape.shape_type,
            ShapeType::Custom(ref name) if name == "chevron"
        ));
        assert_eq!(
            shape
                .adjust_values
                .as_ref()
                .and_then(|values| values.get("adj"))
                .copied(),
            Some(20_000.0)
        );
        assert!((shape.rotation - 30.0).abs() < 1e-6);
        assert!(shape.flip_h);
        assert!(shape.flip_v);
    }

    fn empty_archive() -> ZipArchive<Cursor<Vec<u8>>> {
        archive_with_media("ppt/empty.txt", b"")
    }

    fn archive_with_media(path: &str, data: &[u8]) -> ZipArchive<Cursor<Vec<u8>>> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        zip.start_file(path, options).expect("start file");
        zip.write_all(data).expect("write file");
        let cursor = zip.finish().expect("finish zip");
        ZipArchive::new(cursor).expect("open archive")
    }
}
