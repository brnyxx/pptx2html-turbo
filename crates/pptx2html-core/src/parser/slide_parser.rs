use std::collections::HashMap;
use std::io::{Read, Seek};

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;
use zip::ZipArchive;

use super::action_parser;
#[cfg(test)]
use super::action_parser::hyperlink_rel_id;
use super::custom_geometry::CustomGeometryState;
use super::custom_guide;
#[cfg(test)]
use super::fill_parser::assign_background_color_target;
pub(crate) use super::fill_parser::parse_line_end;
use super::fill_parser::{ColorTargets, FillEndTargets, FillSaxState};
#[cfg(test)]
use super::fill_parser::{
    assign_color, assign_style_ref_color, assign_style_ref_no_color,
    dispatch_color as dispatch_parsed_color, ensure_style_ref,
};
use super::graphic_frame_parser::{
    GraphicFrameEnd, GraphicFrameSaxState, finish_frame, mime_from_extension,
    resolve_relationship_path as resolve_rel_path,
};
#[cfg(test)]
use super::graphic_frame_parser::{
    read_archive_bytes, read_archive_entry, relationships_path as rels_path_for,
    resolve_relative_file_path,
};
use super::preserved_parser::{PreservedSaxState, unsupported_data};
use super::table_parser::TableSaxState;
#[cfg(test)]
use super::table_parser::assign_cell_color as assign_tc_color;
#[cfg(test)]
use super::table_parser::{TableBuilder, TableCellBuilder, TableRowBuilder};
use super::text_parser::TextSaxState;
pub(crate) use super::text_parser::parse_autofit_ratio;
#[cfg(test)]
use super::text_parser::{
    ParagraphBuilder, RunBuilder,
    apply_paragraph_default_run_properties as apply_paragraph_def_rpr, assign_spacing_defaults,
    assign_spacing_paragraph, assign_typeface, parse_auto_fit as parse_shape_auto_fit,
    parse_body_properties as parse_body_pr, parse_paragraph_properties as parse_para_props,
    parse_run_properties as parse_run_props, parse_spacing as parse_spacing_tag,
};
#[cfg(test)]
use super::text_parser::{
    assign_typeface_to_defaults, assign_typeface_to_paragraph, assign_typeface_to_run,
    store_level_defaults as store_shape_level_defaults,
};
use super::xml_utils;
use crate::error::{PptxError, PptxResult};
use crate::model::*;

