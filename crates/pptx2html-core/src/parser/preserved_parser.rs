use log::warn;
use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::events::{BytesEnd, BytesStart};
use std::io::{Cursor, Read};
use zip::ZipArchive;

use super::slide_parser::ShapeBuilder;
use super::{embedded_parser, media_parser, notes_comments_parser, timing_parser};
use crate::error::PptxResult;
use crate::model::slide::{UnresolvedType, UnsupportedData};
use crate::model::{
    ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily, SupportTier,
};

pub(crate) fn collect_package_diagnostics(data: &[u8]) -> PptxResult<Vec<ConversionDiagnostic>> {
    let mut archive = ZipArchive::new(Cursor::new(data))?;
    let mut diagnostics = Vec::new();
    let mut names = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|file| file.name().to_owned())
        })
        .collect::<Vec<_>>();
    names.sort();

    for name in names {
        notes_comments_parser::collect_part_diagnostics(&name, &mut diagnostics);
        media_parser::collect_part_diagnostics(&name, &mut diagnostics);
        embedded_parser::collect_part_diagnostics(&name, &mut diagnostics);
        if name.ends_with(".rels") {
            embedded_parser::collect_relationship_diagnostics(
                &mut archive,
                &name,
                &mut diagnostics,
            )?;
            continue;
        }
        if name.starts_with("ppt/") && name.ends_with(".xml") {
            collect_xml_diagnostics(&mut archive, &name, &mut diagnostics)?;
        }
    }
    Ok(diagnostics)
}

fn collect_xml_diagnostics(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    diagnostics: &mut Vec<ConversionDiagnostic>,
) -> PptxResult<()> {
    let xml = read_text_entry(archive, name)?;
    timing_parser::collect_diagnostics(name, &xml, diagnostics);
    let mut reader = Reader::from_str(&xml);
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                let qualified_name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if known_element_prefix(&qualified_name) {
                    continue;
                }
                diagnostics.push(ConversionDiagnostic {
                    code: "OOXML_ELEMENT_UNSUPPORTED".to_owned(),
                    family: FeatureFamily::Unsupported,
                    support_tier: SupportTier::Unparsed,
                    stage: None,
                    location: DiagnosticLocation {
                        slide_index: slide_index_from_part(name),
                        part_name: Some(name.to_owned()),
                        qualified_element_name: Some(qualified_name.clone()),
                        relationship_id: embedded_parser::attribute_value(&element, "id"),
                        ..Default::default()
                    },
                    raw_reference: Some(format!("{name}#{qualified_name}")),
                    fallback_kind: FallbackKind::UnknownElement,
                    reason: "Element namespace is not recognized; the element was preserved but not rendered".to_owned(),
                });
            }
            Ok(Event::Eof) => return Ok(()),
            Err(error) => return Err(crate::error::PptxError::Xml(error)),
            _ => {}
        }
    }
}

pub(crate) fn read_text_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> PptxResult<String> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| crate::error::PptxError::MissingFile(name.to_owned()))?;
    let mut xml = String::new();
    file.read_to_string(&mut xml)?;
    Ok(xml)
}

pub(crate) fn part_diagnostic(
    name: &str,
    family: FeatureFamily,
    reason: &str,
) -> ConversionDiagnostic {
    ConversionDiagnostic {
        code: "OOXML_PART_UNSUPPORTED".to_owned(),
        family,
        support_tier: SupportTier::Unparsed,
        stage: None,
        location: DiagnosticLocation {
            part_name: Some(name.to_owned()),
            ..Default::default()
        },
        raw_reference: Some(name.to_owned()),
        fallback_kind: FallbackKind::PreservedPart,
        reason: reason.to_owned(),
    }
}

pub(crate) fn slide_index_from_part(name: &str) -> Option<usize> {
    let file = name
        .strip_prefix("ppt/slides/slide")?
        .strip_suffix(".xml")?;
    file.parse::<usize>().ok()?.checked_sub(1)
}

fn known_element_prefix(qualified_name: &str) -> bool {
    let prefix = qualified_name.split_once(':').map(|(prefix, _)| prefix);
    matches!(
        prefix,
        Some("a" | "c" | "cp" | "dc" | "dcterms" | "dgm" | "m" | "mc" | "p" | "r" | "xsi")
    ) || prefix.is_none()
}

