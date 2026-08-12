use std::collections::BTreeSet;

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use crate::error::{PptxError, PptxResult};
use crate::model::timing::{
    AnimationEffect, AnimationTrigger, ParsedTimingInventory, SlideTransition, TimingEffect,
    TimingFallback, TimingGroup, TimingSource, TimingSourceKind, TransitionKind,
};
use crate::model::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    SupportTier,
};

use super::xml_utils;

const PML: &[u8] = b"http://schemas.openxmlformats.org/presentationml/2006/main";
const MAX_DURATION_MS: u32 = 10_000;

#[derive(Clone)]
struct Frame {
    local: String,
    pml: bool,
    node_type: Option<String>,
    node_id: Option<String>,
    delay_ms: Option<u32>,
    bounded: bool,
}

struct SourceCapture {
    local: String,
    start: usize,
    depth: usize,
    kind: TimingSourceKind,
    identity: String,
}

struct EffectCandidate {
    local: String,
    qualified_name: String,
    depth: usize,
    trigger: Option<AnimationTrigger>,
    delay_ms: Option<u32>,
    transition: Option<String>,
    filter: Option<String>,
    timing_id: Option<String>,
    duration: Option<String>,
    target: Option<String>,
    set_value: Option<String>,
    bounded: bool,
    source_order: usize,
    start: usize,
}

struct UnsupportedCandidate {
    qualified_name: String,
    depth: usize,
    timing_id: Option<String>,
    source_order: usize,
    start: usize,
}

