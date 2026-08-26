use std::collections::{BTreeMap, BTreeSet};

use document2html_core::{SpreadsheetCell, SpreadsheetSemantics};

#[derive(Clone)]
struct DecodedChar {
    value: char,
    start: usize,
    end: usize,
}

struct Annotation {
    start: usize,
    end: usize,
    cell: usize,
    key: String,
}

pub(crate) fn annotate_spreadsheet_html(html: &str, semantics: &SpreadsheetSemantics) -> String {
    let Some(body_start) = html.find("<body") else {
        return html.to_owned();
    };
    let Some(relative_start) = html[body_start..].find('>') else {
        return html.to_owned();
    };
    let content_start = body_start + relative_start + 1;
    let Some(relative_end) = html[content_start..].find("</body>") else {
        return html.to_owned();
    };
    let content_end = content_start + relative_end;
    let text_nodes = text_nodes(html, content_start, content_end);
    let mut cells_by_value: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, cell) in semantics.cells.iter().enumerate() {
        let value = normalize(&cell.displayed_value);
        if !value.is_empty() {
            cells_by_value.entry(value).or_default().push(index);
        }
    }
    let mut annotations = Vec::new();
    for (value, cells) in cells_by_value {
        let mut occurrences = Vec::new();
        for (start, end) in &text_nodes {
            occurrences.extend(find_occurrences(&html[*start..*end], *start, &value));
        }
        if occurrences.len() != cells.len() {
            continue;
        }
        annotations.extend(
            occurrences
                .into_iter()
                .zip(cells)
                .map(|((start, end), cell)| Annotation {
                    start,
                    end,
                    cell,
                    key: value.clone(),
                }),
        );
    }
    let conflicts = conflicting_keys(&annotations);
    annotations.retain(|annotation| !conflicts.contains(&annotation.key));
    annotations.sort_by_key(|annotation| annotation.start);
    let mut output = html.to_owned();
    for annotation in annotations.into_iter().rev() {
        let cell = &semantics.cells[annotation.cell];
        output.insert_str(annotation.end, "</span>");
        output.insert_str(annotation.start, &opening_span(cell));
    }
    output
}

fn text_nodes(html: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let mut nodes = Vec::new();
    let mut cursor = start;
    while cursor < end {
        let Some(tag_offset) = html[cursor..end].find('<') else {
            nodes.push((cursor, end));
            break;
        };
        let tag_start = cursor + tag_offset;
        if tag_start > cursor {
            nodes.push((cursor, tag_start));
        }
        let Some(tag_end) = html[tag_start..end].find('>') else {
            break;
        };
        cursor = tag_start + tag_end + 1;
    }
    nodes
}

fn find_occurrences(raw: &str, offset: usize, needle: &str) -> Vec<(usize, usize)> {
    let decoded = normalized_chars(raw);
    let needle = normalize(needle).chars().collect::<Vec<_>>();
    if needle.is_empty() || decoded.len() < needle.len() {
        return Vec::new();
    }
    decoded
        .windows(needle.len())
        .enumerate()
        .filter_map(|(index, window)| {
            if window
                .iter()
                .zip(&needle)
                .all(|(actual, expected)| actual.value == *expected)
                && boundary(&decoded, index, needle.len())
            {
                Some((
                    offset + window[0].start,
                    offset + window[needle.len() - 1].end,
                ))
            } else {
                None
            }
        })
        .collect()
}

fn boundary(value: &[DecodedChar], start: usize, length: usize) -> bool {
    let left = start == 0 || value[start - 1].value == ' ';
    let end = start + length;
    let right = end == value.len() || value[end].value == ' ';
    left && right
}

fn normalized_chars(raw: &str) -> Vec<DecodedChar> {
    let mut decoded = decode_chars(raw);
    let mut normalized: Vec<DecodedChar> = Vec::new();
    for mut value in decoded.drain(..) {
        if value.value.is_whitespace() {
            if normalized.is_empty() || normalized.last().is_some_and(|last| last.value == ' ') {
                continue;
            }
            value.value = ' ';
        }
        normalized.push(value);
    }
    if normalized.last().is_some_and(|last| last.value == ' ') {
        normalized.pop();
    }
    normalized
}

fn decode_chars(raw: &str) -> Vec<DecodedChar> {
    let mut values = Vec::new();
    let mut cursor = 0;
    while cursor < raw.len() {
        if raw.as_bytes()[cursor] == b'&'
            && let Some(relative_end) = raw[cursor..].find(';')
        {
            let end = cursor + relative_end + 1;
            if let Some(value) = decode_entity(&raw[cursor + 1..end - 1]) {
                values.push(DecodedChar {
                    value,
                    start: cursor,
                    end,
                });
                cursor = end;
                continue;
            }
        }
        let Some(value) = raw[cursor..].chars().next() else {
            break;
        };
        let end = cursor + value.len_utf8();
        values.push(DecodedChar {
            value,
            start: cursor,
            end,
        });
        cursor = end;
    }
    values
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        value if value.starts_with("#x") || value.starts_with("#X") => {
            char::from_u32(u32::from_str_radix(&value[2..], 16).ok()?)
        }
        value if value.starts_with('#') => char::from_u32(value[1..].parse().ok()?),
        _ => None,
    }
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn conflicting_keys(annotations: &[Annotation]) -> BTreeSet<String> {
    let mut conflicts = BTreeSet::new();
    for (index, left) in annotations.iter().enumerate() {
        for right in &annotations[index + 1..] {
            if left.start < right.end && right.start < left.end {
                conflicts.insert(left.key.clone());
                conflicts.insert(right.key.clone());
            }
        }
    }
    conflicts
}

fn opening_span(cell: &SpreadsheetCell) -> String {
    format!(
        "<span data-cell-coordinate=\"{}\" data-worksheet=\"{}\">",
        escape_attribute(&cell.coordinate),
        escape_attribute(&cell.worksheet)
    )
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
#[path = "spreadsheet_html_tests.rs"]
mod tests;
