use std::collections::HashMap;
use std::io::{Read, Seek};

use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

use super::pattern_fill_parser::PatternBuilder;
use super::slide_parser::{parse_autofit_ratio, parse_guide_formula_value, parse_line_end};
use super::xml_utils;
use crate::error::{PptxError, PptxResult};
use crate::model::presentation::ClrMap;
use crate::model::*;

/// Parse slideMaster XML into SlideMaster
pub fn parse_slide_master<R: Read + Seek>(
    xml: &str,
    rels: &HashMap<String, String>,
    archive: &mut ZipArchive<R>,
) -> PptxResult<SlideMaster> {
    let mut reader = Reader::from_str(xml);
    let mut master = SlideMaster::default();

    let mut depth: Vec<String> = Vec::new();
    let mut in_tx_styles = false;
    let mut tx_style_kind: Option<TxStyleKind> = None;
    let mut current_lvl: Option<usize> = None;
    let mut current_para_defaults: Option<ParagraphDefaults> = None;
    let mut current_run_defaults: Option<RunDefaults> = None;
    let mut in_def_rpr = false;
    let mut current_color: Option<Color> = None;

    // Spacing parsing state for txStyles
    let mut in_ln_spc = false;
    let mut in_spc_bef = false;
    let mut in_spc_aft = false;

    // Shape parsing state
    let mut in_sp_tree = false;
    let mut current_shape: Option<MasterShapeBuilder> = None;
    let mut in_nv_pr = false;
    let mut in_shape_tx_body = false;
    let mut in_shape_lst_style = false;
    let mut shape_current_lvl: Option<usize> = None;
    let mut shape_para_defaults: Option<ParagraphDefaults> = None;
    let mut shape_run_defaults: Option<RunDefaults> = None;
    let mut in_shape_def_rpr = false;
    let mut shape_current_color: Option<Color> = None;
    let mut in_shape_ln_spc = false;
    let mut in_shape_spc_bef = false;
    let mut in_shape_spc_aft = false;
    let mut in_shape_ln = false;
    let mut in_shape_av_lst = false;

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
                    "solidFill" if in_bg_pr && !in_bg_blip_fill => {}
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
                    "srgbClr" if in_bg_pr && !in_bg_blip_fill && !in_def_rpr => {
                        assign_background_color(
                            &mut bg_solid_color,
                            &mut bg_grad_stops,
                            in_bg_grad_fill,
                            bg_gs_pos,
                            false,
                            xml_utils::attr_str(e, "val"),
                        );
                    }
                    "schemeClr" if in_bg_pr && !in_bg_blip_fill && !in_def_rpr => {
                        assign_background_color(
                            &mut bg_solid_color,
                            &mut bg_grad_stops,
                            in_bg_grad_fill,
                            bg_gs_pos,
                            true,
                            xml_utils::attr_str(e, "val"),
                        );
                    }
                    "spTree" => in_sp_tree = true,
                    "txStyles" => in_tx_styles = true,
                    "titleStyle" if in_tx_styles => {
                        tx_style_kind = Some(TxStyleKind::Title);
                    }
                    "bodyStyle" if in_tx_styles => {
                        tx_style_kind = Some(TxStyleKind::Body);
                    }
                    "otherStyle" if in_tx_styles => {
                        tx_style_kind = Some(TxStyleKind::Other);
                    }
                    // Level paragraph properties (lvl1pPr .. lvl9pPr)
                    s if tx_style_kind.is_some() && is_lvl_ppr(s) => {
                        let lvl = parse_lvl_index(s);
                        current_lvl = Some(lvl);
                        let mut pd = ParagraphDefaults::default();
                        parse_lvl_ppr_attrs(e, &mut pd);
                        current_para_defaults = Some(pd);
                    }
                    "defRPr" if current_lvl.is_some() => {
                        in_def_rpr = true;
                        let mut rd = RunDefaults::default();
                        parse_def_rpr_attrs(e, &mut rd);
                        current_run_defaults = Some(rd);
                    }
                    // Spacing containers inside lvlNpPr
                    "lnSpc" if current_lvl.is_some() && !in_def_rpr => {
                        in_ln_spc = true;
                    }
                    "spcBef" if current_lvl.is_some() && !in_def_rpr => {
                        in_spc_bef = true;
                    }
                    "spcAft" if current_lvl.is_some() && !in_def_rpr => {
                        in_spc_aft = true;
                    }
                    // Color elements inside defRPr
                    "srgbClr" if in_def_rpr => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            current_color = Some(Color::rgb(val));
                        }
                    }
                    "schemeClr" if in_def_rpr => {
                        current_color = xml_utils::attr_str(e, "val").map(Color::theme);
                    }
                    // Shape in spTree
                    "sp" if in_sp_tree && !in_tx_styles => {
                        current_shape = Some(MasterShapeBuilder::default());
                    }
                    "txBody" if current_shape.is_some() && !in_tx_styles => {
                        in_shape_tx_body = true;
                    }
                    "bodyPr" if current_shape.is_some() && in_shape_tx_body => {
                        if let Some(sb) = current_shape.as_mut() {
                            apply_shape_body_pr(sb, e);
                        }
                    }
                    s @ ("normAutofit" | "noAutofit" | "spAutoFit")
                        if current_shape.is_some() && in_shape_tx_body =>
                    {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.text_auto_fit = parse_shape_auto_fit(s, e);
                        }
                    }
                    "lstStyle" if in_shape_tx_body => {
                        in_shape_lst_style = true;
                    }
                    s if in_shape_lst_style && is_lvl_ppr(s) => {
                        let lvl = parse_lvl_index(s);
                        shape_current_lvl = Some(lvl);
                        let mut pd = ParagraphDefaults::default();
                        parse_lvl_ppr_attrs(e, &mut pd);
                        shape_para_defaults = Some(pd);
                    }
                    "defRPr" if in_shape_lst_style && shape_current_lvl.is_some() => {
                        in_shape_def_rpr = true;
                        let mut rd = RunDefaults::default();
                        parse_def_rpr_attrs(e, &mut rd);
                        shape_run_defaults = Some(rd);
                    }
                    "lnSpc"
                        if in_shape_lst_style
                            && shape_current_lvl.is_some()
                            && !in_shape_def_rpr =>
                    {
                        in_shape_ln_spc = true;
                    }
                    "spcBef"
                        if in_shape_lst_style
                            && shape_current_lvl.is_some()
                            && !in_shape_def_rpr =>
                    {
                        in_shape_spc_bef = true;
                    }
                    "spcAft"
                        if in_shape_lst_style
                            && shape_current_lvl.is_some()
                            && !in_shape_def_rpr =>
                    {
                        in_shape_spc_aft = true;
                    }
                    "srgbClr" if in_shape_def_rpr => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            shape_current_color = Some(Color::rgb(val));
                        }
                    }
                    "schemeClr" if in_shape_def_rpr => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            shape_current_color = Some(Color::theme(val));
                        }
                    }
                    "nvPr" if current_shape.is_some() => {
                        in_nv_pr = true;
                    }
                    "ln" if current_shape.is_some() && depth.iter().any(|d| d == "spPr") => {
                        in_shape_ln = true;
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
                            apply_master_shape_transform(sb, e);
                        }
                    }
                    "prstGeom" if current_shape.is_some() => {
                        if let Some(sb) = current_shape.as_mut() {
                            set_master_preset_geometry(sb, e);
                        }
                    }
                    "avLst" if current_shape.is_some() => {
                        in_shape_av_lst = true;
                    }
                    "gd" if in_shape_av_lst => {
                        if let Some(sb) = current_shape.as_mut() {
                            store_master_adjust_value(sb, e);
                        }
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
                        master.background = Some(Fill::Pattern(pattern.finish()));
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
                    // Background gradient direction (Empty variant)
                    "lin" if in_bg_grad_fill => {
                        if let Some(ang) = xml_utils::attr_str(e, "ang") {
                            bg_grad_angle = ang.parse::<f64>().unwrap_or(0.0) / 60_000.0;
                        }
                        bg_grad_type = GradientType::Linear;
                    }
                    "bodyPr" if current_shape.is_some() && in_shape_tx_body => {
                        if let Some(sb) = current_shape.as_mut() {
                            apply_shape_body_pr(sb, e);
                        }
                    }
                    s @ ("normAutofit" | "noAutofit" | "spAutoFit")
                        if current_shape.is_some() && in_shape_tx_body =>
                    {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.text_auto_fit = parse_shape_auto_fit(s, e);
                        }
                    }
                    // Background gradient path type (Empty variant)
                    "path" if in_bg_grad_fill => {
                        if let Some(val) = xml_utils::attr_str(e, "path") {
                            bg_grad_type = GradientType::from_path_attr(&val);
                        }
                    }
                    // Background solid/gradient color (Empty variant)
                    "srgbClr" if in_bg_pr && !in_bg_blip_fill && !in_def_rpr => {
                        assign_background_color(
                            &mut bg_solid_color,
                            &mut bg_grad_stops,
                            in_bg_grad_fill && depth.iter().any(|d| d == "gs"),
                            bg_gs_pos,
                            false,
                            xml_utils::attr_str(e, "val"),
                        );
                    }
                    "schemeClr" if in_bg_pr && !in_bg_blip_fill && !in_def_rpr => {
                        assign_background_color(
                            &mut bg_solid_color,
                            &mut bg_grad_stops,
                            in_bg_grad_fill && depth.iter().any(|d| d == "gs"),
                            bg_gs_pos,
                            true,
                            xml_utils::attr_str(e, "val"),
                        );
                    }
                    // ClrMap
                    "clrMap" => {
                        master.clr_map = parse_clr_map_element(e);
                    }
                    // Placeholder in shape
                    "ph" if in_nv_pr && current_shape.is_some() => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.placeholder = Some(parse_placeholder_attrs(e));
                        }
                    }
                    s @ ("latin" | "ea" | "cs") if in_shape_def_rpr => {
                        set_run_default_typeface(shape_run_defaults.as_mut(), s, e);
                    }
                    s @ ("srgbClr" | "schemeClr") if in_shape_def_rpr => {
                        set_run_default_color(shape_run_defaults.as_mut(), s, e);
                    }
                    s @ ("spcPct" | "spcPts")
                        if in_shape_lst_style
                            && shape_current_lvl.is_some()
                            && (in_shape_ln_spc || in_shape_spc_bef || in_shape_spc_aft) =>
                    {
                        if let Some(spacing) = parse_spacing_value(s, e) {
                            assign_spacing_target(
                                shape_para_defaults.as_mut(),
                                spacing,
                                in_shape_ln_spc,
                                in_shape_spc_bef,
                                in_shape_spc_aft,
                            );
                        }
                    }
                    // Font in defRPr
                    s @ ("latin" | "ea" | "cs") if in_def_rpr => {
                        set_run_default_typeface(current_run_defaults.as_mut(), s, e);
                    }
                    // Color elements (Empty variant) inside defRPr
                    s @ ("srgbClr" | "schemeClr") if in_def_rpr => {
                        set_run_default_color(current_run_defaults.as_mut(), s, e);
                    }
                    "srgbClr" if in_shape_ln => {
                        if let Some(val) = xml_utils::attr_str(e, "val")
                            && let Some(sb) = current_shape.as_mut()
                        {
                            sb.border.color = Color::rgb(val);
                        }
                    }
                    "schemeClr" if in_shape_ln => {
                        if let Some(val) = xml_utils::attr_str(e, "val")
                            && let Some(sb) = current_shape.as_mut()
                        {
                            sb.border.color = Color::theme(val);
                        }
                    }
                    // Spacing percentage/points (inside lnSpc/spcBef/spcAft)
                    s @ ("spcPct" | "spcPts")
                        if current_lvl.is_some() && (in_ln_spc || in_spc_bef || in_spc_aft) =>
                    {
                        if let Some(spacing) = parse_spacing_value(s, e) {
                            assign_spacing_target(
                                current_para_defaults.as_mut(),
                                spacing,
                                in_ln_spc,
                                in_spc_bef,
                                in_spc_aft,
                            );
                        }
                    }
                    // Empty lvlNpPr (no children)
                    s if tx_style_kind.is_some() && is_lvl_ppr(s) => {
                        let lvl = parse_lvl_index(s);
                        let mut pd = ParagraphDefaults::default();
                        parse_lvl_ppr_attrs(e, &mut pd);
                        store_level_defaults(&tx_style_kind, &mut master.tx_styles, lvl, pd);
                    }
                    // Empty defRPr (no children)
                    "defRPr" if current_lvl.is_some() && !in_def_rpr => {
                        let mut rd = RunDefaults::default();
                        parse_def_rpr_attrs(e, &mut rd);
                        if let Some(pd) = current_para_defaults.as_mut() {
                            pd.def_run_props = Some(rd);
                        }
                    }
                    // Position/size for shapes — only inside <a:xfrm>
                    "off" if current_shape.is_some() && depth.iter().any(|d| d == "xfrm") => {
                        if let Some(sb) = current_shape.as_mut() {
                            set_shape_offset(sb, e);
                        }
                    }
                    "ext" if current_shape.is_some() && depth.iter().any(|d| d == "xfrm") => {
                        if let Some(sb) = current_shape.as_mut() {
                            set_shape_extent(sb, e);
                        }
                    }
                    "xfrm" if current_shape.is_some() => {
                        if let Some(sb) = current_shape.as_mut() {
                            apply_master_shape_transform(sb, e);
                        }
                    }
                    "prstGeom" if current_shape.is_some() => {
                        if let Some(sb) = current_shape.as_mut() {
                            set_master_preset_geometry(sb, e);
                        }
                    }
                    "gd" if in_shape_av_lst => {
                        if let Some(sb) = current_shape.as_mut() {
                            store_master_adjust_value(sb, e);
                        }
                    }
                    "noFill" if in_shape_ln => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.border.style = BorderStyle::None;
                            sb.border.width = 0.0;
                            sb.border.no_fill = true;
                        }
                    }
                    "prstDash" if in_shape_ln => {
                        if let Some(sb) = current_shape.as_mut()
                            && let Some(val) = xml_utils::attr_str(e, "val")
                        {
                            apply_shape_dash(&mut sb.border, &val);
                        }
                    }
                    "round" if in_shape_ln => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.border.join = LineJoin::Round;
                        }
                    }
                    "bevel" if in_shape_ln => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.border.join = LineJoin::Bevel;
                        }
                    }
                    "miter" if in_shape_ln => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.border.join = LineJoin::Miter;
                            sb.border.miter_limit = xml_utils::attr_str(e, "lim")
                                .and_then(|v| v.parse::<f64>().ok())
                                .map(|v| v / 100_000.0);
                        }
                    }
                    "headEnd" if in_shape_ln => {
                        if let Some(sb) = current_shape.as_mut() {
                            sb.border.head_end = parse_line_end(e);
                        }
                    }
                    "tailEnd" if in_shape_ln => {
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
                        master.background = bg_pattern
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
                        finalize_background(
                            &mut master,
                            rels,
                            archive,
                            &mut bg_blip_rel_id,
                            &mut bg_solid_color,
                            &mut bg_grad_stops,
                            &mut bg_grad_type,
                            bg_grad_angle,
                        );
                    }
                    "txStyles" => {
                        in_tx_styles = false;
                        tx_style_kind = None;
                    }
                    "txBody" => in_shape_tx_body = false,
                    "lstStyle" => in_shape_lst_style = false,
                    "titleStyle" | "bodyStyle" | "otherStyle" => {
                        tx_style_kind = None;
                    }
                    "defRPr" if in_def_rpr => {
                        in_def_rpr = false;
                        finalize_run_defaults(
                            &mut current_color,
                            &mut current_run_defaults,
                            current_para_defaults.as_mut(),
                        );
                    }
                    "defRPr" if in_shape_def_rpr => {
                        in_shape_def_rpr = false;
                        finalize_run_defaults(
                            &mut shape_current_color,
                            &mut shape_run_defaults,
                            shape_para_defaults.as_mut(),
                        );
                    }
                    s if is_lvl_ppr(s) && current_lvl.is_some() => {
                        if let (Some(pd), Some(lvl)) = (current_para_defaults.take(), current_lvl) {
                            store_level_defaults(&tx_style_kind, &mut master.tx_styles, lvl, pd);
                        }
                        current_lvl = None;
                    }
                    s if in_shape_lst_style && is_lvl_ppr(s) => {
                        if let (Some(pd), Some(lvl)) =
                            (shape_para_defaults.take(), shape_current_lvl)
                        {
                            store_shape_level_defaults(&mut current_shape, lvl, pd);
                        }
                        shape_current_lvl = None;
                    }
                    "srgbClr" | "schemeClr" if in_def_rpr => {
                        if let Some(color) = current_color.take()
                            && let Some(rd) = current_run_defaults.as_mut()
                        {
                            rd.color = Some(color);
                        }
                    }
                    // End of spacing containers
                    "lnSpc" => {
                        in_ln_spc = false;
                        in_shape_ln_spc = false;
                    }
                    "spcBef" => {
                        in_spc_bef = false;
                        in_shape_spc_bef = false;
                    }
                    "spcAft" => {
                        in_spc_aft = false;
                        in_shape_spc_aft = false;
                    }
                    "spTree" => in_sp_tree = false,
                    "nvPr" => in_nv_pr = false,
                    "ln" if in_shape_ln => {
                        in_shape_ln = false;
                        if let Some(sb) = current_shape.as_mut()
                            && !sb.border.no_fill
                            && sb.border.width > 0.0
                            && matches!(sb.border.style, BorderStyle::None)
                        {
                            sb.border.style = BorderStyle::Solid;
                        }
                    }
                    "avLst" if in_shape_av_lst => {
                        in_shape_av_lst = false;
                    }
                    "sp" if current_shape.is_some() => {
                        if let Some(shape) = current_shape.take() {
                            master.shapes.push(shape.build());
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

    Ok(master)
}

// Helper types and functions

enum TxStyleKind {
    Title,
    Body,
    Other,
}

pub fn is_lvl_ppr(s: &str) -> bool {
    matches!(
        s,
        "lvl1pPr"
            | "lvl2pPr"
            | "lvl3pPr"
            | "lvl4pPr"
            | "lvl5pPr"
            | "lvl6pPr"
            | "lvl7pPr"
            | "lvl8pPr"
            | "lvl9pPr"
    )
}

pub fn parse_lvl_index(s: &str) -> usize {
    // "lvl1pPr" -> 0, "lvl2pPr" -> 1, ...
    s.chars()
        .find(|c| c.is_ascii_digit())
        .and_then(|c| c.to_digit(10))
        .map(|d| (d as usize).saturating_sub(1))
        .unwrap_or(0)
}

pub fn parse_lvl_ppr_attrs(e: &quick_xml::events::BytesStart<'_>, pd: &mut ParagraphDefaults) {
    if let Some(algn) = xml_utils::attr_str(e, "algn") {
        pd.alignment = Some(Alignment::from_ooxml(&algn));
    }
    if let Some(mar_l) = xml_utils::attr_str(e, "marL") {
        pd.margin_left = Some(Emu::parse_emu(&mar_l).to_pt());
    }
    if let Some(indent) = xml_utils::attr_str(e, "indent") {
        pd.indent = Some(Emu::parse_emu(&indent).to_pt());
    }
}

pub fn parse_def_rpr_attrs(e: &quick_xml::events::BytesStart<'_>, rd: &mut RunDefaults) {
    if let Some(sz) = xml_utils::attr_str(e, "sz") {
        rd.font_size = sz.parse::<f64>().ok().map(|v| v / 100.0);
    }
    if let Some(spc) = xml_utils::attr_str(e, "spc") {
        rd.letter_spacing = spc.parse::<f64>().ok().map(|v| v / 100.0);
    }
    if let Some(baseline) = xml_utils::attr_str(e, "baseline") {
        rd.baseline = baseline.parse::<i32>().ok();
    }
    if let Some(cap) = xml_utils::attr_str(e, "cap") {
        rd.capitalization = Some(TextCapitalization::from_ooxml(&cap));
    }
    if let Some(u) = xml_utils::attr_str(e, "u") {
        rd.underline = Some(UnderlineType::from_ooxml(&u));
    }
    if let Some(strike) = xml_utils::attr_str(e, "strike") {
        rd.strikethrough = Some(StrikethroughType::from_ooxml(&strike));
    }
    if let Some(b) = xml_utils::attr_str(e, "b") {
        rd.bold = Some(b == "1" || b == "true");
    }
    if let Some(i) = xml_utils::attr_str(e, "i") {
        rd.italic = Some(i == "1" || i == "true");
    }
}

/// Parse <p:clrMap .../> attributes into ClrMap
pub fn parse_clr_map_element(e: &quick_xml::events::BytesStart<'_>) -> ClrMap {
    let mut clr_map = ClrMap::default();
    for attr in e.attributes().flatten() {
        let key = xml_utils::local_name(attr.key.as_ref());
        let val = String::from_utf8_lossy(&attr.value);
        clr_map.set(key, &val);
    }
    clr_map
}

/// Parse <p:ph type="..." idx="..."/> placeholder attributes
pub fn parse_placeholder_attrs(e: &quick_xml::events::BytesStart<'_>) -> PlaceholderInfo {
    let mut info = PlaceholderInfo::default();
    if let Some(t) = xml_utils::attr_str(e, "type") {
        info.ph_type = PlaceholderType::from_ooxml(&t);
    }
    if let Some(idx) = xml_utils::attr_str(e, "idx") {
        info.idx = idx.parse::<u32>().ok();
    }
    info
}

fn apply_shape_body_pr(sb: &mut MasterShapeBuilder, e: &quick_xml::events::BytesStart<'_>) {
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

fn parse_shape_auto_fit(local: &str, e: &quick_xml::events::BytesStart<'_>) -> AutoFit {
    match local {
        "normAutofit" => AutoFit::Normal {
            font_scale: parse_autofit_ratio(e, "fontScale"),
            line_spacing_reduction: parse_autofit_ratio(e, "lnSpcReduction"),
        },
        "noAutofit" => AutoFit::NoAutoFit,
        "spAutoFit" => AutoFit::Shrink,
        _ => AutoFit::None,
    }
}

fn assign_background_color(
    solid_color: &mut Option<Color>,
    grad_stops: &mut Vec<GradientStop>,
    in_gradient_stop: bool,
    position: f64,
    themed: bool,
    value: Option<String>,
) {
    if let Some(value) = value {
        let color = if themed {
            Color::theme(value)
        } else {
            Color::rgb(value)
        };
        if in_gradient_stop {
            grad_stops.push(GradientStop { position, color });
        } else {
            *solid_color = Some(color);
        }
    }
}

fn set_run_default_typeface(
    run_defaults: Option<&mut RunDefaults>,
    local: &str,
    e: &quick_xml::events::BytesStart<'_>,
) {
    if let Some(run_defaults) = run_defaults
        && let Some(typeface) = xml_utils::attr_str(e, "typeface")
    {
        match local {
            "latin" => run_defaults.font_latin = Some(typeface),
            "ea" => run_defaults.font_ea = Some(typeface),
            "cs" => run_defaults.font_cs = Some(typeface),
            _ => {}
        }
    }
}

fn set_run_default_color(
    run_defaults: Option<&mut RunDefaults>,
    local: &str,
    e: &quick_xml::events::BytesStart<'_>,
) {
    if let Some(run_defaults) = run_defaults
        && let Some(value) = xml_utils::attr_str(e, "val")
    {
        run_defaults.color = Some(match local {
            "srgbClr" => Color::rgb(value),
            "schemeClr" => Color::theme(value),
            _ => return,
        });
    }
}

fn parse_spacing_value(local: &str, e: &quick_xml::events::BytesStart<'_>) -> Option<SpacingValue> {
    let value = xml_utils::attr_str(e, "val")?.parse::<f64>().ok()?;
    Some(match local {
        "spcPct" => SpacingValue::Percent(value / 100_000.0),
        "spcPts" => SpacingValue::Points(value / 100.0),
        _ => return None,
    })
}

fn assign_spacing_target(
    paragraph_defaults: Option<&mut ParagraphDefaults>,
    spacing: SpacingValue,
    in_ln_spc: bool,
    in_spc_bef: bool,
    in_spc_aft: bool,
) {
    if let Some(paragraph_defaults) = paragraph_defaults {
        if in_ln_spc {
            paragraph_defaults.line_spacing = Some(spacing);
        } else if in_spc_bef {
            paragraph_defaults.space_before = Some(spacing);
        } else if in_spc_aft {
            paragraph_defaults.space_after = Some(spacing);
        }
    }
}

fn set_shape_offset(sb: &mut MasterShapeBuilder, e: &quick_xml::events::BytesStart<'_>) {
    sb.position.x = Emu::parse_emu(&xml_utils::attr_str(e, "x").unwrap_or_default());
    sb.position.y = Emu::parse_emu(&xml_utils::attr_str(e, "y").unwrap_or_default());
}

fn set_shape_extent(sb: &mut MasterShapeBuilder, e: &quick_xml::events::BytesStart<'_>) {
    sb.size.width = Emu::parse_emu(&xml_utils::attr_str(e, "cx").unwrap_or_default());
    sb.size.height = Emu::parse_emu(&xml_utils::attr_str(e, "cy").unwrap_or_default());
}

fn apply_shape_dash(border: &mut Border, value: &str) {
    border.style = match value {
        "solid" => BorderStyle::Solid,
        "dash" | "lgDash" | "sysDash" => BorderStyle::Dashed,
        "dot" | "sysDot" | "lgDashDot" | "lgDashDotDot" | "sysDashDot" | "sysDashDotDot" => {
            BorderStyle::Dotted
        }
        _ => BorderStyle::Solid,
    };
    border.dash_style = match value {
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

fn finalize_run_defaults(
    current_color: &mut Option<Color>,
    current_run_defaults: &mut Option<RunDefaults>,
    paragraph_defaults: Option<&mut ParagraphDefaults>,
) {
    if let (Some(color), Some(run_defaults)) = (current_color.take(), current_run_defaults.as_mut())
        && run_defaults.color.is_none()
    {
        run_defaults.color = Some(color);
    }
    if let Some(paragraph_defaults) = paragraph_defaults {
        paragraph_defaults.def_run_props = current_run_defaults.take();
    }
}

#[allow(clippy::too_many_arguments)]
fn finalize_background<R: Read + Seek>(
    master: &mut SlideMaster,
    rels: &HashMap<String, String>,
    archive: &mut ZipArchive<R>,
    bg_blip_rel_id: &mut Option<String>,
    bg_solid_color: &mut Option<Color>,
    bg_grad_stops: &mut Vec<GradientStop>,
    bg_grad_type: &mut GradientType,
    bg_grad_angle: f64,
) {
    if let Some(rel_id) = bg_blip_rel_id.take() {
        if let Some(target) = rels.get(&rel_id) {
            let path = resolve_master_rel_path("ppt/slideMasters", target);
            if let Ok(mut entry) = archive.by_name(&path) {
                let mut buf = Vec::new();
                let _ = std::io::Read::read_to_end(&mut entry, &mut buf);
                if !buf.is_empty() {
                    let ct = bg_mime_from_ext(&path);
                    master.background = Some(Fill::Image(ImageFill {
                        rel_id,
                        data: buf,
                        content_type: ct,
                    }));
                }
            }
        }
    } else if let Some(color) = bg_solid_color.take() {
        master.background = Some(Fill::Solid(SolidFill { color }));
    } else if !bg_grad_stops.is_empty() {
        master.background = Some(Fill::Gradient(GradientFill {
            gradient_type: std::mem::take(bg_grad_type),
            stops: std::mem::take(bg_grad_stops),
            angle: bg_grad_angle,
        }));
    }
}

fn store_level_defaults(
    kind: &Option<TxStyleKind>,
    tx_styles: &mut TxStyles,
    lvl: usize,
    pd: ParagraphDefaults,
) {
    if lvl >= 9 {
        return;
    }
    let list_style = match kind {
        Some(TxStyleKind::Title) => tx_styles.title_style.get_or_insert_with(ListStyle::default),
        Some(TxStyleKind::Body) => tx_styles.body_style.get_or_insert_with(ListStyle::default),
        Some(TxStyleKind::Other) => tx_styles.other_style.get_or_insert_with(ListStyle::default),
        None => return,
    };
    list_style.levels[lvl] = Some(pd);
}

#[derive(Default)]
struct MasterShapeBuilder {
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

impl MasterShapeBuilder {
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

fn apply_master_shape_transform(
    shape: &mut MasterShapeBuilder,
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

fn set_master_preset_geometry(
    shape: &mut MasterShapeBuilder,
    e: &quick_xml::events::BytesStart<'_>,
) {
    if let Some(prst) = xml_utils::attr_str(e, "prst") {
        shape.preset_geometry = Some(prst);
    }
}

fn store_master_adjust_value(
    shape: &mut MasterShapeBuilder,
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
    shape: &mut Option<MasterShapeBuilder>,
    lvl: usize,
    pd: ParagraphDefaults,
) {
    if lvl >= 9 {
        return;
    }
    if let Some(shape) = shape.as_mut() {
        let list_style = shape.list_style.get_or_insert_with(ListStyle::default);
        list_style.levels[lvl] = Some(pd);
    }
}

fn resolve_master_rel_path(base_dir: &str, target: &str) -> String {
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

fn bg_mime_from_ext(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "emf" => "image/x-emf",
        "wmf" => "image/x-wmf",
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
    fn parse_slide_master_covers_background_styles_and_shape_builders() {
        let xml = concat!(
            r#"<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">"#,
            r#"<p:cSld><p:bg><p:bgPr><a:blipFill><a:blip r:embed="rIdBg"/></a:blipFill></p:bgPr></p:bg><p:spTree>"#,
            r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>"#,
            r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Shape 1"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="2"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm><a:ln w="12700" cap="rnd" cmpd="dbl" algn="in"><a:prstDash val="lgDashDot"/><a:schemeClr val="accent3"/><a:round/><a:headEnd type="triangle" w="sm" len="lg"/><a:tailEnd type="oval" w="lg" len="sm"/></a:ln></p:spPr><p:txBody><a:bodyPr anchor="ctr" anchorCtr="1" rot="5400000" vert="vert270" lIns="45720" tIns="91440" rIns="45720" bIns="91440" wrap="none"></a:bodyPr><a:lstStyle><a:lvl1pPr algn="ctr"><a:lnSpc><a:spcPct val="150000"/></a:lnSpc><a:spcBef><a:spcPts val="1200"/></a:spcBef><a:spcAft><a:spcPct val="30000"/></a:spcAft><a:defRPr sz="1800" b="1"><a:latin typeface="Calibri"/><a:ea typeface="Malgun Gothic"/><a:cs typeface="Mangal"/><a:srgbClr val="FF0000"/></a:defRPr></a:lvl1pPr></a:lstStyle><a:normAutofit fontScale="50000" lnSpcReduction="20000"/></p:txBody></p:sp>"#,
            r#"<p:sp><p:nvSpPr><p:cNvPr id="3" name="Shape 2"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="914400" y="0"/><a:ext cx="457200" cy="457200"/></a:xfrm><a:ln w="6350"><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr anchor="b" wrap="square"/><a:spAutoFit/><a:lstStyle><a:lvl2pPr><a:defRPr sz="1600"><a:schemeClr val="accent2"/></a:defRPr></a:lvl2pPr></a:lstStyle></p:txBody></p:sp>"#,
            r#"</p:spTree></p:cSld>"#,
            r#"<p:txStyles><p:titleStyle><a:lvl1pPr algn="ctr"><a:lnSpc><a:spcPct val="90000"/></a:lnSpc><a:spcBef><a:spcPts val="1200"/></a:spcBef><a:spcAft><a:spcPct val="50000"/></a:spcAft><a:defRPr sz="2400" b="1"><a:latin typeface="Aptos"/><a:ea typeface="Yu Gothic"/><a:cs typeface="Noto Sans Devanagari"/><a:schemeClr val="accent2"/></a:defRPr></a:lvl1pPr></p:titleStyle></p:txStyles>"#,
            r#"<p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/></p:sldMaster>"#
        );

        let rels = HashMap::from([("rIdBg".to_string(), "../media/background.png".to_string())]);
        let mut archive = archive_with_entries(&[("ppt/media/background.png", b"pngdata")]);
        let master = parse_slide_master(xml, &rels, &mut archive).expect("master parses");

        assert_eq!(
            std::mem::discriminant(&master.background),
            std::mem::discriminant(&Some(Fill::Image(ImageFill::default())))
        );
        assert_eq!(master.shapes.len(), 2);

        let first = &master.shapes[0];
        assert_eq!(first.placeholder.as_ref().and_then(|p| p.idx), Some(2));
        let first_body = first.text_body.as_ref().expect("text body");
        assert_eq!(
            std::mem::discriminant(&first_body.vertical_align),
            std::mem::discriminant(&VerticalAlign::Middle)
        );
        assert!(first_body.anchor_center);
        assert_eq!(first_body.text_rotation_deg, 90.0);
        assert_eq!(first.vertical_text.as_deref(), Some("vert270"));
        assert!(!first_body.word_wrap);
        assert!(matches!(
            first_body.auto_fit,
            AutoFit::Normal {
                font_scale: Some(0.5),
                line_spacing_reduction: Some(0.2)
            }
        ));
        assert_eq!(
            std::mem::discriminant(&first.border.dash_style),
            std::mem::discriminant(&DashStyle::LongDashDot)
        );
        assert_eq!(
            std::mem::discriminant(&first.border.join),
            std::mem::discriminant(&LineJoin::Round)
        );
        assert_eq!(
            first
                .border
                .head_end
                .as_ref()
                .map(|e| std::mem::discriminant(&e.end_type)),
            Some(std::mem::discriminant(&LineEndType::Triangle))
        );
        assert_eq!(
            first
                .border
                .tail_end
                .as_ref()
                .map(|e| std::mem::discriminant(&e.end_type)),
            Some(std::mem::discriminant(&LineEndType::Oval))
        );

        let second = &master.shapes[1];
        let second_body = second.text_body.as_ref().expect("second body");
        assert_eq!(
            std::mem::discriminant(&second_body.vertical_align),
            std::mem::discriminant(&VerticalAlign::Bottom)
        );
        assert_eq!(
            std::mem::discriminant(&second_body.auto_fit),
            std::mem::discriminant(&AutoFit::Shrink)
        );
        assert!(second_body.word_wrap);
        assert!(second.border.no_fill);
        assert_eq!(second.border.width, 0.0);

        let title_style = master.tx_styles.title_style.as_ref().expect("title style");
        let lvl1 = title_style.levels[0].as_ref().expect("lvl1");
        assert_eq!(
            lvl1.alignment
                .as_ref()
                .map(std::mem::discriminant::<Alignment>),
            Some(std::mem::discriminant(&Alignment::Center))
        );
        assert!(
            matches!(lvl1.line_spacing, Some(SpacingValue::Percent(v)) if (v - 0.9).abs() < 1e-6)
        );
        assert!(
            matches!(lvl1.space_before, Some(SpacingValue::Points(v)) if (v - 12.0).abs() < 1e-6)
        );
        assert!(
            matches!(lvl1.space_after, Some(SpacingValue::Percent(v)) if (v - 0.5).abs() < 1e-6)
        );
        let run = lvl1.def_run_props.as_ref().expect("run defaults");
        assert_eq!(run.font_latin.as_deref(), Some("Aptos"));
        assert_eq!(run.font_ea.as_deref(), Some("Yu Gothic"));
        assert_eq!(run.font_cs.as_deref(), Some("Noto Sans Devanagari"));
        assert_eq!(
            run.color.as_ref().and_then(|c| c.to_css()).as_deref(),
            Some("#ED7D31")
        );
        assert_eq!(master.clr_map.get("tx1").map(String::as_str), Some("dk1"));
    }

    #[test]
    fn master_helper_functions_cover_path_mime_and_shape_building() {
        assert_eq!(
            resolve_master_rel_path("ppt/slideMasters", "../media/image1.png"),
            "ppt/media/image1.png"
        );
        assert_eq!(
            resolve_master_rel_path("ppt/slideMasters", "media/image1.png"),
            "ppt/slideMasters/media/image1.png"
        );
        assert_eq!(bg_mime_from_ext("bg.png"), "image/png");
        assert_eq!(bg_mime_from_ext("bg.jpg"), "image/jpeg");
        assert_eq!(bg_mime_from_ext("bg.gif"), "image/gif");
        assert_eq!(bg_mime_from_ext("bg.bmp"), "image/bmp");
        assert_eq!(bg_mime_from_ext("bg.svg"), "image/svg+xml");
        assert_eq!(bg_mime_from_ext("bg.emf"), "image/x-emf");
        assert_eq!(bg_mime_from_ext("bg.wmf"), "image/x-wmf");
        assert_eq!(bg_mime_from_ext("bg.unknown"), "image/png");

        let mut tx_styles = TxStyles::default();
        store_level_defaults(
            &Some(TxStyleKind::Title),
            &mut tx_styles,
            9,
            ParagraphDefaults::default(),
        );
        store_level_defaults(&None, &mut tx_styles, 0, ParagraphDefaults::default());
        assert!(tx_styles.title_style.is_none());

        let mut shape = Some(MasterShapeBuilder::default());
        store_shape_level_defaults(&mut shape, 9, ParagraphDefaults::default());
        store_shape_level_defaults(
            &mut shape,
            0,
            ParagraphDefaults {
                alignment: Some(Alignment::Center),
                ..Default::default()
            },
        );
        let built = shape.expect("shape builder").build();
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
mod tests {
    use std::collections::HashMap;
    use std::io::{Cursor, Write};

    use zip::ZipArchive;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    #[test]
    fn parse_slide_master_parses_gradient_tx_styles_and_shape_body_variants() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
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
          <p:cNvPr id="2" name="Title Placeholder"/>
          <p:cNvSpPr/>
          <p:nvPr><p:ph type="title" idx="1"/></p:nvPr>
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
          <a:bodyPr anchor="ctr" anchorCtr="1" rot="5400000" vert="vert270" lIns="91440" tIns="45720" rIns="182880" bIns="22860" wrap="none"></a:bodyPr>
          <a:noAutofit/>
          <a:lstStyle>
            <a:lvl1pPr algn="r" marL="457200" indent="-228600">
              <a:lnSpc><a:spcPct val="90000"/></a:lnSpc>
              <a:spcBef><a:spcPts val="1200"/></a:spcBef>
              <a:spcAft><a:spcPct val="50000"/></a:spcAft>
              <a:defRPr sz="2400" spc="200" baseline="30000" cap="all" u="dbl" strike="sngStrike" b="1" i="1">
                <a:latin typeface="Aptos"/>
                <a:ea typeface="Yu Gothic"/>
                <a:cs typeface="Noto Sans Devanagari"/>
                <a:schemeClr val="accent2"/>
              </a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="Normal Autofit"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></a:xfrm></p:spPr>
        <p:txBody>
          <a:bodyPr anchor="b" lIns="12700" tIns="12700" rIns="12700" bIns="12700" wrap="square"/>
          <a:normAutofit fontScale="80000" lnSpcReduction="25000"/>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="4" name="Shrink Autofit"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></a:xfrm><a:ln w="12700"><a:noFill/></a:ln></p:spPr>
        <p:txBody>
          <a:bodyPr anchor="t" vert="horz"></a:bodyPr>
          <a:spAutoFit/>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr algn="ctr"><a:defRPr sz="2200"><a:schemeClr val="accent4"/></a:defRPr></a:lvl1pPr>
    </p:titleStyle>
    <p:bodyStyle>
      <a:lvl1pPr algn="l"><a:defRPr sz="2000"><a:srgbClr val="112233"/></a:defRPr></a:lvl1pPr>
    </p:bodyStyle>
    <p:otherStyle>
      <a:lvl1pPr algn="just"><a:defRPr sz="1800"><a:schemeClr val="accent5"/></a:defRPr></a:lvl1pPr>
    </p:otherStyle>
  </p:txStyles>
</p:sldMaster>"#;

        let mut archive = archive_with_entries(&[]);
        let master = parse_slide_master(xml, &HashMap::new(), &mut archive)
            .expect("master xml should parse");

        assert!(master.background.is_some());
        assert!(master.tx_styles.title_style.is_some());
        assert!(master.tx_styles.body_style.is_some());
        assert!(master.tx_styles.other_style.is_some());
        assert_eq!(master.shapes.len(), 3);

        let title_shape = &master.shapes[0];
        assert_eq!(
            title_shape
                .placeholder
                .as_ref()
                .and_then(|ph| ph.ph_type.as_ref())
                .map(std::mem::discriminant::<PlaceholderType>),
            Some(std::mem::discriminant(&PlaceholderType::Title))
        );
        let title_body = title_shape.text_body.as_ref().expect("title text body");
        assert_eq!(
            std::mem::discriminant(&title_body.vertical_align),
            std::mem::discriminant(&VerticalAlign::Middle)
        );
        assert!(title_body.anchor_center);
        assert_eq!(
            std::mem::discriminant(&title_body.auto_fit),
            std::mem::discriminant(&AutoFit::NoAutoFit)
        );
        assert_eq!(title_shape.vertical_text.as_deref(), Some("vert270"));
        assert_eq!(
            title_shape.border.color.to_css().as_deref(),
            Some("#A5A5A5")
        );

        let normal_autofit = master.shapes[1]
            .text_body
            .as_ref()
            .expect("normal autofit text body");
        assert!(matches!(
            normal_autofit.auto_fit,
            AutoFit::Normal {
                font_scale: Some(scale),
                line_spacing_reduction: Some(reduction),
            } if (scale - 0.8).abs() < 1e-6 && (reduction - 0.25).abs() < 1e-6
        ));

        let shrink_autofit = master.shapes[2]
            .text_body
            .as_ref()
            .expect("shrink autofit text body");
        assert_eq!(
            std::mem::discriminant(&shrink_autofit.auto_fit),
            std::mem::discriminant(&AutoFit::Shrink)
        );
        assert!(master.shapes[2].border.no_fill);
    }

    #[test]
    fn parse_slide_master_resolves_background_blip_fill_assets() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:blipFill><a:blip r:embed="rIdBg"></a:blip></a:blipFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sldMaster>"#;

        let rels = HashMap::from([("rIdBg".to_string(), "../media/background.svg".to_string())]);
        let mut archive = archive_with_entries(&[("ppt/media/background.svg", b"<svg/>")]);
        let master = parse_slide_master(xml, &rels, &mut archive).expect("master image background");

        assert!(matches!(
            &master.background,
            Some(Fill::Image(fill))
                if fill.rel_id == "rIdBg"
                    && fill.content_type == "image/svg+xml"
                    && fill.data == b"<svg/>"
        ));
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

    use quick_xml::events::BytesStart;
    use zip::ZipArchive;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    #[test]
    fn helper_parsers_cover_level_run_color_and_placeholder_defaults() {
        assert!(is_lvl_ppr("lvl1pPr"));
        assert!(!is_lvl_ppr("bodyPr"));
        assert_eq!(parse_lvl_index("lvl1pPr"), 0);
        assert_eq!(parse_lvl_index("lvl9pPr"), 8);
        assert_eq!(parse_lvl_index("unknown"), 0);

        let lvl = bytes_start(
            "a:lvl1pPr",
            &[("algn", "ctr"), ("marL", "457200"), ("indent", "-228600")],
        );
        let mut pd = ParagraphDefaults::default();
        parse_lvl_ppr_attrs(&lvl, &mut pd);
        assert_eq!(
            pd.alignment
                .as_ref()
                .map(std::mem::discriminant::<Alignment>),
            Some(std::mem::discriminant(&Alignment::Center))
        );
        assert_eq!(pd.margin_left, Some(36.0));
        assert_eq!(pd.indent, Some(-18.0));

        let def_rpr = bytes_start(
            "a:defRPr",
            &[
                ("sz", "2400"),
                ("spc", "200"),
                ("baseline", "30000"),
                ("cap", "all"),
                ("u", "dbl"),
                ("strike", "sngStrike"),
                ("b", "1"),
                ("i", "true"),
            ],
        );
        let mut rd = RunDefaults::default();
        parse_def_rpr_attrs(&def_rpr, &mut rd);
        assert_eq!(rd.font_size, Some(24.0));
        assert_eq!(rd.letter_spacing, Some(2.0));
        assert_eq!(rd.baseline, Some(30000));
        assert_eq!(rd.bold, Some(true));
        assert_eq!(rd.italic, Some(true));
        assert_eq!(
            rd.capitalization
                .as_ref()
                .map(std::mem::discriminant::<TextCapitalization>),
            Some(std::mem::discriminant(&TextCapitalization::All))
        );
        assert_eq!(
            rd.underline
                .as_ref()
                .map(std::mem::discriminant::<UnderlineType>),
            Some(std::mem::discriminant(&UnderlineType::Double))
        );
        assert_eq!(
            rd.strikethrough
                .as_ref()
                .map(std::mem::discriminant::<StrikethroughType>),
            Some(std::mem::discriminant(&StrikethroughType::Single))
        );

        let clr_map = parse_clr_map_element(&bytes_start(
            "p:clrMap",
            &[("bg1", "lt1"), ("tx1", "dk1"), ("accent1", "accent1")],
        ));
        assert_eq!(clr_map.get("bg1").map(String::as_str), Some("lt1"));
        assert_eq!(clr_map.get("tx1").map(String::as_str), Some("dk1"));

        let placeholder =
            parse_placeholder_attrs(&bytes_start("p:ph", &[("type", "body"), ("idx", "3")]));
        assert_eq!(placeholder.idx, Some(3));
        assert_eq!(
            placeholder
                .ph_type
                .as_ref()
                .map(std::mem::discriminant::<PlaceholderType>),
            Some(std::mem::discriminant(&PlaceholderType::Body))
        );

        let mut tx_styles = TxStyles::default();
        store_level_defaults(
            &Some(TxStyleKind::Title),
            &mut tx_styles,
            0,
            ParagraphDefaults::default(),
        );
        store_level_defaults(
            &Some(TxStyleKind::Body),
            &mut tx_styles,
            1,
            ParagraphDefaults::default(),
        );
        store_level_defaults(
            &Some(TxStyleKind::Other),
            &mut tx_styles,
            2,
            ParagraphDefaults::default(),
        );
        assert!(
            tx_styles
                .title_style
                .as_ref()
                .and_then(|s| s.levels[0].as_ref())
                .is_some()
        );
        assert!(
            tx_styles
                .body_style
                .as_ref()
                .and_then(|s| s.levels[1].as_ref())
                .is_some()
        );
        assert!(
            tx_styles
                .other_style
                .as_ref()
                .and_then(|s| s.levels[2].as_ref())
                .is_some()
        );

        let mut builder = Some(MasterShapeBuilder {
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
        });
        store_shape_level_defaults(&mut builder, 0, ParagraphDefaults::default());
        let shape = builder.expect("builder").build();
        let text_body = shape.text_body.expect("shape text body");
        assert_eq!(
            std::mem::discriminant(&text_body.vertical_align),
            std::mem::discriminant(&VerticalAlign::Middle)
        );
        assert!(text_body.anchor_center);
        assert!(!text_body.word_wrap);
        assert_eq!(
            std::mem::discriminant(&text_body.auto_fit),
            std::mem::discriminant(&AutoFit::Shrink)
        );
        assert_eq!(shape.vertical_text.as_deref(), Some("vert270"));

        assert_eq!(
            resolve_master_rel_path("ppt/slideMasters", "../media/image.png"),
            "ppt/media/image.png"
        );
        assert_eq!(
            resolve_master_rel_path("ppt/slideMasters", "media/image.png"),
            "ppt/slideMasters/media/image.png"
        );
        assert_eq!(bg_mime_from_ext("bg.png"), "image/png");
        assert_eq!(bg_mime_from_ext("bg.jpg"), "image/jpeg");
        assert_eq!(bg_mime_from_ext("bg.gif"), "image/gif");
        assert_eq!(bg_mime_from_ext("bg.bmp"), "image/bmp");
        assert_eq!(bg_mime_from_ext("bg.svg"), "image/svg+xml");
        assert_eq!(bg_mime_from_ext("bg.emf"), "image/x-emf");
        assert_eq!(bg_mime_from_ext("bg.wmf"), "image/x-wmf");
        assert_eq!(bg_mime_from_ext("bg.unknown"), "image/png");
    }

    #[test]
    fn parse_slide_master_handles_gradient_and_image_backgrounds() {
        let gradient_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
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
    </p:spTree>
  </p:cSld>
  <p:txStyles>
    <p:titleStyle><a:lvl1pPr algn="ctr"/></p:titleStyle>
  </p:txStyles>
</p:sldMaster>"#;

        let mut gradient_archive = empty_archive();
        let gradient_master =
            parse_slide_master(gradient_xml, &HashMap::new(), &mut gradient_archive)
                .expect("gradient master should parse");
        assert!(matches!(
            &gradient_master.background,
            Some(Fill::Gradient(fill)) if fill.stops.len() == 2
        ));

        let image_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr><a:blipFill><a:blip r:embed="rIdBg"/></a:blipFill></p:bgPr>
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
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2"
            accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6"
            hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

        let mut image_archive = archive_with_media("ppt/media/bg.png", b"png-data");
        let rels = HashMap::from([("rIdBg".to_string(), "../media/bg.png".to_string())]);
        let image_master =
            parse_slide_master(image_xml, &rels, &mut image_archive).expect("image master parses");

        assert!(matches!(
            &image_master.background,
            Some(Fill::Image(ImageFill { content_type, data, .. }))
                if content_type == "image/png" && data == b"png-data"
        ));
        assert_eq!(
            image_master.clr_map.get("bg1").map(String::as_str),
            Some("lt1")
        );
        let shape = &image_master.shapes[0];
        assert_eq!(shape.position.x.to_pt(), 1.0);
        assert_eq!(shape.position.y.to_pt(), 2.0);
        assert_eq!(shape.size.width.to_pt(), 30.0);
        assert_eq!(shape.size.height.to_pt(), 20.0);
        assert_eq!(shape.placeholder.as_ref().and_then(|ph| ph.idx), Some(1));
        assert_eq!(
            shape
                .placeholder
                .as_ref()
                .and_then(|ph| ph.ph_type.as_ref())
                .map(std::mem::discriminant::<PlaceholderType>),
            Some(std::mem::discriminant(&PlaceholderType::Body))
        );
        assert_eq!(
            std::mem::discriminant(&shape.border.cap),
            std::mem::discriminant(&LineCap::Round)
        );
        assert_eq!(
            std::mem::discriminant(&shape.border.compound),
            std::mem::discriminant(&CompoundLine::Double)
        );
        assert_eq!(
            std::mem::discriminant(&shape.border.alignment),
            std::mem::discriminant(&LineAlignment::Inset)
        );
        assert_eq!(
            std::mem::discriminant(&shape.border.join),
            std::mem::discriminant(&LineJoin::Miter)
        );
        assert_eq!(shape.border.miter_limit, Some(2.0));
        assert_eq!(
            std::mem::discriminant(&shape.border.dash_style),
            std::mem::discriminant(&DashStyle::LongDashDot)
        );
        assert_eq!(shape.border.color.to_css().as_deref(), Some("#A5A5A5"));
        assert_eq!(
            shape
                .border
                .head_end
                .as_ref()
                .map(|e| std::mem::discriminant(&e.end_type)),
            Some(std::mem::discriminant(&LineEndType::Triangle))
        );
        assert_eq!(
            shape
                .border
                .tail_end
                .as_ref()
                .map(|e| std::mem::discriminant(&e.end_type)),
            Some(std::mem::discriminant(&LineEndType::Stealth))
        );
        let text_body = shape.text_body.as_ref().expect("shape text body");
        assert_eq!(
            std::mem::discriminant(&text_body.vertical_align),
            std::mem::discriminant(&VerticalAlign::Middle)
        );
        assert!(text_body.anchor_center);
        assert!((text_body.text_rotation_deg - 90.0).abs() < 1e-6);
        assert!(!text_body.word_wrap);
        assert_eq!(shape.vertical_text.as_deref(), Some("vert270"));
    }

    #[test]
    fn parse_slide_master_covers_empty_txstyle_shape_spacing_and_line_variants() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Shape A"/>
          <p:cNvSpPr/>
          <p:nvPr><p:ph type="body" idx="4"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="12700" y="25400"/>
            <a:ext cx="381000" cy="254000"/>
          </a:xfrm>
          <a:ln w="12700">
            <a:noFill/>
            <a:prstDash val="sysDashDotDot"/>
            <a:round/>
            <a:headEnd type="arrow" w="sm" len="lg"/>
            <a:tailEnd type="diamond" w="lg" len="sm"/>
          </a:ln>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr" anchorCtr="1" rot="5400000" vert="horz" wrap="none"/>
          <a:lstStyle>
            <a:lvl1pPr><a:spcAft><a:spcPct val="30000"/></a:spcAft><a:defRPr sz="1800"><a:latin typeface="Shape Latin"/><a:ea typeface="Shape EA"/><a:cs typeface="Shape CS"/><a:srgbClr val="336699"/></a:defRPr></a:lvl1pPr>
            <a:lvl2pPr><a:spcAft><a:spcPts val="1800"/></a:spcAft><a:defRPr sz="1600"><a:schemeClr val="accent4"/></a:defRPr></a:lvl2pPr>
          </a:lstStyle>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="Shape B"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></a:xfrm><a:ln w="12700"><a:bevel/></a:ln></p:spPr>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="4" name="Shape C"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></a:xfrm><a:ln w="12700"><a:miter lim="300000"/></a:ln></p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr><a:defRPr sz="1500"/></a:lvl1pPr>
    </p:titleStyle>
    <p:bodyStyle>
      <a:lvl1pPr><a:spcAft><a:spcPct val="25000"/></a:spcAft><a:defRPr sz="2000"><a:latin typeface="Body Latin"/><a:ea typeface="Body EA"/><a:cs typeface="Body CS"/><a:srgbClr val="112233"/></a:defRPr></a:lvl1pPr>
    </p:bodyStyle>
    <p:otherStyle>
      <a:lvl2pPr><a:spcAft><a:spcPts val="2400"/></a:spcAft><a:defRPr sz="1600"><a:schemeClr val="accent2"/></a:defRPr></a:lvl2pPr>
      <a:lvl3pPr algn="l"/>
    </p:otherStyle>
  </p:txStyles>
</p:sldMaster>"#;

        let mut archive = empty_archive();
        let master = parse_slide_master(xml, &HashMap::new(), &mut archive)
            .expect("master should parse empty spacing and line variants");

        let title_lvl1 = master
            .tx_styles
            .title_style
            .as_ref()
            .and_then(|style| style.levels[0].as_ref())
            .expect("title level 1");
        assert_eq!(
            title_lvl1
                .def_run_props
                .as_ref()
                .and_then(|run| run.font_size),
            Some(15.0)
        );

        let body_lvl1 = master
            .tx_styles
            .body_style
            .as_ref()
            .and_then(|style| style.levels[0].as_ref())
            .expect("body level 1");
        assert!(
            matches!(body_lvl1.space_after, Some(SpacingValue::Percent(v)) if (v - 0.25).abs() < 1e-6)
        );
        let body_run = body_lvl1.def_run_props.as_ref().expect("body run defaults");
        assert_eq!(body_run.font_latin.as_deref(), Some("Body Latin"));
        assert_eq!(body_run.font_ea.as_deref(), Some("Body EA"));
        assert_eq!(body_run.font_cs.as_deref(), Some("Body CS"));
        assert_eq!(
            body_run.color.as_ref().and_then(Color::to_css).as_deref(),
            Some("#112233")
        );

        let other_lvl2 = master
            .tx_styles
            .other_style
            .as_ref()
            .and_then(|style| style.levels[1].as_ref())
            .expect("other level 2");
        assert!(
            matches!(other_lvl2.space_after, Some(SpacingValue::Points(v)) if (v - 24.0).abs() < 1e-6)
        );
        assert_eq!(
            other_lvl2
                .def_run_props
                .as_ref()
                .map(|run| run.color.as_ref().map(|color| color.kind.clone())),
            Some(Some(ColorKind::Theme("accent2".to_string())))
        );
        let other_lvl3 = master
            .tx_styles
            .other_style
            .as_ref()
            .and_then(|style| style.levels[2].as_ref())
            .expect("other level 3");
        assert_eq!(
            other_lvl3
                .alignment
                .as_ref()
                .map(std::mem::discriminant::<Alignment>),
            Some(std::mem::discriminant(&Alignment::Left))
        );

        let shape_a = &master.shapes[0];
        assert_eq!(shape_a.position.x.to_pt(), 1.0);
        assert_eq!(shape_a.position.y.to_pt(), 2.0);
        assert_eq!(shape_a.size.width.to_pt(), 30.0);
        assert_eq!(shape_a.size.height.to_pt(), 20.0);
        assert!(shape_a.border.no_fill);
        assert_eq!(shape_a.border.width, 0.0);
        assert_eq!(
            std::mem::discriminant(&shape_a.border.dash_style),
            std::mem::discriminant(&DashStyle::SystemDashDotDot)
        );
        assert_eq!(
            std::mem::discriminant(&shape_a.border.join),
            std::mem::discriminant(&LineJoin::Round)
        );
        assert_eq!(
            shape_a
                .border
                .head_end
                .as_ref()
                .map(|end| std::mem::discriminant(&end.end_type)),
            Some(std::mem::discriminant(&LineEndType::Arrow))
        );
        assert_eq!(
            shape_a
                .border
                .tail_end
                .as_ref()
                .map(|end| std::mem::discriminant(&end.end_type)),
            Some(std::mem::discriminant(&LineEndType::Diamond))
        );

        let shape_a_body = shape_a.text_body.as_ref().expect("shape a text body");
        assert_eq!(
            std::mem::discriminant(&shape_a_body.vertical_align),
            std::mem::discriminant(&VerticalAlign::Middle)
        );
        assert!(shape_a_body.anchor_center);
        assert!(!shape_a_body.word_wrap);
        assert_eq!(shape_a.vertical_text, None);
        let shape_a_lvl1 = shape_a_body
            .list_style
            .as_ref()
            .and_then(|style| style.levels[0].as_ref())
            .expect("shape a lvl1");
        assert!(
            matches!(shape_a_lvl1.space_after, Some(SpacingValue::Percent(v)) if (v - 0.3).abs() < 1e-6)
        );
        let shape_a_lvl1_run = shape_a_lvl1
            .def_run_props
            .as_ref()
            .expect("shape a lvl1 run");
        assert_eq!(shape_a_lvl1_run.font_latin.as_deref(), Some("Shape Latin"));
        assert_eq!(shape_a_lvl1_run.font_ea.as_deref(), Some("Shape EA"));
        assert_eq!(shape_a_lvl1_run.font_cs.as_deref(), Some("Shape CS"));
        assert_eq!(
            shape_a_lvl1_run
                .color
                .as_ref()
                .and_then(Color::to_css)
                .as_deref(),
            Some("#336699")
        );
        let shape_a_lvl2 = shape_a_body
            .list_style
            .as_ref()
            .and_then(|style| style.levels[1].as_ref())
            .expect("shape a lvl2");
        assert!(
            matches!(shape_a_lvl2.space_after, Some(SpacingValue::Points(v)) if (v - 18.0).abs() < 1e-6)
        );
        assert_eq!(
            shape_a_lvl2
                .def_run_props
                .as_ref()
                .map(|run| run.color.as_ref().map(|color| color.kind.clone())),
            Some(Some(ColorKind::Theme("accent4".to_string())))
        );

        assert_eq!(
            std::mem::discriminant(&master.shapes[1].border.join),
            std::mem::discriminant(&LineJoin::Bevel)
        );
        assert_eq!(
            std::mem::discriminant(&master.shapes[2].border.join),
            std::mem::discriminant(&LineJoin::Miter)
        );
        assert_eq!(master.shapes[2].border.miter_limit, Some(3.0));
    }

    #[test]
    fn parse_slide_master_preserves_preset_geometry_adjustments_and_transform() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Master Chevron"/>
        <p:cNvSpPr/>
        <p:nvPr/>
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
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

        let mut archive = empty_archive();
        let master =
            parse_slide_master(xml, &HashMap::new(), &mut archive).expect("master should parse");
        let shape = &master.shapes[0];

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

    fn bytes_start<'a>(name: &'a str, attrs: &[(&'a str, &'a str)]) -> BytesStart<'a> {
        let mut start = BytesStart::new(name);
        for (key, value) in attrs {
            start.push_attribute((*key, *value));
        }
        start
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
