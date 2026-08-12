use std::io::{Cursor, Read};

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;
use zip::ZipArchive;

use super::chart_parser::{self, ChartFallbackReason};
use super::graphic_frame_parser::{
    CHART_RELATIONSHIP, CHARTEX_RELATIONSHIP, ElementNamespace, relationships_path,
    safe_relative_file_path, select_chart_preview,
};
use super::picture_bullet_parser::ContentTypes;
use super::{relationships, xml_utils};
use crate::error::PptxResult;
use crate::model::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, Emu, FallbackKind, FeatureFamily,
    Position, Size, SupportTier,
};

const PRESENTATION_NS: &[u8] = b"http://schemas.openxmlformats.org/presentationml/2006/main";
const DRAWING_NS: &[u8] = b"http://schemas.openxmlformats.org/drawingml/2006/main";
const CLASSIC_CHART_NS: &[u8] = b"http://schemas.openxmlformats.org/drawingml/2006/chart";
const CHARTEX_NS: &[u8] = b"http://schemas.microsoft.com/office/drawing/2014/chartex";

#[derive(Default)]
struct ChartFrame {
    relationship_id: Option<String>,
    relationship_type: Option<String>,
    qualified_name: Option<String>,
    position: Option<Position>,
    size: Option<Size>,
    invalid_ancestry: bool,
}

pub(crate) fn collect(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    content_types: &ContentTypes,
    diagnostics: &mut Vec<ConversionDiagnostic>,
) -> PptxResult<()> {
    let mut slides = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|file| file.name().to_owned())
        })
        .filter(|name| name.starts_with("ppt/slides/slide") && name.ends_with(".xml"))
        .collect::<Vec<_>>();
    slides.sort();
    for slide_part in slides {
        let slide_xml = read_text(archive, &slide_part)?;
        let rels_path = relationships_path(&slide_part);
        let relationship_records = read_text(archive, &rels_path)
            .ok()
            .and_then(|xml| relationships::parse_relationship_records(&xml).ok())
            .unwrap_or_default();
        for frame in chart_frames(&slide_xml)? {
            if let Some(diagnostic) = audit_frame(
                archive,
                content_types,
                &slide_part,
                &relationship_records,
                frame,
            ) {
                diagnostics.push(diagnostic);
            }
        }
    }
    Ok(())
}

fn audit_frame(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    content_types: &ContentTypes,
    slide_part: &str,
    relationships: &[relationships::Relationship],
    mut frame: ChartFrame,
) -> Option<ConversionDiagnostic> {
    let forced_reason = frame
        .invalid_ancestry
        .then_some(ChartFallbackReason::InvalidAncestry);
    let expected_relationship_type = frame
        .relationship_type
        .as_deref()
        .unwrap_or(CHART_RELATIONSHIP);
    let relationship = frame
        .relationship_id
        .as_ref()
        .and_then(|id| relationships.iter().find(|item| item.id == *id));
    let (reason, chart_path, chart_xml, outcome) = match relationship {
        None if frame.relationship_id.is_none() => {
            (ChartFallbackReason::MissingRelationshipId, None, None, None)
        }
        None => (ChartFallbackReason::MissingRelationship, None, None, None),
        Some(item)
            if item.relationship_type != expected_relationship_type
                || !matches!(item.target_mode, relationships::TargetMode::Internal) =>
        {
            (ChartFallbackReason::MissingRelationship, None, None, None)
        }
        Some(item) => {
            let Some(path) = safe_relative_file_path(slide_part, &item.target) else {
                return Some(build_diagnostic(
                    slide_part,
                    frame,
                    ChartFallbackReason::MissingPart,
                    None,
                    None,
                    None,
                    None,
                ));
            };
            let Ok(xml) = read_text(archive, &path) else {
                return Some(build_diagnostic(
                    slide_part,
                    frame,
                    ChartFallbackReason::MissingPart,
                    Some(path),
                    None,
                    None,
                    None,
                ));
            };
            match chart_parser::classify_and_parse(&xml) {
                Ok(outcome) if outcome.fallback_reason.is_none() && forced_reason.is_none() => {
                    return None;
                }
                Ok(outcome) => {
                    if !frame.invalid_ancestry {
                        frame.qualified_name = outcome
                            .qualified_name
                            .clone()
                            .or(frame.qualified_name.clone());
                    }
                    (
                        forced_reason
                            .or(outcome.fallback_reason)
                            .unwrap_or(ChartFallbackReason::NoSeries),
                        Some(path),
                        Some(xml),
                        Some(outcome),
                    )
                }
                Err(_) => (
                    forced_reason.unwrap_or(ChartFallbackReason::InvalidXml),
                    Some(path),
                    Some(xml),
                    None,
                ),
            }
        }
    };
    let preview = chart_path.as_deref().and_then(|path| {
        let rels_xml = read_text(archive, &relationships_path(path)).ok()?;
        let records = relationships::parse_relationship_records(&rels_xml).ok()?;
        select_chart_preview(path, records, content_types, archive)
    });
    let preview_metadata = preview.as_ref().map(|item| {
        format!(
            "id={},type={},target={},part={},mime={},bytes={}",
            item.relationship_id,
            item.relationship_type,
            item.target,
            item.part_name,
            item.mime,
            item.bytes.len()
        )
    });
    let summary = outcome.as_ref().map(|item| item.series_summary.as_str());
    let inventory = outcome.as_ref().map(|item| item.element_inventory.as_str());
    Some(build_diagnostic(
        slide_part,
        frame,
        reason,
        chart_xml,
        summary,
        inventory,
        preview_metadata.as_deref(),
    ))
}

