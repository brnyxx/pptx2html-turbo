use std::fmt::Write;

use crate::model::{
    Action, ActionSet, ActionTarget, CapabilityStage, ConversionDiagnostic, DiagnosticLocation,
    FallbackKind, FeatureFamily, Presentation, Shape, ShapeType, SupportTier, TextBody,
    is_safe_external_uri,
};

use super::{RenderCtx, action_diagnostics, escape_html};

fn target_name(target: &ActionTarget) -> &'static str {
    match target {
        ActionTarget::ExternalUri(_) => "external",
        ActionTarget::InternalSlide(_) => "slide",
        ActionTarget::Next => "next",
        ActionTarget::Previous => "previous",
        ActionTarget::First => "first",
        ActionTarget::Last => "last",
        ActionTarget::NoOp => "none",
        ActionTarget::MediaPlay => "media",
        ActionTarget::Unsupported(_) => "unsupported",
    }
}

fn data_attributes(action: &Action, hover: bool) -> String {
    let mut attributes = String::new();
    let name = target_name(&action.target);
    if hover {
        let _ = write!(attributes, " data-hover-action=\"{name}\"");
    } else {
        let _ = write!(attributes, " data-action=\"{name}\"");
        if let ActionTarget::InternalSlide(index) = action.target {
            let _ = write!(attributes, " data-slide-target=\"{index}\"");
        }
    }
    if let Some(tooltip) = action.tooltip.as_deref() {
        let _ = write!(attributes, " title=\"{}\"", escape_html(tooltip));
    }
    attributes
}

fn action_attributes(actions: &ActionSet) -> String {
    let mut attributes = String::new();
    if let Some(click) = actions.click.as_ref() {
        attributes.push_str(&data_attributes(click, false));
    }
    if let Some(hover) = actions.hover.as_ref() {
        attributes.push_str(&data_attributes(hover, true));
    }
    attributes
}

pub(super) fn render_run_wrapper(
    actions: &ActionSet,
    legacy_hyperlink: Option<&str>,
    run_style: &str,
    segment_html: &str,
    identity: &str,
    ctx: &RenderCtx<'_>,
    html: &mut String,
) {
    action_diagnostics::emit(actions, ctx, identity);
    if actions.click.is_none() && legacy_hyperlink.is_some_and(|uri| !is_safe_external_uri(uri)) {
        let slide_index = ctx.collector.borrow().current_slide_index;
        ctx.collector
            .borrow_mut()
            .diagnostics
            .push(ConversionDiagnostic {
                code: "ACTION_UNSAFE_URI".to_owned(),
                family: FeatureFamily::Text,
                support_tier: SupportTier::Fallback,
                stage: Some(CapabilityStage::Rendered),
                location: DiagnosticLocation {
                    slide_index: Some(slide_index),
                    qualified_element_name: Some("legacy:hyperlink".to_owned()),
                    ..Default::default()
                },
                raw_reference: None,
                fallback_kind: FallbackKind::ActionMetadata,
                reason: format!("trigger=click;mode=legacy;identity={identity}"),
            });
    }
    let safe_external = actions
        .click
        .as_ref()
        .and_then(|action| match &action.target {
            ActionTarget::ExternalUri(uri) if is_safe_external_uri(uri) => Some(uri.as_str()),
            _ => None,
        })
        .or_else(|| {
            actions
                .click
                .is_none()
                .then_some(legacy_hyperlink)
                .flatten()
                .filter(|uri| is_safe_external_uri(uri))
        });
    let attributes = action_attributes(actions);
    if let Some(href) = safe_external {
        let _ = write!(
            html,
            "<a class=\"run\" href=\"{}\" style=\"{run_style}\" target=\"_blank\" rel=\"noopener noreferrer\"{attributes}>{segment_html}</a>",
            escape_html(href)
        );
    } else if actions.click.is_some() || actions.hover.is_some() {
        let _ = write!(
            html,
            "<span class=\"run\" role=\"button\" tabindex=\"0\"{attributes} style=\"{run_style}\">{segment_html}</span>"
        );
    } else {
        let _ = write!(
            html,
            "<span class=\"run\" style=\"{run_style}\">{segment_html}</span>"
        );
    }
}

