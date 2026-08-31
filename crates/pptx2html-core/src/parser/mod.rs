//! PPTX ZIP/XML parser
//! PPTX = ZIP archive containing OOXML (PresentationML) XML files

mod action_parser;
mod action_relationship;
mod chart_diagnostics;
mod chart_parser;
mod custom_geometry;
mod custom_guide;
mod effect_diagnostics;
mod embedded_parser;
mod fill_parser;
mod graphic_frame_parser;
mod layout_parser;
pub mod master_parser;
mod media_parser;
mod notes_comments_parser;
mod pattern_fill_parser;
mod picture_bullet_diagnostics;
mod picture_bullet_parser;
mod presentation_extension_parser;
mod preserved_parser;
pub mod relationships;
mod slide_parser;
mod table_parser;
pub(crate) mod table_style_builtins;
mod table_style_package_diagnostics;
mod table_style_parser;
mod table_style_values;
mod table_style_xml;
mod text_parser;
mod theme_parser;
mod timing_parser;
mod xml_utils;

pub(crate) use presentation_extension_parser::diagnostics as presentation_extension_diagnostics;
pub(crate) use preserved_parser::collect_package_diagnostics;

use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

use log::{info, warn};
use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

use crate::error::{PptxError, PptxResult};
use crate::model::{
    Emu, ListStyle, NotesCommentsInventory, Presentation, PresentationExtensionMetadata, Shape,
    ShapeType, Size, TableStyle, TableStyleSourceKind,
};

#[derive(Debug, Clone)]
struct SlideRef {
    rel_id: String,
}

/// SAX-based streaming parser for PPTX (ZIP + OOXML) packages.
pub struct PptxParser;

const MAX_XML_PART_BYTES: u64 = 16 * 1024 * 1024;

fn is_xml_part(name: &str) -> bool {
    name == "[Content_Types].xml" || name.ends_with(".xml") || name.ends_with(".rels")
}

impl PptxParser {
    /// Parse PPTX from file path.
    pub fn parse_file(path: &Path) -> PptxResult<Presentation> {
        let data = std::fs::read(path)?;
        Self::parse_bytes(&data)
    }

    /// Parse PPTX from in-memory byte data.
    pub fn parse_bytes(data: &[u8]) -> PptxResult<Presentation> {
        Self::parse_bytes_with_metadata(data).map(|(presentation, _, _, _)| presentation)
    }

    pub(crate) fn parse_bytes_with_metadata(
        data: &[u8],
    ) -> PptxResult<(
        Presentation,
        NotesCommentsInventory,
        Vec<crate::model::timing::ParsedTimingInventory>,
        Vec<PresentationExtensionMetadata>,
    )> {
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor)?;
        Self::validate_package_xml_safety(&mut archive)?;

        let content_types = Self::read_entry(&mut archive, "[Content_Types].xml")
            .map(|xml| picture_bullet_parser::ContentTypes::parse(&xml))
            .unwrap_or_default();

        // Detect password-protected PPTX (EncryptedPackage OLE stream)
        let is_encrypted = (0..archive.len()).any(|i| {
            archive
                .by_index(i)
                .map(|f| f.name() == "EncryptedPackage" || f.name() == "EncryptionInfo")
                .unwrap_or(false)
        });
        if is_encrypted {
            return Err(PptxError::UnsupportedFormat(
                "password-protected PPTX".to_string(),
            ));
        }

        let mut presentation = Presentation::default();
        let mut notes_comments = NotesCommentsInventory::default();

        // 1. Parse slide size, slide rel IDs, and default text style from presentation.xml
        let pres_xml = Self::read_entry(&mut archive, "ppt/presentation.xml").map_err(|_| {
            PptxError::MissingFile("ppt/presentation.xml — not a valid PPTX".to_string())
        })?;
        let (slide_size, slide_refs, default_text_style) = Self::parse_presentation_xml(&pres_xml)?;
        let presentation_extensions = presentation_extension_parser::parse(&pres_xml);
        presentation.slide_size = slide_size;
        presentation.default_text_style = default_text_style;

        if let Ok(core_xml) = Self::read_entry(&mut archive, "docProps/core.xml") {
            presentation.title = Self::parse_core_title(&core_xml);
        }

        // 2. Parse presentation.xml.rels (all relationships)
        let rels_xml = Self::read_entry(&mut archive, "ppt/_rels/presentation.xml.rels")?;
        let pres_relationships_exact =
            notes_comments_parser::relationship_document_is_exact(&rels_xml);
        let pres_relationships = relationships::parse_relationship_records(&rels_xml)?;
        let pres_rels = relationships::target_map(&pres_relationships);
        let comment_authors = notes_comments_parser::collect_authors(
            &mut archive,
            &pres_relationships,
            pres_relationships_exact,
            &mut notes_comments,
        );
        notes_comments_parser::collect_handout_masters(
            &mut archive,
            &pres_relationships,
            pres_relationships_exact,
            &mut notes_comments,
        );

