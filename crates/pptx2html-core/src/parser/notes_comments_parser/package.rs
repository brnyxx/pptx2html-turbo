use std::io::{Read, Seek};

use zip::ZipArchive;

use super::AuthorIndex;
use super::relationship::{self, IssueContext, NOTES_MASTER};
use super::xml;
use crate::model::{
    AnnotationIssueCode, CommentKind, NotesCommentsInventory, SlideComment, SlideNote,
};
use crate::parser::relationships::{Relationship, parse_relationship_records};

pub(super) fn collect_comments(
    kind: CommentKind,
    slide: usize,
    relation: &Relationship,
    part: &str,
    part_xml: &str,
    authors: &AuthorIndex,
    inventory: &mut NotesCommentsInventory,
) {
    let (records, malformed) = match kind {
        CommentKind::Legacy => xml::legacy_comments(part_xml),
        CommentKind::Modern => xml::modern_comments(part_xml),
    };
    if let Some(reason) = malformed {
        let code = if reason.contains("root namespace") {
            AnnotationIssueCode::ElementNamespaceInvalid
        } else {
            AnnotationIssueCode::PartMalformed
        };
        relationship::issue(
            inventory,
            code,
            IssueContext {
                slide: Some(slide),
                part: Some(part),
                relation: Some(relation),
                element: Some("cmLst"),
                text: None,
            },
            reason,
        );
    }
    let author_map = match kind {
        CommentKind::Legacy => &authors.legacy,
        CommentKind::Modern => &authors.modern,
    };
    for record in records {
        let author = author_map.get(&record.author_id).cloned().flatten();
        if author.is_none() {
            relationship::unresolved_author_issue(
                inventory,
                slide,
                part,
                relation,
                &record.author_id,
                &record.text,
            );
        }
        inventory.comments.push(SlideComment {
            kind,
            slide_number: slide,
            part_name: part.to_owned(),
            relationship_id: relation.id.clone(),
            id: record.id,
            parent_id: record.parent_id,
            author_id: record.author_id,
            author,
            created_at: record.created_at,
            text: record.text,
            raw_extension_xml: record.raw_extension_xml,
        });
    }
}

pub(super) fn collect_note<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    slide: usize,
    relation: &Relationship,
    part: &str,
    part_xml: &str,
    inventory: &mut NotesCommentsInventory,
) {
    let (text, malformed) = xml::notes_text(part_xml);
    if let Some(reason) = malformed {
        relationship::issue(
            inventory,
            AnnotationIssueCode::PartMalformed,
            IssueContext {
                slide: Some(slide),
                part: Some(part),
                relation: Some(relation),
                element: None,
                text: Some(&text),
            },
            reason,
        );
    }
    let notes_master = notes_master(archive, slide, part, inventory);
    inventory.notes.push(SlideNote {
        slide_number: slide,
        part_name: part.to_owned(),
        relationship_id: relation.id.clone(),
        text,
        notes_master_part: notes_master.as_ref().map(|value| value.0.clone()),
        notes_master_relationship_id: notes_master.map(|value| value.1),
    });
}

fn notes_master<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    slide: usize,
    part: &str,
    inventory: &mut NotesCommentsInventory,
) -> Option<(String, String)> {
    let rels_name = relationship::relationships_path(part);
    let rels_xml = match archive.by_name(&rels_name) {
        Ok(mut file) => {
            let mut text = String::new();
            if file.read_to_string(&mut text).is_err() {
                relationship::part_issue(
                    inventory,
                    Some(slide),
                    &rels_name,
                    None,
                    "Notes Slide relationships part is not UTF-8 XML".to_owned(),
                );
                return None;
            }
            text
        }
        Err(_) => {
            relationship::issue(
                inventory,
                AnnotationIssueCode::PartMissing,
                IssueContext {
                    slide: Some(slide),
                    part: Some(&rels_name),
                    relation: None,
                    element: Some("Relationships"),
                    text: None,
                },
                "Notes Slide relationships part is missing".to_owned(),
            );
            return None;
        }
    };
    if !relationship::document_is_exact(&rels_xml) {
        relationship::namespace_issue(
            inventory,
            Some(slide),
            &rels_name,
            "Notes Slide relationships namespace or parent context is invalid",
        );
        return None;
    }
    let relations = match parse_relationship_records(&rels_xml) {
        Ok(value) => value,
        Err(error) => {
            relationship::part_issue(inventory, Some(slide), &rels_name, None, error.to_string());
            return None;
        }
    };
    let Some(relation) = relationship::unique(&relations, &[NOTES_MASTER], inventory, Some(slide))
        .into_iter()
        .next()
    else {
        relationship::issue(
            inventory,
            AnnotationIssueCode::PartMissing,
            IssueContext {
                slide: Some(slide),
                part: Some(&rels_name),
                relation: None,
                element: Some("notesMaster"),
                text: None,
            },
            "Notes Slide has no notes-master relationship".to_owned(),
        );
        return None;
    };
    let target =
        relationship::internal_part(part, relation, "ppt/notesMasters/", Some(slide), inventory)?;
    let master_xml = relationship::read_part(archive, &target, relation, Some(slide), inventory)?;
    if !xml::notes_master_document_is_exact(&master_xml) {
        relationship::issue(
            inventory,
            AnnotationIssueCode::PartMalformed,
            IssueContext {
                slide: Some(slide),
                part: Some(&target),
                relation: Some(relation),
                element: Some("p:notesMaster"),
                text: None,
            },
            "Notes master root namespace or element is invalid".to_owned(),
        );
        return None;
    }
    Some((target, relation.id.clone()))
}
