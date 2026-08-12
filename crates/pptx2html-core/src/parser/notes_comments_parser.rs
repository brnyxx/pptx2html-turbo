mod package;
mod relationship;
mod relationship_xml;
mod xml;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::{Read, Seek};

use zip::ZipArchive;

use super::relationships::Relationship;
use crate::model::{CommentAuthor, CommentKind, HandoutMasterMetadata, NotesCommentsInventory};
use relationship::{
    HANDOUT_MASTER, LEGACY_AUTHORS, LEGACY_COMMENTS, MODERN_AUTHORS, MODERN_COMMENTS, NOTES_SLIDE,
};

#[derive(Default)]
pub(crate) struct AuthorIndex {
    pub(super) legacy: HashMap<String, Option<CommentAuthor>>,
    pub(super) modern: HashMap<String, Option<CommentAuthor>>,
}

pub(crate) fn collect_authors<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    relationships: &[Relationship],
    relationships_exact: bool,
    inventory: &mut NotesCommentsInventory,
) -> AuthorIndex {
    if !relationships_exact {
        relationship::namespace_issue(
            inventory,
            None,
            "ppt/_rels/presentation.xml.rels",
            "Presentation relationships namespace or parent context is invalid",
        );
        return AuthorIndex::default();
    }
    let mut authors = AuthorIndex::default();
    for relation in relationship::unique(
        relationships,
        &[LEGACY_AUTHORS, MODERN_AUTHORS],
        inventory,
        None,
    ) {
        let Some(part) =
            relationship::internal_part("ppt/presentation.xml", relation, "ppt/", None, inventory)
        else {
            continue;
        };
        let Some(part_xml) = relationship::read_part(archive, &part, relation, None, inventory)
        else {
            continue;
        };
        let (parsed, malformed) = if relation.relationship_type == LEGACY_AUTHORS {
            xml::legacy_authors(&part_xml, &part)
        } else {
            xml::modern_authors(&part_xml, &part)
        };
        if let Some(reason) = malformed {
            relationship::part_issue(inventory, None, &part, Some(relation), reason);
        }
        let target = if relation.relationship_type == LEGACY_AUTHORS {
            &mut authors.legacy
        } else {
            &mut authors.modern
        };
        for author in parsed {
            inventory.authors.push(author.clone());
            match target.entry(author.id.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(Some(author));
                }
                Entry::Occupied(mut entry) => {
                    if entry.get().is_some() {
                        relationship::duplicate_author_issue(
                            inventory, &part, relation, &author.id,
                        );
                    }
                    entry.insert(None);
                }
            }
        }
    }
    authors
}

pub(crate) fn collect_handout_masters<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    relationships: &[Relationship],
    relationships_exact: bool,
    inventory: &mut NotesCommentsInventory,
) {
    if !relationships_exact {
        return;
    }
    for relation in relationship::unique(relationships, &[HANDOUT_MASTER], inventory, None) {
        let Some(part) = relationship::internal_part(
            "ppt/presentation.xml",
            relation,
            "ppt/handoutMasters/",
            None,
            inventory,
        ) else {
            continue;
        };
        let Some(part_xml) = relationship::read_part(archive, &part, relation, None, inventory)
        else {
            continue;
        };
        let (metadata, malformed) = xml::handout_master_metadata(&part_xml);
        if let Some(reason) = malformed {
            relationship::part_issue(inventory, None, &part, Some(relation), reason);
            continue;
        }
        inventory.handout_masters.push(HandoutMasterMetadata {
            part_name: part,
            relationship_id: relation.id.clone(),
            name: metadata.name,
            shape_count: metadata.shape_count,
            text: metadata.text,
        });
    }
}

pub(crate) fn collect_slide<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    slide_number: usize,
    slide_part: &str,
    relationships: &[Relationship],
    relationships_exact: bool,
    authors: &AuthorIndex,
    inventory: &mut NotesCommentsInventory,
) {
    if !relationships_exact {
        relationship::namespace_issue(
            inventory,
            Some(slide_number),
            &relationship::relationships_path(slide_part),
            "Slide relationships namespace or parent context is invalid",
        );
        return;
    }
    for relation in relationship::unique(
        relationships,
        &[NOTES_SLIDE, LEGACY_COMMENTS, MODERN_COMMENTS],
        inventory,
        Some(slide_number),
    ) {
        let (prefix, kind) = match relation.relationship_type.as_str() {
            NOTES_SLIDE => ("ppt/notesSlides/", None),
            LEGACY_COMMENTS => ("ppt/comments/", Some(CommentKind::Legacy)),
            MODERN_COMMENTS => ("ppt/comments/", Some(CommentKind::Modern)),
            _ => continue,
        };
        let Some(part) = relationship::internal_part(
            slide_part,
            relation,
            prefix,
            Some(slide_number),
            inventory,
        ) else {
            continue;
        };
        let Some(part_xml) =
            relationship::read_part(archive, &part, relation, Some(slide_number), inventory)
        else {
            continue;
        };
        if let Some(kind) = kind {
            package::collect_comments(
                kind,
                slide_number,
                relation,
                &part,
                &part_xml,
                authors,
                inventory,
            );
        } else {
            package::collect_note(archive, slide_number, relation, &part, &part_xml, inventory);
        }
    }
}

pub(crate) fn relationship_document_is_exact(xml: &str) -> bool {
    relationship::document_is_exact(xml)
}

pub(crate) fn collect_part_diagnostics(_: &str, _: &mut Vec<crate::model::ConversionDiagnostic>) {}
