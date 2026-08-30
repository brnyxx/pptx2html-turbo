use crate::{NativeError, NativeResult};

pub(super) struct StartTag {
    pub(super) name: String,
    pub(super) self_closing: bool,
    pub(super) namespaces: Vec<(String, String)>,
}

pub(super) fn tag_end(text: &str, start: usize) -> NativeResult<usize> {
    let mut quote = None;
    for (offset, byte) in text.as_bytes()[start + 1..].iter().copied().enumerate() {
        match quote {
            Some(delimiter) if byte == delimiter => quote = None,
            Some(_) if byte == b'<' => {
                return malformed("xl/workbook.xml has an invalid attribute value");
            }
            Some(_) => {}
            None if matches!(byte, b'\'' | b'\"') => quote = Some(byte),
            None if byte == b'>' => return Ok(start + offset + 1),
            None => {}
        }
    }
    malformed("xl/workbook.xml has an unterminated tag")
}

pub(super) fn parse_start_tag(content: &str) -> NativeResult<StartTag> {
    let bytes = content.as_bytes();
    let name_end = xml_name_end(bytes, 0)?;
    let name = content[..name_end].to_owned();
    let mut position = name_end;
    let mut self_closing = false;
    let mut namespaces = Vec::new();
    while position < bytes.len() {
        skip_xml_whitespace(bytes, &mut position);
        if position == bytes.len() {
            break;
        }
        if bytes[position] == b'/' {
            position += 1;
            skip_xml_whitespace(bytes, &mut position);
            if position != bytes.len() {
                return malformed("xl/workbook.xml has an invalid self-closing tag");
            }
            self_closing = true;
            break;
        }
        let attribute_start = position;
        position = xml_name_end(bytes, position)?;
        let attribute_name = &content[attribute_start..position];
        skip_xml_whitespace(bytes, &mut position);
        if bytes.get(position) != Some(&b'=') {
            return malformed("xl/workbook.xml has an invalid attribute");
        }
        position += 1;
        skip_xml_whitespace(bytes, &mut position);
        let Some(quote) = bytes
            .get(position)
            .copied()
            .filter(|byte| matches!(byte, b'\'' | b'\"'))
        else {
            return malformed("xl/workbook.xml has an unquoted attribute");
        };
        position += 1;
        let value_start = position;
        while let Some(byte) = bytes.get(position).copied() {
            if byte == b'<' {
                return malformed("xl/workbook.xml has an invalid attribute value");
            }
            position += 1;
            if byte == quote {
                break;
            }
        }
        if position == bytes.len() && bytes[position - 1] != quote {
            return malformed("xl/workbook.xml has an unterminated attribute");
        }
        let value = &content[value_start..position - 1];
        validate_xml_text(value)?;
        let namespace_prefix = if attribute_name == "xmlns" {
            Some("")
        } else {
            attribute_name.strip_prefix("xmlns:")
        };
        if let Some(prefix) = namespace_prefix {
            if !prefix.is_empty() && !valid_xml_prefix(prefix) {
                return malformed("xl/workbook.xml has an invalid namespace declaration");
            }
            if namespaces.iter().any(|(declared, _)| declared == prefix) {
                return malformed("xl/workbook.xml has a duplicate namespace declaration");
            }
            namespaces.push((prefix.to_owned(), decode_xml_text(value)?));
        }
    }
    Ok(StartTag {
        name,
        self_closing,
        namespaces,
    })
}

pub(super) fn parse_end_tag(content: &str) -> NativeResult<String> {
    let bytes = content.as_bytes();
    let end = xml_name_end(bytes, 0)?;
    if !bytes[end..].iter().all(u8::is_ascii_whitespace) {
        return malformed("xl/workbook.xml has an invalid closing tag");
    }
    Ok(content[..end].to_owned())
}

fn xml_name_end(bytes: &[u8], start: usize) -> NativeResult<usize> {
    if !bytes
        .get(start)
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
    {
        return malformed("xl/workbook.xml has an invalid element or attribute name");
    }
    let mut end = start + 1;
    while bytes.get(end).is_some_and(|byte| {
        byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'-' | b'.' | b':')
    }) {
        end += 1;
    }
    Ok(end)
}

