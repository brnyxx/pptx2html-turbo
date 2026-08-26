use std::io::{Cursor, Read};

use quick_xml::events::BytesStart;
use zip::ZipArchive;

use crate::{DocumentError, DocumentResult};

mod display;
mod package;
mod styles;
mod worksheet;

const MAX_SEMANTIC_PART_BYTES: u64 = 256 * 1024 * 1024;

/// Ceiling on the total decompressed bytes read from one package. A per-part
/// limit alone still admits an archive of many large parts, so the aggregate
/// is tracked across every part read for a single workbook.
const MAX_SEMANTIC_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

/// Tracks decompressed bytes consumed while reading one package.
struct PartBudget {
    remaining: u64,
}

impl PartBudget {
    const fn new() -> Self {
        Self {
            remaining: MAX_SEMANTIC_TOTAL_BYTES,
        }
    }

    fn charge(&mut self, part: &str, bytes: u64) -> DocumentResult<()> {
        self.remaining = self.remaining.checked_sub(bytes).ok_or_else(|| {
            DocumentError::PackageMetadataTooLarge {
                part: part.to_owned(),
                limit: MAX_SEMANTIC_TOTAL_BYTES,
            }
        })?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadsheetCell {
    pub worksheet: String,
    pub coordinate: String,
    pub displayed_value: String,
    /// Whether `displayed_value` is trusted to match the rendered text.
    ///
    /// A cell whose number format cannot be reproduced still converts, but it
    /// is excluded from coordinate attribution so no coordinate is claimed for
    /// text that may differ from what a spreadsheet application shows.
    pub attributable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadsheetSemantics {
    pub cells: Vec<SpreadsheetCell>,
}

pub fn parse_xlsx_semantics(data: &[u8]) -> DocumentResult<SpreadsheetSemantics> {
    let mut archive = ZipArchive::new(Cursor::new(data))?;
    let mut budget = PartBudget::new();
    let workbook = read_required_part(&mut archive, "xl/workbook.xml", &mut budget)?;
    let relationships =
        read_required_part(&mut archive, "xl/_rels/workbook.xml.rels", &mut budget)?;
    let shared_strings = read_optional_part(&mut archive, "xl/sharedStrings.xml", &mut budget)?
        .map(|xml| package::parse_shared_strings(&xml))
        .transpose()?
        .unwrap_or_default();
    let styles = read_optional_part(&mut archive, "xl/styles.xml", &mut budget)?
        .map(|xml| styles::parse_styles(&xml))
        .transpose()?
        .unwrap_or_default();
    let sheets = package::parse_workbook(&workbook, &relationships)?;
    let mut cells = Vec::new();
    for sheet in sheets {
        let xml = read_required_part(&mut archive, &sheet.path, &mut budget)?;
        cells.extend(worksheet::parse_worksheet(
            &xml,
            &sheet.name,
            &shared_strings,
            &styles,
        )?);
    }
    Ok(SpreadsheetSemantics { cells })
}

fn read_required_part(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    budget: &mut PartBudget,
) -> DocumentResult<String> {
    read_optional_part(archive, name, budget)?
        .ok_or_else(|| DocumentError::MissingPackagePart(name.to_owned()))
}

fn read_optional_part(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    budget: &mut PartBudget,
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
    // Charge the declared size first so an oversized declaration is rejected
    // before any decompression work happens.
    budget.charge(name, file.size())?;
    // The declared size is attacker-controlled, so cap the actual read too and
    // reject any part that decompresses beyond what it declared.
    let allowance = file.size().min(MAX_SEMANTIC_PART_BYTES);
    let mut xml = String::new();
    let read = file
        .by_ref()
        .take(allowance.saturating_add(1))
        .read_to_string(&mut xml)? as u64;
    if read > allowance {
        return Err(DocumentError::PackageMetadataTooLarge {
            part: name.to_owned(),
            limit: MAX_SEMANTIC_PART_BYTES,
        });
    }
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

#[cfg(test)]
mod tests {
    use super::{MAX_SEMANTIC_PART_BYTES, MAX_SEMANTIC_TOTAL_BYTES, PartBudget};
    use crate::DocumentError;

    /// Many parts that each satisfy the per-part limit must still be refused
    /// once their combined decompressed size exceeds the aggregate ceiling.
    #[test]
    fn aggregate_budget_refuses_many_individually_legal_parts() {
        let mut budget = PartBudget::new();
        let part = MAX_SEMANTIC_PART_BYTES;
        let allowed = MAX_SEMANTIC_TOTAL_BYTES / part;

        for index in 0..allowed {
            budget
                .charge("xl/worksheets/sheet.xml", part)
                .unwrap_or_else(|error| panic!("charge {index} must fit: {error}"));
        }

        let error = budget
            .charge("xl/worksheets/overflow.xml", part)
            .expect_err("aggregate ceiling must reject the overflowing part");
        match error {
            DocumentError::PackageMetadataTooLarge { part, limit } => {
                assert_eq!(part, "xl/worksheets/overflow.xml");
                assert_eq!(limit, MAX_SEMANTIC_TOTAL_BYTES);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    /// Ordinary workbooks with several modest parts stay well inside the
    /// aggregate ceiling.
    #[test]
    fn aggregate_budget_admits_ordinary_multi_part_workbooks() {
        let mut budget = PartBudget::new();
        for name in ["xl/workbook.xml", "xl/sharedStrings.xml"] {
            budget.charge(name, 4 * 1024 * 1024).expect("typical part");
        }
    }
}
