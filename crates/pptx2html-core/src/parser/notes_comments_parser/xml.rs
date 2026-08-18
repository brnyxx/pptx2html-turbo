mod authors;
mod comments;

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

pub(super) use authors::{legacy_authors, modern_authors};
pub(super) use comments::{legacy_comments, modern_comments};

pub(super) const PML: &[u8] = b"http://schemas.openxmlformats.org/presentationml/2006/main";
pub(super) const DML: &[u8] = b"http://schemas.openxmlformats.org/drawingml/2006/main";
pub(super) const P188: &[u8] = b"http://schemas.microsoft.com/office/powerpoint/2018/8/main";

#[derive(Default)]
pub(super) struct HandoutMasterMetadata {
    pub name: Option<String>,
    pub shape_count: usize,
    pub text: String,
}

pub(super) fn handout_master_metadata(xml: &str) -> (HandoutMasterMetadata, Option<String>) {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut depth = 0_usize;
    let mut root_closed = false;
    let mut metadata = HandoutMasterMetadata::default();
    let mut capture_text = false;
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element))) => {
                if root_closed {
                    return (
                        HandoutMasterMetadata::default(),
                        Some("Handout master has multiple root elements".to_owned()),
                    );
                }
                if depth == 0
                    && (!bound(&namespace, PML)
                        || element.local_name().as_ref() != b"handoutMaster")
                {
                    return (
                        HandoutMasterMetadata::default(),
                        Some("Handout master root namespace or element is invalid".to_owned()),
                    );
                }
                if bound(&namespace, PML) && element.local_name().as_ref() == b"cSld" {
                    metadata.name = optional_attr(&element, "name");
                } else if bound(&namespace, PML) && element.local_name().as_ref() == b"sp" {
                    metadata.shape_count += 1;
                } else if bound(&namespace, DML) && element.local_name().as_ref() == b"t" {
                    capture_text = true;
                }
                depth += 1;
            }
            Ok((namespace, Event::Empty(element)))
                if depth == 0
                    && (!bound(&namespace, PML)
                        || element.local_name().as_ref() != b"handoutMaster") =>
            {
                return (
                    HandoutMasterMetadata::default(),
                    Some("Handout master root namespace or element is invalid".to_owned()),
                );
            }
            Ok((_, Event::Text(value))) if capture_text => match value.unescape() {
                Ok(value) => metadata.text.push_str(&value),
                Err(error) => return (HandoutMasterMetadata::default(), Some(error.to_string())),
            },
            Ok((namespace, Event::End(element))) => {
                if bound(&namespace, DML) && element.local_name().as_ref() == b"t" {
                    capture_text = false;
                }
                depth = depth.saturating_sub(1);
                root_closed |= depth == 0;
            }
            Ok((_, Event::Eof)) => break,
            Err(error) => return (HandoutMasterMetadata::default(), Some(error.to_string())),
            _ => {}
        }
        buffer.clear();
    }
    if root_closed {
        (metadata, None)
    } else {
        (
            HandoutMasterMetadata::default(),
            Some("Handout master root is incomplete".to_owned()),
        )
    }
}

pub(super) fn notes_text(xml: &str) -> (String, Option<String>) {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut stack = Vec::new();
    let mut text = String::new();
    let mut valid_root = false;
    let mut root_closed = false;
    let mut multiple_roots = false;
    let mut paragraph_count = 0_usize;
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element))) => {
                let node = node(namespace, &element);
                if stack.is_empty() {
                    if root_closed {
                        multiple_roots = true;
                    } else {
                        valid_root = node == (PML.to_vec(), "notes".to_owned());
                    }
                }
                if node == (DML.to_vec(), "p".to_owned()) && notes_text_body(&stack) {
                    if paragraph_count > 0 && !text.ends_with('\n') {
                        text.push('\n');
                    }
                    paragraph_count += 1;
                } else if node == (DML.to_vec(), "br".to_owned()) && notes_paragraph(&stack) {
                    text.push('\n');
                }
                stack.push(node);
            }
            Ok((namespace, Event::Empty(element))) => {
                let node = node(namespace, &element);
                if stack.is_empty() {
                    if root_closed {
                        multiple_roots = true;
                    } else if node == (PML.to_vec(), "notes".to_owned()) {
                        valid_root = true;
                        root_closed = true;
                    }
                }
                if node == (DML.to_vec(), "br".to_owned()) && notes_paragraph(&stack) {
                    text.push('\n');
                }
            }
            Ok((_, Event::Text(value))) if notes_context(&stack) => match value.unescape() {
                Ok(value) => text.push_str(&value),
                Err(error) => return (text, Some(error.to_string())),
            },
            Ok((_, Event::CData(value))) if notes_context(&stack) => {
                text.push_str(&String::from_utf8_lossy(value.as_ref()));
            }
            Ok((_, Event::End(_))) => {
                stack.pop();
                root_closed |= valid_root && stack.is_empty();
            }
            Ok((_, Event::Eof)) => break,
            Err(error) => return (text, Some(error.to_string())),
            _ => {}
        }
        buffer.clear();
    }
    if valid_root && root_closed && !multiple_roots {
        (text, None)
    } else {
        (
            String::new(),
            Some("Notes root namespace or element is invalid".to_owned()),
        )
    }
}

