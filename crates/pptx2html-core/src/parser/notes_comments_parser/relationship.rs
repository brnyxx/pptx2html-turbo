use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};

use zip::ZipArchive;

use crate::model::{AnnotationIssue, AnnotationIssueCode, NotesCommentsInventory};
use crate::parser::relationships::{Relationship, TargetMode, resolve_internal_target};

pub(super) const LEGACY_COMMENTS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments";
pub(super) const LEGACY_AUTHORS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/commentAuthors";
pub(super) const NOTES_SLIDE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide";
pub(super) const NOTES_MASTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesMaster";
pub(super) const MODERN_COMMENTS: &str =
    "http://schemas.microsoft.com/office/2018/10/relationships/comments";
pub(super) const MODERN_AUTHORS: &str =
    "http://schemas.microsoft.com/office/2018/10/relationships/authors";

pub(super) struct IssueContext<'a> {
    pub slide: Option<usize>,
    pub part: Option<&'a str>,
    pub relation: Option<&'a Relationship>,
    pub element: Option<&'a str>,
    pub text: Option<&'a str>,
}

pub(super) fn unique<'a>(
    relationships: &'a [Relationship],
    types: &[&str],
    inventory: &mut NotesCommentsInventory,
    slide: Option<usize>,
) -> Vec<&'a Relationship> {
    let mut counts = HashMap::new();
    for relationship in relationships {
        *counts.entry(relationship.id.as_str()).or_insert(0_usize) += 1;
    }
    let mut reported = HashSet::new();
    relationships
        .iter()
        .filter(|relation| types.contains(&relation.relationship_type.as_str()))
        .filter(|relation| {
            if counts.get(relation.id.as_str()) == Some(&1) {
                return true;
            }
            if reported.insert(relation.id.as_str()) {
                issue(
                    inventory,
                    AnnotationIssueCode::RelationshipDuplicate,
                    IssueContext {
                        slide,
                        part: None,
                        relation: Some(relation),
                        element: None,
                        text: None,
                    },
                    "Every candidate with a duplicate relationship ID was ignored".to_owned(),
                );
            }
            false
        })
        .collect()
}

pub(super) fn internal_part(
    owner: &str,
    relation: &Relationship,
    prefix: &str,
    slide: Option<usize>,
    inventory: &mut NotesCommentsInventory,
) -> Option<String> {
    let target = if relation.target_mode == TargetMode::Internal {
        resolve_internal_target(owner, &relation.target).ok()
    } else {
        None
    };
    if let Some(target) =
        target.filter(|target| target.starts_with(prefix) && target.ends_with(".xml"))
    {
        return Some(target);
    }
    issue(
        inventory,
        AnnotationIssueCode::RelationshipUnsafe,
        IssueContext {
            slide,
            part: None,
            relation: Some(relation),
            element: None,
            text: None,
        },
        "Annotation relationship target is not a safe internal part of the required type"
            .to_owned(),
    );
    None
}

pub(super) fn read_part<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    part: &str,
    relation: &Relationship,
    slide: Option<usize>,
    inventory: &mut NotesCommentsInventory,
) -> Option<String> {
    let Ok(mut file) = archive.by_name(part) else {
        issue(
            inventory,
            AnnotationIssueCode::PartMissing,
            IssueContext {
                slide,
                part: Some(part),
                relation: Some(relation),
                element: None,
                text: None,
            },
            "Annotation part is missing".to_owned(),
        );
        return None;
    };
    let mut text = String::new();
    if file.read_to_string(&mut text).is_err() {
        part_issue(
            inventory,
            slide,
            part,
            Some(relation),
            "Annotation part is not UTF-8 XML".to_owned(),
        );
        return None;
    }
    Some(text)
}

pub(super) fn namespace_issue(
    inventory: &mut NotesCommentsInventory,
    slide: Option<usize>,
    part: &str,
    reason: &str,
) {
    issue(
        inventory,
        AnnotationIssueCode::ElementNamespaceInvalid,
        IssueContext {
            slide,
            part: Some(part),
            relation: None,
            element: Some("Relationships"),
            text: None,
        },
        reason.to_owned(),
    );
}

pub(super) fn part_issue(
    inventory: &mut NotesCommentsInventory,
    slide: Option<usize>,
    part: &str,
    relation: Option<&Relationship>,
    reason: String,
) {
    issue(
        inventory,
        AnnotationIssueCode::PartMalformed,
        IssueContext {
            slide,
            part: Some(part),
            relation,
            element: None,
            text: None,
        },
        reason,
    );
}

pub(super) fn unresolved_author_issue(
    inventory: &mut NotesCommentsInventory,
    slide: usize,
    part: &str,
    relation: &Relationship,
    author_id: &str,
    text: &str,
) {
    inventory.issues.push(AnnotationIssue {
        code: AnnotationIssueCode::AuthorUnresolved,
        slide_number: Some(slide),
        part_name: Some(part.to_owned()),
        relationship_id: Some(relation.id.clone()),
        relationship_type: Some(relation.relationship_type.clone()),
        qualified_element_name: Some("cm".to_owned()),
        author_id: Some(author_id.to_owned()),
        text: Some(text.to_owned()),
        reason: format!("Comment author {author_id} was not resolved"),
    });
}

pub(super) fn duplicate_author_issue(
    inventory: &mut NotesCommentsInventory,
    part: &str,
    relation: &Relationship,
    author_id: &str,
) {
    inventory.issues.push(AnnotationIssue {
        code: AnnotationIssueCode::AuthorDuplicate,
        slide_number: None,
        part_name: Some(part.to_owned()),
        relationship_id: Some(relation.id.clone()),
        relationship_type: Some(relation.relationship_type.clone()),
        qualified_element_name: Some("cmAuthor".to_owned()),
        author_id: Some(author_id.to_owned()),
        text: None,
        reason: format!("Comment author ID {author_id} is ambiguous"),
    });
}

pub(super) fn issue(
    inventory: &mut NotesCommentsInventory,
    code: AnnotationIssueCode,
    context: IssueContext<'_>,
    reason: String,
) {
    inventory.issues.push(AnnotationIssue {
        code,
        slide_number: context.slide,
        part_name: context.part.map(str::to_owned),
        relationship_id: context.relation.map(|value| value.id.clone()),
        relationship_type: context
            .relation
            .map(|value| value.relationship_type.clone()),
        qualified_element_name: context.element.map(str::to_owned),
        author_id: None,
        text: context.text.map(str::to_owned),
        reason,
    });
}

pub(super) fn relationships_path(part: &str) -> String {
    part.rsplit_once('/').map_or_else(
        || format!("_rels/{part}.rels"),
        |(dir, file)| format!("{dir}/_rels/{file}.rels"),
    )
}

pub(super) fn document_is_exact(xml: &str) -> bool {
    super::relationship_xml::document_is_exact(xml)
}