#[derive(Default)]
pub(crate) struct PreservedSaxState {
    capturing: bool,
    raw_xml: String,
}

impl PreservedSaxState {
    pub(crate) fn start_capture(&mut self) {
        self.capturing = true;
        self.raw_xml.clear();
    }

    pub(crate) fn capture_start(&mut self, element: &BytesStart<'_>, local: &str) {
        if self.capturing && local != "graphicData" {
            append_start_element(element, local, &mut self.raw_xml);
        }
    }

    pub(crate) fn capture_empty(&mut self, element: &BytesStart<'_>, local: &str) {
        if self.capturing {
            append_empty_element(element, local, &mut self.raw_xml);
        }
    }

    pub(crate) fn capture_text(&mut self, text: &str) -> bool {
        if !self.capturing {
            return false;
        }
        self.raw_xml.push_str(text);
        true
    }

    pub(crate) fn capture_end(&mut self, element: &BytesEnd<'_>, local: &str) {
        if self.capturing && local != "graphicData" {
            append_end_element(element, local, &mut self.raw_xml);
        }
    }

    pub(crate) fn finish_capture(&mut self, shape: &mut Option<ShapeBuilder>) {
        if !self.capturing {
            return;
        }
        self.capturing = false;
        finish_raw_capture(shape, &mut self.raw_xml);
    }
}

pub(crate) struct UnsupportedGraphic {
    pub(crate) label: &'static str,
    pub(crate) element_type: UnresolvedType,
}

pub(crate) fn classify_unsupported_graphic(uri: &str) -> Option<UnsupportedGraphic> {
    if uri.contains("diagram") || uri.contains("dgm") {
        warn!("SmartArt diagram detected — rendering placeholder");
        Some(UnsupportedGraphic {
            label: "SmartArt",
            element_type: UnresolvedType::SmartArt,
        })
    } else if uri.contains("oleObject") {
        warn!("OLE object detected — rendering placeholder");
        Some(UnsupportedGraphic {
            label: "OLE Object",
            element_type: UnresolvedType::OleObject,
        })
    } else if uri.contains("math") || uri.contains("omml") {
        warn!("Math equation detected — rendering placeholder");
        Some(UnsupportedGraphic {
            label: "Math Equation",
            element_type: UnresolvedType::MathEquation,
        })
    } else {
        None
    }
}

pub(crate) fn unsupported_data(
    label: String,
    element_type: Option<UnresolvedType>,
    raw_xml: Option<String>,
) -> UnsupportedData {
    UnsupportedData {
        label,
        element_type: element_type.unwrap_or(UnresolvedType::SmartArt),
        raw_xml,
    }
}

pub(crate) fn finish_raw_capture(shape: &mut Option<ShapeBuilder>, raw_xml: &mut String) {
    if let Some(shape) = shape.as_mut()
        && !raw_xml.is_empty()
    {
        shape.raw_xml_capture = Some(raw_xml.clone());
    }
    raw_xml.clear();
}

pub(crate) fn append_start_element(
    element: &BytesStart<'_>,
    fallback_name: &str,
    output: &mut String,
) {
    output.push('<');
    append_element_name(element.name().as_ref(), fallback_name, output);
    append_attributes(element, output);
    output.push('>');
}

pub(crate) fn append_empty_element(
    element: &BytesStart<'_>,
    fallback_name: &str,
    output: &mut String,
) {
    output.push('<');
    append_element_name(element.name().as_ref(), fallback_name, output);
    append_attributes(element, output);
    output.push_str("/>");
}

pub(crate) fn append_end_element(element: &BytesEnd<'_>, fallback_name: &str, output: &mut String) {
    output.push_str("</");
    append_element_name(element.name().as_ref(), fallback_name, output);
    output.push('>');
}

fn append_element_name(name: &[u8], fallback_name: &str, output: &mut String) {
    output.push_str(std::str::from_utf8(name).unwrap_or(fallback_name));
}

fn append_attributes(element: &BytesStart<'_>, output: &mut String) {
    for attribute in element.attributes().flatten() {
        let key = std::str::from_utf8(attribute.key.as_ref()).unwrap_or("");
        let value = std::str::from_utf8(&attribute.value).unwrap_or("");
        output.push(' ');
        output.push_str(key);
        output.push_str("=\"");
        output.push_str(value);
        output.push('"');
    }
}
