use std::fmt::Write;

use super::custom_geometry_diagnostic::{self, CustomGeometryMetadata};

use super::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    Position, RenderCtx, Size, SupportTier, UnresolvedElement, UnresolvedType, UnsupportedData,
    escape_html,
};

pub(super) fn render_unsupported(
    data: &UnsupportedData,
    pos: Position,
    size: Size,
    ctx: &RenderCtx<'_>,
    html: &mut String,
) {
    let mut coll = ctx.collector.borrow_mut();
    let placeholder_id = format!("unresolved-s{}-e{}", coll.current_slide_index, coll.counter);
    coll.counter += 1;

    let type_attr = match data.element_type {
        UnresolvedType::SmartArt => "smartart",
        UnresolvedType::OleObject => "ole",
        UnresolvedType::MathEquation => "math",
        UnresolvedType::CustomGeometry => "custom-geometry",
    };

    let escaped = escape_html(&data.label);
    let _ = writeln!(
        html,
        "<div class=\"unresolved-element\" id=\"{placeholder_id}\" \
                 data-type=\"{type_attr}\" data-slide=\"{}\">\
                 <span>[{escaped}]</span></div>",
        coll.current_slide_index
    );

    let pos_non_zero = pos.x.0 != 0 || pos.y.0 != 0;
    let size_non_zero = size.width.0 != 0 || size.height.0 != 0;
    let slide_idx = coll.current_slide_index;
    let metadata = custom_geometry_diagnostic::metadata(data);
    let diagnostic = fallback_diagnostic(data, slide_idx, pos, size, &placeholder_id, &metadata);
    let elem = UnresolvedElement {
        slide_index: slide_idx,
        element_type: data.element_type.clone(),
        placeholder_id,
        position: if pos_non_zero { Some(pos) } else { None },
        size: if size_non_zero { Some(size) } else { None },
        raw_xml: metadata.raw_reference.clone(),
        data_model: metadata.data_model,
    };
    coll.diagnostics.push(diagnostic);
    coll.elements.push(elem);

    drop(coll);
    html.push_str("</div>\n");
}

fn fallback_diagnostic(
    data: &UnsupportedData,
    slide_index: usize,
    position: Position,
    size: Size,
    placeholder_id: &str,
    metadata: &CustomGeometryMetadata,
) -> ConversionDiagnostic {
    let (code, qualified_name, fallback_kind) = match data.element_type {
        UnresolvedType::SmartArt => (
            "DRAWINGML_SMARTART_FALLBACK",
            "dgm:relIds",
            FallbackKind::SmartArtPlaceholder,
        ),
        UnresolvedType::OleObject => (
            "PRESENTATIONML_OLE_FALLBACK",
            "p:oleObj",
            FallbackKind::OlePlaceholder,
        ),
        UnresolvedType::MathEquation => (
            "PRESENTATIONML_MATH_FALLBACK",
            "m:oMath",
            FallbackKind::MathPlaceholder,
        ),
        UnresolvedType::CustomGeometry => (
            "DRAWINGML_CUSTOM_GEOMETRY_FALLBACK",
            "a:custGeom",
            FallbackKind::CustomGeometryPlaceholder,
        ),
    };
    let relationship_id = if data.element_type == UnresolvedType::CustomGeometry {
        placeholder_id.to_owned()
    } else {
        metadata
            .raw_reference
            .as_deref()
            .and_then(extract_relationship_id)
            .unwrap_or_else(|| placeholder_id.to_owned())
    };
    ConversionDiagnostic {
        code: code.to_owned(),
        family: FeatureFamily::Unsupported,
        support_tier: SupportTier::Fallback,
        stage: Some(CapabilityStage::Rendered),
        location: DiagnosticLocation {
            slide_index: Some(slide_index),
            part_name: Some(format!("ppt/slides/slide{}.xml", slide_index + 1)),
            relationship_id: Some(relationship_id),
            qualified_element_name: Some(qualified_name.to_owned()),
            position: Some(position),
            size: Some(size),
            ..Default::default()
        },
        raw_reference: metadata.raw_reference.clone(),
        fallback_kind,
        reason: format!("{} was rendered as a non-fatal placeholder", data.label),
    }
}

fn extract_relationship_id(xml: &str) -> Option<String> {
    ["r:id=\"", "r:dm=\""]
        .into_iter()
        .find_map(|marker| {
            xml.split_once(marker)
                .and_then(|(_, rest)| rest.split_once('"'))
        })
        .map(|(value, _)| value.to_owned())
}

pub(super) fn sort_and_deduplicate(diagnostics: &mut Vec<ConversionDiagnostic>) {
    diagnostics.sort_by(|left, right| {
        diagnostic_sort_key(left)
            .cmp(&diagnostic_sort_key(right))
            .then_with(|| super::action_diagnostics::compare(left, right))
    });
    diagnostics.dedup_by(|left, right| {
        super::action_diagnostics::exact_duplicate(left, right)
            || (!left.code.starts_with("ACTION_")
                && !right.code.starts_with("ACTION_")
                && diagnostic_deduplication_key(left) == diagnostic_deduplication_key(right))
    });
}

type DiagnosticKey<'a> = (
    Option<&'a str>,
    Option<usize>,
    u8,
    Option<usize>,
    Option<&'a str>,
    Option<&'a str>,
    Option<&'a str>,
);

