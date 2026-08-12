use quick_xml::events::Event;
use quick_xml::reader::NsReader;

use super::{P188, PML, attr, bound, local, optional_attr};
use crate::model::{CommentAuthor, CommentKind};

pub(in crate::parser::notes_comments_parser) fn legacy_authors(
    xml: &str,
    part: &str,
) -> (Vec<CommentAuthor>, Option<String>) {
    authors(
        xml,
        part,
        CommentKind::Legacy,
        PML,
        "cmAuthorLst",
        "cmAuthor",
    )
}

pub(in crate::parser::notes_comments_parser) fn modern_authors(
    xml: &str,
    part: &str,
) -> (Vec<CommentAuthor>, Option<String>) {
    authors(xml, part, CommentKind::Modern, P188, "authorLst", "author")
}

fn authors(
    xml: &str,
    part: &str,
    kind: CommentKind,
    namespace: &[u8],
    root: &str,
    child: &str,
) -> (Vec<CommentAuthor>, Option<String>) {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut authors = Vec::new();
    let mut depth = 0_usize;
    let mut valid_root = false;
    let mut root_closed = false;
    let mut malformed = None;
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((resolved, Event::Start(element))) => {
                let local = local(&element);
                if depth == 0 {
                    if root_closed {
                        malformed.get_or_insert("Author XML has multiple roots".to_owned());
                    } else {
                        valid_root = bound(&resolved, namespace) && local == root;
                    }
                }
                if valid_root && depth == 1 && bound(&resolved, namespace) && local == child {
                    authors.push(CommentAuthor {
                        id: attr(&element, "id"),
                        name: attr(&element, "name"),
                        initials: optional_attr(&element, "initials"),
                        part_name: part.to_owned(),
                        kind,
                    });
                }
                depth += 1;
            }
            Ok((resolved, Event::Empty(element))) => {
                let local = local(&element);
                if depth == 0 {
                    if root_closed {
                        malformed.get_or_insert("Author XML has multiple roots".to_owned());
                    } else if bound(&resolved, namespace) && local == root {
                        valid_root = true;
                        root_closed = true;
                    } else {
                        malformed.get_or_insert(
                            "Author root namespace or element is invalid".to_owned(),
                        );
                    }
                }
                if valid_root && depth == 1 && bound(&resolved, namespace) && local == child {
                    authors.push(CommentAuthor {
                        id: attr(&element, "id"),
                        name: attr(&element, "name"),
                        initials: optional_attr(&element, "initials"),
                        part_name: part.to_owned(),
                        kind,
                    });
                }
            }
            Ok((_, Event::End(_))) => {
                depth = depth.saturating_sub(1);
                root_closed |= valid_root && depth == 0;
            }
            Ok((_, Event::Eof)) => break,
            Err(error) => return (authors, Some(error.to_string())),
            _ => {}
        }
        buffer.clear();
    }
    if valid_root && root_closed && malformed.is_none() {
        (authors, None)
    } else {
        (
            Vec::new(),
            Some(
                malformed
                    .unwrap_or_else(|| "Author root namespace or element is invalid".to_owned()),
            ),
        )
    }
}
