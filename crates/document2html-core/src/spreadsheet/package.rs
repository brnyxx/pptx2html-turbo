use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::attribute;
use crate::{DocumentError, DocumentResult};

const WORKSHEET_REL_SUFFIX: &str = "/relationships/worksheet";
const CHARTSHEET_REL_SUFFIX: &str = "/relationships/chartsheet";
const WORKSHEET_PREFIX: &str = "xl/worksheets/";
const CHARTSHEET_PREFIX: &str = "xl/chartsheets/";

enum SheetRelationship {
    Worksheet(String),
    Chartsheet,
    Other,
}

pub(super) struct SheetPart {
    pub(super) name: String,
    pub(super) path: String,
}

pub(super) fn parse_workbook(
    workbook: &str,
    relationships: &str,
) -> DocumentResult<Vec<SheetPart>> {
    let relationships = parse_relationships(relationships)?;
    let mut reader = Reader::from_str(workbook);
    let mut sheets = Vec::new();
    loop {
        match reader.read_event()? {
            Event::Empty(element) | Event::Start(element)
                if element.local_name().as_ref() == b"sheet" =>
            {
                let name = attribute(&element, b"name")?
                    .filter(|value| !value.is_empty())
                    .ok_or(DocumentError::UnsupportedFormat)?;
                let relationship =
                    attribute(&element, b"id")?.ok_or(DocumentError::UnsupportedFormat)?;
                let target = relationships
                    .get(&relationship)
                    .ok_or(DocumentError::UnsupportedFormat)?;
                match target {
                    SheetRelationship::Worksheet(path) => sheets.push(SheetPart {
                        name,
                        path: path.clone(),
                    }),
                    SheetRelationship::Chartsheet => {}
                    SheetRelationship::Other => return Err(DocumentError::UnsupportedFormat),
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(sheets)
}

fn parse_relationships(xml: &str) -> DocumentResult<HashMap<String, SheetRelationship>> {
    let mut reader = Reader::from_str(xml);
    let mut relationships = HashMap::new();
    loop {
        match reader.read_event()? {
            Event::Empty(element) | Event::Start(element)
                if element.local_name().as_ref() == b"Relationship" =>
            {
                let id = attribute(&element, b"Id")?.ok_or(DocumentError::UnsupportedFormat)?;
                let kind = attribute(&element, b"Type")?.unwrap_or_default();
                let relationship = if kind.ends_with(WORKSHEET_REL_SUFFIX) {
                    let target_mode = attribute(&element, b"TargetMode")?;
                    let target =
                        attribute(&element, b"Target")?.ok_or(DocumentError::UnsupportedFormat)?;
                    if target_mode.is_some() {
                        return Err(DocumentError::UnsupportedFormat);
                    }
                    SheetRelationship::Worksheet(sheet_path(&target, WORKSHEET_PREFIX)?)
                } else if kind.ends_with(CHARTSHEET_REL_SUFFIX) {
                    let target_mode = attribute(&element, b"TargetMode")?;
                    let target =
                        attribute(&element, b"Target")?.ok_or(DocumentError::UnsupportedFormat)?;
                    if target_mode.is_some() {
                        return Err(DocumentError::UnsupportedFormat);
                    }
                    sheet_path(&target, CHARTSHEET_PREFIX)?;
                    SheetRelationship::Chartsheet
                } else {
                    SheetRelationship::Other
                };
                if relationships.insert(id, relationship).is_some() {
                    return Err(DocumentError::UnsupportedFormat);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(relationships)
}

fn sheet_path(target: &str, prefix: &str) -> DocumentResult<String> {
    let target = target.trim_start_matches('/');
    let path = if target.starts_with("xl/") {
        target.to_owned()
    } else {
        format!("xl/{target}")
    };
    if path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "..")
        || !path.starts_with(prefix)
        || !path.ends_with(".xml")
    {
        return Err(DocumentError::UnsupportedFormat);
    }
    Ok(path)
}

pub(super) fn parse_shared_strings(xml: &str) -> DocumentResult<Vec<String>> {
    let mut reader = Reader::from_str(xml);
    let mut values = Vec::new();
    let mut current = None;
    let mut in_text = false;
    loop {
        match reader.read_event()? {
            Event::Start(element) if element.local_name().as_ref() == b"si" => {
                current = Some(String::new());
            }
            Event::Start(element) if element.local_name().as_ref() == b"t" => in_text = true,
            Event::Text(text) if in_text => {
                if let Some(value) = &mut current {
                    value.push_str(&text.unescape()?);
                }
            }
            Event::End(element) if element.local_name().as_ref() == b"t" => in_text = false,
            Event::End(element) if element.local_name().as_ref() == b"si" => {
                values.push(current.take().ok_or(DocumentError::UnsupportedFormat)?);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(values)
}
