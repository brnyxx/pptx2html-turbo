use std::io::{Cursor, Read};

use quick_xml::events::BytesStart;
use zip::ZipArchive;

use crate::{DocumentError, DocumentResult};

mod package;
mod worksheet;

const MAX_SEMANTIC_PART_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadsheetCell {
    pub worksheet: String,
    pub coordinate: String,
    pub displayed_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadsheetSemantics {
    pub cells: Vec<SpreadsheetCell>,
}

pub fn parse_xlsx_semantics(data: &[u8]) -> DocumentResult<SpreadsheetSemantics> {
    let mut archive = ZipArchive::new(Cursor::new(data))?;
    let workbook = read_required_part(&mut archive, "xl/workbook.xml")?;
    let relationships = read_required_part(&mut archive, "xl/_rels/workbook.xml.rels")?;
    let shared_strings = read_optional_part(&mut archive, "xl/sharedStrings.xml")?
        .map(|xml| package::parse_shared_strings(&xml))
        .transpose()?
        .unwrap_or_default();
    let sheets = package::parse_workbook(&workbook, &relationships)?;
    let mut cells = Vec::new();
    for sheet in sheets {
        let xml = read_required_part(&mut archive, &sheet.path)?;
        cells.extend(worksheet::parse_worksheet(
            &xml,
            &sheet.name,
            &shared_strings,
        )?);
    }
    Ok(SpreadsheetSemantics { cells })
}

fn read_required_part(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> DocumentResult<String> {
    read_optional_part(archive, name)?
        .ok_or_else(|| DocumentError::MissingPackagePart(name.to_owned()))
}

fn read_optional_part(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> DocumentResult<Option<String>> {
    let mut file = match archive.by_name(name) {
        Ok(file) => file,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if file.size() > MAX_SEMANTIC_PART_BYTES {
        return Err(DocumentError::PackageMetadataTooLarge {
            part: name.to_owned(),
            limit: MAX_SEMANTIC_PART_BYTES,
        });
    }
    let mut xml = String::new();
    file.read_to_string(&mut xml)?;
    Ok(Some(xml))
}

fn attribute(element: &BytesStart<'_>, key: &[u8]) -> DocumentResult<Option<String>> {
    let mut result = None;
    for value in element.attributes().with_checks(true) {
        let value = value.map_err(quick_xml::Error::from)?;
        if value.key.local_name().as_ref() != key {
            continue;
        }
        if result.is_some() {
            return Err(DocumentError::UnsupportedFormat);
        }
        result = Some(value.unescape_value()?.into_owned());
    }
    Ok(result)
}