fn build_diagnostic(
    slide_part: &str,
    frame: ChartFrame,
    reason: ChartFallbackReason,
    raw_xml: Option<String>,
    series_summary: Option<&str>,
    element_inventory: Option<&str>,
    preview: Option<&str>,
) -> ConversionDiagnostic {
    let _exactness_disposition = "fallback";
    let summary = series_summary.unwrap_or("series=unknown,cache_points=unknown");
    let inventory = element_inventory.unwrap_or("unknown");
    let preview_summary = preview.unwrap_or("none");
    let qualified_type = frame
        .qualified_name
        .clone()
        .unwrap_or_else(|| "c:chart".to_owned());
    ConversionDiagnostic {
        code: if reason.is_structure_unsupported() {
            "CHART_STRUCTURE_UNSUPPORTED"
        } else {
            "DRAWINGML_CHART_FALLBACK"
        }
        .to_owned(),
        family: FeatureFamily::Charts,
        support_tier: SupportTier::Fallback,
        stage: Some(CapabilityStage::Rendered),
        location: DiagnosticLocation {
            slide_index: super::preserved_parser::slide_index_from_part(slide_part),
            part_name: Some(slide_part.to_owned()),
            relationship_id: frame.relationship_id,
            relationship_type: frame.relationship_type,
            qualified_element_name: frame.qualified_name.or_else(|| Some("c:chart".to_owned())),
            position: frame.position,
            size: frame.size,
        },
        raw_reference: raw_xml.map(|xml| {
            diagnostic_payload(&xml, summary, inventory, &qualified_type, preview_summary)
        }),
        fallback_kind: FallbackKind::PreservedPart,
        reason: format!(
            "Chart direct rendering rejected: {}; {}; preview_relationship={preview_summary}",
            reason.as_str(),
            summary
        ),
    }
}

fn diagnostic_payload(
    xml: &str,
    summary: &str,
    inventory: &str,
    qualified_type: &str,
    preview: &str,
) -> String {
    let fallback_mode = if preview == "none" {
        "placeholder"
    } else {
        "preview"
    };
    format!(
        "{{\"raw_xml\":\"{}\",\"series_summary\":\"{}\",\"element_inventory\":\"{}\",\"qualified_type\":\"{}\",\"chart_fallback_mode\":\"{}\",\"selected_preview_relationship\":\"{}\"}}",
        json_escape(xml),
        json_escape(summary),
        json_escape(inventory),
        json_escape(qualified_type),
        fallback_mode,
        json_escape(preview)
    )
}

fn json_escape(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control <= '\u{1f}' => {
                use std::fmt::Write;
                let _ = write!(output, "\\u{:04x}", u32::from(control));
            }
            other => output.push(other),
        }
    }
    output
}

fn chart_frames(xml: &str) -> PptxResult<Vec<ChartFrame>> {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut frames = Vec::new();
    let mut frame: Option<ChartFrame> = None;
    let mut in_transform = false;
    let mut path: Vec<(ElementNamespace, String)> = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => {
                let namespace = chart_element_namespace(reader.resolve_element(element.name()).0);
                inspect_frame_element(
                    &reader,
                    namespace,
                    &element,
                    &path,
                    &mut frame,
                    &mut in_transform,
                );
                path.push((
                    namespace,
                    xml_utils::local_name(element.name().as_ref()).to_owned(),
                ));
            }
            Ok(Event::Empty(element)) => {
                let namespace = chart_element_namespace(reader.resolve_element(element.name()).0);
                inspect_frame_element(
                    &reader,
                    namespace,
                    &element,
                    &path,
                    &mut frame,
                    &mut in_transform,
                );
            }
            Ok(Event::End(element)) => {
                let namespace = chart_element_namespace(reader.resolve_element(element.name()).0);
                let element_name = element.name();
                let local = xml_utils::local_name(element_name.as_ref());
                if namespace == ElementNamespace::Presentation && local == "xfrm" {
                    in_transform = false;
                }
                if namespace == ElementNamespace::Presentation
                    && local == "graphicFrame"
                    && let Some(current) = frame.take()
                    && (current.qualified_name.is_some() || current.invalid_ancestry)
                {
                    frames.push(current);
                }
                path.pop();
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(crate::error::PptxError::Xml(error)),
            _ => {}
        }
        buffer.clear();
    }
    Ok(frames)
}

