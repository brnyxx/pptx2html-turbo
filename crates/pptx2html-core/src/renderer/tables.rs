use std::fmt::Write;

use super::bullets::{ParagraphRenderContext, TextStyleCtx};
use super::{
    HtmlRenderer, RenderCtx, TableData, VerticalAlign, dash_style_to_css, escape_html, push_sep,
};
use super::{table_style_diagnostics, table_styles};

impl HtmlRenderer {
    pub(super) fn render_table(table: &TableData, ctx: &RenderCtx<'_>, html: &mut String) {
        let total_width: f64 = table.col_widths.iter().sum();
        html.push_str("<table");
        if let Some(reference) = &table.style {
            let _ = write!(
                html,
                " data-table-style-id=\"{}\"",
                escape_html(&reference.id)
            );
            let _ = write!(
                html,
                " data-table-style-source=\"{}\"",
                reference.source_kind.as_str()
            );
            table_style_diagnostics::emit(reference, table, ctx);
        }
        let mut table_css =
            String::from("width:100%; height:100%; border-collapse:collapse; table-layout:fixed");
        if let Some(fill) = table
            .style
            .as_ref()
            .and_then(|reference| reference.definition.as_ref())
            .and_then(|definition| definition.table_background.as_ref())
        {
            if matches!(fill, crate::model::Fill::NoFill) {
                push_sep(&mut table_css);
                table_css.push_str("background-color: transparent");
            } else {
                Self::fill_to_css_buf(fill, ctx, &mut table_css);
            }
        }
        let _ = writeln!(html, " style=\"{table_css}\">\n<colgroup>");
        for w in &table.col_widths {
            let pct = if total_width > 0.0 {
                w / total_width * 100.0
            } else {
                0.0
            };
            let _ = writeln!(html, "<col style=\"width:{pct:.1}%\"/>");
        }
        html.push_str("</colgroup>\n");

        for (row_idx, row) in table.rows.iter().enumerate() {
            let tr_style = format!("height:{:.1}px", row.height);

            let _ = writeln!(html, "<tr style=\"{tr_style}\">");
            let mut logical_col = 0usize;
            for cell in &row.cells {
                let span = usize::try_from(cell.col_span.max(1)).unwrap_or(1);
                if cell.h_merge {
                    continue;
                }
                if cell.v_merge {
                    logical_col = logical_col.saturating_add(span);
                    continue;
                }

                let mut td_style = String::with_capacity(128);
                let resolved = table_styles::resolve(table, cell, row_idx, logical_col, span);
                if let Some(fill) = &resolved.fill {
                    if matches!(fill, crate::model::Fill::NoFill) {
                        td_style.push_str("background-color: transparent");
                    } else {
                        Self::fill_to_css_buf(fill, ctx, &mut td_style);
                    }
                }
                if let Some(true) = resolved.text.bold {
                    push_sep(&mut td_style);
                    td_style.push_str("font-weight: bold");
                }
                if let Some(true) = resolved.text.italic {
                    push_sep(&mut td_style);
                    td_style.push_str("font-style: italic");
                }
                if let Some(color) = resolved
                    .text
                    .color
                    .as_ref()
                    .and_then(|color| ctx.color_to_css(color))
                {
                    push_sep(&mut td_style);
                    let _ = write!(td_style, "color: {color}");
                }
                if let Some(font) = &resolved.text.font_family {
                    let resolved_font = ctx
                        .pres
                        .primary_theme()
                        .and_then(|theme| theme.font_scheme.resolve_typeface(font))
                        .unwrap_or(font);
                    push_sep(&mut td_style);
                    let _ = write!(td_style, "font-family: '{}'", escape_html(resolved_font));
                }

                // Cell borders
                push_border(&mut td_style, "left", resolved.left.as_ref(), ctx);
                push_border(&mut td_style, "right", resolved.right.as_ref(), ctx);
                push_border(&mut td_style, "top", resolved.top.as_ref(), ctx);
                push_border(&mut td_style, "bottom", resolved.bottom.as_ref(), ctx);

                // Cell margins and vertical alignment
                let va = match cell.vertical_align {
                    VerticalAlign::Top => "top",
                    VerticalAlign::Middle => "middle",
                    VerticalAlign::Bottom => "bottom",
                };
                push_sep(&mut td_style);
                let _ = write!(
                    td_style,
                    "padding: {:.1}pt {:.1}pt {:.1}pt {:.1}pt; vertical-align: {}",
                    cell.margin_top, cell.margin_right, cell.margin_bottom, cell.margin_left, va
                );

                let _ = write!(html, "<td");
                let _ = write!(html, " data-table-cell=\"r{row_idx}c{logical_col}\"");
                if let Some(region) = resolved.last_region {
                    let _ = write!(html, " data-table-style-region=\"{}\"", region.as_ooxml());
                }
                if cell.col_span > 1 {
                    let _ = write!(html, " colspan=\"{}\"", cell.col_span);
                }
                if cell.row_span > 1 {
                    let _ = write!(html, " rowspan=\"{}\"", cell.row_span);
                }
                let _ = writeln!(html, " style=\"{td_style}\">");
                if let Some(ref tb) = cell.text_body {
                    let mut auto_num_counters: [i32; 9] = [0; 9];
                    let text_style = TextStyleCtx::from_local_list_style(tb.list_style.as_ref());
                    for para in &tb.paragraphs {
                        Self::render_paragraph_with_defaults(
                            para,
                            ctx,
                            &mut auto_num_counters,
                            ParagraphRenderContext {
                                text_style: &text_style,
                                font_ref_font: None,
                                font_ref_color: None,
                                font_scale: None,
                                line_spacing_reduction: None,
                            },
                            html,
                        );
                    }
                }
                html.push_str("</td>\n");
                logical_col = logical_col.saturating_add(span);
            }
            html.push_str("</tr>\n");
        }
        html.push_str("</table>\n");
    }
}

fn push_border(
    style: &mut String,
    side: &str,
    border: Option<&crate::model::Border>,
    ctx: &RenderCtx<'_>,
) {
    let Some(border) = border.filter(|border| !border.no_fill && border.width > 0.0) else {
        return;
    };
    let color = ctx
        .color_to_css(&border.color)
        .unwrap_or_else(|| "#000".to_owned());
    push_sep(style);
    let _ = write!(
        style,
        "border-{side}: {:.1}pt {} {color}",
        border.width,
        dash_style_to_css(&border.dash_style)
    );
}
