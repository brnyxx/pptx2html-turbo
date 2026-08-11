use super::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    RenderCtx, SupportTier, TableData,
};
use crate::model::{TableStyleIssue, TableStyleReference};

pub(super) fn emit(
    reference: &TableStyleReference,
    table: &TableData,
    table_id: u32,
    ctx: &RenderCtx<'_>,
) {
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
                "table_id={table_id};style_id={};source_kind={};firstRow={};lastRow={};firstCol={};lastCol={};bandRow={};bandCol={}",
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
                format!(
                    "table_id={table_id};style_id={};primitive={primitive}",
                    reference.id
                ),
                "Table style primitive was preserved but not rendered",
            );
        }
        for primitive in &definition.unsupported_references {
            push(
                ctx,
                "TABLE_STYLE_PRIMITIVE_UNSUPPORTED",
                CapabilityStage::Parsed,
                DiagnosticLocation {
                    slide_index: Some(slide_index),
                    part_name: Some("ppt/tableStyles.xml".to_owned()),
                    qualified_element_name: Some(format!("a:{}", primitive.name)),
                    ..Default::default()
                },
                format!(
                    "table_id={table_id};style_id={};primitive={};idx={};color={:?};modifiers={:?}",
                    reference.id,
                    primitive.name,
                    primitive.idx.as_deref().unwrap_or("absent"),
                    primitive.color.as_ref().map(|color| &color.kind),
                    primitive
                        .color
                        .as_ref()
                        .map(|color| color.modifiers.as_slice())
                ),
                "Table style reference primitive was preserved but not rendered",
            );
        }
        for fill_ref in definition.table_background_ref.iter().chain(
            definition
                .regions
                .iter()
                .filter_map(|(_, style)| style.fill_ref.as_ref()),
        ) {
            let resolved = ctx.pres.primary_theme().and_then(|theme| {
                crate::resolver::style_ref::resolve_fill_ref(
                    fill_ref,
                    &theme.fmt_scheme,
                    &theme.color_scheme,
                    ctx.clr_map.unwrap_or(&ctx.pres.clr_map),
                )
            });
            if resolved.is_none() {
                push(
                    ctx,
                    "TABLE_STYLE_PRIMITIVE_UNSUPPORTED",
                    CapabilityStage::Resolved,
                    DiagnosticLocation {
                        slide_index: Some(slide_index),
                        part_name: Some("ppt/tableStyles.xml".to_owned()),
                        qualified_element_name: Some("a:fillRef".to_owned()),
                        ..Default::default()
                    },
                    format!(
                        "table_id={table_id};style_id={};fill_ref_idx={};color={:?};modifiers={:?}",
                        reference.id, fill_ref.idx, fill_ref.color.kind, fill_ref.color.modifiers,
                    ),
                    "Table style fillRef could not be resolved from the parsed theme format scheme; no fill was invented",
                );
            }
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
                format!("table_id={table_id};style_id={}", reference.id),
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
                format!("table_id={table_id};{name}={value}"),
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