fn inspect_frame_element(
    reader: &NsReader<&[u8]>,
    namespace: ElementNamespace,
    element: &quick_xml::events::BytesStart<'_>,
    path: &[(ElementNamespace, String)],
    frame: &mut Option<ChartFrame>,
    in_transform: &mut bool,
) {
    let element_name = element.name();
    let local = xml_utils::local_name(element_name.as_ref());
    if namespace == ElementNamespace::Presentation && local == "graphicFrame" {
        *frame = Some(ChartFrame::default());
        return;
    }
    if namespace == ElementNamespace::Presentation
        && local == "xfrm"
        && frame.is_some()
        && matches!(path.last(), Some((ElementNamespace::Presentation, parent)) if parent == "graphicFrame")
    {
        *in_transform = true;
        return;
    }
    if namespace == ElementNamespace::Drawing
        && *in_transform
        && let Some(current) = frame.as_mut()
    {
        match local {
            "off" => {
                current.position = Some(Position {
                    x: Emu(attr_i64(element, "x")),
                    y: Emu(attr_i64(element, "y")),
                });
            }
            "ext" => {
                current.size = Some(Size {
                    width: Emu(attr_i64(element, "cx")),
                    height: Emu(attr_i64(element, "cy")),
                });
            }
            _ => {}
        }
    }
    if !matches!(
        namespace,
        ElementNamespace::ClassicChart | ElementNamespace::ChartEx
    ) || local != "chart"
        || frame.is_none()
    {
        return;
    }

    let exact_ancestry = matches!(
        path,
        [..,
            (ElementNamespace::Presentation, frame_name),
            (ElementNamespace::Drawing, graphic_name),
            (ElementNamespace::Drawing, data_name),
        ] if frame_name == "graphicFrame" && graphic_name == "graphic" && data_name == "graphicData"
    );
    let has_foreign_ancestor = path
        .iter()
        .rev()
        .take_while(|(_, name)| name != "graphicFrame")
        .any(|(namespace, _)| *namespace == ElementNamespace::Other);
    let current = frame.as_mut().expect("checked chart frame");
    if !exact_ancestry {
        if !has_foreign_ancestor {
            current.invalid_ancestry = true;
            current.qualified_name =
                Some(String::from_utf8_lossy(element.name().as_ref()).into_owned());
            current.relationship_id = exact_relationship_id(reader, element);
            current.relationship_type = Some(
                if namespace == ElementNamespace::ChartEx {
                    CHARTEX_RELATIONSHIP
                } else {
                    CHART_RELATIONSHIP
                }
                .to_owned(),
            );
        }
        return;
    }

    current.relationship_id = exact_relationship_id(reader, element);
    current.relationship_type = Some(
        if namespace == ElementNamespace::ChartEx {
            CHARTEX_RELATIONSHIP
        } else {
            CHART_RELATIONSHIP
        }
        .to_owned(),
    );
    current.qualified_name = Some(String::from_utf8_lossy(element.name().as_ref()).into_owned());
}

fn chart_element_namespace(namespace: ResolveResult<'_>) -> ElementNamespace {
    let ResolveResult::Bound(namespace) = namespace else {
        return ElementNamespace::Other;
    };
    match namespace.as_ref() {
        PRESENTATION_NS => ElementNamespace::Presentation,
        DRAWING_NS => ElementNamespace::Drawing,
        CLASSIC_CHART_NS => ElementNamespace::ClassicChart,
        CHARTEX_NS => ElementNamespace::ChartEx,
        _ => ElementNamespace::Other,
    }
}

fn exact_relationship_id(
    reader: &NsReader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
) -> Option<String> {
    let mut id = None;
    for attribute in element.attributes().flatten() {
        if xml_utils::local_name(attribute.key.as_ref()) != "id" {
            continue;
        }
        if id.is_some()
            || !matches!(
                reader.resolve_attribute(attribute.key).0,
                ResolveResult::Bound(namespace)
                    if namespace.as_ref()
                        == b"http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            )
        {
            return None;
        }
        id = attribute
            .unescape_value()
            .ok()
            .map(|value| value.into_owned());
    }
    id
}

fn attr_i64(element: &quick_xml::events::BytesStart<'_>, name: &str) -> i64 {
    xml_utils::attr_str(element, name)
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or_default()
}

fn read_text(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> PptxResult<String> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| crate::error::PptxError::MissingFile(name.to_owned()))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}
