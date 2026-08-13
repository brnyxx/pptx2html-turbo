use log::warn;
use quick_xml::events::Event;
use quick_xml::events::{BytesEnd, BytesStart};
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;
use std::io::{Cursor, Read};
use zip::ZipArchive;

use super::slide_parser::ShapeBuilder;
use super::{
    chart_diagnostics, effect_diagnostics, embedded_parser, media_parser, notes_comments_parser,
    picture_bullet_diagnostics, table_style_package_diagnostics, timing_parser, xml_utils,
};
use crate::error::PptxResult;
use crate::model::slide::{UnresolvedType, UnsupportedData};
use crate::model::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    SupportTier,
};

pub(crate) fn collect_package_diagnostics(data: &[u8]) -> PptxResult<Vec<ConversionDiagnostic>> {
    let mut archive = ZipArchive::new(Cursor::new(data))?;
    let mut diagnostics = Vec::new();
    let table_styles_part =
        table_style_package_diagnostics::collect_diagnostics(&mut archive, &mut diagnostics)?;
    let content_types = read_text_entry(&mut archive, "[Content_Types].xml")
        .map(|xml| super::picture_bullet_parser::ContentTypes::parse(&xml))
        .unwrap_or_default();
    diagnostics.extend(thumbnail_diagnostics(&mut archive, &content_types)?);
    chart_diagnostics::collect(&mut archive, &content_types, &mut diagnostics)?;
    let mut names = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|file| file.name().to_owned())
        })
        .collect::<Vec<_>>();
    names.sort();

    let unknown_parts = embedded_parser::UnknownPartInventory::collect(&names, &mut diagnostics);
    diagnostics.extend(custom_xml_diagnostics(&mut archive, &names)?);

    for name in names {
        notes_comments_parser::collect_part_diagnostics(&name, &mut diagnostics);
        media_parser::collect_part_diagnostics(&name, &mut diagnostics);
        if name.starts_with("ppt/") && name.ends_with(".xml") {
            let xml = read_text_entry(&mut archive, &name)?;
            let characteristics = additional_characteristics_diagnostics(&name, &xml);
            if !characteristics.is_empty() {
                diagnostics.extend(characteristics);
                continue;
            }
            let bibliography = bibliography_diagnostics(&name, &xml);
            if !bibliography.is_empty() {
                diagnostics.extend(bibliography);
                continue;
            }
        }
        if unknown_parts.contains(&name) {
            continue;
        }
        if name.ends_with(".rels") {
            embedded_parser::collect_relationship_diagnostics(
                &mut archive,
                &name,
                &mut diagnostics,
            )?;
            continue;
        }
        if table_styles_part.as_deref() == Some(name.as_str()) {
            continue;
        }
        if name.starts_with("ppt/") && name.ends_with(".xml") {
            collect_xml_diagnostics(&mut archive, &name, &mut diagnostics)?;
        }
    }
    Ok(diagnostics)
}

fn thumbnail_diagnostics(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    content_types: &super::picture_bullet_parser::ContentTypes,
) -> PptxResult<Vec<ConversionDiagnostic>> {
    const THUMBNAIL: &str =
        "http://schemas.openxmlformats.org/package/2006/relationships/metadata/thumbnail";
    let relationships_xml = read_text_entry(archive, "_rels/.rels")?;
    let relationships = super::relationships::parse_relationship_records(&relationships_xml)?;
    let mut diagnostics = Vec::new();
    for relationship in relationships
        .iter()
        .filter(|relationship| relationship.relationship_type == THUMBNAIL)
    {
        if relationship.target_mode.as_str() != "Internal"
            || relationship.target.starts_with('/')
            || relationship
                .target
                .split('/')
                .any(|segment| segment == "..")
        {
            continue;
        }
        let part = relationship.target.trim_start_matches("./");
        let Ok(mut file) = archive.by_name(part) else {
            continue;
        };
        let byte_length = file.size();
        let mut prefix = [0_u8; 16];
        let read = file.read(&mut prefix).unwrap_or(0);
        let signature = thumbnail_signature(&prefix[..read]);
        diagnostics.push(ConversionDiagnostic {
            code: "PACKAGE_THUMBNAIL_METADATA".to_owned(),
            family: FeatureFamily::Images,
            support_tier: SupportTier::Fallback,
            stage: Some(CapabilityStage::Parsed),
            location: DiagnosticLocation {
                part_name: Some(part.to_owned()),
                relationship_id: Some(relationship.id.clone()),
                relationship_type: Some(relationship.relationship_type.clone()),
                ..Default::default()
            },
            raw_reference: Some(format!(
                "part={part}\nrelationship_id={}\ncontent_type={}\nbyte_length={byte_length}\nsignature={signature}",
                relationship.id,
                content_types.for_part(part).unwrap_or_default()
            )),
            fallback_kind: FallbackKind::PreservedPart,
            reason: "Package thumbnail metadata was preserved without embedding thumbnail bytes"
                .to_owned(),
        });
    }
    Ok(diagnostics)
}