pub(super) fn render_shape_surface(
    actions: &ActionSet,
    shape_id: u32,
    ctx: &RenderCtx<'_>,
    html: &mut String,
) {
    if actions.click.is_none() && actions.hover.is_none() {
        return;
    }
    action_diagnostics::emit(actions, ctx, &format!("shape-{shape_id}"));
    let attributes = action_attributes(actions);
    let safe_external = actions
        .click
        .as_ref()
        .and_then(|action| match &action.target {
            ActionTarget::ExternalUri(uri) if is_safe_external_uri(uri) => Some(uri.as_str()),
            _ => None,
        });
    if let Some(href) = safe_external {
        let _ = write!(
            html,
            "<a class=\"shape-action-surface\" aria-label=\"shape {shape_id}\" href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\"{attributes}></a>",
            escape_html(href)
        );
    } else {
        let _ = write!(
            html,
            "<button class=\"shape-action-surface\" type=\"button\" aria-label=\"shape {shape_id}\"{attributes}></button>"
        );
    }
}

pub(super) const RUNTIME: &str = r#"<script>(()=>{const go=(e)=>{const n=e.target.closest('[data-action]');if(!n)return;const a=n.dataset.action;if(!['slide','next','previous','first','last'].includes(a))return;e.preventDefault();const s=[...document.querySelectorAll('.slide')],c=n.closest('.slide'),i=s.indexOf(c);let t=a==='slide'?document.getElementById('slide-'+n.dataset.slideTarget):a==='first'?s[0]:a==='last'?s.at(-1):s[Math.max(0,Math.min(s.length-1,i+(a==='next'?1:-1)))];if(t)location.hash=t.id};document.addEventListener('click',go);document.addEventListener('keydown',e=>{if((e.key==='Enter'||e.key===' ')&&e.target.matches('[role=button][data-action]')){e.preventDefault();e.target.click()}})})();</script>"#;

pub(super) const CSS: &str = ".run[data-action],.run[data-hover-action],.shape-action-surface~.text-body .run[href],.shape-action-surface~table .run[href],.shape-action-surface~.shape .run[href],.shape-action-surface~.shape>.shape-action-surface{pointer-events:auto}.shape-action-surface{position:absolute;inset:0;z-index:1;border:0;background:transparent;cursor:pointer}.shape-action-surface~.text-body,.shape-action-surface~table{position:relative}.shape-action-surface~.text-body,.shape-action-surface~table,.shape-action-surface~.shape{z-index:2;pointer-events:none}";

fn text_has_actions(body: &TextBody) -> bool {
    body.paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.runs)
        .any(|run| run.actions.click.is_some() || run.actions.hover.is_some())
}

fn shape_has_actions(shape: &Shape) -> bool {
    if shape.actions.click.is_some()
        || shape.actions.hover.is_some()
        || shape.text_body.as_ref().is_some_and(text_has_actions)
    {
        return true;
    }
    match &shape.shape_type {
        ShapeType::Group(children, _) => children.iter().any(shape_has_actions),
        ShapeType::Table(table) => table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .any(|cell| cell.text_body.as_ref().is_some_and(text_has_actions)),
        ShapeType::Rectangle
        | ShapeType::RoundedRectangle
        | ShapeType::Ellipse
        | ShapeType::Triangle
        | ShapeType::Arrow
        | ShapeType::Line
        | ShapeType::TextBox
        | ShapeType::Picture(_)
        | ShapeType::Chart(_)
        | ShapeType::Custom(_)
        | ShapeType::CustomGeom(_)
        | ShapeType::Unsupported(_) => false,
    }
}

pub(super) fn presentation_has_actions(presentation: &Presentation) -> bool {
    presentation
        .slides
        .iter()
        .flat_map(|slide| &slide.shapes)
        .any(shape_has_actions)
}