        let table_styles = match table_style_package_diagnostics::select(&pres_relationships).target
        {
            Some(path) => match Self::read_entry(&mut archive, &path) {
                Ok(xml) => match table_style_parser::parse_table_styles(&xml) {
                    Ok(styles) => styles,
                    Err(_) => {
                        warn!("Table styles part was rejected");
                        Vec::new()
                    }
                },
                Err(_) => {
                    warn!("Table styles part is unavailable");
                    Vec::new()
                }
            },
            None => Vec::new(),
        };

        // 3. Parse themes
        let theme_paths = collect_targets_by_type(&pres_relationships, "theme");
        for theme_target in &theme_paths {
            let theme_full = normalize_ppt_path(theme_target);
            if let Ok(theme_xml) = Self::read_entry(&mut archive, &theme_full) {
                presentation
                    .themes
                    .push(theme_parser::parse_theme(&theme_xml)?);
            }
        }

        // 4. Parse slide masters
        let master_targets = collect_targets_by_type(&pres_relationships, "slideMaster");
        // Map: master target path -> index in presentation.masters
        let mut master_path_to_idx: HashMap<String, usize> = HashMap::new();

        for master_target in &master_targets {
            let master_full = normalize_ppt_path(master_target);
            let master_xml = match Self::read_entry(&mut archive, &master_full) {
                Ok(xml) => xml,
                Err(_) => continue,
            };

            let master_rels_path = Self::rels_path_for(&master_full);
            let (master_relationships, master_rels) =
                if let Ok(rels_xml) = Self::read_entry(&mut archive, &master_rels_path) {
                    let records = relationships::parse_relationship_records(&rels_xml)?;
                    let targets = relationships::target_map(&records);
                    (records, targets)
                } else {
                    (Vec::new(), HashMap::new())
                };

            let mut master =
                master_parser::parse_slide_master(&master_xml, &master_rels, &mut archive)?;

            // Find which theme this master references
            let theme_ref = find_target_by_type(&master_relationships, "theme");
            if let Some(theme_target) = theme_ref {
                let theme_full_path = resolve_relative_path(&master_full, &theme_target);
                master.theme_idx = theme_paths
                    .iter()
                    .position(|tp| normalize_ppt_path(tp) == theme_full_path)
                    .unwrap_or(0);
            }

            let idx = presentation.masters.len();
            let canonical = canonical_part_name(master_target);
            master_path_to_idx.insert(canonical, idx);
            presentation.masters.push(master);
        }

        // Backward compat: copy ClrMap from first master into presentation
        if let Some(first_master) = presentation.masters.first()
            && !first_master.clr_map.is_empty()
        {
            presentation.clr_map = first_master.clr_map.clone();
        }

        // 5. Parse slide layouts
        // Layouts are referenced from master .rels files
        let mut layout_path_to_idx: HashMap<String, usize> = HashMap::new();

        for master_target in &master_targets {
            let master_full = normalize_ppt_path(master_target);
            let master_rels_path = Self::rels_path_for(&master_full);
            let master_relationships =
                if let Ok(rels_xml) = Self::read_entry(&mut archive, &master_rels_path) {
                    relationships::parse_relationship_records(&rels_xml)?
                } else {
                    continue;
                };

            let master_canonical = canonical_part_name(master_target);
            let master_idx = master_path_to_idx
                .get(&master_canonical)
                .copied()
                .unwrap_or(0);

            let layout_targets = collect_targets_by_type(&master_relationships, "slideLayout");
            for layout_target in &layout_targets {
                let layout_full = resolve_relative_path(&master_full, layout_target);
                let layout_canonical = canonical_part_name(&layout_full.replace("ppt/", ""));

                if layout_path_to_idx.contains_key(&layout_canonical) {
                    continue;
                }

                let layout_xml = match Self::read_entry(&mut archive, &layout_full) {
                    Ok(xml) => xml,
                    Err(_) => continue,
                };

                let layout_rels_path = Self::rels_path_for(&layout_full);
                let layout_rels =
                    if let Ok(rels_xml) = Self::read_entry(&mut archive, &layout_rels_path) {
                        relationships::parse_relationships(&rels_xml)?
                    } else {
                        HashMap::new()
                    };

                let mut layout =
                    layout_parser::parse_slide_layout(&layout_xml, &layout_rels, &mut archive)?;
                layout.master_idx = master_idx;

                let idx = presentation.layouts.len();
                layout_path_to_idx.insert(layout_canonical, idx);
                presentation.layouts.push(layout);
            }
        }

