use std::collections::HashMap;
use std::io::{Read, Seek};

use quick_xml::events::BytesStart;
use zip::ZipArchive;

use super::chart_parser;
use super::preserved_parser::PreservedSaxState;
use super::preserved_parser::classify_unsupported_graphic;
use super::relationships;
use super::slide_parser::ShapeBuilder;
use super::table_parser::TableBuilder;
use super::xml_utils;
use crate::error::{PptxError, PptxResult};
use crate::model::{Shape, ShapeType};

#[derive(Default)]
pub(crate) struct GraphicFrameSaxState {
    in_frame: bool,
    in_data: bool,
    is_chart: bool,
}

pub(crate) enum GraphicFrameEnd {
    None,
    FinishFrame,
}

impl GraphicFrameSaxState {
    pub(crate) fn in_frame(&self) -> bool {
        self.in_frame
    }

    pub(crate) fn handle_start(
        &mut self,
        local: &str,
        element: &BytesStart<'_>,
        shape: &mut Option<ShapeBuilder>,
        preserved: &mut PreservedSaxState,
    ) -> bool {
        match local {
            "graphicFrame" => {
                self.in_frame = true;
                start_frame(shape);
                true
            }
            "graphicData" if self.in_frame => {
                self.in_data = true;
                if let Some(uri) = xml_utils::attr_str(element, "uri") {
                    self.is_chart = uri.contains("chart");
                    if classify_data(&uri, shape) && !self.is_chart {
                        preserved.start_capture();
                    }
                }
                true
            }
            "chart" if self.in_data && self.is_chart => {
                assign_chart_relationship(element, shape);
                true
            }
            _ => false,
        }
    }

    pub(crate) fn handle_end(
        &mut self,
        local: &str,
        shape: &mut Option<ShapeBuilder>,
        preserved: &mut PreservedSaxState,
    ) -> GraphicFrameEnd {
        match local {
            "graphicData" => {
                self.in_data = false;
                preserved.finish_capture(shape);
                GraphicFrameEnd::None
            }
            "graphicFrame" => {
                self.in_frame = false;
                self.in_data = false;
                GraphicFrameEnd::FinishFrame
            }
            _ => GraphicFrameEnd::None,
        }
    }

    pub(crate) fn take_chart_flag(&mut self) -> bool {
        std::mem::take(&mut self.is_chart)
    }
}

pub(crate) fn start_frame(shape: &mut Option<ShapeBuilder>) {
    *shape = Some(ShapeBuilder::default());
}

pub(crate) fn classify_data(uri: &str, shape: &mut Option<ShapeBuilder>) -> bool {
    let Some(shape) = shape.as_mut() else {
        return false;
    };
    if uri.contains("chart") {
        shape.is_chart = true;
        return true;
    }
    if let Some(unsupported) = classify_unsupported_graphic(uri) {
        shape.unsupported_content = Some(unsupported.label.to_owned());
        shape.unresolved_type = Some(unsupported.element_type);
        return true;
    }
    false
}

pub(crate) fn assign_chart_relationship(
    element: &BytesStart<'_>,
    shape: &mut Option<ShapeBuilder>,
) {
    if let Some(shape) = shape.as_mut()
        && let Some(rel_id) = xml_utils::attr_str(element, "id")
    {
        shape.chart_rel_id = Some(rel_id);
    }
}

pub(crate) fn finish_frame<R: Read + Seek>(
    is_chart: bool,
    shape: &mut Option<ShapeBuilder>,
    table: &mut Option<TableBuilder>,
    rels: &HashMap<String, String>,
    archive: &mut ZipArchive<R>,
) -> Option<Shape> {
    if is_chart {
        let mut shape = shape.take()?;
        load_chart(&mut shape, rels, archive);
        return Some(shape.build());
    }
    if shape
        .as_ref()
        .is_some_and(|shape| shape.unsupported_content.is_some())
    {
        return shape.take().map(ShapeBuilder::build);
    }
    let mut shape = shape.take()?.build();
    let table = table.take()?;
    shape.shape_type = ShapeType::Table(table.build());
    Some(shape)
}

fn load_chart<R: Read + Seek>(
    shape: &mut ShapeBuilder,
    rels: &HashMap<String, String>,
    archive: &mut ZipArchive<R>,
) {
    let Some(target) = shape
        .chart_rel_id
        .as_ref()
        .and_then(|rel_id| rels.get(rel_id))
    else {
        return;
    };
    let path = resolve_relationship_path("ppt/slides", target);
    let Ok(chart_xml) = read_archive_entry(archive, &path) else {
        return;
    };
    shape.chart_direct_spec = chart_parser::parse_chart(&chart_xml).ok().flatten();

    let chart_rels_path = relationships_path(&path);
    let Ok(chart_rels_xml) = read_archive_entry(archive, &chart_rels_path) else {
        return;
    };
    let Ok(chart_rels) = relationships::parse_relationships(&chart_rels_xml) else {
        return;
    };
    for preview_target in chart_rels.values() {
        let preview_path = resolve_relative_file_path(&path, preview_target);
        let extension = preview_path.rsplit('.').next().unwrap_or("").to_lowercase();
        if !matches!(
            extension.as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tif" | "tiff" | "svg" | "emf" | "wmf"
        ) {
            continue;
        }
        if let Ok(bytes) = read_archive_bytes(archive, &preview_path)
            && !bytes.is_empty()
        {
            shape.chart_preview_mime = Some(mime_from_extension(&preview_path));
            shape.chart_preview_image = Some(bytes);
            break;
        }
    }
}

pub(crate) fn read_archive_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> PptxResult<String> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| PptxError::MissingFile(name.to_owned()))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub(crate) fn read_archive_bytes<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> PptxResult<Vec<u8>> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| PptxError::MissingFile(name.to_owned()))?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(contents)
}

pub(crate) fn relationships_path(part_path: &str) -> String {
    let (directory, file) = part_path.rsplit_once('/').unwrap_or(("", part_path));
    if directory.is_empty() {
        format!("_rels/{file}.rels")
    } else {
        format!("{directory}/_rels/{file}.rels")
    }
}

pub(crate) fn resolve_relative_file_path(base_file: &str, target: &str) -> String {
    let base_directory = base_file
        .rsplit_once('/')
        .map(|(directory, _)| directory)
        .unwrap_or("");
    resolve_relationship_path(base_directory, target)
}

pub(crate) fn resolve_relationship_path(base_directory: &str, target: &str) -> String {
    if !target.contains("../") {
        return format!("{base_directory}/{target}");
    }
    let mut parts = base_directory.split('/').collect::<Vec<_>>();
    for segment in target.split('/') {
        match segment {
            ".." => {
                parts.pop();
            }
            "" | "." => {}
            _ => parts.push(segment),
        }
    }
    parts.join("/")
}

pub(crate) fn mime_from_extension(path: &str) -> String {
    let extension = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        "emf" => "image/x-emf",
        "wmf" => "image/x-wmf",
        _ => "image/png",
    }
    .to_owned()
}
