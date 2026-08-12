use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::NsReader;

use super::{DML, P188, PML, node, optional_attr, tail_matches};
use crate::model::CommentKind;

#[derive(Debug)]
pub(in crate::parser::notes_comments_parser) struct CommentRecord {
    pub id: String,
    pub parent_id: Option<String>,
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
    let mut active = Vec::new();
    let mut records = Vec::new();
    let mut valid_root = false;
    let mut root_closed = false;
    let mut malformed = None;
    let mut extension_start = None;
    let mut comment_order = 0_usize;
    loop {
        let event_start = reader.buffer_position() as usize;
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((resolved, Event::Start(element))) => {
                let current_node = node(resolved, &element);
                if stack.is_empty() {
                    if root_closed {
                        malformed.get_or_insert("Comment XML has multiple roots".to_owned());
                    } else {
                        valid_root = current_node == (namespace.to_vec(), "cmLst".to_owned());
                    }
                }
                if starts_comment(&stack, &current_node, namespace, kind) {
                    let parent_id = active
                        .last()
                        .map(|comment: &ActiveComment| comment.record.id.clone());
                    match record(&element, kind, parent_id) {
                        Ok(record) => {
                            active.push(ActiveComment::new(record, comment_order));
                            comment_order += 1;
                        }
                        Err(reason) => {
                            malformed.get_or_insert(reason);
                        }
                    }
                } else if kind == CommentKind::Modern
                    && current_node == (DML.to_vec(), "p".to_owned())
                    && active.last().is_some_and(|comment| comment.in_text_body)
                {
                    if let Some(comment) = active.last_mut() {
                        comment.start_paragraph();
                    }
                } else if kind == CommentKind::Modern
                    && current_node == (DML.to_vec(), "br".to_owned())
                    && active.last().is_some_and(|comment| comment.in_paragraph)
                    && let Some(comment) = active.last_mut()
                {
                    comment.record.text.push('\n');
                }
                if kind == CommentKind::Modern
                    && current_node == (P188.to_vec(), "txBody".to_owned())
                    && stack.last().is_some_and(is_modern_comment)
                    && let Some(comment) = active.last_mut()
                {
                    comment.in_text_body = true;
                }
                if kind == CommentKind::Modern
                    && current_node == (P188.to_vec(), "extLst".to_owned())
                    && stack.last().is_some_and(is_modern_comment)
                {
                    extension_start = Some(event_start);
                }
                stack.push(current_node);
            }
            Ok((resolved, Event::Empty(element))) => {
                let current_node = node(resolved, &element);
                if stack.is_empty() {
                    if root_closed {
                        malformed.get_or_insert("Comment XML has multiple roots".to_owned());
                    } else if current_node == (namespace.to_vec(), "cmLst".to_owned()) {
                        valid_root = true;
                        root_closed = true;
                    } else {
                        malformed.get_or_insert(
                            "Comment root namespace or element is invalid".to_owned(),
                        );
                    }
                }
                if kind == CommentKind::Modern
                    && current_node == (DML.to_vec(), "br".to_owned())
                    && active.last().is_some_and(|comment| comment.in_paragraph)
                    && let Some(comment) = active.last_mut()
                {
                    comment.record.text.push('\n');
                }
                if kind == CommentKind::Modern
                    && current_node == (P188.to_vec(), "extLst".to_owned())
                    && stack.last().is_some_and(is_modern_comment)
                    && let Some(comment) = active.last_mut()
                {
                    let event_end = reader.buffer_position() as usize;
                    comment.record.raw_extension_xml =
                        xml.get(event_start..event_end).map(str::to_owned);
                }
            }
            Ok((_, Event::Text(value)))
                if active.last().is_some() && text_context(&stack, kind) =>
            {
                if let Some(comment) = active.last_mut() {
                    match value.unescape() {
                        Ok(value) => comment.record.text.push_str(&value),
                        Err(error) => return (Vec::new(), Some(error.to_string())),
                    }
                }
            }
            Ok((_, Event::CData(value)))
                if active.last().is_some() && text_context(&stack, kind) =>
            {
                if let Some(comment) = active.last_mut() {
                    comment
                        .record
                        .text
                        .push_str(&String::from_utf8_lossy(value.as_ref()));
                }
            }
            Ok((_, Event::End(_))) => {
                let ending = stack.last().cloned();
                if kind == CommentKind::Modern
                    && ending
                        .as_ref()
                        .is_some_and(|node| node.0.as_slice() == P188 && node.1 == "extLst")
                {
                    if let (Some(start), Some(comment)) = (extension_start, active.last_mut()) {
                        let event_end = reader.buffer_position() as usize;
                        comment.record.raw_extension_xml =
                            xml.get(start..event_end).map(str::to_owned);
                    }
                    extension_start = None;
                }
                if ending
                    .as_ref()
                    .is_some_and(|node| node.0.as_slice() == DML && node.1 == "p")
                    && let Some(comment) = active.last_mut()
                {
                    comment.in_paragraph = false;
                }
                if ending
                    .as_ref()
                    .is_some_and(|node| node.0.as_slice() == P188 && node.1 == "txBody")
                    && let Some(comment) = active.last_mut()
                {
                    comment.in_text_body = false;
                }
                if ending
                    .as_ref()
                    .is_some_and(|node| node.0.as_slice() == namespace && node.1 == "cm")
                    && let Some(comment) = active.pop()
                {
                    records.push((comment.order, comment.record));
                }
                stack.pop();
                root_closed |= valid_root && stack.is_empty();
            }
            Ok((_, Event::Eof)) => break,
            Err(error) => return (Vec::new(), Some(error.to_string())),
            _ => {}
        }
        buffer.clear();
    }
    if !valid_root || !root_closed {
        return (
            Vec::new(),
            Some("Comment root namespace or element is invalid".to_owned()),
        );
    }
    if let Some(reason) = malformed {
        return (Vec::new(), Some(reason));
    }
    records.sort_by_key(|(order, _)| *order);
    (
        records.into_iter().map(|(_, record)| record).collect(),
        None,
    )
}

