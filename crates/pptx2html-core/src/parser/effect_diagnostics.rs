use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use super::preserved_parser::{append_empty_element, append_end_element, append_start_element};
use super::{embedded_parser, xml_utils};
use crate::model::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    Position, Size, SupportTier,
};

const DRAWINGML_NS: &[u8] = b"http://schemas.openxmlformats.org/drawingml/2006/main";

pub(crate) fn collect(part_name: &str, xml: &str, diagnostics: &mut Vec<ConversionDiagnostic>) {
    let mut reader = NsReader::from_str(xml);
    let mut stack = Vec::new();
    let mut current_shape_id = None;
    let mut position = None;
    let mut size = None;
    let mut encounter = 0usize;
    let mut capture: Option<Capture> = None;

    loop {
        match reader.read_resolved_event() {
            Ok((namespace, Event::Start(element))) => {
                let local = xml_utils::local_name(element.name().as_ref()).to_owned();
                if let Some(active) = capture.as_mut() {
                    append_start_element(&element, &local, &mut active.raw_xml);
                    active.depth += 1;
                } else if is_drawingml(namespace)
                    && let Some(kind) = EffectKind::from_local(&local)
                {
                    let context = context(part_name, &stack, current_shape_id.as_deref());
                    let mut raw_xml = String::new();
                    append_start_element(&element, &local, &mut raw_xml);
                    capture = Some(Capture {
                        kind,
                        context,
                        depth: 1,
                        raw_xml,
                        position,
                        size,
                        encounter,
                    });
                    encounter += 1;
                }
                track_shape_state(
                    part_name,
                    &stack,
                    &local,
                    &element,
                    &mut current_shape_id,
                    &mut position,
                    &mut size,
                );
                stack.push(local);
            }
            Ok((namespace, Event::Empty(element))) => {
                let local = xml_utils::local_name(element.name().as_ref()).to_owned();
                if let Some(active) = capture.as_mut() {
                    append_empty_element(&element, &local, &mut active.raw_xml);
                } else if is_drawingml(namespace)
                    && let Some(kind) = EffectKind::from_local(&local)
                {
                    let context = context(part_name, &stack, current_shape_id.as_deref());
                    let mut raw_xml = String::new();
                    append_empty_element(&element, &local, &mut raw_xml);
                    diagnostics.push(diagnostic(
                        part_name, kind, context, raw_xml, position, size, encounter,
                    ));
                    encounter += 1;
                }
                track_shape_state(
                    part_name,
                    &stack,
                    &local,
                    &element,
                    &mut current_shape_id,
                    &mut position,
                    &mut size,
                );
            }
            Ok((_, Event::Text(text))) => {
                if let Some(active) = capture.as_mut() {
                    active
                        .raw_xml
                        .push_str(&String::from_utf8_lossy(text.as_ref()));
                }
            }
            Ok((_, Event::CData(text))) => {
                if let Some(active) = capture.as_mut() {
                    active
                        .raw_xml
                        .push_str(&String::from_utf8_lossy(text.as_ref()));
                }
            }
            Ok((_, Event::End(element))) => {
                let local = xml_utils::local_name(element.name().as_ref()).to_owned();
                if let Some(active) = capture.as_mut() {
                    append_end_element(&element, &local, &mut active.raw_xml);
                    active.depth = active.depth.saturating_sub(1);
                    if active.depth == 0
                        && let Some(active) = capture.take()
                    {
                        diagnostics.push(diagnostic(
                            part_name,
                            active.kind,
                            active.context,
                            active.raw_xml,
                            active.position,
                            active.size,
                            active.encounter,
                        ));
                    }
                }
                stack.pop();
                if part_name.starts_with("ppt/slides/") && local == "sp" {
                    current_shape_id = None;
                    position = None;
                    size = None;
                }
            }
            Ok((_, Event::Eof)) | Err(_) => break,
            _ => {}
        }
    }
}

fn track_shape_state(
    part_name: &str,
    stack: &[String],
    local: &str,
    element: &BytesStart<'_>,
    current_shape_id: &mut Option<String>,
    position: &mut Option<Position>,
    size: &mut Option<Size>,
) {
    if !part_name.starts_with("ppt/slides/") {
        return;
    }
    if local == "cNvPr" && stack.last().is_some_and(|parent| parent == "nvSpPr") {
        *current_shape_id = embedded_parser::attribute_value(element, "id");
    } else if local == "off" && stack.iter().any(|name| name == "spPr") {
        *position = Some(Position {
            x: crate::model::Emu::parse_emu(
                &embedded_parser::attribute_value(element, "x").unwrap_or_default(),
            ),
            y: crate::model::Emu::parse_emu(
                &embedded_parser::attribute_value(element, "y").unwrap_or_default(),
            ),
        });
    } else if local == "ext" && stack.iter().any(|name| name == "spPr") {
        *size = Some(Size {
            width: crate::model::Emu::parse_emu(
                &embedded_parser::attribute_value(element, "cx").unwrap_or_default(),
            ),
            height: crate::model::Emu::parse_emu(
                &embedded_parser::attribute_value(element, "cy").unwrap_or_default(),
            ),
        });
    }
}