        // 6. Parse slides
        let total_slides = slide_refs.len();
        let slide_order: HashMap<String, usize> = slide_refs
            .iter()
            .enumerate()
            .filter_map(|(index, slide_ref)| {
                pres_rels
                    .get(&slide_ref.rel_id)
                    .map(|target| (normalize_ppt_path(target), index + 1))
            })
            .collect();
        info!("Parsing {total_slides} slide(s)");
        let mut slide_timings = Vec::new();
        for (slide_num, slide_ref) in slide_refs.iter().enumerate() {
            info!("Parsing slide {} of {total_slides}", slide_num + 1);
            if let Some(slide_path) = pres_rels.get(&slide_ref.rel_id) {
                let full_path = normalize_ppt_path(slide_path);
                if let Ok(slide_xml) = Self::read_entry(&mut archive, &full_path) {
                    let slide_rels_path = Self::rels_path_for(&full_path);
                    let (slide_relationships, slide_relationships_exact) =
                        if let Ok(rels_xml) = Self::read_entry(&mut archive, &slide_rels_path) {
                            let exact =
                                notes_comments_parser::relationship_document_is_exact(&rels_xml);
                            (relationships::parse_relationship_records(&rels_xml)?, exact)
                        } else {
                            (Vec::new(), true)
                        };

                    notes_comments_parser::collect_slide(
                        &mut archive,
                        slide_num + 1,
                        &full_path,
                        &slide_relationships,
                        slide_relationships_exact,
                        &comment_authors,
                        &mut notes_comments,
                    );

                    let (selected_slide_xml, _) =
                        embedded_parser::select_alternate_content(&slide_xml, &full_path)?;
                    let mut slide = slide_parser::parse_slide_with_actions(
                        &selected_slide_xml,
                        &slide_relationships,
                        &full_path,
                        &slide_order,
                        &mut archive,
                    )?;
                    embedded_parser::resolve_slide(
                        &mut slide,
                        &slide_relationships,
                        &full_path,
                        &mut archive,
                        &mut presentation.embedded_inventory,
                    );
                    media_parser::resolve_slide(
                        &mut slide,
                        &full_path,
                        &slide_relationships,
                        &content_types,
                        &mut archive,
                    );
                    picture_bullet_parser::resolve_slide(
                        &mut slide,
                        &slide_relationships,
                        &content_types,
                        &mut archive,
                    );
                    resolve_table_style_references(&mut slide.shapes, &table_styles);

                    // Find which layout this slide references
                    let layout_ref = find_target_by_type(&slide_relationships, "slideLayout");
                    if let Some(layout_target) = layout_ref {
                        let layout_full = resolve_relative_path(&full_path, &layout_target);
                        let layout_canonical =
                            canonical_part_name(&layout_full.replace("ppt/", ""));
                        slide.layout_idx = layout_path_to_idx.get(&layout_canonical).copied();
                    }

                    let shape_count = slide.shapes.len();
                    if shape_count > 100 {
                        warn!(
                            "Slide {} has {shape_count} shapes — rendering may be slow",
                            slide_num + 1,
                        );
                    }
                    slide_timings.push(timing_parser::parse(&selected_slide_xml)?);
                    presentation.slides.push(slide);
                }
            }
        }

