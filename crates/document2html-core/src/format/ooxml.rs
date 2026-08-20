use std::io::{Cursor, Read};

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;
use zip::ZipArchive;

use crate::{DocumentError, DocumentFormat, DocumentResult};

const PACKAGE_RELATIONSHIPS_NS: &[u8] =
    b"http://schemas.openxmlformats.org/package/2006/relationships";
const CONTENT_TYPES_NS: &[u8] = b"http://schemas.openxmlformats.org/package/2006/content-types";
const OFFICE_DOCUMENT_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const STRICT_OFFICE_DOCUMENT_RELATIONSHIP: &str =
    "http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocument";
const MAX_PACKAGE_METADATA_BYTES: u64 = 1_048_576;

pub(super) fn detect_ooxml_format(data: &[u8]) -> DocumentResult<DocumentFormat> {
    let mut archive = ZipArchive::new(Cursor::new(data))?;
    let content_types = read_text_entry(&mut archive, "[Content_Types].xml")?;
    let relationships = read_text_entry(&mut archive, "_rels/.rels")?;
    let content_format = parse_content_types(&content_types)?;
    let relationship_format = parse_root_relationships(&relationships)?;
    if content_format != relationship_format {
        return Err(DocumentError::UnsupportedFormat);
    }
    let main_part = main_part_name(content_format);
    archive
        .by_name(main_part)
        .map_err(|_| DocumentError::MissingPackagePart(main_part.to_owned()))?;
    Ok(content_format)
}

fn read_text_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    part_name: &str,
) -> DocumentResult<String> {
    let mut file = archive
        .by_name(part_name)
        .map_err(|_| DocumentError::MissingPackagePart(part_name.to_owned()))?;
    if file.size() > MAX_PACKAGE_METADATA_BYTES {
        return Err(DocumentError::PackageMetadataTooLarge {
            part: part_name.to_owned(),
            limit: MAX_PACKAGE_METADATA_BYTES,
        });
    }
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(text)
}

fn parse_content_types(xml: &str) -> DocumentResult<DocumentFormat> {
    let mut reader = NsReader::from_str(xml);
    let mut root_is_official = false;
    let mut depth = 0_usize;
    let mut detected = None;
    loop {
        match reader.read_resolved_event()? {
            (namespace, Event::Start(ref element)) => {
                if depth == 0 {
                    root_is_official =
                        official_element(&namespace, element, CONTENT_TYPES_NS, b"Types");
                } else if depth == 1
                    && root_is_official
                    && official_element(&namespace, element, CONTENT_TYPES_NS, b"Override")
                {
                    update_content_type_format(element, &mut detected)?;
                }
                depth += 1;
            }
            (namespace, Event::Empty(ref element))
                if depth == 1
                    && root_is_official
                    && official_element(&namespace, element, CONTENT_TYPES_NS, b"Override") =>
            {
                update_content_type_format(element, &mut detected)?;
            }
            (_, Event::End(_)) => depth = depth.saturating_sub(1),
            (_, Event::Eof) => break,
            _ => {}
        }
    }
    detected.ok_or(DocumentError::UnsupportedFormat)
}

fn update_content_type_format(
    element: &BytesStart<'_>,
    detected: &mut Option<DocumentFormat>,
) -> DocumentResult<()> {
    let part = unqualified_attribute(element, b"PartName")?;
    let content_type = unqualified_attribute(element, b"ContentType")?;
    let candidate = ooxml_content_type_format(part.as_deref(), content_type.as_deref());
    merge_detected_format(detected, candidate)
}

