use std::fmt::Write;

use super::{HtmlRenderer, RenderCtx, TableData, VerticalAlign, dash_style_to_css, push_sep};

impl HtmlRenderer {
    pub(super) fn render_table(table: &TableData, ctx: &RenderCtx<'_>, html: &mut String) {
        let total_width: f64 = table.col_widths.iter().sum();
        html.push_str(
            "<table style=\"width:100%; height:100%; border-collapse:collapse; table-layout:fixed;\">\n<colgroup>\n",
        );
        for w in &table.col_widths {
            let pct = if total_width > 0.0 {
                w / total_width * 100.0
            } else {
                0.0
            };
            let _ = writeln!(html, "<col style=\"width:{pct:.1}%\"/>");
        }
        html.push_str("</colgroup>\n");

        let row_count = table.rows.len();
        for (row_idx, row) in table.rows.iter().enumerate() {
            let mut tr_style = format!("height:{:.1}px", row.height);

            // Band row: alternate row shading (odd data rows get subtle background)
            if table.band_row {
                // When first_row is set, banding starts from the second row (index 1)
                let band_idx = if table.first_row {
                    row_idx.wrapping_sub(1)
                } else {
                    row_idx
                };
                if (row_idx != 0 || !table.first_row) && band_idx % 2 == 1 {
                    tr_style.push_str("; background-color: rgba(0,0,0,0.04)");
                }
            }

            // First row emphasis
            if table.first_row && row_idx == 0 {
                tr_style.push_str("; font-weight: bold; border-bottom: 2px solid rgba(0,0,0,0.2)");
            }

            // Last row emphasis
            if table.last_row && row_idx == row_count - 1 {
                tr_style.push_str("; font-weight: bold; border-top: 2px solid rgba(0,0,0,0.2)");
            }

            let _ = writeln!(html, "<tr style=\"{tr_style}\">");
            let col_count = row.cells.len();
            for (col_idx, cell) in row.cells.iter().enumerate() {
                // Skip cells that are continuation of a vertical merge
                if cell.v_merge {
                    continue;
                }

                let mut td_style = String::with_capacity(128);

                // Band column: alternate column shading
                if table.band_col {
                    let band_col_idx = if table.first_col {
                        col_idx.wrapping_sub(1)
                    } else {
                        col_idx
                    };
                    if (col_idx != 0 || !table.first_col) && band_col_idx % 2 == 1 {
                        td_style.push_str("background-color: rgba(0,0,0,0.04)");
                    }
                }

                // First column emphasis
                if table.first_col && col_idx == 0 {
                    if !td_style.is_empty() {
                        td_style.push_str("; ");
                    }
                    td_style.push_str("font-weight: bold");
                }

                // Last column emphasis
                if table.last_col && col_idx == col_count - 1 {
                    if !td_style.is_empty() {
                        td_style.push_str("; ");
                    }
                    td_style.push_str("font-weight: bold");
                }

                // Cell fill
                Self::fill_to_css_buf(&cell.fill, ctx, &mut td_style);

                // Cell borders
                if cell.border_left.width > 0.0 {
                    let color = ctx
                        .color_to_css(&cell.border_left.color)
                        .unwrap_or_else(|| "#000".to_string());
                    push_sep(&mut td_style);
                    let _ = write!(
                        td_style,
                        "border-left: {:.1}pt {} {}",
                        cell.border_left.width,
                        dash_style_to_css(&cell.border_left.dash_style),
                        color
                    );
                }
                if cell.border_right.width > 0.0 {
                    let color = ctx
                        .color_to_css(&cell.border_right.color)
                        .unwrap_or_else(|| "#000".to_string());
                    push_sep(&mut td_style);
                    let _ = write!(
                        td_style,
                        "border-right: {:.1}pt {} {}",
                        cell.border_right.width,
                        dash_style_to_css(&cell.border_right.dash_style),
                        color
                    );
                }
                if cell.border_top.width > 0.0 {
                    let color = ctx
                        .color_to_css(&cell.border_top.color)
                        .unwrap_or_else(|| "#000".to_string());
                    push_sep(&mut td_style);
                    let _ = write!(
                        td_style,
                        "border-top: {:.1}pt {} {}",
                        cell.border_top.width,
                        dash_style_to_css(&cell.border_top.dash_style),
                        color
                    );
                }
                if cell.border_bottom.width > 0.0 {
                    let color = ctx
                        .color_to_css(&cell.border_bottom.color)
                        .unwrap_or_else(|| "#000".to_string());
                    push_sep(&mut td_style);
                    let _ = write!(
                        td_style,
                        "border-bottom: {:.1}pt {} {}",
                        cell.border_bottom.width,
                        dash_style_to_css(&cell.border_bottom.dash_style),
                        color
                    );
                }

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
                if cell.col_span > 1 {
                    let _ = write!(html, " colspan=\"{}\"", cell.col_span);
                }
                if cell.row_span > 1 {
                    let _ = write!(html, " rowspan=\"{}\"", cell.row_span);
                }
                let _ = writeln!(html, " style=\"{td_style}\">");
                if let Some(ref tb) = cell.text_body {
                    let mut auto_num_counters: [i32; 9] = [0; 9];
                    for para in &tb.paragraphs {
                        Self::render_paragraph(para, ctx, &mut auto_num_counters, html);
                    }
                }
                html.push_str("</td>\n");
            }
            html.push_str("</tr>\n");
        }
        html.push_str("</table>\n");
    }
}
