//! Number-format resolution for displayed cell text.
//!
//! Parses `xl/styles.xml` into a cell-format table. Format-code
//! classification lives in [`super::number`].

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::attribute;
use super::number::{NumberFormat, builtin_format, classify_format_code};
use crate::{DocumentError, DocumentResult};

/// Style table: cell format index to resolved number format.
#[derive(Debug, Default)]
pub(super) struct StyleTable {
    formats: Vec<NumberFormat>,
}

impl StyleTable {
    /// Resolves the format for a cell's `s` attribute.
    ///
    /// A missing style part or an out-of-range index means the workbook never
    /// applied a format, which renders as `General`.
    pub(super) fn format(&self, style_index: Option<&str>) -> NumberFormat {
        let Some(raw) = style_index else {
            return NumberFormat::General;
        };
        let Ok(index) = raw.parse::<usize>() else {
            return NumberFormat::Unsupported;
        };
        self.formats
            .get(index)
            .cloned()
            .unwrap_or(NumberFormat::General)
    }
}

/// Parses `xl/styles.xml` into the per-cell-format number format table.
pub(super) fn parse_styles(xml: &str) -> DocumentResult<StyleTable> {
    let custom = parse_custom_formats(xml)?;
    let mut reader = Reader::from_str(xml);
    let mut formats = Vec::new();
    let mut in_cell_xfs = false;
    loop {
        match reader.read_event()? {
            Event::Start(element) if element.local_name().as_ref() == b"cellXfs" => {
                in_cell_xfs = true;
            }
            Event::End(element) if element.local_name().as_ref() == b"cellXfs" => {
                in_cell_xfs = false;
            }
            Event::Empty(element) | Event::Start(element)
                if in_cell_xfs && element.local_name().as_ref() == b"xf" =>
            {
                let id = attribute(&element, b"numFmtId")?.unwrap_or_else(|| "0".to_owned());
                formats.push(resolve_format(&id, &custom));
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(StyleTable { formats })
}

/// Reads `numFmt` overrides, which define ids at or above 164.
fn parse_custom_formats(xml: &str) -> DocumentResult<HashMap<String, String>> {
    let mut reader = Reader::from_str(xml);
    let mut custom = HashMap::new();
    loop {
        match reader.read_event()? {
            Event::Empty(element) | Event::Start(element)
                if element.local_name().as_ref() == b"numFmt" =>
            {
                let id =
                    attribute(&element, b"numFmtId")?.ok_or(DocumentError::UnsupportedFormat)?;
                let code =
                    attribute(&element, b"formatCode")?.ok_or(DocumentError::UnsupportedFormat)?;
                custom.insert(id, code);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(custom)
}

fn resolve_format(id: &str, custom: &HashMap<String, String>) -> NumberFormat {
    if let Some(code) = custom.get(id) {
        return classify_format_code(code);
    }
    builtin_format(id)
}
