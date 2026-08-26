//! Number-format resolution for displayed cell text.
//!
//! Only a bounded subset of ECMA-376 format codes is emulated, because a full
//! format engine would be guesswork. A cell whose format is understood renders
//! its formatted text; a cell whose format changes the visible value in a way
//! this module cannot reproduce is reported as unsupported so the caller can
//! fail closed instead of silently publishing the raw stored number.

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::attribute;
use crate::{DocumentError, DocumentResult};

/// How a cell's stored value must be turned into visible text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NumberFormat {
    /// Render the stored value as-is.
    General,
    /// Multiply by 100 and append `%`, with the given decimal precision.
    Percent { decimals: usize },
    /// Serial date rendered as an ISO calendar date.
    IsoDate,
    /// Serial date rendered as an ISO date and time.
    IsoDateTime,
    /// The format alters the visible value but is not emulated here.
    Unsupported,
}

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
            .copied()
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

/// Built-in format ids defined by ECMA-376 that this module reproduces.
///
/// The General/text/plain-numeric ids common in real corpora keep the stored
/// text, since the stored digits are what a spreadsheet shows for them.
fn builtin_format(id: &str) -> NumberFormat {
    match id {
        // 0 General, 1 `0`, 2 `0.00`, 3/4 thousands-separated, 49 Text.
        "0" | "1" | "2" | "3" | "4" | "49" => NumberFormat::General,
        // 9 is `0%`, 10 is `0.00%`.
        "9" => NumberFormat::Percent { decimals: 0 },
        "10" => NumberFormat::Percent { decimals: 2 },
        // 14-17 are date forms; 22 is date with time.
        "14" | "15" | "16" | "17" => NumberFormat::IsoDate,
        "22" => NumberFormat::IsoDateTime,
        // Any other built-in changes the rendering in a way this module does
        // not reproduce. The cell still converts; it simply cannot be used to
        // claim a coordinate.
        _ => NumberFormat::Unsupported,
    }
}

/// Classifies a custom format code, ignoring literal and colour sections.
fn classify_format_code(code: &str) -> NumberFormat {
    // Only the positive section governs the values considered here.
    let section = code.split(';').next().unwrap_or(code);
    let stripped = strip_literals(section);
    if stripped.is_empty() {
        return NumberFormat::Unsupported;
    }
    if stripped.contains('%') {
        return percent_format(&stripped);
    }
    if is_date_code(&stripped) {
        return if is_date_time_code(&stripped) {
            NumberFormat::IsoDateTime
        } else {
            NumberFormat::IsoDate
        };
    }
    // A pure numeric placeholder run keeps the stored digits.
    if stripped
        .chars()
        .all(|value| matches!(value, '0' | '#' | '.' | ',' | '-' | '+' | ' ' | '?'))
    {
        return NumberFormat::General;
    }
    NumberFormat::Unsupported
}

/// Removes quoted literals, escapes, colours and locale hints so only the
/// structural placeholders remain.
fn strip_literals(code: &str) -> String {
    let mut output = String::new();
    let mut chars = code.chars();
    while let Some(value) = chars.next() {
        match value {
            '"' => {
                for inner in chars.by_ref() {
                    if inner == '"' {
                        break;
                    }
                }
            }
            '[' => {
                for inner in chars.by_ref() {
                    if inner == ']' {
                        break;
                    }
                }
            }
            '\\' | '_' => {
                chars.next();
            }
            '*' => {}
            other => output.push(other),
        }
    }
    output
}

fn percent_format(code: &str) -> NumberFormat {
    let numeric = code.replace('%', "");
    let decimals = numeric
        .split_once('.')
        .map(|(_, fraction)| fraction.chars().filter(|value| *value == '0').count())
        .unwrap_or(0);
    // Anything beyond digit placeholders and separators is not reproduced.
    if numeric
        .chars()
        .all(|value| matches!(value, '0' | '#' | '.' | ',' | ' ' | '?'))
    {
        NumberFormat::Percent { decimals }
    } else {
        NumberFormat::Unsupported
    }
}

fn is_date_code(code: &str) -> bool {
    let lowered = code.to_ascii_lowercase();
    lowered.contains('y') || lowered.contains('d') || lowered.contains('m')
}

fn is_date_time_code(code: &str) -> bool {
    let lowered = code.to_ascii_lowercase();
    lowered.contains('h') || lowered.contains('s')
}