fn skip_xml_whitespace(bytes: &[u8], position: &mut usize) {
    while bytes.get(*position).is_some_and(u8::is_ascii_whitespace) {
        *position += 1;
    }
}

pub(super) fn validate_xml_text(text: &str) -> NativeResult<()> {
    let mut remainder = text;
    while let Some(ampersand) = remainder.find('&') {
        let entity_tail = &remainder[ampersand + 1..];
        let Some(semicolon) = entity_tail.find(';') else {
            return malformed("xl/workbook.xml has an unterminated entity reference");
        };
        let entity = &entity_tail[..semicolon];
        if entity_character(entity).is_none() {
            return malformed("xl/workbook.xml has an invalid entity reference");
        }
        remainder = &entity_tail[semicolon + 1..];
    }
    Ok(())
}

fn decode_xml_text(text: &str) -> NativeResult<String> {
    let mut decoded = String::with_capacity(text.len());
    let mut remainder = text;
    while let Some(ampersand) = remainder.find('&') {
        decoded.push_str(&remainder[..ampersand]);
        let entity_tail = &remainder[ampersand + 1..];
        let Some(semicolon) = entity_tail.find(';') else {
            return malformed("xl/workbook.xml has an unterminated entity reference");
        };
        let Some(character) = entity_character(&entity_tail[..semicolon]) else {
            return malformed("xl/workbook.xml has an invalid entity reference");
        };
        decoded.push(character);
        remainder = &entity_tail[semicolon + 1..];
    }
    decoded.push_str(remainder);
    Ok(decoded)
}

fn entity_character(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "apos" => Some('\''),
        "quot" => Some('"'),
        _ => entity
            .strip_prefix("#x")
            .and_then(|value| u32::from_str_radix(value, 16).ok())
            .or_else(|| {
                entity
                    .strip_prefix('#')
                    .and_then(|value| value.parse().ok())
            })
            .filter(|value| valid_xml_scalar(*value))
            .and_then(char::from_u32),
    }
}

fn valid_xml_scalar(value: u32) -> bool {
    matches!(value, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
}

pub(super) fn is_workbook_name(name: &str) -> bool {
    name == "workbook" || name.strip_suffix(":workbook").is_some_and(valid_xml_prefix)
}

pub(super) fn calc_pr_name(workbook_name: &str) -> String {
    workbook_child_name(workbook_name, "calcPr")
}

pub(super) fn workbook_child_name(workbook_name: &str, child_name: &str) -> String {
    workbook_name.strip_suffix("workbook").map_or_else(
        || child_name.to_owned(),
        |prefix| format!("{prefix}{child_name}"),
    )
}

pub(super) fn element_namespace<'a>(
    name: &str,
    local_name: &str,
    declared: &'a [(String, String)],
    inherited: &'a [(String, String)],
) -> Option<&'a str> {
    let (prefix, local) = name.rsplit_once(':').unwrap_or(("", name));
    if local != local_name || (!prefix.is_empty() && !valid_xml_prefix(prefix)) {
        return None;
    }
    declared
        .iter()
        .rev()
        .chain(inherited.iter().rev())
        .find_map(|(declared_prefix, namespace)| {
            (declared_prefix == prefix).then_some(namespace.as_str())
        })
}

fn valid_xml_prefix(prefix: &str) -> bool {
    !prefix.is_empty()
        && !prefix.contains(':')
        && prefix.bytes().enumerate().all(|(index, byte)| {
            if index == 0 {
                byte.is_ascii_alphabetic() || byte == b'_'
            } else {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')
            }
        })
}

fn malformed<T>(reason: &str) -> NativeResult<T> {
    Err(malformed_error(reason))
}

fn malformed_error(reason: &str) -> NativeError {
    NativeError::MalformedBackendOutput {
        backend: "libreoffice",
        reason: reason.to_owned(),
    }
}
