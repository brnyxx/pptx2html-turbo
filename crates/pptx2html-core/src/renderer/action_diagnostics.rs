use std::cmp::Ordering;

use crate::model::{
    ActionSet, ActionTrigger, CapabilityStage, ConversionDiagnostic, DiagnosticLocation,
    FallbackKind, FeatureFamily, SupportTier,
};

use super::RenderCtx;

#[derive(Clone, Copy)]
pub(super) struct TextActionOwner {
    pub(super) shape_id: u32,
    pub(super) paragraph_index: usize,
    pub(super) table_cell: Option<(usize, usize)>,
}

impl TextActionOwner {
    pub(super) fn run_identity(self, ctx: &RenderCtx<'_>, run_index: usize) -> String {
        let slide = ctx.collector.borrow().current_slide_index + 1;
        let cell = self
            .table_cell
            .map(|(row, column)| format!("/table-cell-r{row}c{column}"))
            .unwrap_or_default();
        format!(
            "slide-{slide}/shape-{}{cell}/paragraph-{}/run-{run_index}",
            self.shape_id, self.paragraph_index
        )
    }
}

pub(super) fn emit(actions: &ActionSet, ctx: &RenderCtx<'_>, identity: &str) {
    for action in [actions.click.as_ref(), actions.hover.as_ref()]
        .into_iter()
        .flatten()
    {
        let Some(issue) = action.issue else {
            continue;
        };
        let trigger = match action.trigger {
            ActionTrigger::Click => "click",
            ActionTrigger::Hover => "hover",
        };
        let mode = action.relationship_mode.as_deref().unwrap_or("none");
        let slide_index = ctx.collector.borrow().current_slide_index;
        ctx.collector
            .borrow_mut()
            .diagnostics
            .push(ConversionDiagnostic {
                code: issue.code().to_owned(),
                family: FeatureFamily::Text,
                support_tier: SupportTier::Fallback,
                stage: Some(CapabilityStage::Rendered),
                location: DiagnosticLocation {
                    slide_index: Some(slide_index),
                    part_name: action.source_part.clone(),
                    relationship_id: action.relationship_id.clone(),
                    relationship_type: action.relationship_type.clone(),
                    qualified_element_name: Some(
                        match action.trigger {
                            ActionTrigger::Click => "a:hlinkClick",
                            ActionTrigger::Hover => "a:hlinkMouseOver",
                        }
                        .to_owned(),
                    ),
                    ..Default::default()
                },
                raw_reference: action.raw_action.clone(),
                fallback_kind: FallbackKind::ActionMetadata,
                reason: format!("trigger={trigger};mode={mode};identity={identity}"),
            });
    }
}

pub(super) fn compare(left: &ConversionDiagnostic, right: &ConversionDiagnostic) -> Ordering {
    if !left.code.starts_with("ACTION_") || !right.code.starts_with("ACTION_") {
        return Ordering::Equal;
    }
    left.code
        .cmp(&right.code)
        .then_with(|| left.reason.cmp(&right.reason))
        .then_with(|| {
            left.location
                .relationship_type
                .cmp(&right.location.relationship_type)
        })
}

pub(super) fn exact_duplicate(left: &ConversionDiagnostic, right: &ConversionDiagnostic) -> bool {
    left.code.starts_with("ACTION_") && right.code.starts_with("ACTION_") && left == right
}