fn thumbnail_signature(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]) {
        "png"
    } else if bytes.starts_with(&[255, 216, 255]) {
        "jpeg"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "gif"
    } else {
        "unknown"
    }
}

fn custom_xml_diagnostics(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    names: &[String],
) -> PptxResult<Vec<ConversionDiagnostic>> {
    let mut diagnostics = Vec::new();
    for data_part in names.iter().filter(|name| {
        name.starts_with("customXml/item") && name.ends_with(".xml") && !name.contains("itemProps")
    }) {
        let suffix = data_part
            .trim_start_matches("customXml/item")
            .trim_end_matches(".xml");
        let properties_part = format!("customXml/itemProps{suffix}.xml");
        let data_xml = read_text_entry(archive, data_part)?;
        let Some(root) = root_qname(&data_xml) else {
            continue;
        };
        let (item_id, schema_uris) = if names.contains(&properties_part) {
            let properties_xml = read_text_entry(archive, &properties_part)?;
            custom_xml_properties(&properties_xml)
        } else {
            (String::new(), Vec::new())
        };
        diagnostics.push(ConversionDiagnostic {
            code: "CUSTOM_XML_DATA_METADATA".to_owned(),
            family: FeatureFamily::Unsupported,
            support_tier: SupportTier::Fallback,
            stage: Some(CapabilityStage::Parsed),
            location: DiagnosticLocation {
                part_name: Some(data_part.clone()),
                qualified_element_name: Some(root.clone()),
                ..Default::default()
            },
            raw_reference: Some(format!(
                "data_part={data_part}\nproperties_part={properties_part}\nitem_id={item_id}\nschema_uri={}\nroot={root}\nraw_xml={}",
                schema_uris.join(","),
                bounded_xml(&data_xml)
            )),
            fallback_kind: FallbackKind::PreservedPart,
            reason: "Custom XML data and properties were preserved as bounded typed metadata"
                .to_owned(),
        });
    }
    Ok(diagnostics)
}

fn root_qname(xml: &str) -> Option<String> {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element) | Event::Empty(element))) => {
                let namespace = match namespace {
                    ResolveResult::Bound(value) => {
                        String::from_utf8_lossy(value.as_ref()).into_owned()
                    }
                    ResolveResult::Unbound | ResolveResult::Unknown(_) => String::new(),
                };
                let local = String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
                return Some(format!("{{{namespace}}}{local}"));
            }
            Ok((_, Event::Eof)) | Err(_) => return None,
            _ => {}
        }
        buffer.clear();
    }
}

fn custom_xml_properties(xml: &str) -> (String, Vec<String>) {
    const CUSTOM_XML: &[u8] = b"http://schemas.openxmlformats.org/officeDocument/2006/customXml";
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut depth = 0_usize;
    let mut item_id = String::new();
    let mut schema_uris = Vec::new();
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element) | Event::Empty(element))) => {
                let valid_namespace = matches!(
                    namespace,
                    ResolveResult::Bound(value) if value.as_ref() == CUSTOM_XML
                );
                let local = element.local_name();
                if depth == 0 {
                    if !valid_namespace || local.as_ref() != b"datastoreItem" {
                        return (String::new(), Vec::new());
                    }
                    item_id = xml_attribute(&element, "itemID");
                } else if valid_namespace && local.as_ref() == b"schemaRef" {
                    let uri = xml_attribute(&element, "uri");
                    if !uri.is_empty() {
                        schema_uris.push(uri);
                    }
                }
                if !element.is_empty() {
                    depth += 1;
                }
            }
            Ok((_, Event::End(_))) => depth = depth.saturating_sub(1),
            Ok((_, Event::Eof)) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    (item_id, schema_uris)
}

