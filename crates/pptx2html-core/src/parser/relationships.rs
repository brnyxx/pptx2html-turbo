use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use super::xml_utils;
use crate::error::PptxResult;

pub(crate) const HYPERLINK_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";
pub(crate) const SLIDE_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetError {
    Empty,
    Absolute,
    Backslash,
    UriScheme,
    EncodedPath,
    DotSegment,
    EmptySegment,
}

impl TargetError {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Absolute => "absolute",
            Self::Backslash => "backslash",
            Self::UriScheme => "uri_scheme",
            Self::EncodedPath => "encoded_path",
            Self::DotSegment => "dot_segment",
            Self::EmptySegment => "empty_segment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    pub id: String,
    pub relationship_type: String,
    pub target: String,
    pub target_mode: TargetMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetMode {
    Internal,
    External,
    Other(String),
}

impl TargetMode {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Internal => "Internal",
            Self::External => "External",
            Self::Other(value) => value,
        }
    }
}

pub fn parse_relationship_records(xml: &str) -> PptxResult<Vec<Relationship>> {
    let mut reader = NsReader::from_str(xml);
    let mut relationships = Vec::new();
    let mut depth = 0_usize;
    let mut official_root = false;

    loop {
        match reader.read_resolved_event() {
            Ok((namespace, Event::Start(ref e))) => {
                if depth == 0 {
                    official_root = official_package_element(&namespace, e, "Relationships");
                } else if depth == 1
                    && official_root
                    && official_package_element(&namespace, e, "Relationship")
                    && let Some(relationship) = parse_relationship(e)?
                {
                    relationships.push(relationship);
                }
                depth += 1;
            }
            Ok((namespace, Event::Empty(ref e))) => {
                if depth == 1
                    && official_root
                    && official_package_element(&namespace, e, "Relationship")
                    && let Some(relationship) = parse_relationship(e)?
                {
                    relationships.push(relationship);
                }
            }
            Ok((_, Event::End(_))) => depth = depth.saturating_sub(1),
            Ok((_, Event::Eof)) => break,
            Err(e) => return Err(crate::error::PptxError::Xml(e)),
            _ => {}
        }
    }

    Ok(relationships)
}

fn official_package_element(
    namespace: &ResolveResult<'_>,
    element: &quick_xml::events::BytesStart<'_>,
    local_name: &str,
) -> bool {
    matches!(namespace, ResolveResult::Bound(value) if value.as_ref() == b"http://schemas.openxmlformats.org/package/2006/relationships")
        && xml_utils::local_name(element.name().as_ref()) == local_name
}

fn parse_relationship(
    element: &quick_xml::events::BytesStart<'_>,
) -> PptxResult<Option<Relationship>> {
    let mut id = None;
    let mut relationship_type = None;
    let mut target = None;
    let mut target_mode = TargetMode::Internal;
    for attribute in element.attributes().flatten() {
        let Ok(key) = std::str::from_utf8(attribute.key.as_ref()) else {
            return Ok(None);
        };
        if key.contains(':') {
            return Ok(None);
        }
        let value = attribute.unescape_value()?.into_owned();
        match key {
            "Id" if id.is_none() => id = Some(value),
            "Type" if relationship_type.is_none() => relationship_type = Some(value),
            "Target" if target.is_none() => target = Some(value),
            "TargetMode" if value == "External" => target_mode = TargetMode::External,
            "TargetMode" if value == "Internal" => target_mode = TargetMode::Internal,
            "TargetMode" => target_mode = TargetMode::Other(value),
            _ => {}
        }
    }
    let (Some(id), Some(relationship_type), Some(target)) = (id, relationship_type, target) else {
        return Ok(None);
    };
    Ok(Some(Relationship {
        id,
        relationship_type,
        target,
        target_mode,
    }))
}

pub(crate) fn resolve_internal_target(
    owner_part: &str,
    target: &str,
) -> Result<String, TargetError> {
    if target.is_empty() {
        return Err(TargetError::Empty);
    }
    if target.starts_with('/') {
        return Err(TargetError::Absolute);
    }
    if target.contains('\\') {
        return Err(TargetError::Backslash);
    }
    if target.contains('%') {
        return Err(TargetError::EncodedPath);
    }
    if target.contains(':') {
        return Err(TargetError::UriScheme);
    }

    let mut path = owner_part
        .rsplit_once('/')
        .map(|(parent, _)| parent.split('/').collect::<Vec<_>>())
        .unwrap_or_default();
    for segment in target.split('/') {
        if segment.is_empty() {
            return Err(TargetError::EmptySegment);
        }
        if segment == "." {
            return Err(TargetError::DotSegment);
        }
        if segment == ".." {
            path.pop().ok_or(TargetError::DotSegment)?;
            continue;
        }
        path.push(segment);
    }
    Ok(path.join("/"))
}

/// Parse .rels file into {rId → target_path} map
pub fn parse_relationships(xml: &str) -> PptxResult<HashMap<String, String>> {
    Ok(target_map(&parse_relationship_records(xml)?))
}

pub fn target_map(relationships: &[Relationship]) -> HashMap<String, String> {
    relationships
        .iter()
        .cloned()
        .map(|relationship| (relationship.id, relationship.target))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relationship_elements_and_attributes_are_namespace_safe() {
        let foreign_root = r#"<x:Relationships xmlns:x="urn:foreign"><x:Relationship Id="rId1" Type="official" Target="secret"/></x:Relationships>"#;
        assert!(parse_relationship_records(foreign_root).unwrap().is_empty());

        let foreign_attribute = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships" xmlns:x="urn:foreign"><Relationship x:Id="rId1" Type="official" Target="secret"/></Relationships>"#;
        assert!(
            parse_relationship_records(foreign_attribute)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn internal_target_is_resolved_relative_to_owner() {
        assert_eq!(
            resolve_internal_target("ppt/presentation.xml", "tableStyles.xml"),
            Ok("ppt/tableStyles.xml".to_owned())
        );
        assert_eq!(
            resolve_internal_target("ppt/slides/slide1.xml", "../notesSlides/notesSlide1.xml"),
            Ok("ppt/notesSlides/notesSlide1.xml".to_owned())
        );
        assert_eq!(
            resolve_internal_target(
                "ppt/slides/sections/slide1.xml",
                "../../notesSlides/notesSlide1.xml"
            ),
            Ok("ppt/notesSlides/notesSlide1.xml".to_owned())
        );
    }

    #[test]
    fn unsafe_targets_are_rejected() {
        for target in [
            "",
            "/ppt/tableStyles.xml",
            "./tableStyles.xml",
            "tables//styles.xml",
            "tables\\styles.xml",
            "https://example.test/styles.xml",
            "%2e%2e/tableStyles.xml",
            "../../../tableStyles.xml",
        ] {
            assert!(resolve_internal_target("ppt/presentation.xml", target).is_err());
        }
    }
}
