use log::warn;
use quick_xml::events::Event;
use quick_xml::events::{BytesEnd, BytesStart};
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;
use std::io::{Cursor, Read};
use zip::ZipArchive;

use super::slide_parser::ShapeBuilder;
use super::{embedded_parser, media_parser, notes_comments_parser, timing_parser, xml_utils};
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
    let mut reader = NsReader::from_str(&xml);
    let mut buffer = Vec::new();
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element) | Event::Empty(element))) => {
                let qualified_name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                let element_name = element.name();
                let local_name = xml_utils::local_name(element_name.as_ref());
                if !known_element(namespace, local_name) {
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
                        reason: "Element namespace or local name is not supported; the element was preserved but not rendered".to_owned(),
                    });
                }
            }
            Ok((_, Event::Eof)) => return Ok(()),
            Err(error) => return Err(crate::error::PptxError::Xml(error)),
            _ => {}
        }
        buffer.clear();
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

fn known_element(namespace: ResolveResult<'_>, local_name: &str) -> bool {
    let ResolveResult::Bound(namespace) = namespace else {
        return false;
    };
    match namespace.as_ref() {
        b"http://schemas.openxmlformats.org/presentationml/2006/main" => {
            known_presentationml_element(local_name)
        }
        b"http://schemas.openxmlformats.org/drawingml/2006/main" => {
            known_drawingml_element(local_name)
        }
        b"http://schemas.openxmlformats.org/drawingml/2006/chart" => {
            known_chart_element(local_name)
        }
        b"http://schemas.openxmlformats.org/drawingml/2006/diagram" => local_name == "relIds",
        b"http://schemas.openxmlformats.org/officeDocument/2006/math" => {
            matches!(local_name, "oMath" | "oMathPara" | "r" | "t")
        }
        b"http://schemas.openxmlformats.org/markup-compatibility/2006" => {
            matches!(local_name, "AlternateContent" | "Choice" | "Fallback")
        }
        b"http://schemas.openxmlformats.org/package/2006/metadata/core-properties" => {
            local_name == "coreProperties"
        }
        b"http://purl.org/dc/elements/1.1/" => local_name == "title",
        b"http://purl.org/dc/terms/" => matches!(local_name, "created" | "modified"),
        _ => false,
    }
}

fn known_presentationml_element(local_name: &str) -> bool {
    matches!(
        local_name,
        "presentation"
            | "sldIdLst"
            | "sldId"
            | "sldMasterIdLst"
            | "sldMasterId"
            | "sldSz"
            | "notesSz"
            | "defaultTextStyle"
            | "sld"
            | "sldMaster"
            | "sldLayout"
            | "notes"
            | "cmLst"
            | "comment"
            | "cSld"
            | "spTree"
            | "nvGrpSpPr"
            | "cNvPr"
            | "cNvGrpSpPr"
            | "nvPr"
            | "grpSpPr"
            | "grpSp"
            | "sp"
            | "nvSpPr"
            | "cNvSpPr"
            | "spPr"
            | "style"
            | "txBody"
            | "pic"
            | "nvPicPr"
            | "cNvPicPr"
            | "blipFill"
            | "graphicFrame"
            | "nvGraphicFramePr"
            | "cNvGraphicFramePr"
            | "xfrm"
            | "cxnSp"
            | "nvCxnSpPr"
            | "cNvCxnSpPr"
            | "stCxn"
            | "endCxn"
            | "oleObj"
            | "ph"
            | "bg"
            | "bgPr"
            | "clrMap"
            | "clrMapOvr"
            | "overrideClrMapping"
            | "masterClrMapping"
            | "txStyles"
            | "titleStyle"
            | "bodyStyle"
            | "otherStyle"
            | "timing"
            | "tnLst"
            | "par"
            | "seq"
            | "cTn"
            | "childTnLst"
            | "condLst"
            | "cond"
            | "tgtEl"
            | "spTgt"
    )
}