fn effect_encounter(diagnostic: &ConversionDiagnostic) -> Option<usize> {
    if !is_ordered_effect(diagnostic) {
        return None;
    }
    diagnostic
        .location
        .relationship_id
        .as_deref()?
        .rsplit_once("-effect-")?
        .1
        .parse()
        .ok()
}

fn is_ordered_effect(diagnostic: &ConversionDiagnostic) -> bool {
    matches!(
        diagnostic.code.as_str(),
        "DRAWINGML_REFLECTION_APPROXIMATE"
            | "DRAWINGML_REFLECTION_FALLBACK"
            | "DRAWINGML_THEME_EFFECT_FALLBACK"
            | "DRAWINGML_3D_FALLBACK"
    )
}

fn diagnostic_sort_key(diagnostic: &ConversionDiagnostic) -> DiagnosticKey<'_> {
    (
        diagnostic.location.part_name.as_deref(),
        diagnostic.location.slide_index,
        u8::from(is_ordered_effect(diagnostic)),
        effect_encounter(diagnostic),
        diagnostic.location.qualified_element_name.as_deref(),
        diagnostic.location.relationship_id.as_deref(),
        if matches!(
            diagnostic.code.as_str(),
            "LEGACY_COMMENT_METADATA" | "MODERN_COMMENT_METADATA"
        ) {
            None
        } else {
            diagnostic.raw_reference.as_deref()
        },
    )
}

fn diagnostic_deduplication_key(diagnostic: &ConversionDiagnostic) -> DiagnosticKey<'_> {
    (
        diagnostic.location.part_name.as_deref(),
        diagnostic.location.slide_index,
        u8::from(is_ordered_effect(diagnostic)),
        effect_encounter(diagnostic),
        diagnostic.location.qualified_element_name.as_deref(),
        diagnostic.location.relationship_id.as_deref(),
        diagnostic.raw_reference.as_deref(),
    )
}

pub(super) fn diagnostics_json(diagnostics: &[ConversionDiagnostic]) -> String {
    let mut json = String::from("[");
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        write_diagnostic_json(&mut json, diagnostic);
    }
    json.push(']');
    json
}

fn write_diagnostic_json(json: &mut String, diagnostic: &ConversionDiagnostic) {
    json.push('{');
    write_json_string(json, "code");
    json.push(':');
    write_json_string(json, &diagnostic.code);
    write_string_field(json, "family", diagnostic.family.as_str());
    write_string_field(json, "support_tier", diagnostic.support_tier.as_str());
    write_optional_string_field(json, "stage", diagnostic.stage.map(CapabilityStage::as_str));
    json.push_str(",\"location\":{");
    write_json_string(json, "slide_index");
    json.push(':');
    if let Some(slide_index) = diagnostic.location.slide_index {
        let _ = write!(json, "{slide_index}");
    } else {
        json.push_str("null");
    }
    write_optional_string_field(json, "part_name", diagnostic.location.part_name.as_deref());
    write_optional_string_field(
        json,
        "relationship_id",
        diagnostic.location.relationship_id.as_deref(),
    );
    write_optional_string_field(
        json,
        "relationship_type",
        diagnostic.location.relationship_type.as_deref(),
    );
    write_optional_string_field(
        json,
        "qualified_element_name",
        diagnostic.location.qualified_element_name.as_deref(),
    );
    write_position(json, diagnostic.location.position);
    write_size(json, diagnostic.location.size);
    json.push('}');
    write_optional_string_field(json, "raw_reference", diagnostic.raw_reference.as_deref());
    write_string_field(json, "fallback_kind", diagnostic.fallback_kind.as_str());
    write_string_field(json, "reason", &diagnostic.reason);
    json.push('}');
}

fn write_string_field(json: &mut String, name: &str, value: &str) {
    json.push(',');
    write_json_string(json, name);
    json.push(':');
    write_json_string(json, value);
}

fn write_optional_string_field(json: &mut String, name: &str, value: Option<&str>) {
    json.push(',');
    write_json_string(json, name);
    json.push(':');
    if let Some(value) = value {
        write_json_string(json, value);
    } else {
        json.push_str("null");
    }
}

fn write_position(json: &mut String, position: Option<Position>) {
    json.push_str(",\"position\":");
    if let Some(position) = position {
        let _ = write!(json, "{{\"x\":{},\"y\":{}}}", position.x.0, position.y.0);
    } else {
        json.push_str("null");
    }
}

fn write_size(json: &mut String, size: Option<Size>) {
    json.push_str(",\"size\":");
    if let Some(size) = size {
        let _ = write!(
            json,
            "{{\"width\":{},\"height\":{}}}",
            size.width.0, size.height.0
        );
    } else {
        json.push_str("null");
    }
}

pub(super) fn write_json_string(json: &mut String, value: &str) {
    json.push('"');
    for character in value.chars() {
        match character {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            '\u{08}' => json.push_str("\\b"),
            '\u{0C}' => json.push_str("\\f"),
            '\n' => json.push_str("\\n"),
            '\r' => json.push_str("\\r"),
            '\t' => json.push_str("\\t"),
            '<' => json.push_str("\\u003C"),
            '>' => json.push_str("\\u003E"),
            '&' => json.push_str("\\u0026"),
            '\u{2028}' => json.push_str("\\u2028"),
            '\u{2029}' => json.push_str("\\u2029"),
            control if control <= '\u{1F}' => {
                let _ = write!(json, "\\u{:04X}", u32::from(control));
            }
            other => json.push(other),
        }
    }
    json.push('"');
}