fn xml_attribute(element: &BytesStart<'_>, name: &str) -> String {
    element
        .attributes()
        .flatten()
        .find(|attribute| xml_utils::local_name(attribute.key.as_ref()) == name)
        .and_then(|attribute| attribute.unescape_value().ok())
        .map(|value| value.into_owned())
        .unwrap_or_default()
}

fn bounded_xml(xml: &str) -> String {
    const LIMIT: usize = 16 * 1024;
    if xml.len() <= LIMIT {
        return xml.to_owned();
    }
    let mut end = LIMIT;
    while !xml.is_char_boundary(end) {
        end -= 1;
    }
    xml[..end].to_owned()
}

fn additional_characteristics_diagnostics(part_name: &str, xml: &str) -> Vec<ConversionDiagnostic> {
    const ADDITIONAL_CHARACTERISTICS: &[u8] =
        b"http://schemas.openxmlformats.org/officeDocument/2006/additionalCharacteristics";
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut depth = 0_usize;
    let mut diagnostics = Vec::new();
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element))) => {
                let is_characteristics = matches!(
                    namespace,
                    ResolveResult::Bound(value) if value.as_ref() == ADDITIONAL_CHARACTERISTICS
                );
                let local = element.local_name();
                if depth == 0
                    && (!is_characteristics || local.as_ref() != b"AdditionalCharacteristics")
                {
                    return Vec::new();
                }
                depth += 1;
            }
            Ok((namespace, Event::Empty(element))) => {
                let is_characteristics = matches!(
                    namespace,
                    ResolveResult::Bound(value) if value.as_ref() == ADDITIONAL_CHARACTERISTICS
                );
                if depth == 0 {
                    return Vec::new();
                }
                if depth == 1
                    && is_characteristics
                    && element.local_name().as_ref() == b"Characteristic"
                {
                    diagnostics.push(characteristic_diagnostic(part_name, &element));
                }
            }
            Ok((_, Event::End(_))) => depth = depth.saturating_sub(1),
            Ok((_, Event::Eof)) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    diagnostics
}

fn characteristic_diagnostic(part_name: &str, element: &BytesStart<'_>) -> ConversionDiagnostic {
    let attribute = |name: &str| {
        element
            .attributes()
            .flatten()
            .find(|attribute| attribute.key.as_ref() == name.as_bytes())
            .and_then(|attribute| attribute.unescape_value().ok())
            .map(|value| value.into_owned())
            .unwrap_or_default()
    };
    ConversionDiagnostic {
        code: "ADDITIONAL_CHARACTERISTIC_METADATA".to_owned(),
        family: FeatureFamily::Unsupported,
        support_tier: SupportTier::Fallback,
        stage: Some(CapabilityStage::Parsed),
        location: DiagnosticLocation {
            part_name: Some(part_name.to_owned()),
            qualified_element_name: Some("ac:Characteristic".to_owned()),
            ..Default::default()
        },
        raw_reference: Some(format!(
            "name={}\nrelation={}\nvalue={}\nvocabulary={}",
            attribute("name"),
            attribute("relation"),
            attribute("val"),
            attribute("vocabulary")
        )),
        fallback_kind: FallbackKind::PreservedPart,
        reason: "Additional package characteristic was preserved as typed metadata".to_owned(),
    }
}

fn bibliography_diagnostics(part_name: &str, xml: &str) -> Vec<ConversionDiagnostic> {
    const BIBLIOGRAPHY: &[u8] =
        b"http://schemas.openxmlformats.org/officeDocument/2006/bibliography";
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut depth = 0_usize;
    let mut source_depth = None;
    let mut current_field: Option<String> = None;
    let mut fields = std::collections::BTreeMap::new();
    let mut diagnostics = Vec::new();
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element))) => {
                let is_bibliography = matches!(
                    namespace,
                    ResolveResult::Bound(value) if value.as_ref() == BIBLIOGRAPHY
                );
                let local = String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
                if depth == 0 && (!is_bibliography || local != "Sources") {
                    return Vec::new();
                }
                if is_bibliography && local == "Source" && depth == 1 {
                    source_depth = Some(depth);
                    fields.clear();
                } else if source_depth.is_some()
                    && is_bibliography
                    && matches!(
                        local.as_str(),
                        "Tag" | "SourceType" | "Title" | "Year" | "First" | "Middle" | "Last"
                    )
                {
                    current_field = Some(local);
                }
                depth += 1;
            }
            Ok((_, Event::Text(value))) if current_field.is_some() => {
                if let (Some(field), Ok(value)) = (current_field.as_ref(), value.unescape()) {
                    fields
                        .entry(field.clone())
                        .or_insert_with(String::new)
                        .push_str(&value);
                }
            }
            Ok((namespace, Event::End(element))) => {
                let is_bibliography = matches!(
                    namespace,
                    ResolveResult::Bound(value) if value.as_ref() == BIBLIOGRAPHY
                );
                let local = String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
                if is_bibliography && current_field.as_deref() == Some(local.as_str()) {
                    current_field = None;
                }
                depth = depth.saturating_sub(1);
                if is_bibliography && local == "Source" && source_depth == Some(depth) {
                    diagnostics.push(bibliography_diagnostic(part_name, &fields));
                    source_depth = None;
                    fields.clear();
                }
            }
            Ok((_, Event::Eof)) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    diagnostics
}

