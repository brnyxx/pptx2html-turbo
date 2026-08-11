use super::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    RenderCtx, SupportTier, TableData,
};
use crate::model::{TableStyleIssue, TableStyleReference};

pub(super) fn emit(reference: &TableStyleReference, table: &TableData, ctx: &RenderCtx<'_>) {
    let slide_index = ctx.collector.borrow().current_slide_index;
    if reference.definition.is_none() {
        push(
            ctx,
            "TABLE_STYLE_DEFINITION_UNAVAILABLE",
            CapabilityStage::Resolved,
            DiagnosticLocation {
                slide_index: Some(slide_index),
                part_name: Some(format!("ppt/slides/slide{}.xml", slide_index + 1)),
                qualified_element_name: Some("a:tableStyleId".to_owned()),
                ..Default::default()
            },
            format!(
                "style_id={};source_kind={};firstRow={};lastRow={};firstCol={};lastCol={};bandRow={};bandCol={}",
                reference.id,
                reference.source_kind.as_str(),
                u8::from(table.first_row),
                u8::from(table.last_row),
                u8::from(table.first_col),
                u8::from(table.last_col),
                u8::from(table.band_row),
                u8::from(table.band_col),
            ),
            "Referenced table style definition is unavailable; no appearance was invented",
        );
    }
    if let Some(definition) = &reference.definition {
        for primitive in &definition.unsupported_primitives {
            push(
                ctx,
                "TABLE_STYLE_PRIMITIVE_UNSUPPORTED",
                CapabilityStage::Parsed,
                DiagnosticLocation {
                    slide_index: Some(slide_index),
                    part_name: Some("ppt/tableStyles.xml".to_owned()),
                    qualified_element_name: Some(format!("a:{primitive}")),
                    ..Default::default()
                },
                format!("style_id={};primitive={primitive}", reference.id),
                "Table style primitive was preserved but not rendered",
            );
        }
    }
    for issue in &reference.issues {
        let (code, location, raw, reason) = match issue {
            TableStyleIssue::DuplicateId => (
                "TABLE_STYLE_DUPLICATE_ID",
                DiagnosticLocation {
                    slide_index: Some(slide_index),
                    part_name: Some("ppt/tableStyles.xml".to_owned()),
                    qualified_element_name: Some("a:tblStyle".to_owned()),
                    ..Default::default()
                },
                format!("style_id={}", reference.id),
                "Duplicate table style ID; the first package definition was used",
            ),
            TableStyleIssue::InvalidBoolean { name, value } => (
                "TABLE_STYLE_INVALID_BOOLEAN",
                DiagnosticLocation {
                    slide_index: Some(slide_index),
                    part_name: Some(format!("ppt/slides/slide{}.xml", slide_index + 1)),
                    qualified_element_name: Some("a:tblPr".to_owned()),
                    ..Default::default()
                },
                format!("{name}={value}"),
                "Invalid table style option boolean; false fallback was used",
            ),
        };
        push(ctx, code, CapabilityStage::Parsed, location, raw, reason);
    }
}

fn push(
    ctx: &RenderCtx<'_>,
    code: &str,
    stage: CapabilityStage,
    location: DiagnosticLocation,
    raw_reference: String,
    reason: &str,
) {
    ctx.collector
        .borrow_mut()
        .diagnostics
        .push(ConversionDiagnostic {
            code: code.to_owned(),
            family: FeatureFamily::Tables,
            support_tier: SupportTier::Approximate,
            stage: Some(stage),
            location,
            raw_reference: Some(raw_reference),
            fallback_kind: FallbackKind::TableStyleDefinitionUnavailable,
            reason: reason.to_owned(),
        });
}
