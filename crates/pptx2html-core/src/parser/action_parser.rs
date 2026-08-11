use std::collections::HashMap;

use quick_xml::events::BytesStart;

use super::action_relationship::{self, ActionContext, RelationshipAction};
use super::slide_parser::ShapeBuilder;
use super::text_parser::RunBuilder;
use crate::model::{Action, ActionIssue, ActionTarget, ActionTrigger, is_safe_external_uri};

#[cfg(test)]
pub(crate) fn hyperlink_rel_id(element: &BytesStart<'_>) -> Option<String> {
    element.attributes().flatten().find_map(|attribute| {
        (attribute.key.as_ref() == b"r:id")
            .then(|| {
                attribute
                    .unescape_value()
                    .ok()
                    .map(|value| value.into_owned())
            })
            .flatten()
    })
}

fn attr(element: &BytesStart<'_>, name: &str) -> Option<String> {
    element.attributes().flatten().find_map(|attribute| {
        (attribute.key.as_ref() == name.as_bytes())
            .then(|| {
                attribute
                    .unescape_value()
                    .ok()
                    .map(|value| value.into_owned())
            })
            .flatten()
    })
}

fn parsed_action(
    element: &BytesStart<'_>,
    trigger: ActionTrigger,
    context: Option<&ActionContext<'_>>,
    legacy_relationships: &HashMap<String, String>,
    relationship_id: Result<Option<String>, ()>,
) -> Action {
    let invalid_relationship_namespace = relationship_id.is_err();
    let rel_id = relationship_id.ok().flatten().filter(|id| !id.is_empty());
    let raw_action = attr(element, "action");
    if invalid_relationship_namespace {
        return Action {
            trigger,
            target: ActionTarget::Unsupported(raw_action.clone().unwrap_or_default()),
            relationship_id: None,
            relationship_type: None,
            relationship_mode: None,
            source_part: context.map(|value| value.owner_part.to_owned()),
            raw_action,
            anchor: attr(element, "anchor"),
            tooltip: attr(element, "tooltip"),
            issue: Some(ActionIssue::RelationshipMismatch),
        };
    }
    if let Some(context) = context
        && let Some(rel_id) = rel_id.clone()
        && raw_action.as_deref() == Some("ppaction://hlinksldjump")
    {
        return action_relationship::resolve(
            RelationshipAction {
                trigger,
                raw_action,
                rel_id,
                internal: true,
                anchor: attr(element, "anchor"),
                tooltip: attr(element, "tooltip"),
            },
            context,
        );
    }
    if let Some(context) = context
        && let Some(rel_id) = rel_id.clone()
        && raw_action.is_none()
    {
        return action_relationship::resolve(
            RelationshipAction {
                trigger,
                raw_action,
                rel_id,
                internal: false,
                anchor: attr(element, "anchor"),
                tooltip: attr(element, "tooltip"),
            },
            context,
        );
    }
    let target = match raw_action.as_deref() {
        Some("ppaction://hlinkshowjump?jump=nextslide") => ActionTarget::Next,
        Some("ppaction://hlinkshowjump?jump=previousslide") => ActionTarget::Previous,
        Some("ppaction://hlinkshowjump?jump=firstslide") => ActionTarget::First,
        Some("ppaction://hlinkshowjump?jump=lastslide") => ActionTarget::Last,
        Some("ppaction://media") => ActionTarget::MediaPlay,
        Some(raw) => ActionTarget::Unsupported(raw.to_owned()),
        None => rel_id
            .as_ref()
            .and_then(|id| legacy_relationships.get(id))
            .cloned()
            .map(ActionTarget::ExternalUri)
            .unwrap_or(ActionTarget::NoOp),
    };
    let issue = match &target {
        ActionTarget::Unsupported(_) => Some(ActionIssue::Unsupported),
        ActionTarget::ExternalUri(uri) if !is_safe_external_uri(uri) => {
            Some(ActionIssue::UnsafeUri)
        }
        _ => None,
    };
    let (relationship_type, relationship_mode) = context
        .zip(rel_id.as_deref())
        .map(|(context, rel_id)| action_relationship::metadata(context, rel_id))
        .unwrap_or_default();
    Action {
        trigger,
        target,
        relationship_id: rel_id,
        relationship_type,
        relationship_mode,
        source_part: context.map(|value| value.owner_part.to_owned()),
        raw_action,
        anchor: attr(element, "anchor"),
        tooltip: attr(element, "tooltip"),
        issue,
    }
}

pub(crate) struct ActionTargets<'a> {
    pub(crate) shape: &'a mut Option<ShapeBuilder>,
    pub(crate) shape_run: &'a mut Option<RunBuilder>,
    pub(crate) cell_run: &'a mut Option<RunBuilder>,
}

pub(crate) fn handle(
    local: &str,
    element: &BytesStart<'_>,
    context: Option<&ActionContext<'_>>,
    legacy_relationships: &HashMap<String, String>,
    run_scope: bool,
    relationship_id: Result<Option<String>, ()>,
    targets: ActionTargets<'_>,
) -> bool {
    let trigger = match local {
        "hlinkClick" => ActionTrigger::Click,
        "hlinkHover" | "hlinkMouseOver" => ActionTrigger::Hover,
        _ => return false,
    };
    let action = parsed_action(
        element,
        trigger,
        context,
        legacy_relationships,
        relationship_id,
    );
    if run_scope {
        for run in [targets.shape_run, targets.cell_run].into_iter().flatten() {
            if let ActionTarget::ExternalUri(uri) = &action.target {
                run.hyperlink = Some(uri.clone());
            }
            run.actions.assign(action.clone());
        }
    } else if let Some(shape) = targets.shape {
        shape.actions.assign(action);
    }
    true
}
