use quick_xml::Reader;
use quick_xml::events::Event;

use super::{SpreadsheetCell, attribute};
use crate::{DocumentError, DocumentResult};

struct PendingCell {
    coordinate: String,
    kind: Option<String>,
    value: String,
    inline: String,
    in_value: bool,
    in_inline_text: bool,
}

pub(super) fn parse_worksheet(
    xml: &str,
    worksheet: &str,
    shared_strings: &[String],
) -> DocumentResult<Vec<SpreadsheetCell>> {
    let mut reader = Reader::from_str(xml);
    let mut pending = None;
    let mut cells = Vec::new();
    loop {
        match reader.read_event()? {
            Event::Start(element) if element.local_name().as_ref() == b"c" => {
                if pending.is_some() {
                    return Err(DocumentError::UnsupportedFormat);
                }
                let coordinate = attribute(&element, b"r")?
                    .filter(|value| valid_coordinate(value))
                    .ok_or(DocumentError::UnsupportedFormat)?;
                pending = Some(PendingCell {
                    coordinate,
                    kind: attribute(&element, b"t")?,
                    value: String::new(),
                    inline: String::new(),
                    in_value: false,
                    in_inline_text: false,
                });
            }
            Event::Start(element) if element.local_name().as_ref() == b"v" => {
                if let Some(cell) = &mut pending {
                    cell.in_value = true;
                }
            }
            Event::Start(element) if element.local_name().as_ref() == b"t" => {
                if let Some(cell) = &mut pending {
                    cell.in_inline_text = true;
                }
            }
            Event::Text(text) => {
                let decoded = text.unescape()?;
                if let Some(cell) = &mut pending {
                    if cell.in_value {
                        cell.value.push_str(&decoded);
                    }
                    if cell.in_inline_text {
                        cell.inline.push_str(&decoded);
                    }
                }
            }
            Event::End(element) if element.local_name().as_ref() == b"v" => {
                if let Some(cell) = &mut pending {
                    cell.in_value = false;
                }
            }
            Event::End(element) if element.local_name().as_ref() == b"t" => {
                if let Some(cell) = &mut pending {
                    cell.in_inline_text = false;
                }
            }
            Event::End(element) if element.local_name().as_ref() == b"c" => {
                let cell = pending.take().ok_or(DocumentError::UnsupportedFormat)?;
                if let Some(displayed_value) = displayed_value(&cell, shared_strings)?
                    && !displayed_value.is_empty()
                {
                    cells.push(SpreadsheetCell {
                        worksheet: worksheet.to_owned(),
                        coordinate: cell.coordinate,
                        displayed_value,
                    });
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if pending.is_some() {
        return Err(DocumentError::UnsupportedFormat);
    }
    Ok(cells)
}

fn displayed_value(
    cell: &PendingCell,
    shared_strings: &[String],
) -> DocumentResult<Option<String>> {
    match cell.kind.as_deref() {
        Some("inlineStr") => Ok(Some(cell.inline.clone())),
        Some("s") => {
            let index = cell
                .value
                .parse::<usize>()
                .map_err(|_| DocumentError::UnsupportedFormat)?;
            Ok(Some(
                shared_strings
                    .get(index)
                    .ok_or(DocumentError::UnsupportedFormat)?
                    .clone(),
            ))
        }
        Some("b") => match cell.value.as_str() {
            "0" => Ok(Some("FALSE".to_owned())),
            "1" => Ok(Some("TRUE".to_owned())),
            _ => Err(DocumentError::UnsupportedFormat),
        },
        Some("str" | "e" | "n") | None => Ok((!cell.value.is_empty()).then(|| cell.value.clone())),
        Some(_) => Err(DocumentError::UnsupportedFormat),
    }
}

fn valid_coordinate(value: &str) -> bool {
    let split = value
        .bytes()
        .position(|byte| byte.is_ascii_digit())
        .unwrap_or(value.len());
    let (column, row) = value.split_at(split);
    if column.is_empty()
        || column.len() > 3
        || !column.bytes().all(|byte| byte.is_ascii_uppercase())
        || row.starts_with('0')
    {
        return false;
    }
    let Some(row) = row.parse::<u32>().ok() else {
        return false;
    };
    let column = column
        .bytes()
        .fold(0_u32, |value, byte| value * 26 + u32::from(byte - b'A' + 1));
    column <= 16_384 && (1..=1_048_576).contains(&row)
}
