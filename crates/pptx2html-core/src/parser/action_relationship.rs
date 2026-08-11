use std::collections::HashMap;

use super::relationships::{
    HYPERLINK_RELATIONSHIP, Relationship, SLIDE_RELATIONSHIP, TargetMode, resolve_internal_target,
};
use crate::model::{Action, ActionIssue, ActionTarget, ActionTrigger, is_safe_external_uri};

pub(crate) struct ActionContext<'a> {
    pub(crate) owner_part: &'a str,
    pub(crate) relationships: &'a [Relationship],
    pub(crate) slide_order: &'a HashMap<String, usize>,
}

pub(crate) fn metadata(
    context: &ActionContext<'_>,
    rel_id: &str,
) -> (Option<String>, Option<String>) {
    context
        .relationships
        .iter()
        .find(|relationship| relationship.id == rel_id)
        .map(|relationship| {
            (
                Some(relationship.relationship_type.clone()),
                Some(relationship.target_mode.as_str().to_owned()),
            )
        })
        .unwrap_or_default()
}

pub(crate) struct RelationshipAction {
    pub(crate) trigger: ActionTrigger,
    pub(crate) raw_action: Option<String>,
    pub(crate) rel_id: String,
    pub(crate) internal: bool,
    pub(crate) anchor: Option<String>,
    pub(crate) tooltip: Option<String>,
}

pub(crate) fn resolve(input: RelationshipAction, context: &ActionContext<'_>) -> Action {
    let matching: Vec<_> = context
        .relationships
        .iter()
        .filter(|relationship| relationship.id == input.rel_id)
        .collect();
    let relationship = matching.first().copied();
    let (relationship_type, relationship_mode) = metadata(context, &input.rel_id);
    let (target, issue) = if matching.len() > 1 {
        (
            ActionTarget::Unsupported(input.raw_action.clone().unwrap_or_default()),
            Some(ActionIssue::DuplicateRelationship),
        )
    } else if let Some(relationship) = relationship {
        if input.internal {
            internal_target(
                relationship,
                context,
                input.raw_action.as_deref().unwrap_or_default(),
            )
        } else {
            external_target(
                relationship,
                input.raw_action.as_deref().unwrap_or_default(),
            )
        }
    } else {
        (
            ActionTarget::Unsupported(input.raw_action.clone().unwrap_or_default()),
            Some(ActionIssue::MissingRelationship),
        )
    };
    Action {
        trigger: input.trigger,
        target,
        relationship_id: Some(input.rel_id),
        relationship_type,
        relationship_mode,
        source_part: Some(context.owner_part.to_owned()),
        raw_action: input.raw_action,
        anchor: input.anchor,
        tooltip: input.tooltip,
        issue,
    }
}

fn external_target(relationship: &Relationship, raw: &str) -> (ActionTarget, Option<ActionIssue>) {
    if relationship.relationship_type != HYPERLINK_RELATIONSHIP
        || relationship.target_mode != TargetMode::External
    {
        return (
            ActionTarget::Unsupported(raw.to_owned()),
            Some(ActionIssue::RelationshipMismatch),
        );
    }
    let issue = (!is_safe_external_uri(&relationship.target)).then_some(ActionIssue::UnsafeUri);
    (
        ActionTarget::ExternalUri(relationship.target.clone()),
        issue,
    )
}

fn internal_target(
    relationship: &Relationship,
    context: &ActionContext<'_>,
    raw: &str,
) -> (ActionTarget, Option<ActionIssue>) {
    if relationship.relationship_type != SLIDE_RELATIONSHIP
        || relationship.target_mode != TargetMode::Internal
    {
        return (
            ActionTarget::Unsupported(raw.to_owned()),
            Some(ActionIssue::RelationshipMismatch),
        );
    }
    let Ok(part) = resolve_internal_target(context.owner_part, &relationship.target) else {
        return (
            ActionTarget::Unsupported(raw.to_owned()),
            Some(ActionIssue::UnsafeInternalTarget),
        );
    };
    context
        .slide_order
        .get(&part)
        .copied()
        .map(|index| (ActionTarget::InternalSlide(index), None))
        .unwrap_or((
            ActionTarget::Unsupported(raw.to_owned()),
            Some(ActionIssue::UnresolvedSlide),
        ))
}