fn is_drawingml(namespace: ResolveResult<'_>) -> bool {
    matches!(namespace, ResolveResult::Bound(value) if value.as_ref() == DRAWINGML_NS)
}

#[derive(Clone, Copy)]
enum EffectKind {
    Reflection,
    Scene3d,
    Shape3d,
    EffectDag,
}

impl EffectKind {
    fn from_local(local: &str) -> Option<Self> {
        match local {
            "reflection" => Some(Self::Reflection),
            "scene3d" => Some(Self::Scene3d),
            "sp3d" => Some(Self::Shape3d),
            "effectDag" => Some(Self::EffectDag),
            _ => None,
        }
    }

    fn qualified_name(self) -> &'static str {
        match self {
            Self::Reflection => "a:reflection",
            Self::Scene3d => "a:scene3d",
            Self::Shape3d => "a:sp3d",
            Self::EffectDag => "a:effectDag",
        }
    }
}

#[derive(Clone)]
enum EffectContext {
    DirectShape(String),
    ThemeStyle,
    Other,
}

fn context(part_name: &str, stack: &[String], shape_id: Option<&str>) -> EffectContext {
    if part_name.starts_with("ppt/slides/")
        && stack.iter().any(|name| name == "spPr")
        && let Some(shape_id) = shape_id
    {
        EffectContext::DirectShape(shape_id.to_owned())
    } else if part_name.starts_with("ppt/theme/") && stack.iter().any(|name| name == "effectStyle")
    {
        EffectContext::ThemeStyle
    } else {
        EffectContext::Other
    }
}

struct Capture {
    kind: EffectKind,
    context: EffectContext,
    depth: usize,
    raw_xml: String,
    position: Option<Position>,
    size: Option<Size>,
    encounter: usize,
}

fn diagnostic(
    part_name: &str,
    kind: EffectKind,
    context: EffectContext,
    raw_xml: String,
    position: Option<Position>,
    size: Option<Size>,
    encounter: usize,
) -> ConversionDiagnostic {
    let direct_reflection =
        matches!(kind, EffectKind::Reflection) && matches!(context, EffectContext::DirectShape(_));
    let theme = matches!(context, EffectContext::ThemeStyle);
    let (code, tier, stage, fallback_kind, reason) = if direct_reflection {
        (
            "DRAWINGML_REFLECTION_APPROXIMATE",
            SupportTier::Approximate,
            CapabilityStage::Rendered,
            FallbackKind::StyleApproximation,
            "Reflection was rendered with bounded browser blur, transform, and mask primitives; this approximation does not claim PowerPoint fidelity".to_owned(),
        )
    } else if theme {
        (
            "DRAWINGML_THEME_EFFECT_FALLBACK",
            SupportTier::Fallback,
            CapabilityStage::Parsed,
            FallbackKind::PreservedEffectMetadata,
            format!(
                "{} in a theme effect style was preserved as raw metadata; inherited rendering was not invented",
                kind.qualified_name()
            ),
        )
    } else if matches!(kind, EffectKind::Reflection) {
        (
            "DRAWINGML_REFLECTION_FALLBACK",
            SupportTier::Fallback,
            CapabilityStage::Parsed,
            FallbackKind::PreservedEffectMetadata,
            "Reflection outside a directly renderable slide shape was preserved as raw metadata"
                .to_owned(),
        )
    } else {
        (
            "DRAWINGML_3D_FALLBACK",
            SupportTier::Fallback,
            CapabilityStage::Parsed,
            FallbackKind::PreservedEffectMetadata,
            format!(
                "{} was preserved in source order as raw metadata and not rendered as Office 3D",
                kind.qualified_name()
            ),
        )
    };
    let owner = match context {
        EffectContext::DirectShape(shape_id) => format!("shape-{shape_id}"),
        EffectContext::ThemeStyle | EffectContext::Other => "part".to_owned(),
    };
    ConversionDiagnostic {
        code: code.to_owned(),
        family: FeatureFamily::Shapes,
        support_tier: tier,
        stage: Some(stage),
        location: DiagnosticLocation {
            slide_index: super::preserved_parser::slide_index_from_part(part_name),
            part_name: Some(part_name.to_owned()),
            relationship_id: Some(format!("{owner}-effect-{encounter:04}")),
            qualified_element_name: Some(kind.qualified_name().to_owned()),
            position,
            size,
            ..Default::default()
        },
        raw_reference: Some(raw_xml),
        fallback_kind,
        reason,
    }
}
