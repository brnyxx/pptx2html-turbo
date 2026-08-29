use quick_xml::Reader;
use quick_xml::events::Event;

use super::display::{Display, formatted_value, iso_date_text};
use super::styles::StyleTable;
use super::{SpreadsheetCell, attribute};
use crate::{DocumentError, DocumentResult};

struct PendingCell {
    coordinate: String,
    kind: Option<String>,
    style: Option<String>,
    value: String,
    inline: String,
    in_value: bool,
    in_inline_text: bool,
}

pub(super) fn parse_worksheet(
    xml: &str,
    worksheet: &str,
    shared_strings: &[String],
    styles: &StyleTable,
) -> DocumentResult<Vec<SpreadsheetCell>> {
    let mut reader = Reader::from_str(xml);
    let mut pending = None;
    let mut cells = Vec::new();
    let mut current_row = None;
    let mut previous_row = None;
    let mut previous_column = None;
    loop {
        match reader.read_event()? {
            Event::Start(element) if element.local_name().as_ref() == b"row" => {
                if current_row.is_some() {
                    return Err(DocumentError::UnsupportedFormat);
                }
                let row = row_number(attribute(&element, b"r")?, previous_row)?;
                current_row = Some(row);
                previous_column = None;
            }
            Event::Start(element) if element.local_name().as_ref() == b"c" => {
                if pending.is_some() {
                    return Err(DocumentError::UnsupportedFormat);
                }
                let coordinate = cell_coordinate(
                    attribute(&element, b"r")?,
                    current_row,
                    &mut previous_column,
                )?;
                pending = Some(PendingCell {
                    coordinate,
                    kind: attribute(&element, b"t")?,
                    style: attribute(&element, b"s")?,
                    value: String::new(),
                    inline: String::new(),
                    in_value: false,
                    in_inline_text: false,
                });
            }
            Event::Empty(element) if element.local_name().as_ref() == b"row" => {
                if current_row.is_some() {
                    return Err(DocumentError::UnsupportedFormat);
                }
                previous_row = Some(row_number(attribute(&element, b"r")?, previous_row)?);
            }
            Event::Empty(element) if element.local_name().as_ref() == b"c" => {
                if pending.is_some() {
                    return Err(DocumentError::UnsupportedFormat);
                }
                let coordinate = cell_coordinate(
                    attribute(&element, b"r")?,
                    current_row,
                    &mut previous_column,
                )?;
                let cell = PendingCell {
                    coordinate,
                    kind: attribute(&element, b"t")?,
                    style: attribute(&element, b"s")?,
                    value: String::new(),
                    inline: String::new(),
                    in_value: false,
                    in_inline_text: false,
                };
                finish_cell(cell, worksheet, shared_strings, styles, &mut cells)?;
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
                finish_cell(cell, worksheet, shared_strings, styles, &mut cells)?;
            }
            Event::End(element) if element.local_name().as_ref() == b"row" => {
                if pending.is_some() {
                    return Err(DocumentError::UnsupportedFormat);
                }
                previous_row = Some(current_row.take().ok_or(DocumentError::UnsupportedFormat)?);
                previous_column = None;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if pending.is_some() || current_row.is_some() {
        return Err(DocumentError::UnsupportedFormat);
    }
    Ok(cells)
}

fn finish_cell(
    cell: PendingCell,
    worksheet: &str,
    shared_strings: &[String],
    styles: &StyleTable,
    cells: &mut Vec<SpreadsheetCell>,
) -> DocumentResult<()> {
    let coordinate = cell.coordinate.clone();
    match displayed_value(&cell, shared_strings, styles)? {
        Some(Display::Trusted(displayed_value)) if !displayed_value.is_empty() => {
            cells.push(SpreadsheetCell {
                worksheet: worksheet.to_owned(),
                coordinate,
                displayed_value,
                attributable: true,
            });
        }
        // The value converts but its visible text cannot be reproduced, so it
        // is recorded as unattributable instead of being dropped or guessed.
        Some(Display::Unattributable) => cells.push(SpreadsheetCell {
            worksheet: worksheet.to_owned(),
            coordinate,
            displayed_value: String::new(),
            attributable: false,
        }),
        _ => {}
    }
    Ok(())
}

fn displayed_value(
    cell: &PendingCell,
    shared_strings: &[String],
    styles: &StyleTable,
) -> DocumentResult<Option<Display>> {
    match cell.kind.as_deref() {
        Some("inlineStr") => Ok(Some(Display::Trusted(cell.inline.clone()))),
        Some("s") => {
            let index = cell
                .value
                .parse::<usize>()
                .map_err(|_| DocumentError::UnsupportedFormat)?;
            Ok(Some(Display::Trusted(
                shared_strings
                    .get(index)
                    .ok_or(DocumentError::UnsupportedFormat)?
                    .clone(),
            )))
        }
        Some("b") => match cell.value.as_str() {
            "0" => Ok(Some(Display::Trusted("FALSE".to_owned()))),
            "1" => Ok(Some(Display::Trusted("TRUE".to_owned()))),
            _ => Err(DocumentError::UnsupportedFormat),
        },
        // ECMA-376 ISO 8601 date cells convert directly from their text form.
        Some("d") => Ok(Some(iso_date_text(&cell.value))),
        // Text-ish values display verbatim; numbers go through the number
        // format so percentages and serial dates render as displayed.
        Some("str" | "e") => {
            Ok((!cell.value.is_empty()).then(|| Display::Trusted(cell.value.clone())))
        }
        Some("n") | None => Ok((!cell.value.is_empty())
            .then(|| formatted_value(&cell.value, &styles.format(cell.style.as_deref())))),
        Some(_) => Err(DocumentError::UnsupportedFormat),
    }
}

fn row_number(value: Option<String>, previous_row: Option<u32>) -> DocumentResult<u32> {
    let row = match value {
        Some(value) => {
            let row = parse_row(&value).ok_or(DocumentError::UnsupportedFormat)?;
            if previous_row.is_some_and(|previous| row <= previous) {
                return Err(DocumentError::UnsupportedFormat);
            }
            row
        }
        None => previous_row
            .unwrap_or(0)
            .checked_add(1)
            .filter(|row| *row <= 1_048_576)
            .ok_or(DocumentError::UnsupportedFormat)?,
    };
    Ok(row)
}

fn cell_coordinate(
    value: Option<String>,
    current_row: Option<u32>,
    previous_column: &mut Option<u32>,
) -> DocumentResult<String> {
    let row = current_row.ok_or(DocumentError::UnsupportedFormat)?;
    let (column, coordinate) = match value {
        Some(value) => {
            let (column, coordinate_row) =
                parse_coordinate(&value).ok_or(DocumentError::UnsupportedFormat)?;
            if coordinate_row != row || previous_column.is_some_and(|previous| column <= previous) {
                return Err(DocumentError::UnsupportedFormat);
            }
            (column, value)
        }
        None => {
            let column = previous_column
                .unwrap_or(0)
                .checked_add(1)
                .filter(|column| *column <= 16_384)
                .ok_or(DocumentError::UnsupportedFormat)?;
            (column, format!("{}{}", column_name(column), row))
        }
    };
    *previous_column = Some(column);
    Ok(coordinate)
}

fn parse_coordinate(value: &str) -> Option<(u32, u32)> {
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
        return None;
    }
    let row = parse_row(row)?;
    let column = column
        .bytes()
        .fold(0_u32, |value, byte| value * 26 + u32::from(byte - b'A' + 1));
    (column <= 16_384).then_some((column, row))
}

fn parse_row(value: &str) -> Option<u32> {
    if value.is_empty()
        || value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let row = value.parse::<u32>().ok()?;
    (1..=1_048_576).contains(&row).then_some(row)
}

fn column_name(mut column: u32) -> String {
    let mut name = String::new();
    while column > 0 {
        let remainder = (column - 1) % 26;
        name.push(char::from(b'A' + remainder as u8));
        column = (column - 1) / 26;
    }
    name.chars().rev().collect()
}
