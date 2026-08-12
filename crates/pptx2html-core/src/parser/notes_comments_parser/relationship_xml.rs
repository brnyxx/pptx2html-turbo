use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

pub(super) fn document_is_exact(xml: &str) -> bool {
    const RELS: &[u8] = b"http://schemas.openxmlformats.org/package/2006/relationships";
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
                let bound =
                    matches!(namespace, ResolveResult::Bound(value) if value.as_ref() == RELS);
                let local = element.local_name();
                if depth == 0 {
                    valid_root = bound && local.as_ref() == b"Relationships";
                } else if !bound || depth != 1 || local.as_ref() != b"Relationship" {
                    return false;
                }
                depth += 1;
            }
            Ok((namespace, Event::Empty(element))) => {
                let bound =
                    matches!(namespace, ResolveResult::Bound(value) if value.as_ref() == RELS);
                if depth == 0 {
                    if valid_root || root_closed {
                        return false;
                    }
                    valid_root = bound && element.local_name().as_ref() == b"Relationships";
                    root_closed = valid_root;
                    if !valid_root {
                        return false;
                    }
                    buffer.clear();
                    continue;
                }
                if !valid_root
                    || !bound
                    || depth != 1
                    || element.local_name().as_ref() != b"Relationship"
                {
                    return false;
                }
            }
            Ok((_, Event::End(_))) => {
                depth = depth.saturating_sub(1);
                root_closed |= valid_root && depth == 0;
            }
            Ok((_, Event::Text(text))) if !text.as_ref().iter().all(u8::is_ascii_whitespace) => {
                return false;
            }
            Ok((_, Event::Eof)) => return valid_root && root_closed && depth == 0,
            Err(_) => return false,
            _ => {}
        }
        buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::document_is_exact;

    #[test]
    fn valid_empty_relationship_document_is_exact() {
        assert!(document_is_exact(
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#,
        ));
    }

    #[test]
    fn empty_relationship_document_rejects_trailing_root() {
        assert!(!document_is_exact(
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#,
        ));
    }
}