fn parse_root_relationships(xml: &str) -> DocumentResult<DocumentFormat> {
    let mut reader = NsReader::from_str(xml);
    let mut root_is_official = false;
    let mut depth = 0_usize;
    let mut detected = None;
    loop {
        match reader.read_resolved_event()? {
            (namespace, Event::Start(ref element)) => {
                if depth == 0 {
                    root_is_official = official_element(
                        &namespace,
                        element,
                        PACKAGE_RELATIONSHIPS_NS,
                        b"Relationships",
                    );
                } else if depth == 1
                    && root_is_official
                    && official_element(
                        &namespace,
                        element,
                        PACKAGE_RELATIONSHIPS_NS,
                        b"Relationship",
                    )
                {
                    update_relationship_format(element, &mut detected)?;
                }
                depth += 1;
            }
            (namespace, Event::Empty(ref element))
                if depth == 1
                    && root_is_official
                    && official_element(
                        &namespace,
                        element,
                        PACKAGE_RELATIONSHIPS_NS,
                        b"Relationship",
                    ) =>
            {
                update_relationship_format(element, &mut detected)?;
            }
            (_, Event::End(_)) => depth = depth.saturating_sub(1),
            (_, Event::Eof) => break,
            _ => {}
        }
    }
    detected.ok_or(DocumentError::UnsupportedFormat)
}

fn update_relationship_format(
    element: &BytesStart<'_>,
    detected: &mut Option<DocumentFormat>,
) -> DocumentResult<()> {
    let relationship_type = unqualified_attribute(element, b"Type")?;
    if !matches!(
        relationship_type.as_deref(),
        Some(OFFICE_DOCUMENT_RELATIONSHIP | STRICT_OFFICE_DOCUMENT_RELATIONSHIP)
    ) {
        return Ok(());
    }
    let target = unqualified_attribute(element, b"Target")?;
    let candidate = target.as_deref().and_then(target_format);
    merge_detected_format(detected, candidate)
}

fn target_format(target: &str) -> Option<DocumentFormat> {
    match target.trim_start_matches('/') {
        "word/document.xml" => Some(DocumentFormat::Docx),
        "xl/workbook.xml" => Some(DocumentFormat::Xlsx),
        "ppt/presentation.xml" => Some(DocumentFormat::Pptx),
        _ => None,
    }
}

fn unqualified_attribute(element: &BytesStart<'_>, key: &[u8]) -> DocumentResult<Option<String>> {
    let mut value = None;
    for attribute in element.attributes().flatten() {
        if attribute.key.as_ref() != key {
            continue;
        }
        if value.is_some() {
            return Ok(None);
        }
        value = Some(attribute.unescape_value()?.into_owned());
    }
    Ok(value)
}

fn official_element(
    namespace: &ResolveResult<'_>,
    element: &BytesStart<'_>,
    expected_namespace: &[u8],
    local_name: &[u8],
) -> bool {
    matches!(namespace, ResolveResult::Bound(value) if value.as_ref() == expected_namespace)
        && element.local_name().as_ref() == local_name
}

fn merge_detected_format(
    detected: &mut Option<DocumentFormat>,
    candidate: Option<DocumentFormat>,
) -> DocumentResult<()> {
    let Some(candidate) = candidate else {
        return Ok(());
    };
    if detected.is_some_and(|existing| existing != candidate) {
        return Err(DocumentError::UnsupportedFormat);
    }
    *detected = Some(candidate);
    Ok(())
}

fn ooxml_content_type_format(
    part_name: Option<&str>,
    content_type: Option<&str>,
) -> Option<DocumentFormat> {
    match (part_name, content_type) {
        (
            Some("/word/document.xml"),
            Some(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
            ),
        ) => Some(DocumentFormat::Docx),
        (
            Some("/xl/workbook.xml"),
            Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"),
        ) => Some(DocumentFormat::Xlsx),
        (
            Some("/ppt/presentation.xml"),
            Some(
                "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
            ),
        ) => Some(DocumentFormat::Pptx),
        _ => None,
    }
}

const fn main_part_name(format: DocumentFormat) -> &'static str {
    match format {
        DocumentFormat::Docx => "word/document.xml",
        DocumentFormat::Xlsx => "xl/workbook.xml",
        DocumentFormat::Pptx => "ppt/presentation.xml",
        DocumentFormat::Doc | DocumentFormat::Xls | DocumentFormat::Ppt | DocumentFormat::Pdf => "",
    }
}
