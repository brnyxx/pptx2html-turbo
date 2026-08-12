use std::collections::HashSet;

use crate::model::{
    CapabilityStage, CommentKind, ConversionDiagnostic, DiagnosticLocation, FallbackKind,
    FeatureFamily, NotesCommentsInventory, SlideComment, SupportTier,
};

pub(super) fn append(
    inventory: &NotesCommentsInventory,
    diagnostics: &mut Vec<ConversionDiagnostic>,
    include_slide: impl Fn(usize) -> bool,
    include_unreferenced_authors: bool,
) {
    let referenced_authors: HashSet<_> = inventory
        .comments
        .iter()
        .filter(|comment| include_slide(comment.slide_number))
        .map(|comment| (comment.kind.as_str(), comment.author_id.as_str()))
        .collect();
    for author in &inventory.authors {
        if !include_unreferenced_authors
            && !referenced_authors.contains(&(author.kind.as_str(), author.id.as_str()))
        {
            continue;
        }
        diagnostics.push(ConversionDiagnostic {
            code: "COMMENT_AUTHOR_METADATA".to_owned(),
            family: FeatureFamily::Unsupported,
            support_tier: SupportTier::Fallback,
            stage: Some(CapabilityStage::Parsed),
            location: DiagnosticLocation {
                part_name: Some(author.part_name.clone()),
                qualified_element_name: Some(
                    match author.kind {
                        CommentKind::Legacy => "p:cmAuthor",
                        CommentKind::Modern => "p188:author",
                    }
                    .to_owned(),
                ),
                ..Default::default()
            },
            raw_reference: Some(format!(
                "kind={}\nid={}\nname={}\ninitials={}",
                author.kind.as_str(),
                author.id,
                author.name,
                author.initials.as_deref().unwrap_or(""),
            )),
            fallback_kind: FallbackKind::PreservedPart,
            reason: "Comment author was preserved as package metadata".to_owned(),
        });
    }

    for note in &inventory.notes {
        if !include_slide(note.slide_number) {
            continue;
        }
        diagnostics.push(ConversionDiagnostic {
            code: "NOTES_SLIDE_METADATA".to_owned(),
            family: FeatureFamily::Unsupported,
            support_tier: SupportTier::Fallback,
            stage: Some(CapabilityStage::Parsed),
            location: DiagnosticLocation {
                slide_index: note.slide_number.checked_sub(1),
                part_name: Some(note.part_name.clone()),
                relationship_id: Some(note.relationship_id.clone()),
                qualified_element_name: Some("p:notes".to_owned()),
                ..Default::default()
            },
            raw_reference: Some(format!(
                "kind=notes\nslide_number={}\npart={}\nrelationship_id={}\nnotes_master={}\nnotes_master_relationship_id={}\ntext={}",
                note.slide_number,
                note.part_name,
                note.relationship_id,
                note.notes_master_part.as_deref().unwrap_or(""),
                note.notes_master_relationship_id.as_deref().unwrap_or(""),
                note.text,
            )),
            fallback_kind: FallbackKind::PreservedPart,
            reason: "Slide notes were preserved as off-canvas metadata".to_owned(),
        });
    }

    for comment in &inventory.comments {
        if !include_slide(comment.slide_number) {
            continue;
        }
        diagnostics.push(comment_diagnostic(comment));
        if let Some(raw_xml) = &comment.raw_extension_xml {
            diagnostics.push(ConversionDiagnostic {
                code: "MODERN_COMMENT_EXTENSION_FALLBACK".to_owned(),
                family: FeatureFamily::Unsupported,
                support_tier: SupportTier::Fallback,
                stage: Some(CapabilityStage::Parsed),
                location: comment_location(comment, "p188:extLst"),
                raw_reference: Some(raw_xml.clone()),
                fallback_kind: FallbackKind::UnknownElement,
                reason: "Modern comment extension XML was preserved without exact interpretation"
                    .to_owned(),
            });
        }
    }

    for issue in &inventory.issues {
        if issue
            .slide_number
            .is_some_and(|slide_number| !include_slide(slide_number))
        {
            continue;
        }
        diagnostics.push(ConversionDiagnostic {
            code: issue.code.as_str().to_owned(),
            family: FeatureFamily::Unsupported,
            support_tier: SupportTier::Fallback,
            stage: Some(CapabilityStage::Parsed),
            location: DiagnosticLocation {
                slide_index: issue.slide_number.and_then(|value| value.checked_sub(1)),
                part_name: issue.part_name.clone(),
                relationship_id: issue.relationship_id.clone(),
                relationship_type: issue.relationship_type.clone(),
                qualified_element_name: issue.qualified_element_name.clone(),
                ..Default::default()
            },
            raw_reference: issue.text.as_ref().map(|text| {
                format!(
                    "slide_number={}\nauthor_id={}\ntext={text}",
                    issue.slide_number.unwrap_or_default(),
                    issue.author_id.as_deref().unwrap_or(""),
                )
            }),
            fallback_kind: FallbackKind::PreservedPart,
            reason: issue.reason.clone(),
        });
    }
}

fn comment_diagnostic(comment: &SlideComment) -> ConversionDiagnostic {
    let author = comment.author.as_ref();
    ConversionDiagnostic {
        code: match comment.kind {
            CommentKind::Legacy => "LEGACY_COMMENT_METADATA",
            CommentKind::Modern => "MODERN_COMMENT_METADATA",
        }
        .to_owned(),
        family: FeatureFamily::Unsupported,
        support_tier: SupportTier::Fallback,
        stage: Some(CapabilityStage::Parsed),
        location: comment_location(
            comment,
            match comment.kind {
                CommentKind::Legacy => "p:cm",
                CommentKind::Modern => "p188:cm",
            },
        ),
        raw_reference: Some(format!(
            "kind={}\nslide_number={}\npart={}\nrelationship_id={}\nid={}\nparent_id={}\nauthor_id={}\nauthor={}\nauthor_part={}\ncreated={}\ntext={}",
            comment.kind.as_str(),
            comment.slide_number,
            comment.part_name,
            comment.relationship_id,
            comment.id,
            comment.parent_id.as_deref().unwrap_or(""),
            comment.author_id,
            author.map_or("", |value| value.name.as_str()),
            author.map_or("", |value| value.part_name.as_str()),
            comment.created_at.as_deref().unwrap_or(""),
            comment.text,
        )),
        fallback_kind: FallbackKind::PreservedPart,
        reason: "Slide comment was preserved as off-canvas metadata".to_owned(),
    }
}

fn comment_location(comment: &SlideComment, element: &str) -> DiagnosticLocation {
    DiagnosticLocation {
        slide_index: comment.slide_number.checked_sub(1),
        part_name: Some(comment.part_name.clone()),
        relationship_id: Some(comment.relationship_id.clone()),
        qualified_element_name: Some(element.to_owned()),
        ..Default::default()
    }
}
