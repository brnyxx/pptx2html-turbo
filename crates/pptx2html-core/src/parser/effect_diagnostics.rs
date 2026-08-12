use std::fmt::Write;

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use super::preserved_parser::{append_empty_element, append_end_element, append_start_element};
use super::xml_utils;
use crate::model::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    Position, Size, SupportTier,
};

const DRAWINGML_NS: &[u8] = b"http://schemas.openxmlformats.org/drawingml/2006/main";
const PRESENTATIONML_NS: &[u8] = b"http://schemas.openxmlformats.org/presentationml/2006/main";
const RAW_XML_LIMIT_BYTES: usize = 65_536;
const TYPED_VALUE_LIMIT_BYTES: usize = 1_024;

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
                    active.properties.observe(
                        active.kind,
                        &active.path,
                        &local,
                        &namespace,
                        &element,
                    );
                    active.path.push(Ancestor {
                        local: local.clone(),
                        namespace: Namespace::from_resolved(&namespace),
                    });
                    active.depth += 1;
                } else if is_drawingml(&namespace)
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
                        properties: EffectProperties::from_root(kind, &element),
                        path: vec![Ancestor {
                            local: local.clone(),
                            namespace: Namespace::Drawing,
                        }],
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
                    &namespace,
                    &element,
                    (&mut current_shape_id, &mut position, &mut size),
                );
                stack.push(Ancestor {
                    local,
                    namespace: Namespace::from_resolved(&namespace),
                });
            }
            Ok((namespace, Event::Empty(element))) => {
                let local = xml_utils::local_name(element.name().as_ref()).to_owned();
                if let Some(active) = capture.as_mut() {
                    append_empty_element(&element, &local, &mut active.raw_xml);
                    active.properties.observe(
                        active.kind,
                        &active.path,
                        &local,
                        &namespace,
                        &element,
                    );
                } else if is_drawingml(&namespace)
                    && let Some(kind) = EffectKind::from_local(&local)
                {
                    let context = context(part_name, &stack, current_shape_id.as_deref());
                    let mut raw_xml = String::new();
                    append_empty_element(&element, &local, &mut raw_xml);
                    diagnostics.push(diagnostic(
                        part_name,
                        Capture {
                            kind,
                            context,
                            depth: 0,
                            raw_xml,
                            properties: EffectProperties::from_root(kind, &element),
                            path: Vec::new(),
                            position,
                            size,
                            encounter,
                        },
                    ));
                    encounter += 1;
                }
                track_shape_state(
                    part_name,
                    &stack,
                    &local,
                    &namespace,
                    &element,
                    (&mut current_shape_id, &mut position, &mut size),
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
                    if active.depth > 0 {
                        active.path.pop();
                    }
                    if active.depth == 0
                        && let Some(active) = capture.take()
                    {
                        diagnostics.push(diagnostic(part_name, active));
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Namespace {
    Drawing,
    Presentation,
    Other,
}

impl Namespace {
    fn from_resolved(namespace: &ResolveResult<'_>) -> Self {
        match namespace {
            ResolveResult::Bound(value) if value.as_ref() == DRAWINGML_NS => Self::Drawing,
            ResolveResult::Bound(value) if value.as_ref() == PRESENTATIONML_NS => {
                Self::Presentation
            }
            _ => Self::Other,
        }
    }
}

struct Ancestor {
    local: String,
    namespace: Namespace,
}

fn track_shape_state(
    part_name: &str,
    stack: &[Ancestor],
    local: &str,
    namespace: &ResolveResult<'_>,
    element: &BytesStart<'_>,
    state: (
        &mut Option<String>,
        &mut Option<Position>,
        &mut Option<Size>,
    ),
) {
    let (current_shape_id, position, size) = state;
    if !part_name.starts_with("ppt/slides/") {
        return;
    }
    if local == "cNvPr"
        && Namespace::from_resolved(namespace) == Namespace::Presentation
        && stack.last().is_some_and(|parent| {
            parent.local == "nvSpPr" && parent.namespace == Namespace::Presentation
        })
    {
        *current_shape_id = attribute(element, "id")
            .and_then(|value| value.parse::<u32>().ok())
            .map(|value| value.to_string());
    } else if local == "off"
        && Namespace::from_resolved(namespace) == Namespace::Drawing
        && valid_direct_shape_ancestors(stack)
    {
        *position = Some(Position {
            x: crate::model::Emu::parse_emu(&attribute(element, "x").unwrap_or_default()),
            y: crate::model::Emu::parse_emu(&attribute(element, "y").unwrap_or_default()),
        });
    } else if local == "ext"
        && Namespace::from_resolved(namespace) == Namespace::Drawing
        && valid_direct_shape_ancestors(stack)
    {
        *size = Some(Size {
            width: crate::model::Emu::parse_emu(&attribute(element, "cx").unwrap_or_default()),
            height: crate::model::Emu::parse_emu(&attribute(element, "cy").unwrap_or_default()),
        });
    }
}

fn is_drawingml(namespace: &ResolveResult<'_>) -> bool {
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

    fn name(self) -> &'static str {
        match self {
            Self::Reflection => "reflection",
            Self::Scene3d => "scene3d",
            Self::Shape3d => "sp3d",
            Self::EffectDag => "effectDag",
        }
    }
}

#[derive(Clone)]
enum EffectContext {
    DirectShape(String),
    ThemeStyle,
    Other,
}

fn context(part_name: &str, stack: &[Ancestor], shape_id: Option<&str>) -> EffectContext {
    if part_name.starts_with("ppt/slides/")
        && valid_direct_shape_ancestors(stack)
        && let Some(shape_id) = shape_id
    {
        EffectContext::DirectShape(shape_id.to_owned())
    } else if part_name.starts_with("ppt/theme/") && valid_theme_ancestors(stack) {
        EffectContext::ThemeStyle
    } else {
        EffectContext::Other
    }
}

fn valid_direct_shape_ancestors(stack: &[Ancestor]) -> bool {
    let Some(sp_pr_index) = stack.iter().rposition(|ancestor| ancestor.local == "spPr") else {
        return false;
    };
    if stack[sp_pr_index].namespace != Namespace::Presentation {
        return false;
    }
    stack.iter().enumerate().all(|(index, ancestor)| {
        if index <= sp_pr_index {
            ancestor.namespace == Namespace::Presentation
                && matches!(
                    ancestor.local.as_str(),
                    "sld" | "cSld" | "spTree" | "sp" | "spPr"
                )
        } else {
            ancestor.namespace == Namespace::Drawing
                && matches!(
                    ancestor.local.as_str(),
                    "xfrm" | "effectLst" | "effectDag" | "cont"
                )
        }
    })
}

fn valid_theme_ancestors(stack: &[Ancestor]) -> bool {
    stack.iter().all(|ancestor| {
        ancestor.namespace == Namespace::Drawing
            && matches!(
                ancestor.local.as_str(),
                "theme"
                    | "themeElements"
                    | "fmtScheme"
                    | "effectStyleLst"
                    | "effectStyle"
                    | "effectLst"
                    | "effectDag"
                    | "cont"
            )
    }) && stack.iter().any(|ancestor| ancestor.local == "effectStyle")
}

struct Capture {
    kind: EffectKind,
    context: EffectContext,
    depth: usize,
    raw_xml: String,
    properties: EffectProperties,
    path: Vec<Ancestor>,
    position: Option<Position>,
    size: Option<Size>,
    encounter: usize,
}

#[derive(Default)]
struct EffectProperties {
    blur_radius_emu: Option<String>,
    start_alpha: Option<String>,
    end_alpha: Option<String>,
    start_position: Option<String>,
    end_position: Option<String>,
    distance_emu: Option<String>,
    direction: Option<String>,
    fade_direction: Option<String>,
    scale_x: Option<String>,
    scale_y: Option<String>,
    skew_x: Option<String>,
    skew_y: Option<String>,
    alignment: Option<String>,
    rotate_with_shape: Option<String>,
    camera_preset: Option<String>,
    camera_fov: Option<String>,
    camera_zoom: Option<String>,
    camera_latitude: Option<String>,
    camera_longitude: Option<String>,
    camera_revolution: Option<String>,
    light_rig: Option<String>,
    light_direction: Option<String>,
    light_latitude: Option<String>,
    light_longitude: Option<String>,
    light_revolution: Option<String>,
    material: Option<String>,
    shape_depth_emu: Option<String>,
    extrusion_height_emu: Option<String>,
    contour_width_emu: Option<String>,
    top_bevel_preset: Option<String>,
    top_bevel_width_emu: Option<String>,
    top_bevel_height_emu: Option<String>,
    bottom_bevel_preset: Option<String>,
    bottom_bevel_width_emu: Option<String>,
    bottom_bevel_height_emu: Option<String>,
    dag_name: Option<String>,
}

impl EffectProperties {
    fn from_root(kind: EffectKind, element: &BytesStart<'_>) -> Self {
        let mut properties = Self::default();
        match kind {
            EffectKind::Reflection => properties.observe_reflection(element),
            EffectKind::Shape3d => {
                properties.shape_depth_emu = attribute(element, "z");
                properties.extrusion_height_emu = attribute(element, "extrusionH");
                properties.contour_width_emu = attribute(element, "contourW");
                properties.material = attribute(element, "prstMaterial");
            }
            EffectKind::EffectDag => properties.dag_name = attribute(element, "name"),
            EffectKind::Scene3d => {}
        }
        properties
    }

    fn observe_reflection(&mut self, element: &BytesStart<'_>) {
        self.blur_radius_emu = attribute(element, "blurRad");
        self.start_alpha = attribute(element, "stA");
        self.end_alpha = attribute(element, "endA");
        self.start_position = attribute(element, "stPos");
        self.end_position = attribute(element, "endPos");
        self.distance_emu = attribute(element, "dist");
        self.direction = attribute(element, "dir");
        self.fade_direction = attribute(element, "fadeDir");
        self.scale_x = attribute(element, "sx");
        self.scale_y = attribute(element, "sy");
        self.skew_x = attribute(element, "kx");
        self.skew_y = attribute(element, "ky");
        self.alignment = attribute(element, "algn");
        self.rotate_with_shape = attribute(element, "rotWithShape");
    }

    fn observe(
        &mut self,
        kind: EffectKind,
        path: &[Ancestor],
        local: &str,
        namespace: &ResolveResult<'_>,
        element: &BytesStart<'_>,
    ) {
        if !is_drawingml(namespace) {
            return;
        }
        match (kind, local) {
            (EffectKind::Scene3d, "camera") if path_is(path, &["scene3d"]) => {
                self.camera_preset = attribute(element, "prst");
                self.camera_fov = attribute(element, "fov");
                self.camera_zoom = attribute(element, "zoom");
            }
            (EffectKind::Scene3d, "lightRig") if path_is(path, &["scene3d"]) => {
                self.light_rig = attribute(element, "rig");
                self.light_direction = attribute(element, "dir");
            }
            (EffectKind::Scene3d, "rot") if path_is(path, &["scene3d", "camera"]) => {
                self.camera_latitude = attribute(element, "lat");
                self.camera_longitude = attribute(element, "lon");
                self.camera_revolution = attribute(element, "rev");
            }
            (EffectKind::Scene3d, "rot") if path_is(path, &["scene3d", "lightRig"]) => {
                self.light_latitude = attribute(element, "lat");
                self.light_longitude = attribute(element, "lon");
                self.light_revolution = attribute(element, "rev");
            }
            (EffectKind::Shape3d, "bevelT") if path_is(path, &["sp3d"]) => {
                self.top_bevel_preset = attribute(element, "prst");
                self.top_bevel_width_emu = attribute(element, "w");
                self.top_bevel_height_emu = attribute(element, "h");
            }
            (EffectKind::Shape3d, "bevelB") if path_is(path, &["sp3d"]) => {
                self.bottom_bevel_preset = attribute(element, "prst");
                self.bottom_bevel_width_emu = attribute(element, "w");
                self.bottom_bevel_height_emu = attribute(element, "h");
            }
            _ => {}
        }
    }

    fn write_json(&self, json: &mut String) {
        let fields = [
            ("blur_radius_emu", self.blur_radius_emu.as_deref()),
            ("start_alpha", self.start_alpha.as_deref()),
            ("end_alpha", self.end_alpha.as_deref()),
            ("start_position", self.start_position.as_deref()),
            ("end_position", self.end_position.as_deref()),
            ("distance_emu", self.distance_emu.as_deref()),
            ("direction", self.direction.as_deref()),
            ("fade_direction", self.fade_direction.as_deref()),
            ("scale_x", self.scale_x.as_deref()),
            ("scale_y", self.scale_y.as_deref()),
            ("skew_x", self.skew_x.as_deref()),
            ("skew_y", self.skew_y.as_deref()),
            ("alignment", self.alignment.as_deref()),
            ("rotate_with_shape", self.rotate_with_shape.as_deref()),
            ("camera_preset", self.camera_preset.as_deref()),
            ("camera_fov", self.camera_fov.as_deref()),
            ("camera_zoom", self.camera_zoom.as_deref()),
            ("camera_latitude", self.camera_latitude.as_deref()),
            ("camera_longitude", self.camera_longitude.as_deref()),
            ("camera_revolution", self.camera_revolution.as_deref()),
            ("light_rig", self.light_rig.as_deref()),
            ("light_direction", self.light_direction.as_deref()),
            ("light_latitude", self.light_latitude.as_deref()),
            ("light_longitude", self.light_longitude.as_deref()),
            ("light_revolution", self.light_revolution.as_deref()),
            ("material", self.material.as_deref()),
            ("shape_depth_emu", self.shape_depth_emu.as_deref()),
            ("extrusion_height_emu", self.extrusion_height_emu.as_deref()),
            ("contour_width_emu", self.contour_width_emu.as_deref()),
            ("top_bevel_preset", self.top_bevel_preset.as_deref()),
            ("top_bevel_width_emu", self.top_bevel_width_emu.as_deref()),
            ("top_bevel_height_emu", self.top_bevel_height_emu.as_deref()),
            ("bottom_bevel_preset", self.bottom_bevel_preset.as_deref()),
            (
                "bottom_bevel_width_emu",
                self.bottom_bevel_width_emu.as_deref(),
            ),
            (
                "bottom_bevel_height_emu",
                self.bottom_bevel_height_emu.as_deref(),
            ),
            ("dag_name", self.dag_name.as_deref()),
        ];
        for (name, value) in fields {
            if let Some(value) = value {
                write_bounded_typed_field(json, name, value);
            }
        }
    }
}

fn path_is(path: &[Ancestor], expected: &[&str]) -> bool {
    path.len() == expected.len()
        && path.iter().zip(expected).all(|(ancestor, expected)| {
            ancestor.namespace == Namespace::Drawing && ancestor.local == *expected
        })
}

fn write_bounded_typed_field(json: &mut String, name: &str, value: &str) {
    let end = floor_char_boundary(value, TYPED_VALUE_LIMIT_BYTES);
    write_json_string_field(json, name, &value[..end]);
    if end < value.len() {
        let _ = write!(
            json,
            ",\"{name}_original_bytes\":{},\"{name}_truncated\":true,\"{name}_hash_fnv1a64\":\"{:016x}\"",
            value.len(),
            fnv1a64(value.as_bytes())
        );
    }
}

fn attribute(element: &BytesStart<'_>, name: &str) -> Option<String> {
    element.attributes().flatten().find_map(|attribute| {
        (attribute.key.as_ref() == name.as_bytes())
            .then(|| String::from_utf8_lossy(&attribute.value).into_owned())
    })
}

fn diagnostic(part_name: &str, capture: Capture) -> ConversionDiagnostic {
    let Capture {
        kind,
        context,
        raw_xml,
        properties,
        position,
        size,
        encounter,
        ..
    } = capture;
    let direct_reflection =
        matches!(kind, EffectKind::Reflection) && matches!(context, EffectContext::DirectShape(_));
    let theme = matches!(context, EffectContext::ThemeStyle);
    let raw = bounded_raw_metadata(kind, encounter, &properties, &raw_xml);
    let truncation = if raw.truncated {
        format!(
            "; raw XML was truncated to {RAW_XML_LIMIT_BYTES} bytes with full length and FNV-1a hash preserved"
        )
    } else {
        String::new()
    };
    let (code, tier, stage, fallback_kind, reason) = if direct_reflection {
        (
            "DRAWINGML_REFLECTION_APPROXIMATE",
            SupportTier::Approximate,
            CapabilityStage::Rendered,
            FallbackKind::PreservedPart,
            format!(
                "Reflection was rendered with bounded browser blur, transform, and mask primitives; this approximation does not claim PowerPoint fidelity{truncation}"
            ),
        )
    } else if theme {
        (
            "DRAWINGML_THEME_EFFECT_FALLBACK",
            SupportTier::Fallback,
            CapabilityStage::Parsed,
            FallbackKind::PreservedPart,
            format!(
                "{} in a theme effect style was preserved as typed and raw metadata; inherited rendering was not invented{truncation}",
                kind.qualified_name()
            ),
        )
    } else if matches!(kind, EffectKind::Reflection) {
        (
            "DRAWINGML_REFLECTION_FALLBACK",
            SupportTier::Fallback,
            CapabilityStage::Parsed,
            FallbackKind::PreservedPart,
            format!(
                "Reflection outside a namespace-validated directly renderable slide shape was preserved as typed and raw metadata{truncation}"
            ),
        )
    } else {
        (
            "DRAWINGML_3D_FALLBACK",
            SupportTier::Fallback,
            CapabilityStage::Parsed,
            FallbackKind::PreservedPart,
            format!(
                "{} was preserved in source order as typed and raw metadata and not rendered as Office 3D{truncation}",
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
        raw_reference: Some(raw.json),
        fallback_kind,
        reason,
    }
}

struct BoundedRawMetadata {
    json: String,
    truncated: bool,
}

fn bounded_raw_metadata(
    kind: EffectKind,
    encounter: usize,
    properties: &EffectProperties,
    raw_xml: &str,
) -> BoundedRawMetadata {
    let original_bytes = raw_xml.len();
    let truncated = original_bytes > RAW_XML_LIMIT_BYTES;
    let end = if truncated {
        floor_char_boundary(raw_xml, RAW_XML_LIMIT_BYTES)
    } else {
        original_bytes
    };
    let hash = fnv1a64(raw_xml.as_bytes());
    let mut json = String::from("{");
    write_json_string_pair(&mut json, "schema", "drawingml-effect-metadata-v1");
    write_json_string_field(&mut json, "kind", kind.name());
    let _ = write!(json, ",\"encounter_order\":{encounter}");
    properties.write_json(&mut json);
    let _ = write!(
        json,
        ",\"raw_xml_limit_bytes\":{RAW_XML_LIMIT_BYTES},\"raw_xml_original_bytes\":{original_bytes},\"raw_xml_truncated\":{truncated},\"raw_xml_hash_fnv1a64\":\"{hash:016x}\""
    );
    write_json_string_field(&mut json, "raw_xml", &raw_xml[..end]);
    json.push('}');
    BoundedRawMetadata { json, truncated }
}

fn floor_char_boundary(value: &str, maximum: usize) -> usize {
    let mut end = maximum.min(value.len());
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    end
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn write_json_string_pair(json: &mut String, name: &str, value: &str) {
    write_json_string(json, name);
    json.push(':');
    write_json_string(json, value);
}

fn write_json_string_field(json: &mut String, name: &str, value: &str) {
    json.push(',');
    write_json_string_pair(json, name, value);
}

fn write_json_string(json: &mut String, value: &str) {
    json.push('"');
    for character in value.chars() {
        match character {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            '\n' => json.push_str("\\n"),
            '\r' => json.push_str("\\r"),
            '\t' => json.push_str("\\t"),
            character if character <= '\u{001f}' => {
                let _ = write!(json, "\\u{:04x}", character as u32);
            }
            character => json.push(character),
        }
    }
    json.push('"');
}