pub(super) fn notes_master_document_is_exact(xml: &str) -> bool {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut depth = 0_usize;
    let mut valid_root = false;
    let mut root_closed = false;
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element))) => {
                if root_closed {
                    return false;
                }
                if depth == 0 {
                    valid_root =
                        bound(&namespace, PML) && element.local_name().as_ref() == b"notesMaster";
                }
                depth += 1;
            }
            Ok((namespace, Event::Empty(element))) if depth == 0 => {
                if root_closed {
                    return false;
                }
                valid_root =
                    bound(&namespace, PML) && element.local_name().as_ref() == b"notesMaster";
                root_closed = valid_root;
            }
            Ok((_, Event::End(_))) => {
                depth = depth.saturating_sub(1);
                root_closed |= valid_root && depth == 0;
            }
            Ok((_, Event::Eof)) => return valid_root && root_closed && depth == 0,
            Err(_) => return false,
            _ => {}
        }
        buffer.clear();
    }
}

fn notes_context(stack: &[(Vec<u8>, String)]) -> bool {
    stack
        .first()
        .is_some_and(|node| node.0.as_slice() == PML && node.1 == "notes")
        && (tail_matches(
            stack,
            &[(PML, "txBody"), (DML, "p"), (DML, "r"), (DML, "t")],
        ) || tail_matches(
            stack,
            &[(PML, "txBody"), (DML, "p"), (DML, "fld"), (DML, "t")],
        ))
}

fn notes_text_body(stack: &[(Vec<u8>, String)]) -> bool {
    stack
        .first()
        .is_some_and(|node| node.0.as_slice() == PML && node.1 == "notes")
        && tail_matches(stack, &[(PML, "txBody")])
}

fn notes_paragraph(stack: &[(Vec<u8>, String)]) -> bool {
    notes_text_body(stack) || tail_matches(stack, &[(PML, "txBody"), (DML, "p")])
}

pub(super) fn tail_matches(stack: &[(Vec<u8>, String)], expected: &[(&[u8], &str)]) -> bool {
    stack.len() >= expected.len()
        && stack[stack.len() - expected.len()..]
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual.0.as_slice() == expected.0 && actual.1 == expected.1)
}

pub(super) fn node(namespace: ResolveResult<'_>, element: &BytesStart<'_>) -> (Vec<u8>, String) {
    let namespace = match namespace {
        ResolveResult::Bound(value) => value.as_ref().to_vec(),
        _ => Vec::new(),
    };
    (namespace, local(element))
}

pub(super) fn bound(namespace: &ResolveResult<'_>, expected: &[u8]) -> bool {
    matches!(namespace, ResolveResult::Bound(value) if value.as_ref() == expected)
}

pub(super) fn local(element: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(element.local_name().as_ref()).into_owned()
}

pub(super) fn attr(element: &BytesStart<'_>, name: &str) -> String {
    optional_attr(element, name).unwrap_or_default()
}

pub(super) fn optional_attr(element: &BytesStart<'_>, name: &str) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.as_ref() == name.as_bytes())
        .and_then(|attribute| {
            attribute
                .unescape_value()
                .ok()
                .map(|value| value.into_owned())
        })
}