        // 7. Build presentation (already populated)
        Ok((
            presentation,
            notes_comments,
            slide_timings,
            presentation_extensions,
        ))
    }

    fn validate_package_xml_safety<R: Read + Seek>(archive: &mut ZipArchive<R>) -> PptxResult<()> {
        for index in 0..archive.len() {
            let mut part = archive.by_index(index)?;
            if part.is_dir() || !is_xml_part(part.name()) {
                continue;
            }
            if part.size() > MAX_XML_PART_BYTES {
                return Err(PptxError::UnsupportedFormat(format!(
                    "XML part exceeds 16 MiB: {}",
                    part.name()
                )));
            }
            let mut xml = Vec::with_capacity(part.size() as usize);
            part.read_to_end(&mut xml)?;
            if xml
                .windows(b"<!DOCTYPE".len())
                .any(|window| window == b"<!DOCTYPE")
            {
                return Err(PptxError::UnsupportedFormat(format!(
                    "XML document type declarations are forbidden: {}",
                    part.name()
                )));
            }
        }
        Ok(())
    }

    /// Read a ZIP entry
    fn read_entry<R: Read + std::io::Seek>(
        archive: &mut ZipArchive<R>,
        name: &str,
    ) -> PptxResult<String> {
        let mut file = archive
            .by_name(name)
            .map_err(|_| PptxError::MissingFile(name.to_string()))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }

    /// Build relationships file path: ppt/slides/slide1.xml -> ppt/slides/_rels/slide1.xml.rels
    fn rels_path_for(path: &str) -> String {
        if let Some((dir, file)) = path.rsplit_once('/') {
            format!("{dir}/_rels/{file}.rels")
        } else {
            format!("_rels/{path}.rels")
        }
    }

    /// Extract slide size, slide relationship IDs, and defaultTextStyle from presentation.xml
    fn parse_presentation_xml(xml: &str) -> PptxResult<(Size, Vec<SlideRef>, Option<ListStyle>)> {
        let mut reader = Reader::from_str(xml);
        let mut slide_size = Size::default();
        let mut slide_refs = Vec::new();

        // defaultTextStyle parsing state
        let mut in_default_text_style = false;
        let mut default_text_style = ListStyle::default();
        let mut has_default_text_style = false;
        let mut current_lvl: Option<usize> = None;
        let mut current_para_defaults: Option<crate::model::ParagraphDefaults> = None;
        let mut current_run_defaults: Option<crate::model::RunDefaults> = None;
        let mut in_def_rpr = false;
        let mut current_color: Option<crate::model::Color> = None;
        let mut in_ln_spc = false;
        let mut in_spc_bef = false;
        let mut in_spc_aft = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let name = e.name();
                    let local = xml_utils::local_name(name.as_ref());
                    match local {
                        "defaultTextStyle" => {
                            in_default_text_style = true;
                            has_default_text_style = true;
                        }
                        // Level paragraph properties inside defaultTextStyle
                        s if in_default_text_style && master_parser::is_lvl_ppr(s) => {
                            let lvl = master_parser::parse_lvl_index(s);
                            current_lvl = Some(lvl);
                            let mut pd = crate::model::ParagraphDefaults::default();
                            master_parser::parse_lvl_ppr_attrs(e, &mut pd);
                            current_para_defaults = Some(pd);
                        }
                        "defRPr" if in_default_text_style && current_lvl.is_some() => {
                            in_def_rpr = true;
                            let mut rd = crate::model::RunDefaults::default();
                            master_parser::parse_def_rpr_attrs(e, &mut rd);
                            current_run_defaults = Some(rd);
                        }
                        // Spacing containers inside defaultTextStyle lvlNpPr
                        "lnSpc"
                            if in_default_text_style && current_lvl.is_some() && !in_def_rpr =>
                        {
                            in_ln_spc = true;
                        }
                        "spcBef"
                            if in_default_text_style && current_lvl.is_some() && !in_def_rpr =>
                        {
                            in_spc_bef = true;
                        }
                        "spcAft"
                            if in_default_text_style && current_lvl.is_some() && !in_def_rpr =>
                        {
                            in_spc_aft = true;
                        }
                        // Color elements inside defRPr
                        "srgbClr" if in_def_rpr => {
                            if let Some(val) = xml_utils::attr_str(e, "val") {
                                current_color = Some(crate::model::Color::rgb(val));
                            }
                        }
                        "schemeClr" if in_def_rpr => {
                            if let Some(val) = xml_utils::attr_str(e, "val") {
                                current_color = Some(crate::model::Color::theme(val));
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let name = e.name();
                    let local = xml_utils::local_name(name.as_ref());
                    match local {
                        "sldSz" => {
                            for attr in e.attributes().flatten() {
                                let key = xml_utils::local_name(attr.key.as_ref());
                                let val = String::from_utf8_lossy(&attr.value);
                                match key {
                                    "cx" => slide_size.width = Emu::parse_emu(&val),
                                    "cy" => slide_size.height = Emu::parse_emu(&val),
                                    _ => {}
                                }
                            }
                        }
                        "sldId" => {
                            let mut rel_id: Option<String> = None;
                            for attr in e.attributes().flatten() {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                if key.ends_with("id") && key.contains(':') {
                                    let val = String::from_utf8_lossy(&attr.value);
                                    rel_id = Some(val.to_string());
                                }
                            }
                            if let Some(rel_id) = rel_id {
                                slide_refs.push(SlideRef { rel_id });
                            }
                        }
                        // Empty lvlNpPr inside defaultTextStyle
                        s if in_default_text_style && master_parser::is_lvl_ppr(s) => {
                            let lvl = master_parser::parse_lvl_index(s);
                            if lvl < 9 {
                                let mut pd = crate::model::ParagraphDefaults::default();
                                master_parser::parse_lvl_ppr_attrs(e, &mut pd);
                                default_text_style.levels[lvl] = Some(pd);
                            }
                        }
                        // Empty defRPr inside defaultTextStyle
                        "defRPr"
                            if in_default_text_style && current_lvl.is_some() && !in_def_rpr =>
                        {
                            let mut rd = crate::model::RunDefaults::default();
                            master_parser::parse_def_rpr_attrs(e, &mut rd);
                            if let Some(pd) = current_para_defaults.as_mut() {
                                pd.def_run_props = Some(rd);
                            }
                        }
                        // Spacing percentage/points inside defaultTextStyle lvlNpPr
                        "spcPct"
                            if in_default_text_style
                                && current_lvl.is_some()
                                && (in_ln_spc || in_spc_bef || in_spc_aft) =>
                        {
                            if let Some(val_str) = xml_utils::attr_str(e, "val")
                                && let Ok(val) = val_str.parse::<f64>()
                            {
                                let spacing = crate::model::SpacingValue::Percent(val / 100_000.0);
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
                            if in_default_text_style
                                && current_lvl.is_some()
                                && (in_ln_spc || in_spc_bef || in_spc_aft) =>
                        {
                            if let Some(val_str) = xml_utils::attr_str(e, "val")
                                && let Ok(val) = val_str.parse::<f64>()
                            {
                                let spacing = crate::model::SpacingValue::Points(val / 100.0);
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
                        // Font inside defRPr
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
                        // Color (Empty variant) inside defRPr
                        "srgbClr" if in_def_rpr => {
                            if let Some(val) = xml_utils::attr_str(e, "val")
                                && let Some(rd) = current_run_defaults.as_mut()
                            {
                                rd.color = Some(crate::model::Color::rgb(val));
                            }
                        }
                        "schemeClr" if in_def_rpr => {
                            if let Some(val) = xml_utils::attr_str(e, "val")
                                && let Some(rd) = current_run_defaults.as_mut()
                            {
                                rd.color = Some(crate::model::Color::theme(val));
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let qname = e.name();
                    let local = xml_utils::local_name(qname.as_ref());
                    match local {
                        "defaultTextStyle" => {
                            in_default_text_style = false;
                        }
                        "defRPr" if in_def_rpr => {
                            in_def_rpr = false;
                            // Assign color from Start+child pattern
                            if let (Some(color), Some(rd)) =
                                (current_color.take(), current_run_defaults.as_mut())
                                && rd.color.is_none()
                            {
                                rd.color = Some(color);
                            }
                            if let Some(pd) = current_para_defaults.as_mut() {
                                pd.def_run_props = current_run_defaults.take();
                            }
                        }
                        // End of spacing containers
                        "lnSpc" if in_default_text_style => {
                            in_ln_spc = false;
                        }
                        "spcBef" if in_default_text_style => {
                            in_spc_bef = false;
                        }
                        "spcAft" if in_default_text_style => {
                            in_spc_aft = false;
                        }
                        s if in_default_text_style
                            && master_parser::is_lvl_ppr(s)
                            && current_lvl.is_some() =>
                        {
                            if let (Some(pd), Some(lvl)) =
                                (current_para_defaults.take(), current_lvl)
                                && lvl < 9
                            {
                                default_text_style.levels[lvl] = Some(pd);
                            }
                            current_lvl = None;
                        }
                        "srgbClr" | "schemeClr" if in_def_rpr => {
                            if let Some(color) = current_color.take()
                                && let Some(rd) = current_run_defaults.as_mut()
                            {
                                rd.color = Some(color);
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

        let dts = if has_default_text_style {
            Some(default_text_style)
        } else {
            None
        };

        Ok((slide_size, slide_refs, dts))
    }

    fn parse_core_title(xml: &str) -> Option<String> {
        let mut reader = Reader::from_str(xml);
        let mut in_title = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) if xml_utils::local_name(e.name().as_ref()) == "title" => {
                    in_title = true;
                }
                Ok(Event::Text(ref e)) if in_title => {
                    let text = match e.unescape() {
                        Ok(text) => text.into_owned(),
                        Err(_) => return None,
                    };
                    if text.is_empty() {
                        return None;
                    }
                    return Some(text);
                }
                Ok(Event::End(ref e)) if xml_utils::local_name(e.name().as_ref()) == "title" => {
                    in_title = false;
                }
                Ok(Event::Eof) => return None,
                Err(_) => return None,
                _ => {}
            }
        }
    }
}

fn resolve_table_style_references(shapes: &mut [Shape], styles: &[TableStyle]) {
    for shape in shapes {
        match &mut shape.shape_type {
            ShapeType::Table(table) => {
                if let Some(reference) = table.style.as_mut() {
                    let mut matching = styles.iter().filter(|style| style.id == reference.id);
                    if let Some(definition) = matching.next() {
                        reference.source_kind = TableStyleSourceKind::Package;
                        reference.definition = Some(Box::new(definition.clone()));
                        if matching.next().is_some() {
                            reference
                                .issues
                                .push(crate::model::TableStyleIssue::DuplicateId);
                        }
                    } else if table_style_builtins::contains(&reference.id) {
                        reference.source_kind = TableStyleSourceKind::BuiltIn;
                    } else {
                        reference.source_kind = TableStyleSourceKind::Invalid;
                    }
                }
            }
            ShapeType::Group(children, _) => resolve_table_style_references(children, styles),
            ShapeType::Rectangle
            | ShapeType::RoundedRectangle
            | ShapeType::Ellipse
            | ShapeType::Triangle
            | ShapeType::Arrow
            | ShapeType::Line
            | ShapeType::TextBox
            | ShapeType::Picture(_)
            | ShapeType::Chart(_)
            | ShapeType::Custom(_)
            | ShapeType::CustomGeom(_)
            | ShapeType::Unsupported(_) => {}
        }
    }
}

// -- Helper functions --

/// Collect all relationship targets whose Type URL contains the given substring
fn collect_targets_by_type(
    relationships: &[relationships::Relationship],
    type_fragment: &str,
) -> Vec<String> {
    let mut targets = relationships
        .iter()
        .filter(|relationship| {
            relationship.relationship_type.rsplit('/').next() == Some(type_fragment)
        })
        .map(|relationship| relationship.target.clone())
        .collect::<Vec<_>>();
    targets.sort();
    targets
}

/// Find single target by type fragment
fn find_target_by_type(
    relationships: &[relationships::Relationship],
    type_fragment: &str,
) -> Option<String> {
    let results = collect_targets_by_type(relationships, type_fragment);
    results.into_iter().next()
}

/// Normalize a target path relative to ppt/ directory
fn normalize_ppt_path(target: &str) -> String {
    if target.starts_with("ppt/") {
        target.to_string()
    } else {
        format!("ppt/{target}")
    }
}

/// Resolve a relative path from a base path
/// e.g. base="ppt/slideMasters/slideMaster1.xml", rel="../slideLayouts/slideLayout1.xml"
///   -> "ppt/slideLayouts/slideLayout1.xml"
fn resolve_relative_path(base: &str, rel: &str) -> String {
    if !rel.contains("..") {
        // Simple case: relative to same directory or already absolute-ish
        if rel.starts_with("ppt/") {
            return rel.to_string();
        }
        if let Some((dir, _)) = base.rsplit_once('/') {
            return format!("{dir}/{rel}");
        }
        return rel.to_string();
    }

    // Handle ../ navigation
    let base_parts: Vec<&str> = base.split('/').collect();
    // Remove filename from base
    let mut base_dir: Vec<&str> = base_parts[..base_parts.len() - 1].to_vec();

    for segment in rel.split('/') {
        if segment == ".." {
            base_dir.pop();
        } else {
            base_dir.push(segment);
        }
    }

    base_dir.join("/")
}

/// Extract canonical part name from a target path (just the filename without path)
fn canonical_part_name(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| path.to_string())
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use tempfile::tempdir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    #[test]
    fn relationship_target_and_path_helpers_cover_edge_cases() {
        let rels_xml = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
            <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
            <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="slideLayouts/slideLayout1.xml"/>
            <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
            <Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
        </Relationships>"#;
        let rels = relationships::parse_relationship_records(rels_xml)
            .expect("relationship records should parse");

        assert_eq!(
            collect_targets_by_type(&rels, "slide"),
            vec![
                "slides/slide1.xml".to_string(),
                "slides/slide2.xml".to_string()
            ]
        );
        assert_eq!(
            collect_targets_by_type(&rels, "slideLayout"),
            vec!["slideLayouts/slideLayout1.xml".to_string()]
        );
        assert_eq!(
            find_target_by_type(&rels, "theme"),
            Some("theme/theme1.xml".to_string())
        );
        assert_eq!(find_target_by_type(&rels, "missing"), None);

        assert_eq!(
            PptxParser::rels_path_for("ppt/slides/slide1.xml"),
            "ppt/slides/_rels/slide1.xml.rels"
        );
        assert_eq!(
            PptxParser::rels_path_for("slide1.xml"),
            "_rels/slide1.xml.rels"
        );
        assert_eq!(
            normalize_ppt_path("slides/slide1.xml"),
            "ppt/slides/slide1.xml"
        );
        assert_eq!(
            normalize_ppt_path("ppt/slides/slide1.xml"),
            "ppt/slides/slide1.xml"
        );
        assert_eq!(
            resolve_relative_path(
                "ppt/slideMasters/slideMaster1.xml",
                "../slideLayouts/slideLayout1.xml"
            ),
            "ppt/slideLayouts/slideLayout1.xml"
        );
        assert_eq!(
            resolve_relative_path("ppt/slides/slide1.xml", "media/image1.png"),
            "ppt/slides/media/image1.png"
        );
        assert_eq!(canonical_part_name("ppt/slides/Slide1.XML"), "slide1.xml");
        assert_eq!(canonical_part_name("slide1.xml"), "slide1.xml");
    }

    #[test]
    fn parse_core_title_handles_present_empty_and_invalid_titles() {
        assert_eq!(
            PptxParser::parse_core_title(
                r#"<cp:coreProperties xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties">
                    <dc:title>Quarterly Review</dc:title>
                </cp:coreProperties>"#
            ),
            Some("Quarterly Review".to_string())
        );
        assert_eq!(
            PptxParser::parse_core_title(
                r#"<cp:coreProperties xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties">
                    <dc:title></dc:title>
                </cp:coreProperties>"#
            ),
            None
        );
        assert_eq!(
            PptxParser::parse_core_title(
                r#"<cp:coreProperties xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties">
                    <dc:title>&invalid;</dc:title>
                </cp:coreProperties>"#
            ),
            None
        );
    }

    #[test]
    fn parse_presentation_xml_reads_slide_refs_and_default_text_style() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId1"/>
    <p:sldId id="257" r:id="rId2"/>
  </p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:defaultTextStyle>
    <a:lvl1pPr algn="ctr" marL="457200" indent="-228600">
      <a:lnSpc><a:spcPct val="90000"/></a:lnSpc>
      <a:spcBef><a:spcPts val="1200"/></a:spcBef>
      <a:defRPr sz="2400" spc="200" baseline="30000" cap="all" u="dbl" strike="sngStrike" b="1" i="1">
        <a:latin typeface="Aptos"/>
        <a:ea typeface="Yu Gothic"/>
        <a:cs typeface="Noto Sans Devanagari"/>
        <a:schemeClr val="accent2"/>
      </a:defRPr>
    </a:lvl1pPr>
    <a:lvl2pPr algn="r"/>
  </p:defaultTextStyle>
</p:presentation>"#;

        let (slide_size, slide_refs, default_text_style) =
            PptxParser::parse_presentation_xml(xml).expect("presentation xml should parse");

        assert_eq!(slide_size.width.to_px(), Emu::parse_emu("9144000").to_px());
        assert_eq!(slide_size.height.to_px(), Emu::parse_emu("6858000").to_px());
        assert_eq!(slide_refs.len(), 2);
        assert_eq!(slide_refs[0].rel_id, "rId1");
        assert_eq!(slide_refs[1].rel_id, "rId2");

        let style = default_text_style.expect("default text style should exist");
        let lvl1 = style.levels[0].as_ref().expect("level 1 defaults");
        assert_eq!(
            lvl1.alignment.as_ref().map(|alignment| alignment.to_css()),
            Some("center")
        );
        assert_eq!(lvl1.margin_left, Some(36.0));
        assert_eq!(lvl1.indent, Some(-18.0));
        assert!(matches!(
            lvl1.line_spacing,
            Some(crate::model::SpacingValue::Percent(v)) if (v - 0.9).abs() < 1e-6
        ));
        assert!(matches!(
            lvl1.space_before,
            Some(crate::model::SpacingValue::Points(v)) if (v - 12.0).abs() < 1e-6
        ));
        let run = lvl1.def_run_props.as_ref().expect("default run properties");
        assert_eq!(run.font_size, Some(24.0));
        assert_eq!(run.letter_spacing, Some(2.0));
        assert_eq!(run.baseline, Some(30000));
        assert_eq!(run.bold, Some(true));
        assert_eq!(run.italic, Some(true));
        assert_eq!(run.font_latin.as_deref(), Some("Aptos"));
        assert_eq!(run.font_ea.as_deref(), Some("Yu Gothic"));
        assert_eq!(run.font_cs.as_deref(), Some("Noto Sans Devanagari"));
        assert_eq!(
            run.color.as_ref().and_then(|c| c.to_css()).as_deref(),
            Some("#ED7D31")
        );
        assert_eq!(
            style.levels[1]
                .as_ref()
                .and_then(|lvl| lvl.alignment.as_ref())
                .map(|alignment| alignment.to_css()),
            Some("right")
        );
    }

    #[test]
    fn parse_bytes_detects_encrypted_packages() {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        zip.start_file("EncryptedPackage", options).unwrap();
        zip.write_all(b"secret").unwrap();
        let bytes = zip.finish().unwrap().into_inner();

        let err = PptxParser::parse_bytes(&bytes).expect_err("encrypted pptx should fail");
        assert!(
            matches!(err, PptxError::UnsupportedFormat(msg) if msg == "password-protected PPTX")
        );
    }

    #[test]
    fn parse_bytes_rejects_doctype_in_unreferenced_xml_part() {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("security/entity.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"<!DOCTYPE root [<!ENTITY x "expanded">]><root>&x;</root>"#)
            .unwrap();
        let bytes = zip.finish().unwrap().into_inner();

        let err = PptxParser::parse_bytes(&bytes).expect_err("DOCTYPE should fail");
        assert!(
            matches!(
                err,
                PptxError::UnsupportedFormat(ref msg)
                    if msg == "XML document type declarations are forbidden: security/entity.xml"
            ),
            "{err}"
        );
    }

    #[test]
    fn parse_bytes_rejects_doctype_hidden_after_malformed_xml() {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("security/malformed.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"<root attr="unterminated<!DOCTYPE root>"#)
            .unwrap();
        let bytes = zip.finish().unwrap().into_inner();

        let err = PptxParser::parse_bytes(&bytes).expect_err("hidden DOCTYPE should fail");
        assert!(
            matches!(
                err,
                PptxError::UnsupportedFormat(ref msg)
                    if msg == "XML document type declarations are forbidden: security/malformed.xml"
            ),
            "{err}"
        );
    }

    #[test]
    fn parse_bytes_rejects_oversized_unreferenced_xml_part() {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("security/oversized.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&vec![b' '; MAX_XML_PART_BYTES as usize + 1])
            .unwrap();
        let bytes = zip.finish().unwrap().into_inner();

        let err = PptxParser::parse_bytes(&bytes).expect_err("oversized XML should fail");
        assert!(
            matches!(
                err,
                PptxError::UnsupportedFormat(ref msg)
                    if msg == "XML part exceeds 16 MiB: security/oversized.xml"
            ),
            "{err}"
        );
    }

    #[test]
    fn parse_file_reads_minimal_fixture_from_disk() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("minimal.pptx");
        std::fs::write(&path, build_minimal_pptx()).expect("write pptx");

        let presentation = PptxParser::parse_file(&path).expect("parse_file should succeed");
        assert_eq!(presentation.slides.len(), 1);
        assert_eq!(
            presentation.slide_size.width.to_px(),
            Emu::parse_emu("9144000").to_px()
        );
    }

    #[test]
    fn parse_bytes_reports_missing_presentation_xml() {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#,
        )
        .unwrap();

        let bytes = zip.finish().unwrap().into_inner();
        let err =
            PptxParser::parse_bytes(&bytes).expect_err("missing presentation.xml should fail");
        assert!(matches!(
            err,
            PptxError::MissingFile(msg) if msg.contains("ppt/presentation.xml")
        ));
    }

    #[test]
    fn parse_bytes_tolerates_missing_theme_master_and_layout_parts() {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
</Types>"#,
        )
        .unwrap();

        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
        )
        .unwrap();

        zip.start_file("ppt/presentation.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldIdLst><p:sldId id="256" r:id="rIdSlide"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
        )
        .unwrap();

        zip.start_file("ppt/_rels/presentation.xml.rels", options)
            .unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdSlide" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/missing-theme.xml"/>
  <Relationship Id="rIdMaster" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/missingMaster.xml"/>
</Relationships>"#,
        )
        .unwrap();

        zip.start_file("ppt/slides/slide1.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sld>"#,
        )
        .unwrap();

        let bytes = zip.finish().unwrap().into_inner();
        let presentation =
            PptxParser::parse_bytes(&bytes).expect("parser should tolerate missing optional parts");
        assert_eq!(presentation.slides.len(), 1);
        assert!(presentation.themes.is_empty());
        assert!(presentation.masters.is_empty());
        assert!(presentation.layouts.is_empty());
    }

    #[test]
    fn parse_bytes_reports_missing_presentation_relationships() {
        let bytes = build_zip(&[
            (
                "[Content_Types].xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
</Types>"#,
            ),
            (
                "_rels/.rels",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldIdLst><p:sldId id="256" r:id="rIdSlide"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sld>"#,
            ),
        ]);

        let err = PptxParser::parse_bytes(&bytes)
            .expect_err("missing presentation relationships should fail");
        assert!(matches!(
            err,
            PptxError::MissingFile(path) if path == "ppt/_rels/presentation.xml.rels"
        ));
    }

    #[test]
    fn parse_bytes_reports_invalid_relationship_and_theme_xml() {
        let invalid_rels = build_zip(&[
            (
                "[Content_Types].xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
</Types>"#,
            ),
            (
                "_rels/.rels",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldIdLst><p:sldId id="256" r:id="rIdSlide"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
            ),
            ("ppt/_rels/presentation.xml.rels", b"<Relationships"),
            (
                "ppt/slides/slide1.xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sld>"#,
            ),
        ]);
        assert!(matches!(
            PptxParser::parse_bytes(&invalid_rels),
            Err(PptxError::Xml(_))
        ));

        let invalid_theme = build_zip(&[
            (
                "[Content_Types].xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#,
            ),
            (
                "_rels/.rels",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldIdLst><p:sldId id="256" r:id="rIdSlide"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdSlide" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sld>"#,
            ),
            ("ppt/theme/theme1.xml", b"<a:theme"),
        ]);
        assert!(matches!(
            PptxParser::parse_bytes(&invalid_theme),
            Err(PptxError::Xml(_))
        ));
    }

    #[test]
    fn parse_bytes_reports_invalid_master_and_slide_relationship_xml() {
        let invalid_master_rels = build_zip(&[
            (
                "[Content_Types].xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
</Types>"#,
            ),
            (
                "_rels/.rels",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rIdMaster"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rIdSlide"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdMaster" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rIdSlide" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#,
            ),
            (
                "ppt/slideMasters/slideMaster1.xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sldMaster>"#,
            ),
            ("ppt/slideMasters/_rels/slideMaster1.xml.rels", b"<Relationships"),
            (
                "ppt/slides/slide1.xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sld>"#,
            ),
        ]);
        assert!(matches!(
            PptxParser::parse_bytes(&invalid_master_rels),
            Err(PptxError::Xml(_))
        ));

        let invalid_slide_rels = build_zip(&[
            (
                "[Content_Types].xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
</Types>"#,
            ),
            (
                "_rels/.rels",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
            ),
            (
                "ppt/presentation.xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldIdLst><p:sldId id="256" r:id="rIdSlide"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
            ),
            (
                "ppt/_rels/presentation.xml.rels",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdSlide" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sld>"#,
            ),
            ("ppt/slides/_rels/slide1.xml.rels", b"<Relationships"),
        ]);
        assert!(matches!(
            PptxParser::parse_bytes(&invalid_slide_rels),
            Err(PptxError::Xml(_))
        ));
    }

    fn build_minimal_pptx() -> Vec<u8> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#,
        )
        .unwrap();

        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
        )
        .unwrap();

        zip.start_file("ppt/presentation.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst/>
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId1"/>
  </p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
        )
        .unwrap();

        zip.start_file("ppt/_rels/presentation.xml.rels", options)
            .unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#,
        )
        .unwrap();

        zip.start_file("ppt/slides/slide1.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sld>"#,
        )
        .unwrap();

        zip.start_file("ppt/slides/_rels/slide1.xml.rels", options)
            .unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#,
        )
        .unwrap();

        zip.start_file("ppt/theme/theme1.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="TestTheme">
  <a:themeElements>
    <a:clrScheme name="TestColors">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="1F1F1F"/></a:dk2>
      <a:lt2><a:srgbClr val="F7F7F7"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="TestFonts">
      <a:majorFont><a:latin typeface="Calibri"/></a:majorFont>
      <a:minorFont><a:latin typeface="Calibri"/></a:minorFont>
    </a:fontScheme>
  </a:themeElements>
</a:theme>"#,
        )
        .unwrap();

        zip.finish().unwrap().into_inner()
    }

    fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        for (path, bytes) in entries {
            zip.start_file(path, options).expect("start file");
            zip.write_all(bytes).expect("write file");
        }
        zip.finish().expect("finish zip").into_inner()
    }
}