fn known_drawingml_element(local_name: &str) -> bool {
    matches!(
        local_name,
        "theme"
            | "themeElements"
            | "clrScheme"
            | "fontScheme"
            | "fmtScheme"
            | "dk1"
            | "lt1"
            | "dk2"
            | "lt2"
            | "accent1"
            | "accent2"
            | "accent3"
            | "accent4"
            | "accent5"
            | "accent6"
            | "hlink"
            | "folHlink"
            | "srgbClr"
            | "scrgbClr"
            | "sysClr"
            | "schemeClr"
            | "prstClr"
            | "majorFont"
            | "minorFont"
            | "latin"
            | "ea"
            | "cs"
            | "fillStyleLst"
            | "lnStyleLst"
            | "effectStyleLst"
            | "bgFillStyleLst"
            | "solidFill"
            | "gradFill"
            | "noFill"
            | "blipFill"
            | "blip"
            | "srcRect"
            | "stretch"
            | "fillRect"
            | "gsLst"
            | "gs"
            | "lin"
            | "path"
            | "ln"
            | "prstDash"
            | "headEnd"
            | "tailEnd"
            | "effectLst"
            | "outerShdw"
            | "glow"
            | "effectStyle"
            | "xfrm"
            | "off"
            | "ext"
            | "prstGeom"
            | "custGeom"
            | "avLst"
            | "gdLst"
            | "gd"
            | "ahLst"
            | "cxnLst"
            | "cxn"
            | "rect"
            | "pathLst"
            | "moveTo"
            | "lnTo"
            | "cubicBezTo"
            | "quadBezTo"
            | "arcTo"
            | "pt"
            | "close"
            | "graphic"
            | "graphicData"
            | "txBody"
            | "bodyPr"
            | "lstStyle"
            | "p"
            | "pPr"
            | "r"
            | "rPr"
            | "t"
            | "br"
            | "defRPr"
            | "endParaRPr"
            | "buChar"
            | "lnSpc"
            | "spcBef"
            | "spcAft"
            | "spcPct"
            | "spcPts"
            | "normAutofit"
            | "spAutoFit"
            | "noAutofit"
            | "tbl"
            | "tblPr"
            | "tblGrid"
            | "gridCol"
            | "tr"
            | "tc"
            | "tcPr"
            | "extLst"
            | "hlinkClick"
            | "fillRef"
            | "lnRef"
            | "effectRef"
            | "fontRef"
    )
}

fn known_chart_element(local_name: &str) -> bool {
    matches!(
        local_name,
        "chartSpace"
            | "chart"
            | "title"
            | "plotArea"
            | "layout"
            | "legend"
            | "legendPos"
            | "areaChart"
            | "area3DChart"
            | "barChart"
            | "bar3DChart"
            | "lineChart"
            | "line3DChart"
            | "pieChart"
            | "pie3DChart"
            | "doughnutChart"
            | "ofPieChart"
            | "radarChart"
            | "scatterChart"
            | "bubbleChart"
            | "ser"
            | "idx"
            | "order"
            | "tx"
            | "cat"
            | "val"
            | "xVal"
            | "yVal"
            | "bubbleSize"
            | "strLit"
            | "numLit"
            | "ptCount"
            | "pt"
            | "v"
            | "varyColors"
            | "grouping"
            | "barDir"
            | "overlap"
            | "gapWidth"
            | "holeSize"
            | "firstSliceAng"
            | "ofPieType"
            | "splitType"
            | "splitPos"
            | "secondPieSize"
            | "radarStyle"
            | "scatterStyle"
            | "bubbleScale"
            | "sizeRepresents"
            | "showNegBubbles"
            | "marker"
            | "symbol"
            | "smooth"
            | "dLbls"
            | "dLblPos"
            | "showVal"
            | "showCatName"
            | "showSerName"
            | "showPercent"
            | "catAx"
            | "valAx"
            | "axId"
            | "crossAx"
            | "rich"
    )
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
    custom_geometry: Option<crate::model::CustomGeometry>,
) -> UnsupportedData {
    UnsupportedData {
        label,
        element_type: element_type.unwrap_or(UnresolvedType::SmartArt),
        raw_xml,
        custom_geometry,
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
