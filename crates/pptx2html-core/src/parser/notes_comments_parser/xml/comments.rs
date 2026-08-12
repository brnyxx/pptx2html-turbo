use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::NsReader;

use super::{DML, P188, PML, node, optional_attr, tail_matches};
use crate::model::CommentKind;

#[derive(Debug)]
pub(in crate::parser::notes_comments_parser) struct CommentRecord {
    pub id: String,
    pub author_id: String,
    pub created_at: Option<String>,
    pub text: String,
    pub raw_extension_xml: Option<String>,
}

pub(in crate::parser::notes_comments_parser) fn legacy_comments(
    xml: &str,
) -> (Vec<CommentRecord>, Option<String>) {
    comments(xml, CommentKind::Legacy)
}

pub(in crate::parser::notes_comments_parser) fn modern_comments(
    xml: &str,
) -> (Vec<CommentRecord>, Option<String>) {
    comments(xml, CommentKind::Modern)
}

fn comments(xml: &str, kind: CommentKind) -> (Vec<CommentRecord>, Option<String>) {
    let namespace = match kind {
        CommentKind::Legacy => PML,
        CommentKind::Modern => P188,
    };
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut stack = Vec::new();
    let mut records = Vec::new();
    let mut current: Option<CommentRecord> = None;
    let mut valid_root = false;
    let mut extension_start = None;
    let mut paragraph_count = 0_usize;
    let mut malformed = None;
    loop {
        let event_start = reader.buffer_position() as usize;
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((resolved, Event::Start(element))) => {
                let current_node = node(resolved, &element);
                if stack.is_empty() {
                    valid_root = current_node == (namespace.to_vec(), "cmLst".to_owned());
                }
                if valid_root
                    && stack.len() == 1
                    && current_node == (namespace.to_vec(), "cm".to_owned())
                {
                    match record(&element, kind) {
                        Ok(record) => current = Some(record),
                        Err(reason) => {
                            malformed.get_or_insert(reason);
                            current = None;
                        }
                    }
                    paragraph_count = 0;
                }
                if kind == CommentKind::Modern
                    && current.is_some()
                    && current_node == (DML.to_vec(), "p".to_owned())
                    && modern_text_body(&stack)
                {
                    if let Some(record) = current.as_mut()
                        && paragraph_count > 0
                        && !record.text.ends_with('\n')
                    {
                        record.text.push('\n');
                    }
                    paragraph_count += 1;
                } else if kind == CommentKind::Modern
                    && current_node == (DML.to_vec(), "br".to_owned())
                    && modern_paragraph(&stack)
                    && let Some(record) = current.as_mut()
                {
                    record.text.push('\n');
                }
                if kind == CommentKind::Modern
                    && current.is_some()
                    && stack.len() == 2
                    && current_node == (P188.to_vec(), "extLst".to_owned())
                {
                    extension_start = Some(event_start);
                }
                stack.push(current_node);
            }
            Ok((resolved, Event::Empty(element))) => {
                let current_node = node(resolved, &element);
                if kind == CommentKind::Modern
                    && current_node == (DML.to_vec(), "br".to_owned())
                    && modern_paragraph(&stack)
                    && let Some(record) = current.as_mut()
                {
                    record.text.push('\n');
                }
                if kind == CommentKind::Modern
                    && current.is_some()
                    && stack.len() == 2
                    && current_node == (P188.to_vec(), "extLst".to_owned())
                {
                    let event_end = reader.buffer_position() as usize;
                    if let Some(record) = current.as_mut() {
                        record.raw_extension_xml =
                            xml.get(event_start..event_end).map(str::to_owned);
                    }
                }
            }
            Ok((_, Event::Text(value))) if text_context(&stack, kind) => {
                if let Some(record) = current.as_mut() {
                    match value.unescape() {
                        Ok(value) => record.text.push_str(&value),
                        Err(error) => return (records, Some(error.to_string())),
                    }
                }
            }
            Ok((_, Event::CData(value))) if text_context(&stack, kind) => {
                if let Some(record) = current.as_mut() {
                    record
                        .text
                        .push_str(&String::from_utf8_lossy(value.as_ref()));
                }
            }
            Ok((_, Event::End(element))) => {
                let ending_extension = kind == CommentKind::Modern
                    && current.is_some()
                    && stack.len() == 3
                    && stack
                        .last()
                        .is_some_and(|node| node.0.as_slice() == P188 && node.1 == "extLst");
                let ending_comment = String::from_utf8_lossy(element.local_name().as_ref()) == "cm"
                    && stack.len() == 2;
                if ending_extension {
                    let event_end = reader.buffer_position() as usize;
                    if let (Some(start), Some(record)) = (extension_start, current.as_mut()) {
                        record.raw_extension_xml = xml.get(start..event_end).map(str::to_owned);
                    }
                    extension_start = None;
                }
                stack.pop();
                if ending_comment && let Some(record) = current.take() {
                    records.push(record);
                    extension_start = None;
                }
            }
            Ok((_, Event::Eof)) => break,
            Err(error) => return (records, Some(error.to_string())),
            _ => {}
        }
        buffer.clear();
    }
    if valid_root {
        (records, malformed)
    } else {
        (
            Vec::new(),
            Some("Comment root namespace or element is invalid".to_owned()),
        )
    }
}

fn record(element: &BytesStart<'_>, kind: CommentKind) -> Result<CommentRecord, String> {
    let id_attribute = match kind {
        CommentKind::Legacy => "idx",
        CommentKind::Modern => "id",
    };
    let id = optional_attr(element, id_attribute)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Comment is missing required {id_attribute}"))?;
    let author_id = optional_attr(element, "authorId")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Comment is missing required authorId".to_owned())?;
    Ok(CommentRecord {
        id,
        author_id,
        created_at: match kind {
            CommentKind::Legacy => optional_attr(element, "dt"),
            CommentKind::Modern => optional_attr(element, "created"),
        },
        text: String::new(),
        raw_extension_xml: None,
    })
}

fn text_context(stack: &[(Vec<u8>, String)], kind: CommentKind) -> bool {
    match kind {
        CommentKind::Legacy => tail_matches(stack, &[(PML, "cm"), (PML, "text")]),
        CommentKind::Modern => {
            tail_matches(
                stack,
                &[(P188, "txBody"), (DML, "p"), (DML, "r"), (DML, "t")],
            ) || tail_matches(
                stack,
                &[(P188, "txBody"), (DML, "p"), (DML, "fld"), (DML, "t")],
            )
        }
    }
}

fn modern_text_body(stack: &[(Vec<u8>, String)]) -> bool {
    tail_matches(stack, &[(P188, "txBody")])
}

fn modern_paragraph(stack: &[(Vec<u8>, String)]) -> bool {
    tail_matches(stack, &[(P188, "txBody"), (DML, "p")])
}
