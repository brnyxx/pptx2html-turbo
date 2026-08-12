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

use super::picture_bullet_parser::ContentTypes;

const CLASSIC_CHART_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const CHARTEX_URI: &str = "http://schemas.microsoft.com/office/drawing/2014/chartex";
pub(crate) const CHART_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart";
pub(crate) const CHARTEX_RELATIONSHIP: &str =
    "http://schemas.microsoft.com/office/2014/relationships/chartEx";
pub(crate) const IMAGE_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
pub(crate) const MAX_CHART_PREVIEW_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElementNamespace {
    Presentation,
    Drawing,
    ClassicChart,
    ChartEx,
    Other,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ElementContext<'a> {
    pub(crate) namespace: ElementNamespace,
    pub(crate) parent: Option<(&'a str, ElementNamespace)>,
}

#[derive(Default)]
pub(crate) struct GraphicFrameSaxState {
    in_frame: bool,
    in_graphic: bool,
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
        context: ElementContext<'_>,
        element: &BytesStart<'_>,
        chart_relationship_id: Option<String>,
        shape: &mut Option<ShapeBuilder>,
        preserved: &mut PreservedSaxState,
    ) -> bool {
        match (context.namespace, local, context.parent) {
            (ElementNamespace::Presentation, "graphicFrame", _) if !self.in_frame => {
                self.in_frame = true;
                start_frame(shape);
                true
            }
            (
                ElementNamespace::Drawing,
                "graphic",
                Some(("graphicFrame", ElementNamespace::Presentation)),
            ) if self.in_frame => {
                self.in_graphic = true;
                true
            }
            (
                ElementNamespace::Drawing,
                "graphicData",
                Some(("graphic", ElementNamespace::Drawing)),
            ) if self.in_graphic => {
                self.in_data = true;
                if let Some(uri) = xml_utils::attr_str(element, "uri") {
                    self.is_chart = matches!(uri.as_str(), CLASSIC_CHART_URI | CHARTEX_URI);
                    if classify_data(&uri, shape) && !self.is_chart {
                        preserved.start_capture();
                    }
                }
                true
            }
            (
                ElementNamespace::ClassicChart | ElementNamespace::ChartEx,
                "chart",
                Some(("graphicData", ElementNamespace::Drawing)),
            ) if self.in_data && self.is_chart => {
                assign_chart_relationship(chart_relationship_id, shape);
                true
            }
            _ => false,
        }
    }

    pub(crate) fn handle_end(
        &mut self,
        local: &str,
        namespace: ElementNamespace,
        shape: &mut Option<ShapeBuilder>,
        preserved: &mut PreservedSaxState,
    ) -> GraphicFrameEnd {
        match (namespace, local) {
            (ElementNamespace::Drawing, "graphicData") if self.in_data => {
                self.in_data = false;
                preserved.finish_capture(shape);
                GraphicFrameEnd::None
            }
            (ElementNamespace::Drawing, "graphic") if self.in_graphic => {
                self.in_graphic = false;
                GraphicFrameEnd::None
            }
            (ElementNamespace::Presentation, "graphicFrame") if self.in_frame => {
                self.in_frame = false;
                self.in_graphic = false;
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
    if matches!(uri, CLASSIC_CHART_URI | CHARTEX_URI) {
        shape.is_chart = true;
        shape.chart_relationship_type = Some(
            if uri == CHARTEX_URI {
                CHARTEX_RELATIONSHIP
            } else {
                CHART_RELATIONSHIP
            }
            .to_owned(),
        );
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
    relationship_id: Option<String>,
    shape: &mut Option<ShapeBuilder>,
) {
    if let Some(shape) = shape.as_mut() {
        shape.chart_rel_id = relationship_id;
    }
}

pub(crate) fn finish_frame<R: Read + Seek>(
    is_chart: bool,
    shape: &mut Option<ShapeBuilder>,
    table: &mut Option<TableBuilder>,
    _rels: &HashMap<String, String>,
    relationship_records: &[relationships::Relationship],
    content_types: &ContentTypes,
    archive: &mut ZipArchive<R>,
) -> Option<Shape> {
    if is_chart {
        let mut shape = shape.take()?;
        load_chart(&mut shape, relationship_records, content_types, archive);
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
    relationship_records: &[relationships::Relationship],
    content_types: &ContentTypes,
    archive: &mut ZipArchive<R>,
) {
    let Some(rel_id) = shape.chart_rel_id.as_ref() else {
        return;
    };
    let Some(relationship) = relationship_records.iter().find(|item| item.id == *rel_id) else {
        return;
    };
    let expected_relationship_type = shape
        .chart_relationship_type
        .as_deref()
        .unwrap_or(CHART_RELATIONSHIP);
    if relationship.relationship_type != expected_relationship_type
        || !matches!(
            relationship.target_mode,
            relationships::TargetMode::Internal
        )
    {
        return;
    }
    let Some(path) = safe_relationship_path("ppt/slides", &relationship.target) else {
        return;
    };
    let Ok(chart_xml) = read_archive_entry(archive, &path) else {
        return;
    };
    if let Ok(outcome) = chart_parser::classify_and_parse(&chart_xml) {
        shape.chart_direct_spec = outcome.direct_spec;
    }
    let chart_rels_path = relationships_path(&path);
    let Ok(chart_rels_xml) = read_archive_entry(archive, &chart_rels_path) else {
        return;
    };
    let Ok(chart_rels) = relationships::parse_relationship_records(&chart_rels_xml) else {
        return;
    };
    if let Some(preview) = select_chart_preview(&path, chart_rels, content_types, archive) {
        shape.chart_preview_mime = Some(preview.mime);
        shape.chart_preview_image = Some(preview.bytes);
    }
}

pub(crate) struct PreviewSelection {
    pub(crate) relationship_id: String,
    pub(crate) relationship_type: String,
    pub(crate) target: String,
    pub(crate) part_name: String,
    pub(crate) mime: String,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) fn select_chart_preview<R: Read + Seek>(
    chart_path: &str,
    mut chart_rels: Vec<relationships::Relationship>,
    content_types: &ContentTypes,
    archive: &mut ZipArchive<R>,
) -> Option<PreviewSelection> {
    chart_rels.sort_by(|left, right| {
        left.target
            .cmp(&right.target)
            .then_with(|| left.id.cmp(&right.id))
    });
    for relationship in chart_rels {
        if !matches!(
            relationship.target_mode,
            relationships::TargetMode::Internal
        ) || relationship.relationship_type != IMAGE_RELATIONSHIP
        {
            continue;
        }
        let Some(part_name) = safe_relative_file_path(chart_path, &relationship.target) else {
            continue;
        };
        let Some(mime) = content_types.for_part(&part_name) else {
            continue;
        };
        if !matches!(
            mime,
            "image/png" | "image/jpeg" | "image/gif" | "image/webp"
        ) {
            continue;
        }
        if let Ok(bytes) = read_bounded_archive_bytes(archive, &part_name, MAX_CHART_PREVIEW_BYTES)
            && valid_preview_payload(mime, &bytes)
        {
            return Some(PreviewSelection {
                relationship_id: relationship.id,
                relationship_type: relationship.relationship_type,
                target: relationship.target,
                part_name,
                mime: mime.to_owned(),
                bytes,
            });
        }
    }
    None
}

fn valid_preview_payload(mime: &str, bytes: &[u8]) -> bool {
    match mime {
        "image/png" => valid_png(bytes),
        "image/jpeg" => valid_jpeg(bytes),
        "image/gif" => valid_gif(bytes),
        "image/webp" => valid_webp(bytes),
        _ => false,
    }
}

fn valid_png(bytes: &[u8]) -> bool {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return false;
    }
    let mut offset = 8_usize;
    let mut first = true;
    while offset.checked_add(12).is_some_and(|end| end <= bytes.len()) {
        let length = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        let Some(chunk_end) = offset
            .checked_add(12)
            .and_then(|base| base.checked_add(length))
        else {
            return false;
        };
        if chunk_end > bytes.len() {
            return false;
        }
        let kind = &bytes[offset + 4..offset + 8];
        if first {
            if kind != b"IHDR" || length != 13 {
                return false;
            }
            let width = u32::from_be_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
            ]);
            let height = u32::from_be_bytes([
                bytes[offset + 12],
                bytes[offset + 13],
                bytes[offset + 14],
                bytes[offset + 15],
            ]);
            if width == 0 || height == 0 {
                return false;
            }
            first = false;
        }
        if kind == b"IEND" {
            return length == 0 && chunk_end == bytes.len();
        }
        offset = chunk_end;
    }
    false
}

fn valid_jpeg(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || !bytes.starts_with(&[0xff, 0xd8]) || !bytes.ends_with(&[0xff, 0xd9]) {
        return false;
    }
    let mut offset = 2_usize;
    while offset + 4 <= bytes.len() - 2 {
        if bytes[offset] != 0xff {
            return false;
        }
        while offset < bytes.len() && bytes[offset] == 0xff {
            offset += 1;
        }
        if offset >= bytes.len() {
            return false;
        }
        let marker = bytes[offset];
        offset += 1;
        if marker == 0xd9 {
            break;
        }
        if matches!(marker, 0x01 | 0xd0..=0xd8) {
            continue;
        }
        if offset + 2 > bytes.len() {
            return false;
        }
        let length = usize::from(u16::from_be_bytes([bytes[offset], bytes[offset + 1]]));
        if length < 2 || offset + length > bytes.len() {
            return false;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            if length < 8 {
                return false;
            }
            let height = u16::from_be_bytes([bytes[offset + 3], bytes[offset + 4]]);
            let width = u16::from_be_bytes([bytes[offset + 5], bytes[offset + 6]]);
            return width > 0 && height > 0;
        }
        offset += length;
    }
    false
}

fn valid_gif(bytes: &[u8]) -> bool {
    bytes.len() >= 20
        && matches!(&bytes[..6], b"GIF87a" | b"GIF89a")
        && u16::from_le_bytes([bytes[6], bytes[7]]) > 0
        && u16::from_le_bytes([bytes[8], bytes[9]]) > 0
        && bytes[13..bytes.len() - 1].contains(&0x2c)
        && bytes.last() == Some(&0x3b)
}

fn valid_webp(bytes: &[u8]) -> bool {
    if bytes.len() < 30 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return false;
    }
    let declared = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    if declared.checked_add(8) != Some(bytes.len()) {
        return false;
    }
    let chunk_size = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
    let padded_size = chunk_size + (chunk_size & 1);
    if 20_usize.checked_add(padded_size) != Some(bytes.len()) {
        return false;
    }
    match &bytes[12..16] {
        b"VP8X" if chunk_size >= 10 => {
            let width = 1 + u32::from_le_bytes([bytes[24], bytes[25], bytes[26], 0]);
            let height = 1 + u32::from_le_bytes([bytes[27], bytes[28], bytes[29], 0]);
            width > 0 && height > 0
        }
        b"VP8L" if chunk_size >= 5 => bytes[20] == 0x2f,
        b"VP8 " if chunk_size >= 10 => {
            let width = u16::from_le_bytes([bytes[26], bytes[27]]) & 0x3fff;
            let height = u16::from_le_bytes([bytes[28], bytes[29]]) & 0x3fff;
            bytes[23..26] == [0x9d, 0x01, 0x2a] && width > 0 && height > 0
        }
        _ => false,
    }
}

pub(crate) fn safe_relative_file_path(base_file: &str, target: &str) -> Option<String> {
    let base_directory = base_file.rsplit_once('/').map(|(directory, _)| directory)?;
    safe_relationship_path(base_directory, target)
}

fn safe_relationship_path(base_directory: &str, target: &str) -> Option<String> {
    if target.is_empty()
        || target.starts_with('/')
        || target.contains('\\')
        || target.contains('%')
        || target.contains(':')
        || target.split('/').any(str::is_empty)
    {
        return None;
    }
    let mut parts = base_directory.split('/').collect::<Vec<_>>();
    for segment in target.split('/') {
        match segment {
            "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(segment),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
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

fn read_bounded_archive_bytes<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    max_bytes: u64,
) -> PptxResult<Vec<u8>> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| PptxError::MissingFile(name.to_owned()))?;
    if file.size() > max_bytes {
        return Err(PptxError::UnsupportedFormat(format!(
            "chart preview exceeds {max_bytes} bytes"
        )));
    }
    let mut contents = Vec::with_capacity(file.size() as usize);
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

#[cfg(test)]
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

#[cfg(test)]
mod preview_signature_tests {
    use super::valid_preview_payload;

    const PNG: &[u8] = &[
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0,
        0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0, 0x90, 0x77, 0x53, 0xde, 0, 0, 0, 12, b'I', b'D', b'A',
        b'T', 8, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0, 0, 3, 1, 1, 0, 0x18, 0xdd, 0x8d, 0xb1, 0, 0, 0,
        0, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82,
    ];
    const JPEG: &[u8] = &[0xff, 0xd8, 0xff, 0xc0, 0, 8, 8, 0, 1, 0, 1, 1, 0xff, 0xd9];
    const GIF: &[u8] = &[
        b'G', b'I', b'F', b'8', b'9', b'a', 1, 0, 1, 0, 0x80, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff,
        0x2c, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 1, 0x4c, 0, 0x3b,
    ];
    const WEBP: &[u8] = &[
        b'R', b'I', b'F', b'F', 0x26, 0, 0, 0, b'W', b'E', b'B', b'P', b'V', b'P', b'8', b' ',
        0x1a, 0, 0, 0, 0x30, 1, 0, 0x9d, 1, 0x2a, 1, 0, 1, 0, 1, 0x40, 0x26, 0x25, 0xa4, 0, 3,
        0x70, 0, 0xfe, 0xff, 0x3d, 0x58, 0, 0, 0,
    ];

    #[test]
    fn allowlisted_preview_mimes_require_matching_structural_signatures() {
        let cases = [
            ("image/png", PNG),
            ("image/jpeg", JPEG),
            ("image/gif", GIF),
            ("image/webp", WEBP),
        ];
        for (mime, payload) in cases {
            assert!(valid_preview_payload(mime, payload), "{mime}");
            assert!(
                !valid_preview_payload(mime, b"<svg onload=alert(1)/>"),
                "{mime}"
            );
            assert!(
                !valid_preview_payload(mime, b"<script>alert(1)</script>"),
                "{mime}"
            );
            for (_, mismatched) in cases.iter().filter(|(other, _)| *other != mime) {
                assert!(!valid_preview_payload(mime, mismatched), "{mime}");
            }
        }
        assert!(!valid_preview_payload("image/bmp", b"BMpayload"));
        assert!(!valid_preview_payload("image/tiff", b"IIpayload"));
        assert!(!valid_preview_payload("image/svg+xml", b"<svg/>"));
    }
}
