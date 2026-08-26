use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::attribute;
use crate::{DocumentError, DocumentResult};

const WORKSHEET_REL_SUFFIX: &str = "/relationships/worksheet";

pub(super) struct SheetPart {
    pub(super) name: String,
    pub(super) path: String,
}

pub(super) fn parse_workbook(
    workbook: &str,
    relationships: &str,
) -> DocumentResult<Vec<SheetPart>> {
    let targets = parse_relationships(relationships)?;
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
                let target = targets
                    .get(&relationship)
                    .ok_or(DocumentError::UnsupportedFormat)?;
                sheets.push(SheetPart {
                    name,
                    path: worksheet_path(target)?,
                });
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if sheets.is_empty() {
        return Err(DocumentError::UnsupportedFormat);
    }
    Ok(sheets)
}

fn parse_relationships(xml: &str) -> DocumentResult<HashMap<String, String>> {
    let mut reader = Reader::from_str(xml);
    let mut targets = HashMap::new();
    loop {
        match reader.read_event()? {
            Event::Empty(element) | Event::Start(element)
                if element.local_name().as_ref() == b"Relationship" =>
            {
                let kind = attribute(&element, b"Type")?.unwrap_or_default();
                if !kind.ends_with(WORKSHEET_REL_SUFFIX) {
                    continue;
                }
                if attribute(&element, b"TargetMode")?.is_some() {
                    return Err(DocumentError::UnsupportedFormat);
                }
                let id = attribute(&element, b"Id")?.ok_or(DocumentError::UnsupportedFormat)?;
                let target =
                    attribute(&element, b"Target")?.ok_or(DocumentError::UnsupportedFormat)?;
                if targets.insert(id, target).is_some() {
                    return Err(DocumentError::UnsupportedFormat);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(targets)
}

fn worksheet_path(target: &str) -> DocumentResult<String> {
    let target = target.trim_start_matches('/');
    let path = if target.starts_with("xl/") {
        target.to_owned()
    } else {
        format!("xl/{target}")
    };
    if path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "..")
        || !path.starts_with("xl/worksheets/")
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
