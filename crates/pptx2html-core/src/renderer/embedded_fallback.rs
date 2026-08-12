use std::cell::RefCell;
use std::fmt::Write;

use super::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    Position, RenderCtx, Size, SupportTier, UnresolvedCollector, UnresolvedElement, UnresolvedType,
    UnsupportedData, escape_html,
};
use crate::model::Presentation;
use crate::model::embedded::inventory_key;

pub(super) fn render_unsupported(
    shape_id: u32,
    data: &UnsupportedData,
    position: Position,
    size: Size,
    ctx: &RenderCtx<'_>,
    html: &mut String,
) -> bool {
    if data.element_type == UnresolvedType::CustomGeometry {
        return false;
    }
    let raw_xml = data.raw_xml.as_deref().unwrap_or_default();
    let slide_index = ctx.collector.borrow().current_slide_index;
    let domain = match data.element_type {
        UnresolvedType::SmartArt => "smartart",
        UnresolvedType::OleObject => "ole",
        UnresolvedType::MathEquation => "math",
        UnresolvedType::CustomGeometry => unreachable!(),
    };
    let key = inventory_key(
        &format!("ppt/slides/slide{}.xml", slide_index + 1),
        shape_id,
        domain,
        raw_xml,
    );
    let occurrence = {
        let mut collector = ctx.collector.borrow_mut();
        let occurrence = collector
            .embedded_occurrences
            .entry(key.clone())
            .or_default();
        let current = *occurrence;
        *occurrence += 1;
        current
    };
    let Some(inventory) = ctx.pres.embedded_inventory.get(&key, occurrence) else {
        return false;
    };
    let mut collector = ctx.collector.borrow_mut();
    let placeholder_id = format!(
        "unresolved-s{}-e{}",
        collector.current_slide_index, collector.counter
    );
    collector.counter += 1;
    let type_name = match data.element_type {
        UnresolvedType::SmartArt => "smartart",
        UnresolvedType::OleObject => "ole",
        UnresolvedType::MathEquation => "math",
        UnresolvedType::CustomGeometry => unreachable!(),
    };
    let _ = write!(
        html,
        "<div class=\"unresolved-element embedded-fallback\" id=\"{placeholder_id}\" data-type=\"{type_name}\" data-slide=\"{}\"",
        collector.current_slide_index
    );
    if !inventory.source_identity.is_empty() {
        let _ = write!(
            html,
            " data-source-id=\"{}\"",
            escape_html(&inventory.source_identity)
        );
    }
    html.push('>');
    if let Some(preview) = &inventory.preview {
        let label = escape_html(&data.label);
        let _ = write!(
            html,
            "<img class=\"embedded-preview\" src=\"data:{};base64,{}\" alt=\"{label} static preview\" style=\"width:100%;height:100%;object-fit:contain\"><span class=\"embedded-label\" style=\"position:absolute;left:6px;bottom:4px;padding:2px 5px;background:rgba(255,255,255,.85);font:12px sans-serif\">{label} static preview</span>",
            preview.mime_type, preview.base64,
        );
    } else {
        let _ = write!(html, "<span>[{}]</span>", escape_html(&data.label));
    }
    if !inventory.relationships.is_empty() {
        html.push_str("<span class=\"embedded-closure\" aria-hidden=\"true\" hidden>");
        for relationship in &inventory.relationships {
            let _ = write!(
                html,
                "{} {} {} ",
                escape_html(&relationship.id),
                escape_html(&relationship.relationship_type),
                escape_html(&relationship.part_name)
            );
        }
        html.push_str("</span>");
    }
    html.push_str("</div></div>\n");

    let original = raw_xml;
    let raw_reference = inventory.to_json_with_raw_xml(original);
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
        UnresolvedType::CustomGeometry => unreachable!(),
    };
    let slide_index = collector.current_slide_index;
    collector.diagnostics.push(ConversionDiagnostic {
        code: code.to_owned(), family: FeatureFamily::Unsupported, support_tier: SupportTier::Fallback,
        stage: Some(CapabilityStage::Rendered), location: DiagnosticLocation {
            slide_index: Some(slide_index), part_name: Some(format!("ppt/slides/slide{}.xml", slide_index + 1)),
            relationship_id: Some(if inventory.source_identity.is_empty() { placeholder_id.clone() } else { inventory.source_identity.clone() }),
            qualified_element_name: Some(qualified_name.to_owned()), position: Some(position), size: Some(size), ..Default::default()
        }, raw_reference: Some(raw_reference), fallback_kind,
        reason: format!("{} was preserved and rendered as a safe static preview or bounded placeholder; native Office behavior is not claimed", data.label),
    });
    collector.elements.push(UnresolvedElement {
        slide_index,
        element_type: data.element_type.clone(),
        placeholder_id,
        position: ((position.x.0 != 0) || (position.y.0 != 0)).then_some(position),
        size: ((size.width.0 != 0) || (size.height.0 != 0)).then_some(size),
        raw_xml: Some(original.to_owned()),
        data_model: None,
    });
    true
}

pub(super) fn append_diagnostics(
    _presentation: &Presentation,
    _collector: &RefCell<UnresolvedCollector>,
) {
}
