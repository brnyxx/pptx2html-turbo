//! Rendered-text extraction: locates the body, skips tags and raw-text
//! element bodies, and decodes character data into position-preserving
//! characters and whitespace-delimited words.

use std::ops::Range;

/// Elements whose character data is raw text rather than rendered content.
/// Injecting markup inside them would corrupt the stylesheet or script body,
/// so their contents never participate in cell matching.
const RAW_TEXT_ELEMENTS: [&str; 4] = ["style", "script", "textarea", "title"];

/// A decoded character together with the byte range it occupied in the source,
/// so matches can be spliced back at exact original offsets.
#[derive(Clone)]
pub(super) struct DecodedChar {
    pub(super) value: char,
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) fn body_range(html: &str) -> Option<Range<usize>> {
    let body_start = html.find("<body")?;
    let relative_start = html[body_start..].find('>')?;
    let start = body_start + relative_start + 1;
    let relative_end = html[start..].find("</body>")?;
    Some(start..start + relative_end)
}

/// Byte ranges of renderable character data inside the body, excluding tags
/// and the contents of raw-text elements.
pub(super) fn text_nodes(html: &str, content: &Range<usize>) -> Vec<(usize, usize)> {
    let mut nodes = Vec::new();
    let mut cursor = content.start;
    let end = content.end;
    while cursor < end {
        let Some(tag_offset) = html[cursor..end].find('<') else {
            nodes.push((cursor, end));
            break;
        };
        let tag_start = cursor + tag_offset;
        if tag_start > cursor {
            nodes.push((cursor, tag_start));
        }
        let Some(tag_length) = html[tag_start..end].find('>') else {
            break;
        };
        let tag_end = tag_start + tag_length + 1;
        cursor = match raw_text_element(&html[tag_start..tag_end]) {
            // Resume after the matching close tag so the raw body is skipped
            // entirely; an unterminated raw element consumes the remainder.
            Some(name) => find_close_tag(html, tag_end, end, name).unwrap_or(end),
            None => tag_end,
        };
    }
    nodes
}

/// Returns the element name when `tag` opens a raw-text element.
fn raw_text_element(tag: &str) -> Option<&'static str> {
    let inner = tag.strip_prefix('<')?.strip_suffix('>')?;
    if inner.starts_with('/') || inner.ends_with('/') {
        return None;
    }
    let name_end = inner
        .find(|value: char| value.is_whitespace())
        .unwrap_or(inner.len());
    let name = &inner[..name_end];
    RAW_TEXT_ELEMENTS
        .into_iter()
        .find(|candidate| name.eq_ignore_ascii_case(candidate))
}

/// Byte offset just past `</name>`, searched case-insensitively from `from`.
fn find_close_tag(html: &str, from: usize, end: usize, name: &str) -> Option<usize> {
    let mut cursor = from;
    while cursor < end {
        let relative = html[cursor..end].find("</")?;
        let candidate = cursor + relative;
        let after = &html[candidate + 2..end];
        if after.len() >= name.len() && after[..name.len()].eq_ignore_ascii_case(name) {
            if let Some(close) = after[name.len()..].find('>') {
                return Some(candidate + 2 + name.len() + close + 1);
            }
            return None;
        }
        cursor = candidate + 2;
    }
    None
}

/// Decodes character data and collapses whitespace runs to single spaces so
/// matching compares rendered text rather than source formatting.
pub(super) fn normalized_chars(raw: &str) -> Vec<DecodedChar> {
    let mut normalized: Vec<DecodedChar> = Vec::new();
    for mut value in decode_chars(raw) {
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

/// Character index ranges of whitespace-delimited words, as `[start, end)`.
pub(super) fn word_spans(decoded: &[DecodedChar]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut cursor = 0;
    while cursor < decoded.len() {
        if decoded[cursor].value == ' ' {
            cursor += 1;
            continue;
        }
        let start = cursor;
        while cursor < decoded.len() && decoded[cursor].value != ' ' {
            cursor += 1;
        }
        spans.push((start, cursor));
    }
    spans
}

pub(super) fn phrase_text(decoded: &[DecodedChar], start: usize, end: usize) -> String {
    decoded[start..end]
        .iter()
        .map(|value| value.value)
        .collect()
}

/// Collapses whitespace the same way `normalized_chars` does, for cell values.
pub(super) fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
