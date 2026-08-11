use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::xml_utils;
use crate::error::PptxResult;

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
                    let value = String::from_utf8_lossy(&attr.value).to_string();
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
                if !id.is_empty() && !target.is_empty() {
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
