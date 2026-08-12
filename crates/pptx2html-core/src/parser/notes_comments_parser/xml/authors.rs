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
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((resolved, Event::Start(element))) => {
                let local = local(&element);
                if depth == 0 {
                    valid_root = bound(&resolved, namespace) && local == root;
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
                    valid_root = bound(&resolved, namespace) && local == root;
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
            Ok((_, Event::End(_))) => depth = depth.saturating_sub(1),
            Ok((_, Event::Eof)) => break,
            Err(error) => return (authors, Some(error.to_string())),
            _ => {}
        }
        buffer.clear();
    }
    if valid_root {
        (authors, None)
    } else {
        (
            Vec::new(),
            Some("Author root namespace or element is invalid".to_owned()),
        )
    }
}