#[derive(Debug)]
struct ActiveComment {
    record: CommentRecord,
    in_text_body: bool,
    in_paragraph: bool,
    paragraph_count: usize,
    order: usize,
}

impl ActiveComment {
    fn new(record: CommentRecord, order: usize) -> Self {
        Self {
            record,
            in_text_body: false,
            in_paragraph: false,
            paragraph_count: 0,
            order,
        }
    }

    fn start_paragraph(&mut self) {
        if self.paragraph_count > 0 && !self.record.text.ends_with('\n') {
            self.record.text.push('\n');
        }
        self.paragraph_count += 1;
        self.in_paragraph = true;
    }
}

fn starts_comment(
    stack: &[(Vec<u8>, String)],
    current_node: &(Vec<u8>, String),
    namespace: &[u8],
    kind: CommentKind,
) -> bool {
    if current_node.0.as_slice() != namespace || current_node.1 != "cm" {
        return false;
    }
    stack.len() == 1
        || kind == CommentKind::Modern
            && stack
                .last()
                .is_some_and(|node| node.0.as_slice() == P188 && node.1 == "replyLst")
}

fn is_modern_comment(node: &(Vec<u8>, String)) -> bool {
    node.0.as_slice() == P188 && node.1 == "cm"
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

fn record(
    element: &BytesStart<'_>,
    kind: CommentKind,
    parent_id: Option<String>,
) -> Result<CommentRecord, String> {
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
        parent_id,
        author_id,
        created_at: match kind {
            CommentKind::Legacy => optional_attr(element, "dt"),
            CommentKind::Modern => optional_attr(element, "created"),
        },
        text: String::new(),
        raw_extension_xml: None,
    })
}