fn bibliography_diagnostic(
    part_name: &str,
    fields: &std::collections::BTreeMap<String, String>,
) -> ConversionDiagnostic {
    let author = ["First", "Middle", "Last"]
        .into_iter()
        .filter_map(|field| fields.get(field))
        .filter(|value| !value.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    ConversionDiagnostic {
        code: "BIBLIOGRAPHY_SOURCE_METADATA".to_owned(),
        family: FeatureFamily::Unsupported,
        support_tier: SupportTier::Fallback,
        stage: Some(CapabilityStage::Parsed),
        location: DiagnosticLocation {
            part_name: Some(part_name.to_owned()),
            qualified_element_name: Some("b:Source".to_owned()),
            ..Default::default()
        },
        raw_reference: Some(format!(
            "tag={}\nsource_type={}\ntitle={}\nyear={}\nauthor={author}",
            fields.get("Tag").map(String::as_str).unwrap_or_default(),
            fields
                .get("SourceType")
                .map(String::as_str)
                .unwrap_or_default(),
            fields.get("Title").map(String::as_str).unwrap_or_default(),
            fields.get("Year").map(String::as_str).unwrap_or_default(),
        )),
        fallback_kind: FallbackKind::PreservedPart,
        reason: "Bibliography source was preserved as typed metadata".to_owned(),
    }
}

fn collect_xml_diagnostics(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    diagnostics: &mut Vec<ConversionDiagnostic>,
) -> PptxResult<()> {
    let xml = read_text_entry(archive, name)?;
    timing_parser::collect_diagnostics(name, &xml, diagnostics);
    effect_diagnostics::collect(name, &xml, diagnostics);
    picture_bullet_diagnostics::collect(archive, name, &xml, diagnostics)?;
    // Chart parts are classified as a whole by chart_parser so one rejected chart
    // cannot fan out into a diagnostic for every unsupported descendant.
    if name.starts_with("ppt/charts/") {
        return Ok(());
    }
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
        b"http://schemas.microsoft.com/office/drawing/2014/chartex" => local_name == "chart",
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
            | "pattFill"
            | "fgClr"
            | "bgClr"
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
            | "reflection"
            | "effectDag"
            | "cont"
            | "scene3d"
            | "camera"
            | "lightRig"
            | "sp3d"
            | "bevelT"
            | "bevelB"
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
            | "buAutoNum"
            | "buNone"
            | "buBlip"
            | "buFont"
            | "buSzPct"
            | "buSzPts"
            | "buSzTx"
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
            | "tableStyleId"
            | "tblGrid"
            | "gridCol"
            | "tr"
            | "tc"
            | "tcPr"
            | "tblStyleLst"
            | "tblStyle"
            | "wholeTbl"
            | "band1H"
            | "band2H"
            | "band1V"
            | "band2V"
            | "firstCol"
            | "lastCol"
            | "firstRow"
            | "lastRow"
            | "neCell"
            | "nwCell"
            | "seCell"
            | "swCell"
            | "tblBg"
            | "tcTxStyle"
            | "tcStyle"
            | "tcBdr"
            | "fill"
            | "left"
            | "right"
            | "top"
            | "bottom"
            | "insideH"
            | "insideV"
            | "lnL"
            | "lnR"
            | "lnT"
            | "lnB"
            | "extLst"
            | "hlinkClick"
            | "hlinkHover"
            | "hlinkMouseOver"
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