/// Parse slide XML
pub fn parse_slide<R: Read + Seek>(
    xml: &str,
    rels: &HashMap<String, String>,
    archive: &mut ZipArchive<R>,
) -> PptxResult<Slide> {
    let mut reader = NsReader::from_str(xml);
    let mut slide = Slide::default();
    let mut depth: Vec<String> = Vec::new();

    let mut current_shape: Option<ShapeBuilder> = None;
    let mut text = TextSaxState::default();

    // Fill/Border/Color parsing state
    let mut fill = FillSaxState::default();
    let mut in_sp_pr = false;
    let mut in_nv_pr = false;

    // Table parsing state
    let mut graphic_frame = GraphicFrameSaxState::default();
    let mut preserved = PreservedSaxState::default();
    let mut table = TableSaxState::default();

    // Group shape parsing state
    let mut grp_stack: Vec<GroupContext> = Vec::new();
    let mut in_grp_sp_pr = false;

    // Adjust value parsing state
    let mut in_av_lst = false;

    // Custom geometry parsing state
    let mut in_cust_geom = false;
    let mut in_cust_geom_path = false;
    let mut cust_geom_paths: Vec<GeometryPath> = Vec::new();
    let mut cust_geom_cmds: Vec<PathCommand> = Vec::new();
    let mut cust_geom_path_w: f64 = 0.0;
    let mut cust_geom_path_h: f64 = 0.0;
    let mut cust_geom_path_fill = PathFill::Norm;
    let mut cust_geom_pts: Vec<(f64, f64)> = Vec::new();
    let mut in_cust_geom_cmd: Option<String> = None;
    let mut cust_geom_state = CustomGeometryState::new();
    let mut cust_geom_text_rect: Option<GeomRect> = None;
    let mut cust_geom_handles: Vec<AdjustHandle> = Vec::new();
    let mut cust_geom_connection_sites: Vec<ConnectionSite> = Vec::new();
    let mut current_xy_handle: Option<XYAdjustHandle> = None;
    let mut current_polar_handle: Option<PolarAdjustHandle> = None;
    let mut current_connection_site: Option<ConnectionSite> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = xml_utils::local_name(e.name().as_ref()).to_string();
                let drawingml = matches!(
                    reader.resolve_element(e.name()).0,
                    ResolveResult::Bound(namespace)
                        if namespace.as_ref()
                            == b"http://schemas.openxmlformats.org/drawingml/2006/main"
                );
                depth.push(local.clone());

                if local == "cNvPr" && current_shape.is_some() {
                    parse_shape_identity(e, &mut current_shape);
                    continue;
                }
                if local == "stCxn" && current_shape.as_ref().is_some_and(|s| s.is_connector) {
                    parse_connector_ref(e, &mut current_shape, true);
                    continue;
                }
                if local == "endCxn" && current_shape.as_ref().is_some_and(|s| s.is_connector) {
                    parse_connector_ref(e, &mut current_shape, false);
                    continue;
                }

                preserved.capture_start(e, &local);
                if graphic_frame.handle_start(&local, e, &mut current_shape, &mut preserved) {
                    continue;
                }
                if (local != "tableStyleId" || drawingml)
                    && table.handle_start(&local, e, graphic_frame.in_frame())
                {
                    continue;
                }
                if text.handle_start(&local, e, &mut current_shape, table.in_cell) {
                    continue;
                }
                if action_parser::handle_start(
                    &local,
                    e,
                    rels,
                    text.in_run_properties,
                    table.in_run_properties,
                    &mut text.run,
                    &mut table.run,
                ) {
                    continue;
                }
                if fill.handle_start(&local, e, in_sp_pr, &mut current_shape, &text, &table) {
                    continue;
                }

                match local.as_str() {
                    // ── Group shape ──
                    "grpSp" => {
                        grp_stack.push(GroupContext {
                            shapes: Vec::new(),
                            position: Position::default(),
                            size: Size::default(),
                            child_offset: Position::default(),
                            child_extent: Size::default(),
                        });
                    }
                    // Group shape properties
                    "grpSpPr" if !grp_stack.is_empty() => {
                        in_grp_sp_pr = true;
                    }
                    // ── Regular shape handling ──
                    // Shape start
                    "sp" | "pic" | "cxnSp" => {
                        current_shape = Some(ShapeBuilder::default());
                        let sb = current_shape.as_mut().expect("shape builder initialized");
                        if local == "pic" {
                            sb.is_picture = true;
                        }
                        if local == "cxnSp" {
                            sb.is_connector = true;
                        }
                    }
                    // Non-visual properties (contains placeholder)
                    "nvPr" if current_shape.is_some() => {
                        in_nv_pr = true;
                    }
                    // Shape properties
                    "spPr" if current_shape.is_some() => {
                        in_sp_pr = true;
                    }
                    // Transform (rotation, flip)
                    "xfrm" if in_sp_pr => {
                        apply_shape_transform(
                            current_shape.as_mut().expect("shape builder in spPr"),
                            e,
                        );
                    }
                    // ── Adjust values (<a:avLst>) ──
                    "avLst" if in_sp_pr && current_shape.is_some() => {
                        in_av_lst = true;
                    }
                    "gdLst" if in_cust_geom => {
                        in_av_lst = true;
                    }
                    // Image reference (Start variant — blip with child elements)
                    "blip" => {
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            if key.ends_with("embed") {
                                let rel_id = String::from_utf8_lossy(&attr.value).to_string();
                                if let Some(sb) = current_shape.as_mut() {
                                    sb.image_rel_id = Some(rel_id);
                                }
                            }
                        }
                    }
                    // ── Preset geometry (<a:prstGeom>) — Start variant ──
                    // In real PPTX files, prstGeom is usually a Start event
                    // (e.g., <a:prstGeom prst="ellipse"><a:avLst/></a:prstGeom>)
                    "prstGeom" if current_shape.is_some() => {
                        if let Some(sb) = current_shape.as_mut()
                            && let Some(prst) = xml_utils::attr_str(e, "prst")
                        {
                            sb.preset_geometry = Some(prst);
                        }
                    }
                    // ── Custom geometry (<a:custGeom>) — Start variant ──
                    "custGeom" if in_sp_pr && current_shape.is_some() => {
                        in_cust_geom = true;
                        cust_geom_paths.clear();
                        cust_geom_state.reset(
                            current_shape
                                .as_ref()
                                .map(|shape| shape.size)
                                .unwrap_or_default(),
                        );
                        cust_geom_text_rect = None;
                        cust_geom_handles.clear();
                        cust_geom_connection_sites.clear();
                    }
                    // Path inside custGeom pathLst
                    "path" if in_cust_geom => {
                        in_cust_geom_path = true;
                        cust_geom_cmds.clear();
                        cust_geom_path_w = cust_geom_state
                            .path_extent("w", xml_utils::attr_str(e, "w").as_deref());
                        cust_geom_path_h = cust_geom_state
                            .path_extent("h", xml_utils::attr_str(e, "h").as_deref());
                        cust_geom_path_fill = match xml_utils::attr_str(e, "fill").as_deref() {
                            Some("none") => PathFill::None,
                            Some("lighten") => PathFill::Lighten,
                            Some("darken") => PathFill::Darken,
                            Some("lightenLess") => PathFill::LightenLess,
                            Some("darkenLess") => PathFill::DarkenLess,
                            _ => PathFill::Norm,
                        };
                    }
                    // Path drawing commands (Start variants with child <a:pt> elements)
                    "moveTo" | "lnTo" | "cubicBezTo" | "quadBezTo" if in_cust_geom_path => {
                        in_cust_geom_cmd = Some(local.clone());
                        cust_geom_pts.clear();
                    }
                    // arcTo as Start element (some generators emit it with children)
                    "arcTo" if in_cust_geom_path => {
                        let wr = xml_utils::attr_str(e, "wR")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:arcTo", "wR", v))
                            .unwrap_or(0.0);
                        let hr = xml_utils::attr_str(e, "hR")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:arcTo", "hR", v))
                            .unwrap_or(0.0);
                        let st_ang = xml_utils::attr_str(e, "stAng")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:arcTo", "stAng", v))
                            .unwrap_or(0.0);
                        let sw_ang = xml_utils::attr_str(e, "swAng")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:arcTo", "swAng", v))
                            .unwrap_or(0.0);
                        cust_geom_cmds.push(PathCommand::ArcTo {
                            wr,
                            hr,
                            start_angle: st_ang,
                            swing_angle: sw_ang,
                        });
                    }
                    "rect" if in_cust_geom => {
                        let left = xml_utils::attr_str(e, "l")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:rect", "l", v))
                            .unwrap_or(0.0);
                        let top = xml_utils::attr_str(e, "t")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:rect", "t", v))
                            .unwrap_or(0.0);
                        let right = xml_utils::attr_str(e, "r")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:rect", "r", v))
                            .unwrap_or(0.0);
                        let bottom = xml_utils::attr_str(e, "b")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:rect", "b", v))
                            .unwrap_or(0.0);
                        cust_geom_text_rect = Some(GeomRect {
                            left,
                            top,
                            right,
                            bottom,
                        });
                    }
                    "ahXY" if in_cust_geom => {
                        current_xy_handle = Some(XYAdjustHandle {
                            gd_ref_x: xml_utils::attr_str(e, "gdRefX"),
                            gd_ref_y: xml_utils::attr_str(e, "gdRefY"),
                            min_x: xml_utils::attr_str(e, "minX")
                                .as_deref()
                                .map(|v| cust_geom_state.resolve("a:ahXY", "minX", v)),
                            max_x: xml_utils::attr_str(e, "maxX")
                                .as_deref()
                                .map(|v| cust_geom_state.resolve("a:ahXY", "maxX", v)),
                            min_y: xml_utils::attr_str(e, "minY")
                                .as_deref()
                                .map(|v| cust_geom_state.resolve("a:ahXY", "minY", v)),
                            max_y: xml_utils::attr_str(e, "maxY")
                                .as_deref()
                                .map(|v| cust_geom_state.resolve("a:ahXY", "maxY", v)),
                            pos_x: 0.0,
                            pos_y: 0.0,
                        });
                    }
                    "ahPolar" if in_cust_geom => {
                        current_polar_handle = Some(PolarAdjustHandle {
                            gd_ref_r: xml_utils::attr_str(e, "gdRefR"),
                            gd_ref_ang: xml_utils::attr_str(e, "gdRefAng"),
                            min_r: xml_utils::attr_str(e, "minR")
                                .as_deref()
                                .map(|v| cust_geom_state.resolve("a:ahPolar", "minR", v)),
                            max_r: xml_utils::attr_str(e, "maxR")
                                .as_deref()
                                .map(|v| cust_geom_state.resolve("a:ahPolar", "maxR", v)),
                            min_ang: xml_utils::attr_str(e, "minAng")
                                .as_deref()
                                .map(|v| cust_geom_state.resolve("a:ahPolar", "minAng", v)),
                            max_ang: xml_utils::attr_str(e, "maxAng")
                                .as_deref()
                                .map(|v| cust_geom_state.resolve("a:ahPolar", "maxAng", v)),
                            pos_x: 0.0,
                            pos_y: 0.0,
                        });
                    }
                    "cxn" if in_cust_geom => {
                        current_connection_site = Some(ConnectionSite {
                            x: 0.0,
                            y: 0.0,
                            angle: xml_utils::attr_str(e, "ang")
                                .as_deref()
                                .map(|v| cust_geom_state.resolve("a:cxn", "ang", v))
                                .unwrap_or(0.0),
                        });
                    }
                    // close as Start element
                    "close" if in_cust_geom_path => {
                        cust_geom_cmds.push(PathCommand::Close);
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let local = xml_utils::local_name(e.name().as_ref()).to_string();
                let drawingml = matches!(
                    reader.resolve_element(e.name()).0,
                    ResolveResult::Bound(namespace)
                        if namespace.as_ref()
                            == b"http://schemas.openxmlformats.org/drawingml/2006/main"
                );

                if local == "cNvPr" && current_shape.is_some() {
                    parse_shape_identity(e, &mut current_shape);
                    continue;
                }
                if local == "stCxn" && current_shape.as_ref().is_some_and(|s| s.is_connector) {
                    parse_connector_ref(e, &mut current_shape, true);
                    continue;
                }
                if local == "endCxn" && current_shape.as_ref().is_some_and(|s| s.is_connector) {
                    parse_connector_ref(e, &mut current_shape, false);
                    continue;
                }

                preserved.capture_empty(e, &local);
                if graphic_frame.handle_start(&local, e, &mut current_shape, &mut preserved) {
                    continue;
                }
                if (local != "tableStyleId" || drawingml) && table.handle_empty(&local, e) {
                    continue;
                }
                if text.handle_empty(&local, e, &mut current_shape, table.in_cell) {
                    continue;
                }
                if action_parser::handle_start(
                    &local,
                    e,
                    rels,
                    text.in_run_properties,
                    table.in_run_properties,
                    &mut text.run,
                    &mut table.run,
                ) {
                    continue;
                }
                if fill.handle_empty(
                    &local,
                    e,
                    in_sp_pr,
                    &mut current_shape,
                    &mut table,
                    &mut slide,
                ) {
                    continue;
                }
                if let Some(color) = fill.parse_empty_color(&local, e) {
                    fill.route_color(
                        color,
                        ColorTargets {
                            depth: &depth,
                            in_shape_properties: in_sp_pr,
                            shape: &mut current_shape,
                            text: &mut text,
                            table: &mut table,
                        },
                    );
                    continue;
                }

                match local.as_str() {
                    // Shape position/size — only inside <a:xfrm> (or group grpSpPr).
                    // <a:off> and <a:ext> also appear inside <a:extLst> (extension
                    // lists) where they carry a `uri` attribute but no cx/cy.
                    // Parsing those would overwrite the shape size with zeros.
                    "off" if depth_contains(&depth, "xfrm") || in_grp_sp_pr => {
                        if in_grp_sp_pr {
                            // Inside grpSpPr: "off" under xfrm is group position,
                            // "chOff" is handled separately below
                            if let Some(gc) = grp_stack.last_mut() {
                                let x = Emu::parse_emu(
                                    &xml_utils::attr_str(e, "x").unwrap_or_default(),
                                );
                                let y = Emu::parse_emu(
                                    &xml_utils::attr_str(e, "y").unwrap_or_default(),
                                );
                                // Check if this is inside chOff or the outer xfrm off
                                if depth_contains(&depth, "chOff") {
                                    gc.child_offset = Position { x, y };
                                } else {
                                    gc.position = Position { x, y };
                                }
                            }
                        } else if let Some(sb) = current_shape.as_mut() {
                            sb.position.x =
                                Emu::parse_emu(&xml_utils::attr_str(e, "x").unwrap_or_default());
                            sb.position.y =
                                Emu::parse_emu(&xml_utils::attr_str(e, "y").unwrap_or_default());
                        }
                    }
                    "ext" if depth_contains(&depth, "xfrm") || in_grp_sp_pr => {
                        if in_grp_sp_pr {
                            if let Some(gc) = grp_stack.last_mut() {
                                let cx = Emu::parse_emu(
                                    &xml_utils::attr_str(e, "cx").unwrap_or_default(),
                                );
                                let cy = Emu::parse_emu(
                                    &xml_utils::attr_str(e, "cy").unwrap_or_default(),
                                );
                                if depth_contains(&depth, "chExt") {
                                    gc.child_extent = Size {
                                        width: cx,
                                        height: cy,
                                    };
                                } else {
                                    gc.size = Size {
                                        width: cx,
                                        height: cy,
                                    };
                                }
                            }
                        } else if let Some(sb) = current_shape.as_mut() {
                            sb.size.width =
                                Emu::parse_emu(&xml_utils::attr_str(e, "cx").unwrap_or_default());
                            sb.size.height =
                                Emu::parse_emu(&xml_utils::attr_str(e, "cy").unwrap_or_default());
                        }
                    }
                    // Child offset/extent for group (self-closing variant)
                    "chOff" if in_grp_sp_pr => {
                        if let Some(gc) = grp_stack.last_mut() {
                            gc.child_offset.x =
                                Emu::parse_emu(&xml_utils::attr_str(e, "x").unwrap_or_default());
                            gc.child_offset.y =
                                Emu::parse_emu(&xml_utils::attr_str(e, "y").unwrap_or_default());
                        }
                    }
                    "chExt" if in_grp_sp_pr => {
                        if let Some(gc) = grp_stack.last_mut() {
                            gc.child_extent.width =
                                Emu::parse_emu(&xml_utils::attr_str(e, "cx").unwrap_or_default());
                            gc.child_extent.height =
                                Emu::parse_emu(&xml_utils::attr_str(e, "cy").unwrap_or_default());
                        }
                    }
                    // Transform (Empty variant, e.g. connector with no children)
                    "xfrm" if in_sp_pr => {
                        apply_shape_transform(
                            current_shape.as_mut().expect("shape builder in empty xfrm"),
                            e,
                        );
                    }
                    // Preset geometry
                    "prstGeom" => {
                        if let Some(sb) = current_shape.as_mut()
                            && let Some(prst) = xml_utils::attr_str(e, "prst")
                        {
                            sb.preset_geometry = Some(prst);
                        }
                    }
                    // Placeholder
                    "ph" if in_nv_pr && current_shape.is_some() => {
                        current_shape
                            .as_mut()
                            .expect("shape builder for placeholder")
                            .placeholder = Some(super::master_parser::parse_placeholder_attrs(e));
                    }
                    // Image reference (Empty variant)
                    "blip" => {
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            if key.ends_with("embed") {
                                let rel_id = String::from_utf8_lossy(&attr.value).to_string();
                                if let Some(sb) = current_shape.as_mut() {
                                    sb.image_rel_id = Some(rel_id);
                                }
                            }
                        }
                    }
                    // ── Adjust value guide (<a:gd>) inside avLst ──
                    "gd" if in_av_lst => {
                        if let (Some(name), Some(fmla)) = (
                            xml_utils::attr_str(e, "name"),
                            xml_utils::attr_str(e, "fmla"),
                        ) {
                            if in_cust_geom {
                                cust_geom_state.add_guide(name, fmla);
                            } else if let Some(sb) = current_shape.as_mut()
                                && let Ok(value) = parse_guide_formula_value(&fmla, &HashMap::new())
                            {
                                sb.adjust_values.insert(name, value);
                            }
                        }
                    }
                    // ── Custom geometry: point element (<a:pt/>) ──
                    "pt" if in_cust_geom_cmd.is_some() => {
                        let x = xml_utils::attr_str(e, "x")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:pt", "x", v))
                            .unwrap_or(0.0);
                        let y = xml_utils::attr_str(e, "y")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:pt", "y", v))
                            .unwrap_or(0.0);
                        cust_geom_pts.push((x, y));
                    }
                    // ── Custom geometry: self-closing arcTo ──
                    "arcTo" if in_cust_geom_path => {
                        let wr = xml_utils::attr_str(e, "wR")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:arcTo", "wR", v))
                            .unwrap_or(0.0);
                        let hr = xml_utils::attr_str(e, "hR")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:arcTo", "hR", v))
                            .unwrap_or(0.0);
                        let st_ang = xml_utils::attr_str(e, "stAng")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:arcTo", "stAng", v))
                            .unwrap_or(0.0);
                        let sw_ang = xml_utils::attr_str(e, "swAng")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:arcTo", "swAng", v))
                            .unwrap_or(0.0);
                        cust_geom_cmds.push(PathCommand::ArcTo {
                            wr,
                            hr,
                            start_angle: st_ang,
                            swing_angle: sw_ang,
                        });
                    }
                    "rect" if in_cust_geom => {
                        let left = xml_utils::attr_str(e, "l")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:rect", "l", v))
                            .unwrap_or(0.0);
                        let top = xml_utils::attr_str(e, "t")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:rect", "t", v))
                            .unwrap_or(0.0);
                        let right = xml_utils::attr_str(e, "r")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:rect", "r", v))
                            .unwrap_or(0.0);
                        let bottom = xml_utils::attr_str(e, "b")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:rect", "b", v))
                            .unwrap_or(0.0);
                        cust_geom_text_rect = Some(GeomRect {
                            left,
                            top,
                            right,
                            bottom,
                        });
                    }
                    "pos" if in_cust_geom => {
                        let x = xml_utils::attr_str(e, "x")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:pos", "x", v))
                            .unwrap_or(0.0);
                        let y = xml_utils::attr_str(e, "y")
                            .as_deref()
                            .map(|v| cust_geom_state.resolve("a:pos", "y", v))
                            .unwrap_or(0.0);
                        if let Some(handle) = current_xy_handle.as_mut() {
                            handle.pos_x = x;
                            handle.pos_y = y;
                        } else if let Some(handle) = current_polar_handle.as_mut() {
                            handle.pos_x = x;
                            handle.pos_y = y;
                        } else if let Some(cxn) = current_connection_site.as_mut() {
                            cxn.x = x;
                            cxn.y = y;
                        }
                    }
                    // ── Custom geometry: self-closing close ──
                    "close" if in_cust_geom_path => {
                        cust_geom_cmds.push(PathCommand::Close);
                    }
                    // ── Custom geometry: self-closing path (no commands) ──
                    "path" if in_cust_geom => {
                        let w = cust_geom_state
                            .path_extent("w", xml_utils::attr_str(e, "w").as_deref());
                        let h = cust_geom_state
                            .path_extent("h", xml_utils::attr_str(e, "h").as_deref());
                        let fill = match xml_utils::attr_str(e, "fill").as_deref() {
                            Some("none") => PathFill::None,
                            Some("lighten") => PathFill::Lighten,
                            Some("darken") => PathFill::Darken,
                            Some("lightenLess") => PathFill::LightenLess,
                            Some("darkenLess") => PathFill::DarkenLess,
                            _ => PathFill::Norm,
                        };
                        cust_geom_paths.push(GeometryPath {
                            width: w,
                            height: h,
                            commands: Vec::new(),
                            fill,
                        });
                    }
                    // ── Image crop (<a:srcRect>) ──
                    "srcRect" if current_shape.is_some() => {
                        if let Some(sb) = current_shape.as_mut() {
                            let l = xml_utils::attr_str(e, "l")
                                .and_then(|v| v.parse::<f64>().ok())
                                .map(|v| v / 100_000.0)
                                .unwrap_or(0.0);
                            let t = xml_utils::attr_str(e, "t")
                                .and_then(|v| v.parse::<f64>().ok())
                                .map(|v| v / 100_000.0)
                                .unwrap_or(0.0);
                            let r = xml_utils::attr_str(e, "r")
                                .and_then(|v| v.parse::<f64>().ok())
                                .map(|v| v / 100_000.0)
                                .unwrap_or(0.0);
                            let b = xml_utils::attr_str(e, "b")
                                .and_then(|v| v.parse::<f64>().ok())
                                .map(|v| v / 100_000.0)
                                .unwrap_or(0.0);
                            if l > 0.0 || t > 0.0 || r > 0.0 || b > 0.0 {
                                sb.crop = Some(CropRect {
                                    left: l,
                                    top: t,
                                    right: r,
                                    bottom: b,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let value = e.unescape().unwrap_or_default();
                preserved.capture_text(&value);
                if !table.handle_text(&value) {
                    text.handle_text(&value);
                }
            }
            Ok(Event::CData(ref e)) => {
                let value = String::from_utf8_lossy(e.as_ref());
                preserved.capture_text(&value);
                if !table.handle_text(&value) {
                    text.handle_text(&value);
                }
            }
            Ok(Event::End(ref e)) => {
                let local = xml_utils::local_name(e.name().as_ref()).to_string();
                depth.pop();

                preserved.capture_end(e, &local);
                if table.handle_end(&local, &mut fill.current_color) {
                    continue;
                }
                if text.handle_end(&local, &mut current_shape, &mut fill.current_color) {
                    continue;
                }
                if fill.handle_end(
                    &local,
                    FillEndTargets {
                        shape: &mut current_shape,
                        text: &mut text,
                        table: &mut table,
                        slide: &mut slide,
                        relationships: rels,
                        archive,
                    },
                ) {
                    continue;
                }
                if let Some(color) = fill.take_completed_color(&local) {
                    fill.route_color(
                        color,
                        ColorTargets {
                            depth: &depth,
                            in_shape_properties: in_sp_pr,
                            shape: &mut current_shape,
                            text: &mut text,
                            table: &mut table,
                        },
                    );
                    continue;
                }
                if matches!(
                    graphic_frame.handle_end(&local, &mut current_shape, &mut preserved,),
                    GraphicFrameEnd::FinishFrame
                ) {
                    if let Some(shape) = finish_frame(
                        graphic_frame.take_chart_flag(),
                        &mut current_shape,
                        &mut table.builder,
                        rels,
                        archive,
                    ) {
                        if let Some(group) = grp_stack.last_mut() {
                            group.shapes.push(shape);
                        } else {
                            slide.shapes.push(shape);
                        }
                    }
                    continue;
                }

                match local.as_str() {
                    // ── Group shape end ──
                    "grpSp" => {
                        if let Some(gc) = grp_stack.pop() {
                            let group_data = GroupData {
                                child_offset: gc.child_offset,
                                child_extent: gc.child_extent,
                            };
                            let shape = Shape {
                                position: gc.position,
                                size: gc.size,
                                shape_type: ShapeType::Group(gc.shapes, group_data),
                                ..Default::default()
                            };
                            // Nested groups: push to parent group
                            if let Some(parent) = grp_stack.last_mut() {
                                parent.shapes.push(shape);
                            } else {
                                slide.shapes.push(shape);
                            }
                        }
                    }
                    "grpSpPr" => {
                        in_grp_sp_pr = false;
                    }

                    // ── Custom geometry end events ──
                    "moveTo" | "lnTo" if in_cust_geom_cmd.as_deref() == Some(&local) => {
                        if let Some((x, y)) = cust_geom_pts.first() {
                            let cmd = if local == "moveTo" {
                                PathCommand::MoveTo { x: *x, y: *y }
                            } else {
                                PathCommand::LineTo { x: *x, y: *y }
                            };
                            cust_geom_cmds.push(cmd);
                        }
                        in_cust_geom_cmd = None;
                        cust_geom_pts.clear();
                    }
                    "cubicBezTo" if in_cust_geom_cmd.as_deref() == Some("cubicBezTo") => {
                        if cust_geom_pts.len() >= 3 {
                            cust_geom_cmds.push(PathCommand::CubicBezTo {
                                x1: cust_geom_pts[0].0,
                                y1: cust_geom_pts[0].1,
                                x2: cust_geom_pts[1].0,
                                y2: cust_geom_pts[1].1,
                                x: cust_geom_pts[2].0,
                                y: cust_geom_pts[2].1,
                            });
                        }
                        in_cust_geom_cmd = None;
                        cust_geom_pts.clear();
                    }
                    "quadBezTo" if in_cust_geom_cmd.as_deref() == Some("quadBezTo") => {
                        if cust_geom_pts.len() >= 2 {
                            cust_geom_cmds.push(PathCommand::QuadBezTo {
                                x1: cust_geom_pts[0].0,
                                y1: cust_geom_pts[0].1,
                                x: cust_geom_pts[1].0,
                                y: cust_geom_pts[1].1,
                            });
                        }
                        in_cust_geom_cmd = None;
                        cust_geom_pts.clear();
                    }
                    "path" if in_cust_geom_path => {
                        in_cust_geom_path = false;
                        cust_geom_paths.push(GeometryPath {
                            width: cust_geom_path_w,
                            height: cust_geom_path_h,
                            commands: std::mem::take(&mut cust_geom_cmds),
                            fill: cust_geom_path_fill.clone(),
                        });
                    }
                    "ahXY" if current_xy_handle.is_some() => {
                        if let Some(handle) = current_xy_handle.take() {
                            cust_geom_handles.push(AdjustHandle::XY(handle));
                        }
                    }
                    "ahPolar" if current_polar_handle.is_some() => {
                        if let Some(handle) = current_polar_handle.take() {
                            cust_geom_handles.push(AdjustHandle::Polar(handle));
                        }
                    }
                    "cxn" if current_connection_site.is_some() => {
                        if let Some(cxn) = current_connection_site.take() {
                            cust_geom_connection_sites.push(cxn);
                        }
                    }
                    "custGeom" if in_cust_geom => {
                        in_cust_geom = false;
                        if let Some(sb) = current_shape.as_mut() {
                            let has_failures = cust_geom_state.has_failures();
                            let raw_formula = cust_geom_state.single_guide_error_formula();
                            sb.custom_geometry = Some(CustomGeometry {
                                paths: std::mem::take(&mut cust_geom_paths),
                                text_rect: cust_geom_text_rect.take(),
                                adjust_handles: std::mem::take(&mut cust_geom_handles),
                                connection_sites: std::mem::take(&mut cust_geom_connection_sites),
                                guides: cust_geom_state.take_guides(),
                                issues: cust_geom_state.take_issues(),
                            });
                            if has_failures {
                                sb.unsupported_content = Some("Custom Geometry".to_owned());
                                sb.unresolved_type = Some(slide::UnresolvedType::CustomGeometry);
                                sb.raw_xml_capture = raw_formula;
                            }
                        }
                        cust_geom_text_rect = None;
                    }

                    // ── New state end events ──
                    "avLst" | "gdLst" => in_av_lst = false,
                    // End of non-visual properties
                    "nvPr" => {
                        in_nv_pr = false;
                    }
                    // End of shape properties
                    "spPr" => {
                        in_sp_pr = false;
                    }
                    // End of shape
                    "sp" | "pic" | "cxnSp" => {
                        if let Some(mut sb) = current_shape.take() {
                            // For non-picture shapes with blipFill (image-filled rectangles etc.),
                            // load the image data and set Fill::Image before building
                            if !sb.is_picture
                                && let Some(ref rel_id) = sb.image_rel_id
                                && let Some(target) = rels.get(rel_id)
                            {
                                let path = resolve_rel_path("ppt/slides", target);
                                if let Ok(mut entry) = archive.by_name(&path) {
                                    let mut buf = Vec::new();
                                    let _ = entry.read_to_end(&mut buf);
                                    if !buf.is_empty() {
                                        let content_type = mime_from_extension(&path);
                                        sb.fill = Fill::Image(ImageFill {
                                            rel_id: rel_id.clone(),
                                            data: buf,
                                            content_type,
                                        });
                                    }
                                }
                            }
                            let mut shape = sb.build();
                            // Load image data for picture shapes
                            if let ShapeType::Picture(pic) = &mut shape.shape_type
                                && let Some(target) = rels.get(&pic.rel_id)
                            {
                                // Resolve relative paths (e.g., "../media/image1.png")
                                let path = resolve_rel_path("ppt/slides", target);
                                if let Ok(mut entry) = archive.by_name(&path) {
                                    let mut buf = Vec::new();
                                    let _ = entry.read_to_end(&mut buf);
                                    pic.data = buf;
                                    // Detect content type from extension
                                    if pic.content_type.is_empty() {
                                        pic.content_type = mime_from_extension(&path);
                                    }
                                }
                            }
                            // Add shape to group or slide
                            if !grp_stack.is_empty() {
                                if let Some(gc) = grp_stack.last_mut() {
                                    gc.shapes.push(shape);
                                }
                            } else {
                                slide.shapes.push(shape);
                            }
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

    Ok(slide)
}

fn depth_contains(depth: &[String], tag: &str) -> bool {
    depth.iter().any(|d| d == tag)
}

pub(crate) fn parse_guide_formula_value(
    fmla: &str,
    guides: &HashMap<String, f64>,
) -> Result<f64, GuideFormulaError> {
    custom_guide::evaluate(fmla, guides)
}

fn apply_shape_transform(sb: &mut ShapeBuilder, e: &quick_xml::events::BytesStart<'_>) {
    if let Some(rot) = xml_utils::attr_str(e, "rot") {
        sb.rotation = rot.parse::<f64>().unwrap_or(0.0) / 60000.0;
    }
    if let Some(fh) = xml_utils::attr_str(e, "flipH") {
        sb.flip_h = fh == "1" || fh == "true";
    }
    if let Some(fv) = xml_utils::attr_str(e, "flipV") {
        sb.flip_v = fv == "1" || fv == "true";
    }
}

fn parse_shape_identity(e: &quick_xml::events::BytesStart<'_>, shape: &mut Option<ShapeBuilder>) {
    if let Some(sb) = shape.as_mut() {
        sb.id = xml_utils::attr_str(e, "id")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        sb.name = xml_utils::attr_str(e, "name").unwrap_or_default();
    }
}

fn parse_connector_ref(
    e: &quick_xml::events::BytesStart<'_>,
    shape: &mut Option<ShapeBuilder>,
    is_start: bool,
) {
    if let Some(sb) = shape.as_mut() {
        let connection = ConnectionRef {
            shape_id: xml_utils::attr_str(e, "id")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0),
            site_idx: xml_utils::attr_str(e, "idx")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0),
        };
        if is_start {
            sb.start_connection = Some(connection);
        } else {
            sb.end_connection = Some(connection);
        }
    }
}

// ── Builder pattern ──

#[derive(Default)]
pub(crate) struct ShapeBuilder {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) position: Position,
    pub(crate) size: Size,
    rotation: f64,
    flip_h: bool,
    flip_v: bool,
    pub(crate) paragraphs: Vec<TextParagraph>,
    pub(crate) has_text_body: bool,
    is_picture: bool,
    image_rel_id: Option<String>,
    preset_geometry: Option<String>,
    adjust_values: HashMap<String, f64>,
    // Fill/Border
    pub(crate) fill: Fill,
    pub(crate) border_width: f64,
    pub(crate) border_color: Color,
    pub(crate) border_style: BorderStyle,
    pub(crate) border_no_fill: bool,
    pub(crate) dash_style: DashStyle,
    pub(crate) border_cap: LineCap,
    pub(crate) border_compound: CompoundLine,
    pub(crate) border_alignment: LineAlignment,
    pub(crate) border_join: LineJoin,
    pub(crate) miter_limit: Option<f64>,
    pub(crate) head_end: Option<LineEnd>,
    pub(crate) tail_end: Option<LineEnd>,
    // bodyPr
    pub(crate) text_vertical_align: VerticalAlign,
    pub(crate) text_vertical_align_explicit: bool,
    pub(crate) text_anchor_center: bool,
    pub(crate) text_rotation_deg: f64,
    pub(crate) text_margins: TextMargins,
    pub(crate) text_margin_top_explicit: bool,
    pub(crate) text_margin_bottom_explicit: bool,
    pub(crate) text_margin_left_explicit: bool,
    pub(crate) text_margin_right_explicit: bool,
    pub(crate) text_word_wrap: bool,
    pub(crate) text_word_wrap_explicit: bool,
    pub(crate) text_auto_fit: AutoFit,
    pub(crate) text_list_style: Option<ListStyle>,
    pub(crate) vertical_text: Option<String>,
    pub(crate) vertical_text_explicit: bool,
    // Image cropping
    crop: Option<CropRect>,
    // Placeholder and style reference (parsed as None for now)
    placeholder: Option<PlaceholderInfo>,
    pub(crate) style_ref: Option<ShapeStyleRef>,
    // Chart detection
    pub(crate) is_chart: bool,
    pub(crate) chart_rel_id: Option<String>,
    pub(crate) chart_direct_spec: Option<ChartSpec>,
    pub(crate) chart_preview_image: Option<Vec<u8>>,
    pub(crate) chart_preview_mime: Option<String>,
    // Unsupported content type (SmartArt, OLE, Math)
    pub(crate) unsupported_content: Option<String>,
    // Typed classification for unresolved element
    pub(crate) unresolved_type: Option<slide::UnresolvedType>,
    // Raw XML captured from graphicData for unresolved content
    pub(crate) raw_xml_capture: Option<String>,
    // Shape-level effects
    pub(crate) shape_outer_shadow: Option<OuterShadow>,
    pub(crate) shape_glow: Option<GlowEffect>,
    // Custom geometry
    custom_geometry: Option<CustomGeometry>,
    // Connection shape (cxnSp) — defaults to line if no preset geometry
    is_connector: bool,
    start_connection: Option<ConnectionRef>,
    end_connection: Option<ConnectionRef>,
}

impl ShapeBuilder {
    pub(crate) fn build(self) -> Shape {
        let shape_type = if let Some(label) = self.unsupported_content {
            ShapeType::Unsupported(unsupported_data(
                label,
                self.unresolved_type,
                self.raw_xml_capture,
                self.custom_geometry,
            ))
        } else if self.is_chart {
            ShapeType::Chart(ChartData {
                rel_id: self.chart_rel_id.unwrap_or_default(),
                preview_image: self.chart_preview_image,
                preview_mime: self.chart_preview_mime,
                direct_spec: self.chart_direct_spec,
            })
        } else if self.is_picture {
            ShapeType::Picture(PictureData {
                rel_id: self.image_rel_id.unwrap_or_default(),
                crop: self.crop,
                ..Default::default()
            })
        } else if let Some(geom) = self.custom_geometry {
            ShapeType::CustomGeom(geom)
        } else if let Some(ref prst) = self.preset_geometry {
            match prst.as_str() {
                "rect" => ShapeType::Rectangle,
                "roundRect" => ShapeType::RoundedRectangle,
                "ellipse" => ShapeType::Ellipse,
                "triangle" => ShapeType::Triangle,
                other => ShapeType::Custom(other.to_string()),
            }
        } else if self.is_connector {
            // cxnSp without preset geometry defaults to a straight line
            ShapeType::Custom("line".to_string())
        } else {
            ShapeType::TextBox
        };

        let text_body = if self.has_text_body {
            let word_wrap = if self.text_word_wrap_explicit {
                self.text_word_wrap
            } else {
                true
            };
            Some(TextBody {
                paragraphs: self.paragraphs,
                list_style: self.text_list_style,
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
            })
        } else {
            None
        };

        let border = Border {
            width: self.border_width,
            color: self.border_color,
            style: if self.border_no_fill {
                // Explicit <a:noFill/> inside <a:ln>: keep None
                BorderStyle::None
            } else if self.border_width > 0.0 && matches!(self.border_style, BorderStyle::None) {
                BorderStyle::Solid
            } else {
                self.border_style
            },
            dash_style: self.dash_style,
            cap: self.border_cap,
            compound: self.border_compound,
            alignment: self.border_alignment,
            join: self.border_join,
            miter_limit: self.miter_limit,
            head_end: self.head_end,
            tail_end: self.tail_end,
            no_fill: self.border_no_fill,
        };

        let adjust_values = if self.adjust_values.is_empty() {
            None
        } else {
            Some(self.adjust_values)
        };

        let effects = ShapeEffects {
            outer_shadow: self.shape_outer_shadow,
            glow: self.shape_glow,
        };

        Shape {
            id: self.id,
            name: self.name,
            position: self.position,
            size: self.size,
            rotation: self.rotation,
            flip_h: self.flip_h,
            flip_v: self.flip_v,
            shape_type,
            text_body,
            fill: self.fill,
            border,
            placeholder: self.placeholder,
            style_ref: self.style_ref,
            adjust_values,
            start_connection: self.start_connection,
            end_connection: self.end_connection,
            vertical_text: self.vertical_text,
            vertical_text_explicit: self.vertical_text_explicit,
            effects,
            ..Default::default()
        }
    }
}

// ── Group shape context ──

struct GroupContext {
    shapes: Vec<Shape>,
    position: Position,
    size: Size,
    child_offset: Position,
    child_extent: Size,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::{Cursor, Write};

    use quick_xml::events::BytesStart;
    use zip::ZipArchive;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    #[test]
    fn assign_color_routes_to_runs_gradients_borders_and_fills() {
        let mut run = Some(RunBuilder::default());
        let mut shape = Some(ShapeBuilder::default());
        let mut grad_stops = Vec::new();

        assign_color(
            Color::rgb("112233"),
            &["sp".into(), "txBody".into(), "p".into(), "rPr".into()],
            false,
            false,
            true,
            false,
            0.0,
            &mut shape,
            &mut run,
            &mut grad_stops,
        );
        assert_eq!(
            run.as_ref().and_then(|rb| rb.color.to_css()).as_deref(),
            Some("#112233")
        );

        assign_color(
            Color::theme("accent1"),
            &["spPr".into(), "gradFill".into(), "gs".into()],
            false,
            false,
            false,
            true,
            0.25,
            &mut shape,
            &mut run,
            &mut grad_stops,
        );
        assert_eq!(grad_stops.len(), 1);
        assert!((grad_stops[0].position - 0.25).abs() < 1e-6);

        assign_color(
            Color::rgb("445566"),
            &["spPr".into(), "ln".into()],
            false,
            true,
            false,
            false,
            0.0,
            &mut shape,
            &mut run,
            &mut grad_stops,
        );
        let shape_ref = shape.as_ref().expect("shape builder");
        assert_eq!(shape_ref.border_color.to_css().as_deref(), Some("#445566"));
        assert_eq!(
            std::mem::discriminant(&shape_ref.border_style),
            std::mem::discriminant(&BorderStyle::Solid)
        );

        assign_color(
            Color::rgb("778899"),
            &["spPr".into(), "solidFill".into()],
            true,
            false,
            false,
            false,
            0.0,
            &mut shape,
            &mut run,
            &mut grad_stops,
        );
        assert!(matches!(
            shape.as_ref().expect("shape").fill,
            Fill::Solid(ref fill) if fill.color.to_css().as_deref() == Some("#778899")
        ));

        assign_color(
            Color::rgb("AABBCC"),
            &["spPr".into()],
            true,
            false,
            false,
            false,
            0.0,
            &mut shape,
            &mut run,
            &mut grad_stops,
        );
        assert!(matches!(
            shape.as_ref().expect("shape").fill,
            Fill::Solid(ref fill) if fill.color.to_css().as_deref() == Some("#AABBCC")
        ));
    }

    #[test]
    fn style_ref_and_line_end_helpers_cover_supported_variants() {
        let mut style_ref = Some(ShapeStyleRef::default());
        assign_style_ref_color("fillRef", "2", Color::rgb("112233"), &mut style_ref);
        assign_style_ref_color("lnRef", "3", Color::theme("accent2"), &mut style_ref);
        assign_style_ref_color("effectRef", "4", Color::rgb("445566"), &mut style_ref);
        assign_style_ref_color("fontRef", "minor", Color::theme("accent3"), &mut style_ref);
        ensure_style_ref("fillRef", "9", &mut style_ref);
        assign_style_ref_no_color("effectRef", "6", &mut style_ref);

        let style_ref = style_ref.expect("style ref");
        assert_eq!(style_ref.fill_ref.as_ref().map(|s| s.idx), Some(2));
        assert_eq!(style_ref.ln_ref.as_ref().map(|s| s.idx), Some(3));
        assert_eq!(style_ref.effect_ref.as_ref().map(|s| s.idx), Some(6));
        assert_eq!(
            style_ref.font_ref.as_ref().map(|s| s.idx.as_str()),
            Some("minor")
        );

        let arrow = parse_line_end(&bytes_start(
            "a:headEnd",
            &[("type", "arrow"), ("w", "sm"), ("len", "lg")],
        ))
        .expect("arrow line end");
        assert_eq!(
            std::mem::discriminant(&arrow.end_type),
            std::mem::discriminant(&LineEndType::Arrow)
        );
        assert_eq!(
            std::mem::discriminant(&arrow.width),
            std::mem::discriminant(&LineEndSize::Small)
        );
        assert_eq!(
            std::mem::discriminant(&arrow.length),
            std::mem::discriminant(&LineEndSize::Large)
        );
        assert!(parse_line_end(&bytes_start("a:tailEnd", &[("type", "none")])).is_none());
        assert!(parse_line_end(&bytes_start("a:tailEnd", &[("type", "weird")])).is_none());
    }

    #[test]
    fn style_ref_and_line_end_helpers_cover_empty_builders_and_remaining_variants() {
        let mut missing_builder: Option<ShapeStyleRef> = None;
        assign_style_ref_color("fillRef", "1", Color::rgb("ABCDEF"), &mut missing_builder);
        ensure_style_ref("lnRef", "2", &mut missing_builder);
        assign_style_ref_no_color("fontRef", "major", &mut missing_builder);
        assert!(missing_builder.is_none());

        let mut style_ref = Some(ShapeStyleRef::default());
        ensure_style_ref("lnRef", "7", &mut style_ref);
        ensure_style_ref("effectRef", "8", &mut style_ref);
        ensure_style_ref("fontRef", "major", &mut style_ref);
        assign_style_ref_no_color("fillRef", "9", &mut style_ref);
        assign_style_ref_no_color("lnRef", "10", &mut style_ref);
        assign_style_ref_no_color("fontRef", "minor", &mut style_ref);
        assign_style_ref_color("unknownRef", "11", Color::rgb("FFFFFF"), &mut style_ref);
        ensure_style_ref("unknownRef", "12", &mut style_ref);

        let style_ref = style_ref.expect("style ref");
        assert_eq!(style_ref.fill_ref.as_ref().map(|s| s.idx), Some(9));
        assert_eq!(style_ref.ln_ref.as_ref().map(|s| s.idx), Some(10));
        assert_eq!(style_ref.effect_ref.as_ref().map(|s| s.idx), Some(8));
        assert_eq!(
            style_ref.font_ref.as_ref().map(|s| s.idx.as_str()),
            Some("minor")
        );
        assert!(
            style_ref
                .fill_ref
                .as_ref()
                .and_then(|s| s.color.to_css())
                .is_none()
        );
        assert!(
            style_ref
                .effect_ref
                .as_ref()
                .and_then(|s| s.color.to_css())
                .is_none()
        );
        assert!(
            style_ref
                .font_ref
                .as_ref()
                .and_then(|s| s.color.to_css())
                .is_none()
        );

        let triangle = parse_line_end(&bytes_start("a:headEnd", &[("type", "triangle")]))
            .expect("triangle line end");
        assert_eq!(
            std::mem::discriminant(&triangle.end_type),
            std::mem::discriminant(&LineEndType::Triangle)
        );
        assert_eq!(
            std::mem::discriminant(&triangle.width),
            std::mem::discriminant(&LineEndSize::Medium)
        );
        assert_eq!(
            std::mem::discriminant(&triangle.length),
            std::mem::discriminant(&LineEndSize::Medium)
        );

        let stealth = parse_line_end(&bytes_start("a:tailEnd", &[("type", "stealth")]))
            .expect("stealth line end");
        assert_eq!(
            std::mem::discriminant(&stealth.end_type),
            std::mem::discriminant(&LineEndType::Stealth)
        );

        let diamond = parse_line_end(&bytes_start("a:tailEnd", &[("type", "diamond")]))
            .expect("diamond line end");
        assert_eq!(
            std::mem::discriminant(&diamond.end_type),
            std::mem::discriminant(&LineEndType::Diamond)
        );

        let oval = parse_line_end(&bytes_start(
            "a:tailEnd",
            &[("type", "oval"), ("w", "mystery"), ("len", "mystery")],
        ))
        .expect("oval line end");
        assert_eq!(
            std::mem::discriminant(&oval.end_type),
            std::mem::discriminant(&LineEndType::Oval)
        );
        assert_eq!(
            std::mem::discriminant(&oval.width),
            std::mem::discriminant(&LineEndSize::Medium)
        );
        assert_eq!(
            std::mem::discriminant(&oval.length),
            std::mem::discriminant(&LineEndSize::Medium)
        );
    }

    #[test]
    fn guide_formula_and_body_parsers_cover_helper_branches() {
        let guides = HashMap::from([
            ("x".to_string(), 3.0),
            ("y".to_string(), 4.0),
            ("z".to_string(), 12.0),
        ]);
        let evaluate_formula =
            |formula| parse_guide_formula_value(formula, &guides).expect("known formula");
        assert_eq!(evaluate_formula("val x"), 3.0);
        assert_eq!(evaluate_formula("+- 5 4 3"), 6.0);
        assert_eq!(evaluate_formula("*/ 6 4 3"), 8.0);
        assert_eq!(evaluate_formula("+/ 6 4 2"), 5.0);
        assert_eq!(evaluate_formula("pin 1 5 3"), 3.0);
        assert_eq!(evaluate_formula("min 7 3"), 3.0);
        assert_eq!(evaluate_formula("max 7 3"), 7.0);
        assert_eq!(evaluate_formula("?: 1 8 9"), 8.0);
        assert_eq!(evaluate_formula("?: 0 8 9"), 9.0);
        assert_eq!(evaluate_formula("abs -7"), 7.0);
        assert_eq!(evaluate_formula("sqrt 16"), 4.0);
        assert!((evaluate_formula("mod x y z") - 13.0).abs() < 1e-6);
        assert!((evaluate_formula("sin 10 5400000") - 10.0).abs() < 1e-6);
        assert!((evaluate_formula("cos 10 0") - 10.0).abs() < 1e-6);
        assert!((evaluate_formula("cat2 10 y z") - 10.0 * (12.0f64.atan2(4.0)).cos()).abs() < 1e-6);
        assert!((evaluate_formula("sat2 10 y z") - 10.0 * (12.0f64.atan2(4.0)).sin()).abs() < 1e-6);
        assert!(
            (evaluate_formula("at2 x y") - 4.0f64.atan2(3.0).to_degrees() * 60_000.0).abs() < 1e-6
        );
        assert!((evaluate_formula("tan 10 2700000") - 10.0).abs() < 1e-6);
        assert!(parse_guide_formula_value("unknown", &guides).is_err());
        assert_eq!(custom_guide::resolve("42", &guides), Ok(42.0));
        assert_eq!(custom_guide::resolve("x", &guides), Ok(3.0));
        assert_eq!(
            custom_guide::resolve("missing", &guides),
            Err(GuideFormulaError::UnresolvedToken("missing".to_owned()))
        );

        let mut shape = Some(ShapeBuilder::default());
        parse_body_pr(
            &bytes_start(
                "a:bodyPr",
                &[
                    ("anchor", "ctr"),
                    ("anchorCtr", "1"),
                    ("rot", "5400000"),
                    ("lIns", "91440"),
                    ("tIns", "45720"),
                    ("rIns", "182880"),
                    ("bIns", "22860"),
                    ("wrap", "none"),
                    ("vert", "vert270"),
                ],
            ),
            &mut shape,
        );
        let shape = shape.expect("shape builder");
        assert_eq!(
            std::mem::discriminant(&shape.text_vertical_align),
            std::mem::discriminant(&VerticalAlign::Middle)
        );
        assert!(shape.text_anchor_center);
        assert!((shape.text_rotation_deg - 90.0).abs() < 1e-6);
        assert!(!shape.text_word_wrap);
        assert_eq!(shape.vertical_text.as_deref(), Some("vert270"));
        assert_eq!(shape.text_margins.left, 7.2);
        assert_eq!(shape.text_margins.top, 3.6);
        assert_eq!(shape.text_margins.right, 14.4);
        assert_eq!(shape.text_margins.bottom, 1.8);

        let mut horizontal_shape = Some(ShapeBuilder::default());
        parse_body_pr(
            &bytes_start("a:bodyPr", &[("anchorCtr", "true"), ("vert", "horz")]),
            &mut horizontal_shape,
        );
        let horizontal_shape = horizontal_shape.expect("horizontal shape builder");
        assert!(horizontal_shape.text_anchor_center);
        assert_eq!(horizontal_shape.vertical_text, None);

        assert_eq!(
            parse_autofit_ratio(
                &bytes_start("a:normAutofit", &[("fontScale", "250000")]),
                "fontScale"
            ),
            Some(1.0)
        );
        assert_eq!(
            parse_autofit_ratio(
                &bytes_start("a:normAutofit", &[("fontScale", "-1000")]),
                "fontScale"
            ),
            Some(0.0)
        );
    }

    #[test]
    fn shape_helper_fallbacks_cover_invalid_attributes_and_defaults() {
        let guides = HashMap::from([("x".to_string(), 3.0)]);
        for formula in [
            "",
            "val",
            "+- 1 2",
            "*/ 6 4 0",
            "+/ 6 4 0",
            "pin 1 2",
            "min 1",
            "max 1",
            "?: 1 2",
            "abs",
            "mod 1 2",
            "sin 10",
            "cos 10",
            "cat2 10 y",
            "sat2 10 y",
            "at2 10",
            "tan 10",
        ] {
            assert!(
                parse_guide_formula_value(formula, &guides).is_err(),
                "{formula}"
            );
        }
        assert_eq!(
            parse_guide_formula_value("sqrt -9", &guides),
            Err(GuideFormulaError::DomainError {
                operator: "sqrt".to_owned(),
                operand: "-9".to_owned(),
            })
        );

        let auto_fit = parse_shape_auto_fit(
            "normAutofit",
            &bytes_start(
                "a:normAutofit",
                &[("fontScale", "70000"), ("lnSpcReduction", "15000")],
            ),
        );
        assert!(matches!(
            auto_fit,
            AutoFit::Normal {
                font_scale: Some(font_scale),
                line_spacing_reduction: Some(line_spacing_reduction),
            } if (font_scale - 0.7).abs() < 1e-6
                && (line_spacing_reduction - 0.15).abs() < 1e-6
        ));

        let mut paragraph_defaults = ParagraphBuilder::default();
        apply_paragraph_def_rpr(
            &mut paragraph_defaults,
            &bytes_start(
                "a:defRPr",
                &[
                    ("sz", "oops"),
                    ("spc", "oops"),
                    ("baseline", "oops"),
                    ("cap", "small"),
                    ("u", "dashLong"),
                    ("strike", "dblStrike"),
                    ("b", "false"),
                    ("i", "0"),
                ],
            ),
        );
        assert_eq!(paragraph_defaults.def_rpr_font_size, None);
        assert_eq!(paragraph_defaults.def_rpr_letter_spacing, None);
        assert_eq!(paragraph_defaults.def_rpr_baseline, None);
        assert_eq!(
            paragraph_defaults
                .def_rpr_capitalization
                .as_ref()
                .map(std::mem::discriminant::<TextCapitalization>),
            Some(std::mem::discriminant(&TextCapitalization::Small))
        );
        assert_eq!(
            paragraph_defaults
                .def_rpr_underline
                .as_ref()
                .map(std::mem::discriminant::<UnderlineType>),
            Some(std::mem::discriminant(&UnderlineType::DashLong))
        );
        assert_eq!(
            paragraph_defaults
                .def_rpr_strikethrough
                .as_ref()
                .map(std::mem::discriminant::<StrikethroughType>),
            Some(std::mem::discriminant(&StrikethroughType::Double))
        );
        assert_eq!(paragraph_defaults.def_rpr_bold, Some(false));
        assert_eq!(paragraph_defaults.def_rpr_italic, Some(false));

        let mut para = Some(ParagraphBuilder::default());
        parse_para_props(
            &bytes_start(
                "a:pPr",
                &[
                    ("lvl", "oops"),
                    ("indent", "oops"),
                    ("marL", "oops"),
                    ("rtl", "true"),
                ],
            ),
            &mut para,
        );
        let para = para.expect("paragraph");
        assert_eq!(para.level, 0);
        assert_eq!(para.indent, Some(0.0));
        assert_eq!(para.margin_left, Some(0.0));
        assert!(para.rtl);

        let mut run = Some(RunBuilder::default());
        parse_run_props(
            &bytes_start(
                "a:rPr",
                &[
                    ("sz", "oops"),
                    ("b", "false"),
                    ("i", "0"),
                    ("u", "dashLong"),
                    ("strike", "dblStrike"),
                    ("cap", "small"),
                    ("baseline", "oops"),
                    ("spc", "oops"),
                ],
            ),
            &mut run,
        );
        let run = run.expect("run");
        assert_eq!(run.font_size, None);
        assert!(!run.bold);
        assert!(!run.italic);
        assert_eq!(
            std::mem::discriminant(&run.underline),
            std::mem::discriminant(&UnderlineType::DashLong)
        );
        assert_eq!(
            std::mem::discriminant(&run.strikethrough),
            std::mem::discriminant(&StrikethroughType::Double)
        );
        assert_eq!(
            std::mem::discriminant(&run.capitalization),
            std::mem::discriminant(&TextCapitalization::Small)
        );
        assert_eq!(run.baseline, None);
        assert_eq!(run.letter_spacing, None);

        let mut connector = Some(ShapeBuilder::default());
        parse_connector_ref(
            &bytes_start("a:stCxn", &[("id", "oops"), ("idx", "nope")]),
            &mut connector,
            true,
        );
        parse_connector_ref(&bytes_start("a:endCxn", &[]), &mut connector, false);
        let connector = connector.expect("connector");
        assert_eq!(
            connector.start_connection.as_ref().map(|c| c.shape_id),
            Some(0)
        );
        assert_eq!(
            connector.end_connection.as_ref().map(|c| c.site_idx),
            Some(0)
        );

        assert_eq!(
            hyperlink_rel_id(&bytes_start("a:hlinkClick", &[("id", "plain-id")])),
            None
        );

        let mut bg_grad_stops = Vec::new();
        let mut bg_solid_color = None;
        assign_background_color_target(
            Color::rgb("ABCDEF"),
            &["bgPr".into(), "solidFill".into()],
            true,
            0.4,
            &mut bg_grad_stops,
            &mut bg_solid_color,
        );
        assert!(bg_grad_stops.is_empty());
        assert_eq!(
            bg_solid_color
                .as_ref()
                .and_then(|color| color.to_css())
                .as_deref(),
            Some("#ABCDEF")
        );

        let mut missing_shape = None;
        store_shape_level_defaults(&mut missing_shape, 0, ParagraphDefaults::default());
        assert!(missing_shape.is_none());

        let mut level_ignored = Some(ShapeBuilder::default());
        store_shape_level_defaults(&mut level_ignored, 9, ParagraphDefaults::default());
        assert!(
            level_ignored
                .as_ref()
                .and_then(|shape| shape.text_list_style.as_ref())
                .is_none()
        );
    }

    #[test]
    fn helper_dispatch_spacing_and_typeface_paths_cover_remaining_branches() {
        let mut shape = ShapeBuilder::default();
        apply_shape_transform(
            &mut shape,
            &bytes_start(
                "a:xfrm",
                &[("rot", "1800000"), ("flipH", "true"), ("flipV", "1")],
            ),
        );
        assert!((shape.rotation - 30.0).abs() < 1e-6);
        assert!(shape.flip_h);
        assert!(shape.flip_v);
        assert_eq!(
            std::mem::discriminant(&parse_shape_auto_fit(
                "noAutofit",
                &bytes_start("a:noAutofit", &[])
            )),
            std::mem::discriminant(&AutoFit::NoAutoFit)
        );
        assert_eq!(
            std::mem::discriminant(&parse_shape_auto_fit(
                "spAutoFit",
                &bytes_start("a:spAutoFit", &[])
            )),
            std::mem::discriminant(&AutoFit::Shrink)
        );
        assert_eq!(
            std::mem::discriminant(&parse_shape_auto_fit(
                "mystery",
                &bytes_start("a:other", &[])
            )),
            std::mem::discriminant(&AutoFit::None)
        );
        let mut body_shape = Some(ShapeBuilder::default());
        parse_body_pr(
            &bytes_start("a:bodyPr", &[("anchorCtr", "0")]),
            &mut body_shape,
        );
        assert!(!body_shape.as_ref().expect("body shape").text_anchor_center);
        assert!(matches!(
            parse_spacing_tag("spcPct", &bytes_start("a:spcPct", &[("val", "125000")])),
            Some(SpacingValue::Percent(v)) if (v - 1.25).abs() < 1e-6
        ));
        assert!(matches!(
            parse_spacing_tag("spcPts", &bytes_start("a:spcPts", &[("val", "600")])),
            Some(SpacingValue::Points(v)) if (v - 6.0).abs() < 1e-6
        ));
        assert!(parse_spacing_tag("spcPct", &bytes_start("a:spcPct", &[])).is_none());
        assert!(parse_spacing_tag("other", &bytes_start("a:other", &[])).is_none());

        let mut defaults = ParagraphDefaults::default();
        assign_spacing_defaults(
            Some(&mut defaults),
            SpacingValue::Percent(1.1),
            true,
            false,
            false,
        );
        let mut paragraph = ParagraphBuilder::default();
        assign_spacing_paragraph(
            Some(&mut paragraph),
            SpacingValue::Points(4.0),
            false,
            true,
            false,
        );
        assign_spacing_defaults(
            Some(&mut defaults),
            SpacingValue::Points(5.0),
            false,
            false,
            true,
        );
        assign_spacing_paragraph(
            Some(&mut paragraph),
            SpacingValue::Percent(0.9),
            false,
            false,
            true,
        );
        assert!(
            matches!(defaults.line_spacing, Some(SpacingValue::Percent(v)) if (v - 1.1).abs() < 1e-6)
        );
        assert!(
            matches!(defaults.space_after, Some(SpacingValue::Points(v)) if (v - 5.0).abs() < 1e-6)
        );
        assert!(
            matches!(paragraph.space_before, Some(SpacingValue::Points(v)) if (v - 4.0).abs() < 1e-6)
        );
        assert!(
            matches!(paragraph.space_after, Some(SpacingValue::Percent(v)) if (v - 0.9).abs() < 1e-6)
        );
        assign_spacing_defaults(
            Some(&mut defaults),
            SpacingValue::Points(3.0),
            false,
            false,
            true,
        );
        assign_spacing_paragraph(
            Some(&mut paragraph),
            SpacingValue::Percent(0.8),
            false,
            false,
            true,
        );
        assert!(
            matches!(defaults.space_after, Some(SpacingValue::Points(v)) if (v - 3.0).abs() < 1e-6)
        );
        assert!(
            matches!(paragraph.space_after, Some(SpacingValue::Percent(v)) if (v - 0.8).abs() < 1e-6)
        );

        let mut cell_run = Some(RunBuilder::default());
        let mut current_run = Some(RunBuilder::default());
        let mut shape_defaults = Some(RunDefaults::default());
        let mut para = Some(ParagraphBuilder::default());
        let mut empty_cell_run = None;
        assign_typeface(
            "latin",
            &bytes_start("a:latin", &[("typeface", "Aptos")]),
            &mut cell_run,
            false,
            &mut shape_defaults,
            false,
            para.as_mut(),
            &mut current_run,
        );
        assign_typeface(
            "ea",
            &bytes_start("a:ea", &[("typeface", "Meiryo")]),
            &mut empty_cell_run,
            true,
            &mut shape_defaults,
            false,
            None,
            &mut current_run,
        );
        assign_typeface(
            "cs",
            &bytes_start("a:cs", &[("typeface", "Noto Sans Arabic")]),
            &mut empty_cell_run,
            false,
            &mut shape_defaults,
            true,
            para.as_mut(),
            &mut current_run,
        );
        assign_typeface(
            "latin",
            &bytes_start("a:latin", &[("typeface", "Calibri")]),
            &mut empty_cell_run,
            false,
            &mut shape_defaults,
            false,
            None,
            &mut current_run,
        );
        assign_typeface(
            "latin",
            &bytes_start("a:latin", &[]),
            &mut empty_cell_run,
            false,
            &mut shape_defaults,
            false,
            None,
            &mut current_run,
        );
        let mut ignored_run = RunBuilder::default();
        assign_typeface_to_run(&mut ignored_run, "other", "Ignored".to_string());
        let mut ignored_defaults = RunDefaults::default();
        assign_typeface_to_defaults(&mut ignored_defaults, "other", "Ignored".to_string());
        let mut ignored_paragraph = ParagraphBuilder::default();
        assign_typeface_to_paragraph(&mut ignored_paragraph, "other", "Ignored".to_string());
        assert_eq!(
            cell_run.as_ref().and_then(|run| run.font_latin.as_deref()),
            Some("Aptos")
        );
        assert_eq!(
            shape_defaults
                .as_ref()
                .and_then(|run| run.font_ea.as_deref()),
            Some("Meiryo")
        );
        assert_eq!(
            para.as_ref().and_then(|pb| pb.def_rpr_font_cs.as_deref()),
            Some("Noto Sans Arabic")
        );
        assign_typeface_to_run(
            current_run.as_mut().expect("current run"),
            "unknown",
            "Ignored".to_string(),
        );
        assign_typeface_to_defaults(
            shape_defaults.as_mut().expect("shape defaults"),
            "unknown",
            "Ignored".to_string(),
        );
        assign_typeface_to_paragraph(
            para.as_mut().expect("paragraph defaults"),
            "unknown",
            "Ignored".to_string(),
        );
        assign_typeface(
            "latin",
            &bytes_start("a:latin", &[]),
            &mut empty_cell_run,
            false,
            &mut shape_defaults,
            true,
            None,
            &mut current_run,
        );
        assign_typeface(
            "latin",
            &bytes_start("a:latin", &[("typeface", "NoParagraph")]),
            &mut empty_cell_run,
            false,
            &mut shape_defaults,
            true,
            None,
            &mut current_run,
        );

        let mut shape_effect_color = None;
        let mut current_color = None;
        let mut cell_paragraph = Some(ParagraphBuilder::default());
        let mut current_shape_run_defaults = Some(RunDefaults::default());
        let mut current_cell = Some(default_table_cell_builder());
        let mut current_paragraph = Some(ParagraphBuilder::default());
        let mut p_style_builder = Some(ShapeStyleRef::default());
        let mut bg_grad_stops = Vec::new();
        let mut bg_solid_color = None;
        let mut current_shape = Some(ShapeBuilder::default());
        let mut grad_stops = Vec::new();

        dispatch_parsed_color(
            Color::rgb("112233"),
            &["bgPr".into(), "gs".into()],
            false,
            false,
            false,
            false,
            0.5,
            true,
            &mut shape_effect_color,
            false,
            false,
            &mut cell_run,
            &mut current_run,
            false,
            &mut current_color,
            false,
            &mut cell_paragraph,
            false,
            &mut current_shape_run_defaults,
            false,
            &None,
            &mut current_cell,
            false,
            &mut current_paragraph,
            false,
            None,
            None,
            &mut p_style_builder,
            false,
            false,
            false,
            0.0,
            &mut bg_grad_stops,
            &mut bg_solid_color,
            &mut current_shape,
            &mut grad_stops,
        );
        dispatch_parsed_color(
            Color::theme("accent1"),
            &["bgPr".into(), "gs".into()],
            false,
            false,
            false,
            false,
            0.0,
            false,
            &mut shape_effect_color,
            false,
            false,
            &mut cell_run,
            &mut current_run,
            false,
            &mut current_color,
            true,
            &mut cell_paragraph,
            false,
            &mut current_shape_run_defaults,
            false,
            &None,
            &mut current_cell,
            false,
            &mut current_paragraph,
            false,
            None,
            None,
            &mut p_style_builder,
            false,
            false,
            false,
            0.0,
            &mut bg_grad_stops,
            &mut bg_solid_color,
            &mut current_shape,
            &mut grad_stops,
        );
        dispatch_parsed_color(
            Color::rgb("445566"),
            &["bgPr".into(), "gs".into()],
            false,
            false,
            false,
            false,
            0.0,
            false,
            &mut shape_effect_color,
            false,
            false,
            &mut cell_run,
            &mut current_run,
            false,
            &mut current_color,
            false,
            &mut cell_paragraph,
            true,
            &mut current_shape_run_defaults,
            false,
            &None,
            &mut current_cell,
            false,
            &mut current_paragraph,
            false,
            None,
            None,
            &mut p_style_builder,
            false,
            false,
            false,
            0.0,
            &mut bg_grad_stops,
            &mut bg_solid_color,
            &mut current_shape,
            &mut grad_stops,
        );
        dispatch_parsed_color(
            Color::rgb("778899"),
            &["bgPr".into(), "gs".into()],
            false,
            false,
            false,
            false,
            0.0,
            false,
            &mut shape_effect_color,
            false,
            false,
            &mut cell_run,
            &mut current_run,
            false,
            &mut current_color,
            false,
            &mut cell_paragraph,
            false,
            &mut current_shape_run_defaults,
            true,
            &None,
            &mut current_cell,
            false,
            &mut current_paragraph,
            false,
            None,
            None,
            &mut p_style_builder,
            false,
            false,
            false,
            0.0,
            &mut bg_grad_stops,
            &mut bg_solid_color,
            &mut current_shape,
            &mut grad_stops,
        );
        dispatch_parsed_color(
            Color::rgb("AABBCC"),
            &["bgPr".into(), "gs".into()],
            false,
            false,
            false,
            false,
            0.0,
            false,
            &mut shape_effect_color,
            false,
            false,
            &mut cell_run,
            &mut current_run,
            false,
            &mut current_color,
            false,
            &mut cell_paragraph,
            false,
            &mut current_shape_run_defaults,
            false,
            &None,
            &mut current_cell,
            false,
            &mut current_paragraph,
            false,
            None,
            None,
            &mut p_style_builder,
            true,
            false,
            true,
            0.5,
            &mut bg_grad_stops,
            &mut bg_solid_color,
            &mut current_shape,
            &mut grad_stops,
        );
        assert_eq!(
            shape_effect_color
                .as_ref()
                .and_then(|color| color.to_css())
                .as_deref(),
            Some("#112233")
        );
        assert_eq!(
            cell_paragraph
                .as_ref()
                .and_then(|pb| pb.bu_color.as_ref())
                .and_then(|color| color.to_css())
                .as_deref(),
            Some("#4472C4")
        );
        assert_eq!(
            current_shape_run_defaults
                .as_ref()
                .and_then(|rd| rd.color.as_ref())
                .and_then(|color| color.to_css())
                .as_deref(),
            Some("#445566")
        );
        assert!(
            matches!(current_cell.as_ref().map(|cell| &cell.fill), Some(Fill::Solid(fill)) if fill.color.to_css().as_deref() == Some("#778899"))
        );
        dispatch_parsed_color(
            Color::rgb("CCDDEE"),
            &["rPr".into()],
            false,
            false,
            true,
            false,
            0.0,
            false,
            &mut shape_effect_color,
            true,
            false,
            &mut empty_cell_run,
            &mut current_run,
            false,
            &mut current_color,
            false,
            &mut cell_paragraph,
            false,
            &mut current_shape_run_defaults,
            false,
            &None,
            &mut current_cell,
            false,
            &mut current_paragraph,
            false,
            None,
            None,
            &mut p_style_builder,
            false,
            false,
            false,
            0.0,
            &mut bg_grad_stops,
            &mut bg_solid_color,
            &mut current_shape,
            &mut grad_stops,
        );
        dispatch_parsed_color(
            Color::rgb("010203"),
            &["rPr".into()],
            false,
            false,
            false,
            false,
            0.0,
            false,
            &mut shape_effect_color,
            false,
            true,
            &mut cell_run,
            &mut current_run,
            false,
            &mut current_color,
            false,
            &mut cell_paragraph,
            false,
            &mut current_shape_run_defaults,
            false,
            &None,
            &mut current_cell,
            false,
            &mut current_paragraph,
            false,
            None,
            None,
            &mut p_style_builder,
            false,
            false,
            false,
            0.0,
            &mut bg_grad_stops,
            &mut bg_solid_color,
            &mut current_shape,
            &mut grad_stops,
        );
        dispatch_parsed_color(
            Color::rgb("FEDCBA"),
            &["pPr".into()],
            false,
            false,
            false,
            false,
            0.0,
            false,
            &mut shape_effect_color,
            false,
            false,
            &mut cell_run,
            &mut current_run,
            false,
            &mut current_color,
            false,
            &mut cell_paragraph,
            false,
            &mut current_shape_run_defaults,
            false,
            &None,
            &mut current_cell,
            true,
            &mut current_paragraph,
            false,
            None,
            None,
            &mut p_style_builder,
            false,
            false,
            false,
            0.0,
            &mut bg_grad_stops,
            &mut bg_solid_color,
            &mut current_shape,
            &mut grad_stops,
        );
        let mut para_false = Some(ParagraphBuilder::default());
        parse_para_props(&bytes_start("a:pPr", &[("rtl", "false")]), &mut para_false);
        let mut run_false = Some(RunBuilder::default());
        parse_run_props(&bytes_start("a:rPr", &[("b", "false")]), &mut run_false);
        assert_eq!(bg_grad_stops.len(), 1);
        assert_eq!(
            current_run
                .as_ref()
                .and_then(|run| run.highlight.as_ref())
                .and_then(|color| color.to_css())
                .as_deref(),
            Some("#CCDDEE")
        );
        assert_eq!(
            cell_run
                .as_ref()
                .and_then(|run| run.color.to_css())
                .as_deref(),
            Some("#010203")
        );
        assert_eq!(
            current_paragraph
                .as_ref()
                .and_then(|pb| pb.bu_color.as_ref())
                .and_then(|color| color.to_css())
                .as_deref(),
            Some("#FEDCBA")
        );
        assert!(!para_false.as_ref().expect("paragraph false case").rtl);
        assert!(!run_false.as_ref().expect("run false case").bold);
    }

    #[test]
    fn shape_builder_covers_variant_defaults_and_shape_specific_metadata() {
        let unsupported = ShapeBuilder {
            unsupported_content: Some("Math".to_string()),
            raw_xml_capture: Some("<m:oMath/>".to_string()),
            ..Default::default()
        }
        .build();
        assert!(matches!(
            unsupported.shape_type,
            ShapeType::Unsupported(ref data)
                if data.label == "Math"
                    && matches!(data.element_type, UnresolvedType::SmartArt)
                    && data.raw_xml.as_deref() == Some("<m:oMath/>")
        ));

        let round_rect = ShapeBuilder {
            preset_geometry: Some("roundRect".to_string()),
            ..Default::default()
        }
        .build();
        assert!(matches!(round_rect.shape_type, ShapeType::RoundedRectangle));

        let ellipse = ShapeBuilder {
            preset_geometry: Some("ellipse".to_string()),
            ..Default::default()
        }
        .build();
        assert!(matches!(ellipse.shape_type, ShapeType::Ellipse));

        let triangle = ShapeBuilder {
            preset_geometry: Some("rtTriangle".to_string()),
            ..Default::default()
        }
        .build();
        assert!(matches!(
            triangle.shape_type,
            ShapeType::Custom(ref name) if name == "rtTriangle"
        ));

        let custom = ShapeBuilder {
            preset_geometry: Some("hexagon".to_string()),
            ..Default::default()
        }
        .build();
        assert!(matches!(
            custom.shape_type,
            ShapeType::Custom(ref name) if name == "hexagon"
        ));

        let text_box = ShapeBuilder {
            name: "Text Box".to_string(),
            has_text_body: true,
            paragraphs: vec![TextParagraph {
                runs: vec![TextRun {
                    text: "shape text".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            border_no_fill: true,
            border_width: 2.0,
            adjust_values: HashMap::from([("adj".to_string(), 25_000.0)]),
            shape_outer_shadow: Some(OuterShadow {
                blur_radius: 1.0,
                distance: 2.0,
                direction: 30.0,
                color: Color::rgb("112233"),
                alpha: 0.5,
            }),
            shape_glow: Some(GlowEffect {
                radius: 1.5,
                color: Color::theme("accent1"),
                alpha: 0.75,
            }),
            vertical_text: Some("wordArtVert".to_string()),
            vertical_text_explicit: true,
            start_connection: Some(ConnectionRef {
                shape_id: 1,
                site_idx: 2,
            }),
            end_connection: Some(ConnectionRef {
                shape_id: 3,
                site_idx: 4,
            }),
            ..Default::default()
        }
        .build();
        assert!(matches!(text_box.shape_type, ShapeType::TextBox));
        let text_body = text_box.text_body.as_ref().expect("text body");
        assert!(text_body.word_wrap);
        assert!(!text_body.word_wrap_explicit);
        assert_eq!(text_body.paragraphs.len(), 1);
        assert_eq!(
            std::mem::discriminant(&text_box.border.style),
            std::mem::discriminant(&BorderStyle::None)
        );
        assert!(text_box.border.no_fill);
        assert_eq!(
            text_box
                .adjust_values
                .as_ref()
                .and_then(|values| values.get("adj"))
                .copied(),
            Some(25_000.0)
        );
        assert!(text_box.effects.outer_shadow.is_some());
        assert!(text_box.effects.glow.is_some());
        assert_eq!(text_box.vertical_text.as_deref(), Some("wordArtVert"));
        assert!(text_box.vertical_text_explicit);
        assert_eq!(
            text_box.start_connection.as_ref().map(|c| c.shape_id),
            Some(1)
        );
        assert_eq!(
            text_box.end_connection.as_ref().map(|c| c.site_idx),
            Some(4)
        );

        let solid_border = ShapeBuilder {
            border_width: 1.5,
            ..Default::default()
        }
        .build();
        assert_eq!(
            std::mem::discriminant(&solid_border.border.style),
            std::mem::discriminant(&BorderStyle::Solid)
        );

        let explicit_no_wrap = ShapeBuilder {
            has_text_body: true,
            text_word_wrap_explicit: true,
            text_word_wrap: false,
            ..Default::default()
        }
        .build();
        assert_eq!(
            explicit_no_wrap
                .text_body
                .as_ref()
                .map(|body| (body.word_wrap, body.word_wrap_explicit)),
            Some((false, true))
        );
    }

    #[test]
    fn parse_slide_reads_empty_adjust_guides_for_preset_shapes() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="7" name="Adjusted Arrow"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rightArrow">
            <a:avLst>
              <a:gd name="adj1" fmla="val 25000"/>
              <a:gd name="adj2" fmla="val 30000"/>
            </a:avLst>
          </a:prstGeom>
        </p:spPr>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let mut archive = archive_with_entries(&[]);
        let slide = parse_slide(slide_xml, &HashMap::new(), &mut archive).expect("slide parses");
        let shape = slide
            .shapes
            .iter()
            .find(|shape| shape.name == "Adjusted Arrow")
            .expect("adjusted preset shape");

        assert!(matches!(
            shape.shape_type,
            ShapeType::Custom(ref name) if name == "rightArrow"
        ));
        assert_eq!(
            shape
                .adjust_values
                .as_ref()
                .and_then(|values| values.get("adj1"))
                .copied(),
            Some(25_000.0)
        );
        assert_eq!(
            shape
                .adjust_values
                .as_ref()
                .and_then(|values| values.get("adj2"))
                .copied(),
            Some(30_000.0)
        );
    }

    #[test]
    fn shape_paragraph_run_archive_and_table_helpers_cover_remaining_paths() {
        let mut shape = Some(ShapeBuilder::default());
        parse_shape_identity(
            &bytes_start("p:cNvPr", &[("id", "7"), ("name", "Connector")]),
            &mut shape,
        );
        parse_connector_ref(
            &bytes_start("a:stCxn", &[("id", "11"), ("idx", "2")]),
            &mut shape,
            true,
        );
        parse_connector_ref(
            &bytes_start("a:endCxn", &[("id", "12"), ("idx", "3")]),
            &mut shape,
            false,
        );
        let shape = shape.expect("shape");
        assert_eq!(shape.id, 7);
        assert_eq!(shape.name, "Connector");
        assert_eq!(
            shape.start_connection.as_ref().map(|c| c.shape_id),
            Some(11)
        );
        assert_eq!(shape.end_connection.as_ref().map(|c| c.site_idx), Some(3));

        let mut para = Some(ParagraphBuilder::default());
        parse_para_props(
            &bytes_start(
                "a:pPr",
                &[
                    ("algn", "ctr"),
                    ("rtl", "1"),
                    ("lvl", "2"),
                    ("indent", "12700"),
                    ("marL", "25400"),
                ],
            ),
            &mut para,
        );
        let mut run = Some(RunBuilder::default());
        parse_run_props(
            &bytes_start(
                "a:rPr",
                &[
                    ("sz", "2400"),
                    ("b", "1"),
                    ("i", "true"),
                    ("u", "dbl"),
                    ("strike", "sngStrike"),
                    ("cap", "all"),
                    ("baseline", "30000"),
                    ("spc", "200"),
                ],
            ),
            &mut run,
        );
        assert_eq!(
            hyperlink_rel_id(&bytes_start("a:hlinkClick", &[("r:id", "rIdHyper")])),
            Some("rIdHyper".to_string())
        );

        let mut archive = archive_with_entries(&[
            ("ppt/slides/slide1.xml", b"<slide/>".as_slice()),
            ("ppt/media/image.png", b"png".as_slice()),
        ]);
        assert_eq!(
            read_archive_entry(&mut archive, "ppt/slides/slide1.xml").unwrap(),
            "<slide/>"
        );
        assert_eq!(
            read_archive_bytes(&mut archive, "ppt/media/image.png").unwrap(),
            b"png"
        );
        assert!(read_archive_entry(&mut archive, "missing.xml").is_err());
        assert_eq!(
            rels_path_for("ppt/slides/slide1.xml"),
            "ppt/slides/_rels/slide1.xml.rels"
        );
        assert_eq!(
            resolve_relative_file_path("ppt/slides/slide1.xml", "../media/image1.png"),
            "ppt/media/image1.png"
        );
        assert_eq!(
            resolve_rel_path("ppt/slides", "../media/image1.png"),
            "ppt/media/image1.png"
        );
        assert_eq!(
            resolve_rel_path("ppt/slides", "media/image1.png"),
            "ppt/slides/media/image1.png"
        );
        assert_eq!(mime_from_extension("image.png"), "image/png");
        assert_eq!(mime_from_extension("image.jpg"), "image/jpeg");
        assert_eq!(mime_from_extension("image.gif"), "image/gif");
        assert_eq!(mime_from_extension("image.bmp"), "image/bmp");
        assert_eq!(mime_from_extension("image.tif"), "image/tiff");
        assert_eq!(mime_from_extension("image.svg"), "image/svg+xml");
        assert_eq!(mime_from_extension("image.emf"), "image/x-emf");
        assert_eq!(mime_from_extension("image.wmf"), "image/x-wmf");
        assert_eq!(mime_from_extension("image.bin"), "image/png");

        let mut list_shape = Some(ShapeBuilder::default());
        store_shape_level_defaults(&mut list_shape, 0, ParagraphDefaults::default());
        assert!(
            list_shape
                .as_ref()
                .and_then(|s| s.text_list_style.as_ref())
                .and_then(|ls| ls.levels[0].as_ref())
                .is_some()
        );

        let paragraph = para.expect("paragraph").build();
        let built_run = run.expect("run").build();
        assert_eq!(
            std::mem::discriminant(&paragraph.alignment),
            std::mem::discriminant(&Alignment::Center)
        );
        assert!(paragraph.rtl);
        assert_eq!(paragraph.level, 2);
        assert_eq!(paragraph.indent, Some(1.0));
        assert_eq!(paragraph.margin_left, Some(2.0));
        assert_eq!(built_run.style.font_size, Some(24.0));
        assert!(built_run.style.bold);
        assert!(built_run.style.italic);
        assert_eq!(
            std::mem::discriminant(&built_run.style.underline),
            std::mem::discriminant(&UnderlineType::Double)
        );
        assert_eq!(
            std::mem::discriminant(&built_run.style.strikethrough),
            std::mem::discriminant(&StrikethroughType::Single)
        );
        assert_eq!(
            std::mem::discriminant(&built_run.style.capitalization),
            std::mem::discriminant(&TextCapitalization::All)
        );
        assert_eq!(built_run.style.baseline, Some(30000));
        assert_eq!(built_run.style.letter_spacing, Some(2.0));

        let chart_shape = ShapeBuilder {
            is_chart: true,
            chart_rel_id: Some("rIdChart".to_string()),
            chart_direct_spec: Some(ChartSpec::default()),
            ..Default::default()
        }
        .build();
        assert_eq!(
            std::mem::discriminant(&chart_shape.shape_type),
            std::mem::discriminant(&ShapeType::Chart(ChartData::default()))
        );

        let picture_shape = ShapeBuilder {
            is_picture: true,
            image_rel_id: Some("rIdImage".to_string()),
            ..Default::default()
        }
        .build();
        assert_eq!(
            std::mem::discriminant(&picture_shape.shape_type),
            std::mem::discriminant(&ShapeType::Picture(PictureData::default()))
        );

        let connector_shape = ShapeBuilder {
            is_connector: true,
            ..Default::default()
        }
        .build();
        assert!(matches!(
            connector_shape.shape_type,
            ShapeType::Custom(ref name) if name == "line"
        ));

        let unsupported_shape = ShapeBuilder {
            unsupported_content: Some("SmartArt".to_string()),
            unresolved_type: Some(UnresolvedType::SmartArt),
            ..Default::default()
        }
        .build();
        assert_eq!(
            std::mem::discriminant(&unsupported_shape.shape_type),
            std::mem::discriminant(&ShapeType::Unsupported(UnsupportedData {
                label: String::new(),
                element_type: UnresolvedType::SmartArt,
                raw_xml: None,
                custom_geometry: None,
            }))
        );

        let custom_geom_shape = ShapeBuilder {
            custom_geometry: Some(CustomGeometry {
                paths: Vec::new(),
                text_rect: None,
                adjust_handles: Vec::new(),
                connection_sites: Vec::new(),
                guides: Vec::new(),
                issues: Vec::new(),
            }),
            ..Default::default()
        }
        .build();
        assert_eq!(
            std::mem::discriminant(&custom_geom_shape.shape_type),
            std::mem::discriminant(&ShapeType::CustomGeom(CustomGeometry {
                paths: Vec::new(),
                text_rect: None,
                adjust_handles: Vec::new(),
                connection_sites: Vec::new(),
                guides: Vec::new(),
                issues: Vec::new(),
            }))
        );

        let mut cell = Some(default_table_cell_builder());
        cell.as_mut().unwrap().border_left.width = 1.0;
        cell.as_mut().unwrap().border_top.width = 1.0;
        assign_tc_color(Color::rgb("112233"), &Some("lnL".to_string()), &mut cell);
        assign_tc_color(Color::rgb("445566"), &Some("lnT".to_string()), &mut cell);
        assign_tc_color(Color::rgb("778899"), &None, &mut cell);
        let cell = cell.expect("cell").build();
        assert_eq!(cell.border_left.color.to_css().as_deref(), Some("#112233"));
        assert_eq!(cell.border_top.color.to_css().as_deref(), Some("#445566"));
        assert!(matches!(
            &cell.fill,
            Fill::Solid(fill) if fill.color.to_css().as_deref() == Some("#778899")
        ));

        let table = TableBuilder {
            rows: vec![
                TableRowBuilder {
                    height: 18.0,
                    cells: vec![default_table_cell_builder().build()],
                }
                .build(),
            ],
            col_widths: vec![120.0],
            band_row: true,
            ..Default::default()
        }
        .build();
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.col_widths, vec![120.0]);
        assert!(table.band_row);
    }

    #[test]
    fn parse_slide_covers_regular_shape_text_and_style_ref_branches() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="7" name="Styled Shape"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm rot="1800000" flipV="1">
            <a:off x="12700" y="25400"/>
            <a:ext cx="457200" cy="228600"/>
          </a:xfrm>
          <a:gradFill>
            <a:gsLst>
              <a:gs pos="0"><a:prstClr val="orange"/></a:gs>
              <a:gs pos="100000"><a:sysClr lastClr="112233"/></a:gs>
            </a:gsLst>
            <a:path path="circle"/>
          </a:gradFill>
          <a:ln w="12700" cap="flat" cmpd="tri" algn="in"></a:ln>
        </p:spPr>
        <p:style>
          <a:lnRef idx="1"><a:schemeClr val="accent1"/></a:lnRef>
          <a:fillRef idx="2"><a:prstClr val="orange"/></a:fillRef>
          <a:effectRef idx="1"><a:sysClr val="windowText"/></a:effectRef>
          <a:fontRef idx="minor"><a:schemeClr val="accent2"/></a:fontRef>
        </p:style>
        <p:txBody>
          <a:bodyPr wrap="none"></a:bodyPr>
          <a:noAutofit></a:noAutofit>
          <a:lstStyle>
            <a:lvl1pPr algn="r">
              <a:lnSpc><a:spcPct val="80000"/></a:lnSpc>
              <a:spcBef><a:spcPts val="1200"/></a:spcBef>
              <a:spcAft><a:spcPct val="110000"/></a:spcAft>
              <a:defRPr sz="1600" spc="100" baseline="20000" cap="small" u="dash" strike="dblStrike" b="1" i="1">
                <a:schemeClr val="accent2"/>
              </a:defRPr>
            </a:lvl1pPr>
          </a:lstStyle>
          <a:p>
            <a:pPr algn="ctr" rtl="1" lvl="1" indent="12700" marL="25400">
              <a:defRPr sz="2400" spc="200" baseline="30000" cap="all" u="dbl" strike="sngStrike" b="1" i="1"/>
            </a:pPr>
            <a:buClr><a:prstClr val="orange"/></a:buClr>
            <a:r>
              <a:rPr sz="1800">
                <a:hlinkClick r:id="rIdHyper"/>
              </a:rPr>
              <a:t>First</a:t>
            </a:r>
            <a:br/>
            <a:r><a:t>Second</a:t></a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="8" name="Shrink Shape"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="228600" cy="114300"/></a:xfrm>
          <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr"></a:bodyPr>
          <a:spAutoFit></a:spAutoFit>
          <a:p/>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let rels = HashMap::from([("rIdHyper".to_string(), "https://example.com".to_string())]);
        let mut archive = archive_with_entries(&[]);

        let slide = parse_slide(slide_xml, &rels, &mut archive).expect("slide should parse");
        assert_eq!(slide.shapes.len(), 2);

        let shape = &slide.shapes[0];
        assert_eq!(shape.id, 7);
        assert_eq!(shape.name, "Styled Shape");
        assert!((shape.rotation - 30.0).abs() < 1e-6);
        assert!(shape.flip_v);
        assert_eq!(
            std::mem::discriminant(&shape.border.cap),
            std::mem::discriminant(&LineCap::Flat)
        );
        assert_eq!(
            std::mem::discriminant(&shape.border.alignment),
            std::mem::discriminant(&LineAlignment::Inset)
        );
        assert_eq!(
            shape
                .style_ref
                .as_ref()
                .and_then(|style| style.effect_ref.as_ref())
                .map(|effect_ref| effect_ref.idx),
            Some(1)
        );
        assert!(matches!(
            &shape.fill,
            Fill::Gradient(fill)
                if fill.stops.len() == 2
                    && fill.stops[0].color.kind == ColorKind::Preset("orange".to_string())
                    && fill.stops[1].color.kind == ColorKind::Rgb("112233".to_string())
        ));

        let text_body = shape.text_body.as_ref().expect("text body");
        assert!(!text_body.word_wrap);
        assert_eq!(
            std::mem::discriminant(&text_body.auto_fit),
            std::mem::discriminant(&AutoFit::NoAutoFit)
        );
        let list_style = text_body.list_style.as_ref().expect("list style");
        let lvl1 = list_style.levels[0].as_ref().expect("level 1 defaults");
        assert_eq!(
            lvl1.alignment
                .as_ref()
                .map(std::mem::discriminant::<Alignment>),
            Some(std::mem::discriminant(&Alignment::Right))
        );
        assert!(matches!(
            lvl1.line_spacing,
            Some(SpacingValue::Percent(v)) if (v - 0.8).abs() < 1e-6
        ));
        assert!(matches!(
            lvl1.space_before,
            Some(SpacingValue::Points(v)) if (v - 12.0).abs() < 1e-6
        ));
        assert!(matches!(
            lvl1.space_after,
            Some(SpacingValue::Percent(v)) if (v - 1.1).abs() < 1e-6
        ));

        let paragraph = &text_body.paragraphs[0];
        assert_eq!(
            std::mem::discriminant(&paragraph.alignment),
            std::mem::discriminant(&Alignment::Center)
        );
        assert!(paragraph.rtl);
        assert_eq!(paragraph.level, 1);
        assert_eq!(paragraph.runs.len(), 3);
        assert_eq!(
            paragraph.runs[0].hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert!(paragraph.runs[1].is_break);
        let def_rpr = paragraph.def_rpr.as_ref().expect("paragraph defRPr");
        assert_eq!(def_rpr.font_size, Some(24.0));
        assert_eq!(def_rpr.letter_spacing, Some(2.0));
        assert_eq!(def_rpr.baseline, Some(30000));
        assert_eq!(def_rpr.bold, Some(true));
        assert_eq!(def_rpr.italic, Some(true));
        assert_eq!(
            def_rpr
                .capitalization
                .as_ref()
                .map(std::mem::discriminant::<TextCapitalization>),
            Some(std::mem::discriminant(&TextCapitalization::All))
        );
        assert_eq!(
            def_rpr
                .underline
                .as_ref()
                .map(std::mem::discriminant::<UnderlineType>),
            Some(std::mem::discriminant(&UnderlineType::Double))
        );
        assert_eq!(
            def_rpr
                .strikethrough
                .as_ref()
                .map(std::mem::discriminant::<StrikethroughType>),
            Some(std::mem::discriminant(&StrikethroughType::Single))
        );

        let shrink_shape = &slide.shapes[1];
        assert_eq!(
            shrink_shape
                .text_body
                .as_ref()
                .map(|tb| std::mem::discriminant(&tb.auto_fit)),
            Some(std::mem::discriminant(&AutoFit::Shrink))
        );
    }

    #[test]
    fn parse_slide_covers_ole_and_table_cell_variants() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="OLE"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/presentationml/2006/oleObject">
            <p:oleObj progId="Excel.Sheet.12" name="Workbook"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="3" name="Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="1" bandCol="1" firstRow="1" lastRow="1" firstCol="1" lastCol="1"></a:tblPr>
              <a:tblGrid>
                <a:gridCol w="914400"/>
                <a:gridCol w="457200"/>
              </a:tblGrid>
              <a:tr h="457200">
                <a:tc gridSpan="2" rowSpan="2" vMerge="1">
                  <a:txBody>
                    <a:bodyPr/>
                    <a:lstStyle/>
                    <a:p>
                      <a:pPr algn="ctr" lvl="1" indent="91440" marL="45720">
                        <a:defRPr sz="2000" spc="100" baseline="10000" cap="small" u="dashLong" strike="dblStrike" b="1" i="1"/>
                      </a:pPr>
                      <a:buClr><a:schemeClr val="accent2"/></a:buClr>
                      <a:r>
                        <a:rPr sz="1800"><a:hlinkClick r:id="rIdHyper"/></a:rPr>
                        <a:t>Cell</a:t>
                      </a:r>
                      <a:br/>
                    </a:p>
                  </a:txBody>
                  <a:tcPr marL="91440" marR="137160" marT="45720" marB="22860" anchor="b">
                    <a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>
                    <a:lnL w="12700"><a:prstDash val="sysDot"></a:prstDash><a:srgbClr val="FF0000"/></a:lnL>
                    <a:lnR w="12700"><a:prstDash val="lgDashDot"></a:prstDash><a:srgbClr val="0000FF"/></a:lnR>
                    <a:lnT w="12700"><a:prstDash val="lgDashDotDot"></a:prstDash><a:srgbClr val="123456"/></a:lnT>
                    <a:lnB w="12700"><a:prstDash val="sysDash"></a:prstDash><a:srgbClr val="654321"/></a:lnB>
                  </a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let rels = HashMap::from([(
            "rIdHyper".to_string(),
            "https://example.com/table".to_string(),
        )]);
        let mut archive = archive_with_entries(&[]);

        let slide = parse_slide(slide_xml, &rels, &mut archive).expect("slide should parse");
        assert_eq!(slide.shapes.len(), 2);

        assert!(matches!(
            &slide.shapes[0].shape_type,
            ShapeType::Unsupported(data)
                if data.label == "OLE Object"
                    && data
                        .raw_xml
                        .as_deref()
                        .is_some_and(|raw| raw.contains("progId=\"Excel.Sheet.12\""))
        ));

        let table = slide
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape_type {
                ShapeType::Table(table) => Some(table),
                _ => None,
            })
            .expect("table shape");
        assert!(table.band_row && table.band_col && table.first_row && table.last_row);
        assert!(table.first_col && table.last_col);
        assert_eq!(table.col_widths.len(), 2);

        let cell = &table.rows[0].cells[0];
        assert_eq!(cell.col_span, 2);
        assert_eq!(cell.row_span, 2);
        assert!(cell.v_merge);
        assert_eq!(
            std::mem::discriminant(&cell.vertical_align),
            std::mem::discriminant(&VerticalAlign::Bottom)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_left.style),
            std::mem::discriminant(&BorderStyle::Dotted)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_left.dash_style),
            std::mem::discriminant(&DashStyle::SystemDot)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_right.style),
            std::mem::discriminant(&BorderStyle::Dotted)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_right.dash_style),
            std::mem::discriminant(&DashStyle::LongDashDot)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_top.style),
            std::mem::discriminant(&BorderStyle::Dotted)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_top.dash_style),
            std::mem::discriminant(&DashStyle::LongDashDotDot)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_bottom.style),
            std::mem::discriminant(&BorderStyle::Dashed)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_bottom.dash_style),
            std::mem::discriminant(&DashStyle::SystemDash)
        );

        let paragraph = &cell.text_body.as_ref().expect("cell text body").paragraphs[0];
        assert_eq!(
            std::mem::discriminant(&paragraph.alignment),
            std::mem::discriminant(&Alignment::Center)
        );
        assert_eq!(paragraph.level, 1);
        assert_eq!(paragraph.runs.len(), 2);
        assert_eq!(
            paragraph.runs[0].hyperlink.as_deref(),
            Some("https://example.com/table")
        );
        assert!(paragraph.runs[1].is_break);
        let def_rpr = paragraph.def_rpr.as_ref().expect("cell paragraph defRPr");
        assert_eq!(def_rpr.font_size, Some(20.0));
        assert_eq!(def_rpr.letter_spacing, Some(1.0));
        assert_eq!(def_rpr.baseline, Some(10000));
        assert_eq!(def_rpr.bold, Some(true));
        assert_eq!(def_rpr.italic, Some(true));
        assert_eq!(
            def_rpr
                .capitalization
                .as_ref()
                .map(std::mem::discriminant::<TextCapitalization>),
            Some(std::mem::discriminant(&TextCapitalization::Small))
        );
        assert_eq!(
            def_rpr
                .underline
                .as_ref()
                .map(std::mem::discriminant::<UnderlineType>),
            Some(std::mem::discriminant(&UnderlineType::DashLong))
        );
        assert_eq!(
            def_rpr
                .strikethrough
                .as_ref()
                .map(std::mem::discriminant::<StrikethroughType>),
            Some(std::mem::discriminant(&StrikethroughType::Double))
        );
    }

    #[test]
    fn parse_slide_loads_chart_preview_and_picture_crop_variants() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="Chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="914400"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <chart r:id="rIdChart"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:pic>
        <p:nvPicPr>
          <p:cNvPr id="3" name="Picture"/>
          <p:cNvPicPr/>
          <p:nvPr/>
        </p:nvPicPr>
        <p:blipFill>
          <a:blip r:embed="rIdImage"/>
          <a:srcRect l="10000" t="20000" r="30000" b="40000"/>
        </p:blipFill>
        <p:spPr>
          <a:xfrm rot="5400000" flipH="1" flipV="true"/>
          <a:prstGeom prst="rect"/>
        </p:spPr>
      </p:pic>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let slide_rels = HashMap::from([
            ("rIdChart".to_string(), "../charts/chart1.xml".to_string()),
            ("rIdImage".to_string(), "../media/image1.png".to_string()),
        ]);
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
          <c:cat>
            <c:strLit><c:ptCount val="1"/><c:pt idx="0"><c:v>Only</c:v></c:pt></c:strLit>
          </c:cat>
          <c:val>
            <c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>42</c:v></c:pt></c:numLit>
          </c:val>
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
        let mut archive = archive_with_entries(&[
            ("ppt/charts/chart1.xml", chart_xml.as_bytes()),
            ("ppt/charts/_rels/chart1.xml.rels", chart_rels.as_bytes()),
            ("ppt/media/chart-preview.png", b"preview".as_slice()),
            ("ppt/media/image1.png", b"image-bytes".as_slice()),
        ]);

        let slide = parse_slide(slide_xml, &slide_rels, &mut archive).expect("slide should parse");
        assert_eq!(slide.shapes.len(), 2);

        let chart = slide
            .shapes
            .iter()
            .rev()
            .find_map(|shape| match &shape.shape_type {
                ShapeType::Chart(chart) => Some(chart),
                _ => None,
            })
            .expect("chart shape");
        assert_eq!(chart.rel_id, "rIdChart");
        assert!(
            chart.direct_spec.is_some(),
            "expected parsed direct chart spec"
        );
        assert_eq!(chart.preview_image.as_deref(), Some(b"preview".as_slice()));
        assert_eq!(chart.preview_mime.as_deref(), Some("image/png"));

        let picture = slide
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape_type {
                ShapeType::Picture(pic) => Some((shape, pic)),
                _ => None,
            })
            .expect("picture shape");
        assert!((picture.0.rotation - 90.0).abs() < 1e-6);
        assert!(picture.0.flip_h);
        assert!(picture.0.flip_v);
        assert_eq!(picture.1.data, b"image-bytes");
        let crop = picture.1.crop.as_ref().expect("picture crop");
        assert!((crop.left - 0.1).abs() < 1e-6);
        assert!((crop.top - 0.2).abs() < 1e-6);
        assert!((crop.right - 0.3).abs() < 1e-6);
        assert!((crop.bottom - 0.4).abs() < 1e-6);
    }

    #[test]
    fn parse_slide_handles_background_blips_custom_geometry_and_start_tag_connectors() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:o="urn:schemas-microsoft-com:office:office"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:blipFill><a:blip r:embed="rIdBg"></a:blip></a:blipFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""></p:cNvPr><p:cNvGrpSpPr></p:cNvGrpSpPr><p:nvPr></p:nvPr></p:nvGrpSpPr>
      <p:grpSpPr></p:grpSpPr>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="OLE"></p:cNvPr><p:cNvGraphicFramePr></p:cNvGraphicFramePr><p:nvPr></p:nvPr></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"></a:off><a:ext cx="914400" cy="457200"></a:ext></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/presentationml/2006/oleObject">
            <o:OLEObject ProgID="Excel.Sheet.12"><o:Link></o:Link></o:OLEObject>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="Custom Shape"></p:cNvPr><p:cNvSpPr></p:cNvSpPr><p:nvPr></p:nvPr></p:nvSpPr>
        <p:spPr>
          <a:xfrm rot="5400000" flipH="1" flipV="true"><a:off x="12700" y="25400"></a:off><a:ext cx="914400" cy="457200"></a:ext></a:xfrm>
          <a:gradFill>
            <a:gsLst>
              <a:gs pos="0"><a:prstClr val="orange"></a:prstClr></a:gs>
              <a:gs pos="100000"><a:sysClr lastClr="ABCDEF"></a:sysClr></a:gs>
            </a:gsLst>
            <a:path path="shape"></a:path>
          </a:gradFill>
          <a:ln cap="flat" cmpd="tri" algn="in">
            <a:prstDash val="sysDashDot"/>
            <a:sysClr val="windowText"></a:sysClr>
          </a:ln>
          <a:effectLst>
            <a:outerShdw blurRad="12700" dist="25400" dir="5400000">
              <a:prstClr val="orange"></a:prstClr>
              <a:alpha val="75000"></a:alpha>
            </a:outerShdw>
            <a:glow rad="6350">
              <a:sysClr lastClr="123456"></a:sysClr>
              <a:alpha val="60000"></a:alpha>
            </a:glow>
          </a:effectLst>
          <a:custGeom>
            <a:avLst><a:gd name="adj1" fmla="val 50000"></a:gd></a:avLst>
            <a:gdLst><a:gd name="x1" fmla="val 100000"></a:gd></a:gdLst>
            <a:ahLst>
              <a:ahXY gdRefX="adj1" minX="0" maxX="100000" gdRefY="adj1" minY="0" maxY="100000">
                <a:pos x="50000" y="50000"></a:pos>
              </a:ahXY>
            </a:ahLst>
            <a:cxnLst><a:cxn ang="0"><a:pos x="0" y="0"></a:pos></a:cxn></a:cxnLst>
            <a:rect l="0" t="0" r="100000" b="100000"></a:rect>
            <a:pathLst>
              <a:path w="100000" h="100000" fill="darkenLess">
                <a:moveTo><a:pt x="0" y="0"></a:pt></a:moveTo>
                <a:lnTo><a:pt x="100000" y="0"></a:pt></a:lnTo>
                <a:arcTo wR="50000" hR="50000" stAng="0" swAng="5400000"></a:arcTo>
                <a:close></a:close>
              </a:path>
            </a:pathLst>
          </a:custGeom>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="b" anchorCtr="1" rot="1800000" vert="vert" lIns="45720" tIns="91440" rIns="137160" bIns="182880" wrap="none"></a:bodyPr>
          <a:spAutoFit></a:spAutoFit>
          <a:p>
            <a:pPr algn="r" rtl="1" lvl="1" indent="12700" marL="25400">
              <a:lnSpc><a:spcPct val="120000"></a:spcPct></a:lnSpc>
              <a:spcBef><a:spcPts val="1200"></a:spcPts></a:spcBef>
              <a:spcAft><a:spcPct val="25000"></a:spcPct></a:spcAft>
              <a:buClr><a:srgbClr val="334455"></a:srgbClr></a:buClr>
              <a:defRPr sz="2000" spc="100" baseline="30000" cap="all" u="dbl" strike="sngStrike" b="1" i="1"></a:defRPr>
            </a:pPr>
            <a:r>
              <a:rPr sz="1800">
                <a:highlight><a:prstClr val="orange"></a:prstClr></a:highlight>
                <a:effectLst><a:outerShdw blurRad="12700" dist="25400" dir="2700000"><a:sysClr val="windowText"></a:sysClr></a:outerShdw></a:effectLst>
                <a:hlinkClick r:id="rIdHyper"></a:hlinkClick>
              </a:rPr>
              <a:t>Rich Text</a:t>
            </a:r>
            <a:br></a:br>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="4" name="No AutoFit"></p:cNvPr><p:cNvSpPr></p:cNvSpPr><p:nvPr></p:nvPr></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"></a:off><a:ext cx="457200" cy="457200"></a:ext></a:xfrm><a:prstGeom prst="rect"><a:avLst></a:avLst></a:prstGeom></p:spPr>
        <p:txBody><a:bodyPr></a:bodyPr><a:noAutofit></a:noAutofit><a:p><a:r><a:t>None</a:t></a:r></a:p></p:txBody>
      </p:sp>
      <p:pic>
        <p:nvPicPr><p:cNvPr id="5" name="Picture"></p:cNvPr><p:cNvPicPr></p:cNvPicPr><p:nvPr></p:nvPr></p:nvPicPr>
        <p:blipFill><a:blip r:embed="rIdPic"></a:blip></p:blipFill>
        <p:spPr><a:xfrm><a:off x="0" y="0"></a:off><a:ext cx="457200" cy="457200"></a:ext></a:xfrm></p:spPr>
      </p:pic>
      <p:cxnSp>
        <p:nvCxnSpPr><p:cNvPr id="6" name="Connector"></p:cNvPr><p:cNvCxnSpPr></p:cNvCxnSpPr><p:nvPr></p:nvPr></p:nvCxnSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"></a:off><a:ext cx="0" cy="914400"></a:ext></a:xfrm><a:ln><a:headEnd type="triangle" w="lg" len="sm"></a:headEnd><a:tailEnd type="oval" w="sm" len="lg"></a:tailEnd></a:ln></p:spPr>
        <p:stCxn id="10" idx="1"></p:stCxn>
        <p:endCxn id="11" idx="2"></p:endCxn>
      </p:cxnSp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let rels = HashMap::from([
            ("rIdBg".to_string(), "../media/background.png".to_string()),
            ("rIdPic".to_string(), "../media/picture.png".to_string()),
            ("rIdHyper".to_string(), "https://example.com".to_string()),
        ]);
        let mut archive = archive_with_entries(&[
            ("ppt/media/background.png", b"bg-data"),
            ("ppt/media/picture.png", b"pic-data"),
        ]);

        let slide = parse_slide(slide_xml, &rels, &mut archive).expect("slide should parse");

        assert!(matches!(
            &slide.background,
            Some(Fill::Image(fill)) if fill.rel_id == "rIdBg" && fill.data == b"bg-data"
        ));
        assert_eq!(slide.shapes.len(), 5);

        assert!(slide.shapes.iter().any(|shape| matches!(
            &shape.shape_type,
            ShapeType::Unsupported(data)
                if data.raw_xml.as_deref().is_some_and(|raw| raw.contains("OLEObject"))
        )));

        let custom_shape = slide
            .shapes
            .iter()
            .find(|shape| matches!(shape.shape_type, ShapeType::CustomGeom(_)))
            .expect("custom geometry shape");
        assert!(custom_shape.flip_h);
        assert!(custom_shape.flip_v);
        assert_eq!(
            std::mem::discriminant(&custom_shape.border.cap),
            std::mem::discriminant(&LineCap::Flat)
        );
        assert_eq!(
            std::mem::discriminant(&custom_shape.border.compound),
            std::mem::discriminant(&CompoundLine::Triple)
        );
        assert_eq!(
            std::mem::discriminant(&custom_shape.border.alignment),
            std::mem::discriminant(&LineAlignment::Inset)
        );
        let custom_text = custom_shape.text_body.as_ref().expect("custom text body");
        assert_eq!(
            std::mem::discriminant(&custom_text.auto_fit),
            std::mem::discriminant(&AutoFit::Shrink)
        );
        assert_eq!(
            std::mem::discriminant(&custom_text.vertical_align),
            std::mem::discriminant(&VerticalAlign::Bottom)
        );
        assert_eq!(custom_shape.vertical_text.as_deref(), Some("vert"));
        let custom_paragraph = &custom_text.paragraphs[0];
        assert_eq!(
            custom_paragraph.runs[0].hyperlink.as_deref(),
            Some("https://example.com")
        );
        assert!(custom_paragraph.runs[1].is_break);
        assert!(custom_paragraph.runs[0].style.highlight.is_some());
        assert!(custom_paragraph.runs[0].style.shadow.is_some());

        let no_autofit = slide
            .shapes
            .iter()
            .find(|shape| shape.name == "No AutoFit")
            .expect("no autofit shape");
        assert_eq!(
            no_autofit
                .text_body
                .as_ref()
                .map(|body| std::mem::discriminant(&body.auto_fit)),
            Some(std::mem::discriminant(&AutoFit::NoAutoFit))
        );

        let picture = slide
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape_type {
                ShapeType::Picture(pic) => Some(pic),
                _ => None,
            })
            .expect("picture shape");
        assert_eq!(picture.rel_id, "rIdPic");
        assert_eq!(picture.data, b"pic-data");

        let connector = slide
            .shapes
            .iter()
            .find(|shape| matches!(shape.shape_type, ShapeType::Custom(ref name) if name == "line"))
            .expect("connector shape");
        assert_eq!(
            connector
                .start_connection
                .as_ref()
                .map(|conn| conn.shape_id),
            Some(10)
        );
        assert_eq!(
            connector.end_connection.as_ref().map(|conn| conn.site_idx),
            Some(2)
        );
    }

    #[test]
    fn parse_slide_covers_table_start_tags_norm_autofit_and_effect_color_variants() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="FF0000"></a:srgbClr></a:gs>
            <a:gs pos="100000"><a:schemeClr val="accent1"></a:schemeClr></a:gs>
          </a:gsLst>
          <a:path path="rect"></a:path>
        </a:gradFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""></p:cNvPr><p:cNvGrpSpPr></p:cNvGrpSpPr><p:nvPr></p:nvPr></p:nvGrpSpPr>
      <p:grpSpPr></p:grpSpPr>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="Table"></p:cNvPr><p:cNvGraphicFramePr></p:cNvGraphicFramePr><p:nvPr></p:nvPr></p:nvGraphicFramePr>
        <p:xfrm><a:off x="0" y="0"></a:off><a:ext cx="1828800" cy="914400"></a:ext></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr bandRow="true" bandCol="true" firstRow="true" lastRow="true" firstCol="true" lastCol="true"></a:tblPr>
              <a:tblGrid><a:gridCol w="914400"></a:gridCol></a:tblGrid>
              <a:tr h="457200">
                <a:tc gridSpan="1" rowSpan="1" vMerge="true">
                  <a:txBody>
                    <a:bodyPr></a:bodyPr>
                    <a:lstStyle></a:lstStyle>
                    <a:p>
                      <a:pPr algn="ctr" lvl="1" indent="91440" marL="45720">
                        <a:defRPr sz="2000" spc="100" baseline="10000" cap="small" u="dashLong" strike="dblStrike" b="true" i="true"></a:defRPr>
                        <a:lnSpc><a:spcPct val="125000"></a:spcPct></a:lnSpc>
                        <a:spcBef><a:spcPts val="1200"></a:spcPts></a:spcBef>
                        <a:spcAft><a:spcPts val="600"></a:spcPts></a:spcAft>
                        <a:buClr><a:schemeClr val="accent2"></a:schemeClr></a:buClr>
                      </a:pPr>
                      <a:r>
                        <a:rPr sz="1800"><a:hlinkClick r:id="rIdHyper"/></a:rPr>
                        <a:t>Cell</a:t>
                      </a:r>
                      <a:br></a:br>
                    </a:p>
                  </a:txBody>
                  <a:tcPr marL="91440" marR="137160" marT="45720" marB="22860" anchor="b">
                    <a:solidFill><a:srgbClr val="00FF00"></a:srgbClr></a:solidFill>
                    <a:lnL w="12700"><a:prstDash val="solid"></a:prstDash><a:srgbClr val="FF0000"></a:srgbClr></a:lnL>
                    <a:lnR w="12700"><a:prstDash val="dash"></a:prstDash><a:srgbClr val="0000FF"></a:srgbClr></a:lnR>
                    <a:lnT w="12700"><a:prstDash val="dot"></a:prstDash><a:srgbClr val="123456"></a:srgbClr></a:lnT>
                    <a:lnB w="12700"><a:prstDash val="lgDash"></a:prstDash><a:srgbClr val="654321"></a:srgbClr></a:lnB>
                  </a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="Norm Autofit Shape"></p:cNvPr><p:cNvSpPr></p:cNvSpPr><p:nvPr></p:nvPr></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"></a:off><a:ext cx="914400" cy="457200"></a:ext></a:xfrm>
          <a:gradFill>
            <a:gsLst>
              <a:gs pos="0"><a:prstClr val="orange"></a:prstClr></a:gs>
              <a:gs pos="100000"><a:sysClr lastClr="112233"></a:sysClr></a:gs>
            </a:gsLst>
            <a:path path="shape"></a:path>
          </a:gradFill>
          <a:effectLst>
            <a:outerShdw blurRad="12700" dist="25400" dir="5400000"><a:schemeClr val="accent1"></a:schemeClr></a:outerShdw>
            <a:glow rad="6350"><a:sysClr val="windowText"></a:sysClr></a:glow>
          </a:effectLst>
          <a:custGeom>
            <a:pathLst>
              <a:path w="100000" h="100000" fill="lighten"><a:moveTo><a:pt x="0" y="0"></a:pt></a:moveTo></a:path>
              <a:path w="100000" h="100000" fill="darken"><a:moveTo><a:pt x="0" y="0"></a:pt></a:moveTo></a:path>
              <a:path w="100000" h="100000" fill="lightenLess"><a:moveTo><a:pt x="0" y="0"></a:pt></a:moveTo></a:path>
            </a:pathLst>
          </a:custGeom>
        </p:spPr>
        <p:txBody>
          <a:bodyPr wrap="none"></a:bodyPr>
          <a:normAutofit fontScale="70000" lnSpcReduction="15000"></a:normAutofit>
          <a:p><a:r><a:t>Shape</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let rels = HashMap::from([(
            "rIdHyper".to_string(),
            "https://example.com/table-start".to_string(),
        )]);
        let mut archive = archive_with_entries(&[]);

        let slide = parse_slide(slide_xml, &rels, &mut archive).expect("slide parses");

        assert!(matches!(
            slide.background.as_ref(),
            Some(Fill::Gradient(fill))
                if fill.stops.len() == 2
                    && std::mem::discriminant(&fill.gradient_type)
                        == std::mem::discriminant(&GradientType::Rectangular)
        ));

        let table = slide
            .shapes
            .iter()
            .rev()
            .find_map(|shape| match &shape.shape_type {
                ShapeType::Table(table) => Some(table),
                _ => None,
            })
            .expect("table shape");
        assert!(table.band_row && table.band_col && table.first_row && table.last_row);
        assert!(table.first_col && table.last_col);
        let cell = &table.rows[0].cells[0];
        assert!(cell.v_merge);
        assert_eq!(
            std::mem::discriminant(&cell.vertical_align),
            std::mem::discriminant(&VerticalAlign::Bottom)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_left.style),
            std::mem::discriminant(&BorderStyle::Solid)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_left.dash_style),
            std::mem::discriminant(&DashStyle::Solid)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_right.style),
            std::mem::discriminant(&BorderStyle::Dashed)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_right.dash_style),
            std::mem::discriminant(&DashStyle::Dash)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_top.style),
            std::mem::discriminant(&BorderStyle::Dotted)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_top.dash_style),
            std::mem::discriminant(&DashStyle::Dot)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_bottom.style),
            std::mem::discriminant(&BorderStyle::Dashed)
        );
        assert_eq!(
            std::mem::discriminant(&cell.border_bottom.dash_style),
            std::mem::discriminant(&DashStyle::LongDash)
        );

        let paragraph = &cell.text_body.as_ref().expect("table text body").paragraphs[0];
        assert_eq!(
            paragraph.runs.len(),
            2,
            "expected one run plus one line break"
        );
        assert_eq!(
            paragraph.runs[0].hyperlink.as_deref(),
            Some("https://example.com/table-start")
        );
        assert!(paragraph.runs[1].is_break);
        let def_rpr = paragraph.def_rpr.as_ref().expect("table start-tag defRPr");
        assert_eq!(def_rpr.font_size, Some(20.0));
        assert_eq!(def_rpr.letter_spacing, Some(1.0));
        assert_eq!(def_rpr.baseline, Some(10000));
        assert_eq!(def_rpr.bold, Some(true));
        assert_eq!(def_rpr.italic, Some(true));
        assert_eq!(
            def_rpr
                .capitalization
                .as_ref()
                .map(std::mem::discriminant::<TextCapitalization>),
            Some(std::mem::discriminant(&TextCapitalization::Small))
        );
        assert_eq!(
            def_rpr
                .underline
                .as_ref()
                .map(std::mem::discriminant::<UnderlineType>),
            Some(std::mem::discriminant(&UnderlineType::DashLong))
        );
        assert_eq!(
            def_rpr
                .strikethrough
                .as_ref()
                .map(std::mem::discriminant::<StrikethroughType>),
            Some(std::mem::discriminant(&StrikethroughType::Double))
        );

        let shape = slide
            .shapes
            .iter()
            .find(|shape| shape.name == "Norm Autofit Shape")
            .expect("norm autofit shape");
        let auto_fit = shape
            .text_body
            .as_ref()
            .map(|body| &body.auto_fit)
            .expect("normal autofit body");
        assert!(matches!(
            auto_fit,
            AutoFit::Normal {
                font_scale: Some(v),
                line_spacing_reduction: Some(lsr),
            } if (*v - 0.7).abs() < 1e-6 && (*lsr - 0.15).abs() < 1e-6
        ));
        let custom_geom = slide
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape_type {
                ShapeType::CustomGeom(geom) => Some(geom),
                _ => None,
            })
            .expect("custom geometry");
        assert_eq!(custom_geom.paths.len(), 3);
        assert_eq!(
            std::mem::discriminant(&custom_geom.paths[0].fill),
            std::mem::discriminant(&PathFill::Lighten)
        );
        assert_eq!(
            std::mem::discriminant(&custom_geom.paths[1].fill),
            std::mem::discriminant(&PathFill::Darken)
        );
        assert_eq!(
            std::mem::discriminant(&custom_geom.paths[2].fill),
            std::mem::discriminant(&PathFill::LightenLess)
        );
    }

    #[test]
    fn parse_slide_covers_empty_variant_shape_and_table_branches() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
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
                      <a:defRPr sz="2000" spc="100" baseline="10000" cap="small" u="dashLong" strike="dblStrike" b="1" i="true"/>
                      <a:r>
                        <a:rPr sz="1800"/>
                        <a:t>Cell One</a:t>
                      </a:r>
                      <a:r>
                        <a:rPr sz="1800"><a:hlinkClick r:id="rIdCell"/></a:rPr>
                        <a:t>Cell Two</a:t>
                      </a:r>
                      <a:br/>
                    </a:p>
                  </a:txBody>
                  <a:tcPr marL="91440" marR="137160" marT="45720" marB="22860" anchor="ctr">
                    <a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>
                    <a:lnL w="12700"><a:prstDash val="solid"/><a:srgbClr val="FF0000"/></a:lnL>
                    <a:lnR w="12700"><a:prstDash val="dash"/><a:srgbClr val="0000FF"/></a:lnR>
                    <a:lnT w="12700"><a:prstDash val="dot"/><a:srgbClr val="123456"/></a:lnT>
                    <a:lnB w="12700"><a:prstDash val="sysDash"/><a:srgbClr val="654321"/></a:lnB>
                  </a:tcPr>
                </a:tc>
              </a:tr>
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="3" name="Shape Empty Variants"/>
          <p:cNvSpPr/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm rot="5400000" flipH="1"><a:off x="12700" y="25400"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rect"/>
        </p:spPr>
        <p:txBody>
          <a:bodyPr anchor="ctr" wrap="none"/>
          <a:normAutofit fontScale="80000" lnSpcReduction="10000"/>
          <a:p>
            <a:pPr algn="r" lvl="1" indent="12700" marL="25400"/>
            <a:defRPr sz="2400" spc="200" baseline="30000" cap="all" u="dbl" strike="sngStrike" b="true" i="1"/>
            <a:r>
              <a:rPr sz="1800"/>
              <a:t>Shape One</a:t>
            </a:r>
            <a:r>
              <a:rPr sz="1800"><a:hlinkClick r:id="rIdShape"/></a:rPr>
              <a:t>Shape Two</a:t>
            </a:r>
            <a:br/>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="4" name="No Autofit"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="457200" cy="228600"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr>
        <p:txBody><a:bodyPr/><a:noAutofit/><a:p><a:r><a:t>No Autofit</a:t></a:r></a:p></p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="5" name="Shrink Autofit"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="457200" cy="228600"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr>
        <p:txBody><a:bodyPr/><a:spAutoFit/><a:p><a:r><a:t>Shrink Autofit</a:t></a:r></a:p></p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let rels = HashMap::from([
            (
                "rIdCell".to_string(),
                "https://example.com/cell".to_string(),
            ),
            (
                "rIdShape".to_string(),
                "https://example.com/shape".to_string(),
            ),
        ]);
        let mut archive = archive_with_entries(&[]);

        let slide = parse_slide(slide_xml, &rels, &mut archive).expect("slide parses");
        assert_eq!(slide.shapes.len(), 4);

        let table = slide
            .shapes
            .iter()
            .rev()
            .find_map(|shape| match &shape.shape_type {
                ShapeType::Table(table) => Some(table),
                _ => None,
            })
            .expect("table shape");
        assert!(table.band_row && table.band_col && table.first_row && table.last_row);
        assert!(table.first_col && table.last_col);
        let cell = &table.rows[0].cells[0];
        assert_eq!(cell.col_span, 2);
        assert_eq!(
            cell.text_body.as_ref().expect("cell text body").paragraphs[0]
                .runs
                .len(),
            3
        );
        assert_eq!(
            cell.text_body.as_ref().expect("cell text body").paragraphs[0].runs[1]
                .hyperlink
                .as_deref(),
            Some("https://example.com/cell")
        );

        let shape = slide
            .shapes
            .iter()
            .find(|shape| shape.name == "Shape Empty Variants")
            .expect("shape with empty variants");
        let paragraph = &shape
            .text_body
            .as_ref()
            .expect("shape text body")
            .paragraphs[0];
        assert_eq!(paragraph.runs.len(), 3);
        assert_eq!(
            paragraph.runs[1].hyperlink.as_deref(),
            Some("https://example.com/shape")
        );
        let auto_fit = shape
            .text_body
            .as_ref()
            .map(|body| &body.auto_fit)
            .expect("shape empty variants body");
        assert!(matches!(
            auto_fit,
            AutoFit::Normal {
                font_scale: Some(v),
                line_spacing_reduction: Some(lsr),
            } if (*v - 0.8).abs() < 1e-6 && (*lsr - 0.1).abs() < 1e-6
        ));

        let no_autofit = slide
            .shapes
            .iter()
            .find(|shape| shape.name == "No Autofit")
            .expect("no autofit shape");
        assert_eq!(
            no_autofit
                .text_body
                .as_ref()
                .map(|body| std::mem::discriminant(&body.auto_fit)),
            Some(std::mem::discriminant(&AutoFit::NoAutoFit))
        );

        let shrink_autofit = slide
            .shapes
            .iter()
            .find(|shape| shape.name == "Shrink Autofit")
            .expect("shrink autofit shape");
        assert_eq!(
            shrink_autofit
                .text_body
                .as_ref()
                .map(|body| std::mem::discriminant(&body.auto_fit)),
            Some(std::mem::discriminant(&AutoFit::Shrink))
        );
    }

    #[test]
    fn helper_edge_cases_cover_absent_builders_and_defaults() {
        assert_eq!(rels_path_for("chart1.xml"), "_rels/chart1.xml.rels");
        assert_eq!(
            hyperlink_rel_id(&bytes_start("a:hlinkClick", &[("id", "rIdPlain")])),
            None
        );

        let mut missing_style_ref = None;
        assign_style_ref_color("lnRef", "1", Color::rgb("112233"), &mut missing_style_ref);
        ensure_style_ref("fillRef", "1", &mut missing_style_ref);
        assign_style_ref_no_color("effectRef", "2", &mut missing_style_ref);

        let mut style_ref = Some(ShapeStyleRef::default());
        ensure_style_ref("fontRef", "minor", &mut style_ref);
        assign_style_ref_color("unknownRef", "9", Color::rgb("445566"), &mut style_ref);
        assign_style_ref_no_color("unknownRef", "9", &mut style_ref);
        let font_ref = style_ref
            .as_ref()
            .and_then(|style| style.font_ref.as_ref())
            .expect("font ref");
        assert_eq!(font_ref.idx, "minor");
        assert!(font_ref.color.is_none());

        let default_cell = TableCellBuilder::default().build();
        assert_eq!(default_cell.margin_left, TableCell::default().margin_left);

        let mut missing_cell = None;
        assign_tc_color(Color::rgb("778899"), &None, &mut missing_cell);
        let mut cell = Some(TableCellBuilder::default());
        assign_tc_color(Color::rgb("AABBCC"), &Some("lnDiag".to_string()), &mut cell);
        assert_eq!(
            std::mem::discriminant(&cell.expect("cell").fill),
            std::mem::discriminant(&Fill::None)
        );

        let mut missing_shape = None;
        store_shape_level_defaults(&mut missing_shape, 9, ParagraphDefaults::default());
        let mut shape = Some(ShapeBuilder::default());
        store_shape_level_defaults(&mut shape, 10, ParagraphDefaults::default());
        assert!(
            shape
                .as_ref()
                .and_then(|shape| shape.text_list_style.as_ref())
                .is_none()
        );

        let mut anonymous_shape = Some(ShapeBuilder::default());
        parse_shape_identity(&bytes_start("p:cNvPr", &[]), &mut anonymous_shape);
        assert_eq!(anonymous_shape.as_ref().map(|shape| shape.id), Some(0));
        assert_eq!(
            anonymous_shape.as_ref().map(|shape| shape.name.as_str()),
            Some("")
        );
        let mut missing_connector = None;
        parse_connector_ref(&bytes_start("a:stCxn", &[]), &mut missing_connector, true);
        let mut rtl_para = Some(ParagraphBuilder::default());
        parse_para_props(&bytes_start("a:pPr", &[("rtl", "true")]), &mut rtl_para);
        assert!(rtl_para.as_ref().is_some_and(|para| para.rtl));
        assert_eq!(
            resolve_rel_path("ppt/slides", "../media/./image.png"),
            "ppt/media/image.png"
        );

        let mut archive = archive_with_entries(&[]);
        let malformed = "<p:sld xmlns:p=\"p\"><p:cSld><";
        assert!(parse_slide(malformed, &HashMap::new(), &mut archive).is_err());
    }

    fn bytes_start<'a>(name: &'a str, attrs: &[(&'a str, &'a str)]) -> BytesStart<'a> {
        let mut start = BytesStart::new(name);
        for (key, value) in attrs {
            start.push_attribute((*key, *value));
        }
        start
    }

    fn archive_with_entries(entries: &[(&str, &[u8])]) -> ZipArchive<Cursor<Vec<u8>>> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        for (path, data) in entries {
            zip.start_file(path, options).expect("start file");
            zip.write_all(data).expect("write file");
        }
        let cursor = zip.finish().expect("finish zip");
        ZipArchive::new(cursor).expect("open archive")
    }

    fn default_table_cell_builder() -> TableCellBuilder {
        TableCellBuilder {
            text_body: None,
            fill: Fill::None,
            border_left: Border::default(),
            border_right: Border::default(),
            border_top: Border::default(),
            border_bottom: Border::default(),
            col_span: 0,
            row_span: 0,
            v_merge: false,
            h_merge: false,
            explicit_borders: 0,
            margin_left: 7.2,
            margin_right: 7.2,
            margin_top: 3.6,
            margin_bottom: 3.6,
            vertical_align: VerticalAlign::Top,
        }
    }
}
