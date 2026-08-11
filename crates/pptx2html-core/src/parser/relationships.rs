use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

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
    let mut reader = Reader::from_str(xml);
    let mut relationships = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                let name = e.name();
                if xml_utils::local_name(name.as_ref()) != "Relationship" {
                    continue;
                }
                let mut id = String::new();
                let mut relationship_type = String::new();
                let mut target = String::new();
                let mut target_mode = TargetMode::Internal;
                for attr in e.attributes().flatten() {
                    let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                    let value = attr.unescape_value()?.into_owned();
                    match key {
                        "Id" => id = value,
                        "Type" => relationship_type = value,
                        "Target" => target = value,
                        "TargetMode" if value == "External" => {
                            target_mode = TargetMode::External;
                        }
                        "TargetMode" if value == "Internal" => {
                            target_mode = TargetMode::Internal;
                        }
                        "TargetMode" => target_mode = TargetMode::Other(value),
                        _ => {}
                    }
                }
                if !id.is_empty() {
                    relationships.push(Relationship {
                        id,
                        relationship_type,
                        target,
                        target_mode,
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(crate::error::PptxError::Xml(e)),
            _ => {}
        }
    }

    Ok(relationships)
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
        if matches!(segment, "." | "..") {
            return Err(TargetError::DotSegment);
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
    fn internal_target_is_resolved_relative_to_owner() {
        assert_eq!(
            resolve_internal_target("ppt/presentation.xml", "tableStyles.xml"),
            Ok("ppt/tableStyles.xml".to_owned())
        );
    }

    #[test]
    fn unsafe_targets_are_rejected() {
        for target in [
            "",
            "/ppt/tableStyles.xml",
            "../tableStyles.xml",
            "./tableStyles.xml",
            "tables//styles.xml",
            "tables\\styles.xml",
            "https://example.test/styles.xml",
            "%2e%2e/tableStyles.xml",
        ] {
            assert!(resolve_internal_target("ppt/presentation.xml", target).is_err());
        }
    }
}