pub(crate) fn parse(xml: &str) -> PptxResult<ParsedTimingInventory> {
    let mut reader = NsReader::from_str(xml);
    let mut inventory = ParsedTimingInventory::default();
    let mut stack = Vec::<Frame>::new();
    let mut source = None::<SourceCapture>;
    let mut transition_speed = None::<String>;
    let mut transition_kind = None::<TransitionKind>;
    let mut transition_identity = None::<String>;
    let mut transition_unsupported = false;
    let mut shape_ids = BTreeSet::new();
    let mut effect = None::<EffectCandidate>;
    let mut unsupported = None::<UnsupportedCandidate>;
    let mut last_group = None::<usize>;
    let mut order = 0usize;
    let mut previous_position = 0usize;

    loop {
        let event_start = previous_position;
        let (namespace, event) = reader.read_resolved_event().map_err(PptxError::Xml)?;
        let pml = is_pml(&namespace);
        let event = event.into_owned();
        previous_position = reader.buffer_position() as usize;
        match event {
            Event::Start(element) => {
                let local = xml_utils::local_name(element.name().as_ref()).to_owned();
                let qualified = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                let depth = stack.len() + 1;
                if pml
                    && matches!(local.as_str(), "transition" | "timing")
                    && direct_slide_child(&stack)
                    && source.is_none()
                {
                    let identity = if local == "transition" {
                        format!("slide-transition-{}", inventory.sources.len())
                    } else {
                        format!("slide-timing-{}", inventory.sources.len())
                    };
                    let kind = if local == "transition" {
                        transition_speed = attr(&element, "spd");
                        transition_identity = Some(identity.clone());
                        if attr(&element, "advTm").is_some() {
                            transition_unsupported = true;
                        }
                        TimingSourceKind::Transition
                    } else {
                        TimingSourceKind::Timing
                    };
                    source = Some(SourceCapture {
                        local: local.clone(),
                        start: event_start,
                        depth,
                        kind,
                        identity,
                    });
                } else if pml && inside_transition(&stack) {
                    match local.as_str() {
                        "cut" => transition_kind = Some(TransitionKind::Cut),
                        "fade" => transition_kind = Some(TransitionKind::Fade),
                        _ => transition_unsupported = true,
                    }
                }
                if pml
                    && local == "cNvPr"
                    && inside_shape_tree(&stack)
                    && let Some(id) = attr(&element, "id").and_then(|value| value.parse().ok())
                {
                    shape_ids.insert(id);
                }
                if pml {
                    capture_start_delay(&local, &element, &mut stack);
                }
                if pml
                    && inside_timing(&stack)
                    && matches!(local.as_str(), "animEffect" | "set")
                    && effect.is_none()
                    && unsupported.is_none()
                {
                    let (trigger, group_id, delay_ms) = trigger_context(&stack);
                    if trigger == Some(AnimationTrigger::Click) {
                        let group_identity =
                            group_id.clone().unwrap_or_else(|| format!("group-{order}"));
                        let identity_prefix = format!("timing-{group_identity}-group-");
                        if let Some(index) = inventory
                            .groups
                            .iter()
                            .position(|group| group.identity.starts_with(&identity_prefix))
                        {
                            last_group = Some(index);
                        } else {
                            inventory.groups.push(TimingGroup {
                                identity: format!("{identity_prefix}{}", inventory.groups.len()),
                                source_order: order,
                                effects: Vec::new(),
                            });
                            last_group = Some(inventory.groups.len() - 1);
                        }
                    }
                    effect = Some(EffectCandidate {
                        local: local.clone(),
                        qualified_name: qualified.clone(),
                        depth,
                        trigger,
                        delay_ms,
                        transition: attr(&element, "transition"),
                        filter: attr(&element, "filter"),
                        timing_id: None,
                        duration: None,
                        target: None,
                        set_value: None,
                        bounded: true,
                        source_order: order,
                        start: event_start,
                    });
                    order += 1;
                } else if pml
                    && inside_timing(&stack)
                    && (unsupported_time_node(&local, &element)
                        || is_unsupported_command(&local)
                        || (is_unknown_timing_node(&local) && effect.is_none()))
                    && unsupported.is_none()
                {
                    unsupported = Some(UnsupportedCandidate {
                        qualified_name: qualified.clone(),
                        depth,
                        timing_id: None,
                        source_order: order,
                        start: event_start,
                    });
                    order += 1;
                }
                if pml {
                    capture_candidate_values(&local, &element, &mut effect, &mut unsupported);
                }
                stack.push(Frame {
                    local,
                    pml,
                    node_type: if pml {
                        attr(&element, "nodeType")
                    } else {
                        None
                    },
                    node_id: if pml { attr(&element, "id") } else { None },
                    delay_ms: Some(0),
                    bounded: !unsupported_time_node("cTn", &element),
                });
            }
            Event::Empty(element) => {
                let local = xml_utils::local_name(element.name().as_ref()).to_owned();
                let qualified = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if pml
                    && matches!(local.as_str(), "transition" | "timing")
                    && direct_slide_child(&stack)
                {
                    let kind = if local == "transition" {
                        transition_speed = attr(&element, "spd");
                        transition_identity =
                            Some(format!("slide-transition-{}", inventory.sources.len()));
                        TimingSourceKind::Transition
                    } else {
                        TimingSourceKind::Timing
                    };
                    let identity = if local == "transition" {
                        transition_identity
                            .clone()
                            .unwrap_or_else(|| "slide-transition-0".to_owned())
                    } else {
                        format!("slide-timing-{}", inventory.sources.len())
                    };
                    inventory.sources.push(TimingSource {
                        identity,
                        kind,
                        raw_xml: xml
                            .get(event_start..previous_position)
                            .unwrap_or_default()
                            .to_owned(),
                    });
                }
                if pml
                    && local == "cNvPr"
                    && inside_shape_tree(&stack)
                    && let Some(id) = attr(&element, "id").and_then(|value| value.parse().ok())
                {
                    shape_ids.insert(id);
                }
                if pml && inside_transition(&stack) {
                    match local.as_str() {
                        "cut" => transition_kind = Some(TransitionKind::Cut),
                        "fade" => transition_kind = Some(TransitionKind::Fade),
                        _ => transition_unsupported = true,
                    }
                }
                if pml {
                    capture_start_delay(&local, &element, &mut stack);
                    capture_candidate_values(&local, &element, &mut effect, &mut unsupported);
                }
                if pml
                    && inside_timing(&stack)
                    && (is_unsupported_command(&local) || is_unknown_timing_node(&local))
                    && unsupported.is_none()
                    && effect.is_none()
                {
                    push_fallback(
                        &mut inventory,
                        None,
                        qualified,
                        xml.get(event_start..previous_position)
                            .unwrap_or_default()
                            .to_owned(),
                        order,
                        "timing node is outside the supported subset",
                    );
                    order += 1;
                }
            }
            Event::End(element) => {
                let local = xml_utils::local_name(element.name().as_ref()).to_owned();
                let depth = stack.len();
                if pml
                    && effect.as_ref().is_some_and(|candidate| {
                        candidate.depth == depth && candidate.local == local
                    })
                    && let Some(candidate) = effect.take()
                {
                    let raw_xml = xml
                        .get(candidate.start..previous_position)
                        .unwrap_or_default()
                        .to_owned();
                    finish_effect(candidate, raw_xml, &shape_ids, last_group, &mut inventory);
                }
                if pml
                    && unsupported
                        .as_ref()
                        .is_some_and(|candidate| candidate.depth == depth)
                    && let Some(candidate) = unsupported.take()
                {
                    let raw_xml = xml
                        .get(candidate.start..previous_position)
                        .unwrap_or_default()
                        .to_owned();
                    push_fallback(
                        &mut inventory,
                        candidate.timing_id,
                        candidate.qualified_name,
                        raw_xml,
                        candidate.source_order,
                        "timing node is outside the supported subset",
                    );
                }
                if pml
                    && source
                        .as_ref()
                        .is_some_and(|capture| capture.depth == depth && capture.local == local)
                    && let Some(capture) = source.take()
                {
                    let raw_xml = xml
                        .get(capture.start..previous_position)
                        .unwrap_or_default()
                        .to_owned();
                    inventory.sources.push(TimingSource {
                        identity: capture.identity,
                        kind: capture.kind,
                        raw_xml,
                    });
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }

    inventory.groups.retain(|group| !group.effects.is_empty());
    let transition_raw = inventory
        .sources
        .iter()
        .find(|item| item.kind == TimingSourceKind::Transition)
        .map(|item| item.raw_xml.clone())
        .unwrap_or_default();
    if let (Some(identity), Some(kind)) = (transition_identity.clone(), transition_kind) {
        if transition_unsupported {
            push_fallback(
                &mut inventory,
                None,
                "p:transition".to_owned(),
                transition_raw.clone(),
                order,
                "automatic advance or unsupported transition metadata was preserved but not executed",
            );
        }
        inventory.transition = Some(SlideTransition {
            identity,
            kind,
            duration_ms: transition_duration(transition_speed.as_deref()),
        });
    } else if transition_identity.is_some() {
        push_fallback(
            &mut inventory,
            None,
            "p:transition".to_owned(),
            transition_raw,
            order,
            "transition kind is outside the cut/fade subset",
        );
    }
    Ok(inventory)
}

pub(crate) fn collect_diagnostics(
    part_name: &str,
    xml: &str,
    diagnostics: &mut Vec<ConversionDiagnostic>,
) {
    let Ok(inventory) = parse(xml) else {
        return;
    };
    for fallback in inventory.fallbacks {
        diagnostics.push(ConversionDiagnostic {
            code: "PRESENTATIONML_TIMING_FALLBACK".to_owned(),
            family: FeatureFamily::Unsupported,
            support_tier: SupportTier::Fallback,
            stage: Some(CapabilityStage::Parsed),
            location: DiagnosticLocation {
                slide_index: slide_index_from_part(part_name),
                part_name: Some(part_name.to_owned()),
                qualified_element_name: Some(fallback.qualified_name),
                relationship_id: Some(fallback.source_order.to_string()),
                ..Default::default()
            },
            raw_reference: Some(fallback.raw_xml),
            fallback_kind: FallbackKind::UnknownElement,
            reason: fallback.reason,
        });
    }
}

fn finish_effect(
    candidate: EffectCandidate,
    raw_xml: String,
    shape_ids: &BTreeSet<u32>,
    last_group: Option<usize>,
    inventory: &mut ParsedTimingInventory,
) {
    let timing_id = candidate.timing_id.clone();
    let identity = format!(
        "timing-{}-effect-{}",
        timing_id.as_deref().unwrap_or("unknown"),
        candidate.source_order
    );
    let parsed = parse_effect(&candidate, shape_ids);
    let Some((trigger, effect, duration_ms, delay_ms, shape_id)) = parsed else {
        push_fallback(
            inventory,
            timing_id,
            candidate.qualified_name,
            raw_xml,
            candidate.source_order,
            "effect requires a click/with-previous/after-previous trigger, a resolved slide-shape target, a supported effect, and a finite duration from 1 through 10000 ms",
        );
        return;
    };
    let group = if trigger == AnimationTrigger::Click {
        inventory.groups.len().checked_sub(1)
    } else {
        last_group
    };
    let Some(group) = group.and_then(|index| inventory.groups.get_mut(index)) else {
        push_fallback(
            inventory,
            timing_id,
            candidate.qualified_name,
            raw_xml,
            candidate.source_order,
            "with-previous or after-previous effect has no preceding click group",
        );
        return;
    };
    group.effects.push(TimingEffect {
        identity,
        source_order: candidate.source_order,
        trigger,
        effect,
        duration_ms,
        delay_ms,
        shape_id,
        raw_xml,
    });
}

fn parse_effect(
    candidate: &EffectCandidate,
    _shape_ids: &BTreeSet<u32>,
) -> Option<(AnimationTrigger, AnimationEffect, u32, u32, u32)> {
    let trigger = candidate.trigger?;
    let delay_ms = candidate.delay_ms?;
    if !candidate.bounded {
        return None;
    }
    let duration_ms = candidate.duration.as_deref()?.parse::<u32>().ok()?;
    if !(1..=MAX_DURATION_MS).contains(&duration_ms) {
        return None;
    }
    let shape_id = candidate.target.as_deref()?.parse::<u32>().ok()?;
    let effect = if candidate.local == "set" {
        match candidate.set_value.as_deref() {
            Some("visible") | Some("true") => AnimationEffect::Appear,
            Some("hidden") | Some("false") => AnimationEffect::Disappear,
            _ => return None,
        }
    } else {
        match (candidate.filter.as_deref(), candidate.transition.as_deref()) {
            (Some("fade"), Some("in")) => AnimationEffect::FadeIn,
            (Some("fade"), Some("out")) => AnimationEffect::FadeOut,
            (Some("appear"), Some("in")) => AnimationEffect::Appear,
            (Some("appear"), Some("out")) => AnimationEffect::Disappear,
            _ => return None,
        }
    };
    Some((trigger, effect, duration_ms, delay_ms, shape_id))
}

fn capture_candidate_values(
    local: &str,
    element: &BytesStart<'_>,
    effect: &mut Option<EffectCandidate>,
    unsupported: &mut Option<UnsupportedCandidate>,
) {
    if local == "cTn" {
        if let Some(candidate) = effect.as_mut()
            && candidate.timing_id.is_none()
        {
            candidate.timing_id = attr(element, "id");
            candidate.duration = attr(element, "dur");
            candidate.bounded = attr(element, "repeatCount").is_none()
                && attr(element, "repeatDur").is_none()
                && !attr(element, "autoRev").is_some_and(|value| value == "1" || value == "true");
        }
        if let Some(candidate) = unsupported.as_mut()
            && candidate.timing_id.is_none()
        {
            candidate.timing_id = attr(element, "id");
        }
    } else if local == "spTgt" {
        if let Some(candidate) = effect.as_mut() {
            candidate.target = attr(element, "spid");
        }
    } else if matches!(local, "strVal" | "boolVal")
        && let Some(candidate) = effect.as_mut()
    {
        candidate.set_value = attr(element, "val");
    }
}

fn capture_start_delay(local: &str, element: &BytesStart<'_>, stack: &mut [Frame]) {
    if local != "cond"
        || !stack
            .iter()
            .any(|frame| frame.pml && frame.local == "stCondLst")
    {
        return;
    }
    let delay_ms = attr(element, "delay")
        .as_deref()
        .unwrap_or("0")
        .parse::<u32>()
        .ok()
        .filter(|delay| *delay <= MAX_DURATION_MS);
    if let Some(frame) = stack
        .iter_mut()
        .rev()
        .find(|frame| frame.pml && frame.local == "cTn")
    {
        frame.delay_ms = delay_ms;
    }
}

fn trigger_context(stack: &[Frame]) -> (Option<AnimationTrigger>, Option<String>, Option<u32>) {
    let bounded = stack
        .iter()
        .filter(|frame| frame.pml && frame.local == "cTn")
        .all(|frame| frame.bounded);
    let delay_ms = stack
        .iter()
        .filter(|frame| frame.pml && frame.local == "cTn")
        .try_fold(0u32, |total, frame| total.checked_add(frame.delay_ms?))
        .filter(|delay| *delay <= MAX_DURATION_MS);
    for frame in stack.iter().rev() {
        if frame.local != "cTn" || !frame.pml {
            continue;
        }
        let trigger = match frame.node_type.as_deref() {
            Some("clickEffect") => Some(AnimationTrigger::Click),
            Some("withEffect") => Some(AnimationTrigger::WithPrevious),
            Some("afterEffect") => Some(AnimationTrigger::AfterPrevious),
            _ => None,
        };
        if trigger.is_some() {
            return (
                trigger,
                frame.node_id.clone(),
                bounded.then_some(delay_ms).flatten(),
            );
        }
    }
    (None, None, None)
}

fn push_fallback(
    inventory: &mut ParsedTimingInventory,
    timing_id: Option<String>,
    qualified_name: String,
    raw_xml: String,
    source_order: usize,
    reason: &str,
) {
    inventory.fallbacks.push(TimingFallback {
        identity: format!(
            "timing-{}-fallback-{}",
            timing_id.as_deref().unwrap_or("unknown"),
            inventory.fallbacks.len()
        ),
        source_order,
        qualified_name,
        raw_xml,
        reason: reason.to_owned(),
    });
}

fn attr(element: &BytesStart<'_>, name: &str) -> Option<String> {
    xml_utils::attr_str(element, name)
}
fn is_pml(namespace: &ResolveResult<'_>) -> bool {
    matches!(namespace, ResolveResult::Bound(value) if value.as_ref() == PML)
}
fn direct_slide_child(stack: &[Frame]) -> bool {
    matches!(stack, [Frame { local, pml: true, .. }] if local == "sld")
}
fn inside_timing(stack: &[Frame]) -> bool {
    stack
        .iter()
        .any(|frame| frame.pml && frame.local == "timing")
}
fn inside_transition(stack: &[Frame]) -> bool {
    stack
        .iter()
        .any(|frame| frame.pml && frame.local == "transition")
}
fn inside_shape_tree(stack: &[Frame]) -> bool {
    stack
        .iter()
        .any(|frame| frame.pml && frame.local == "spTree")
}
fn unsupported_time_node(local: &str, element: &BytesStart<'_>) -> bool {
    local == "cTn"
        && (attr(element, "repeatCount").is_some()
            || attr(element, "repeatDur").is_some()
            || attr(element, "autoRev").is_some_and(|value| value == "1" || value == "true"))
}
fn is_unknown_timing_node(local: &str) -> bool {
    !matches!(
        local,
        "timing"
            | "tnLst"
            | "par"
            | "seq"
            | "cTn"
            | "childTnLst"
            | "subTnLst"
            | "stCondLst"
            | "endCondLst"
            | "condLst"
            | "cond"
            | "tgtEl"
            | "spTgt"
            | "animEffect"
            | "set"
            | "cBhvr"
            | "to"
            | "strVal"
            | "boolVal"
    ) && !is_unsupported_command(local)
}
fn is_unsupported_command(local: &str) -> bool {
    matches!(
        local,
        "anim" | "animClr" | "animMotion" | "animRot" | "animScale" | "cmd" | "audio" | "video"
    )
}
fn slide_index_from_part(name: &str) -> Option<usize> {
    name.strip_prefix("ppt/slides/slide")?
        .strip_suffix(".xml")?
        .parse::<usize>()
        .ok()?
        .checked_sub(1)
}
fn transition_duration(speed: Option<&str>) -> u32 {
    match speed {
        Some("slow") => 1_000,
        Some("fast") => 250,
        _ => 500,
    }
}
