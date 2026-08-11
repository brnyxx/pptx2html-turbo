use crate::model::{
    Border, Fill, TableCell, TableCellStyle, TableData, TableStyleRegion, TableTextStyle,
};

#[derive(Default)]
pub(super) struct ResolvedTableCellStyle {
    pub(super) fill: Option<Fill>,
    pub(super) left: Option<Border>,
    pub(super) right: Option<Border>,
    pub(super) top: Option<Border>,
    pub(super) bottom: Option<Border>,
    pub(super) text: TableTextStyle,
    pub(super) last_region: Option<TableStyleRegion>,
}

pub(super) fn resolve(
    table: &TableData,
    cell: &TableCell,
    row: usize,
    column: usize,
    column_span: usize,
) -> ResolvedTableCellStyle {
    let Some(definition) = table
        .style
        .as_ref()
        .and_then(|style| style.definition.as_ref())
    else {
        return explicit_style(cell, ResolvedTableCellStyle::default());
    };
    let rows = table.rows.len();
    let columns = table.col_widths.len();
    let last_column = column.saturating_add(column_span).saturating_sub(1);
    let mut result = ResolvedTableCellStyle::default();
    for region in applicable_regions(table, row, column, last_column, rows, columns) {
        if let Some(style) = definition.region(region) {
            apply_region(&mut result, style, row, column, last_column, rows, columns);
            result.last_region = Some(region);
        }
    }
    explicit_style(cell, result)
}

fn applicable_regions(
    table: &TableData,
    row: usize,
    column: usize,
    last_column: usize,
    rows: usize,
    columns: usize,
) -> Vec<TableStyleRegion> {
    let mut regions = vec![TableStyleRegion::WholeTable];
    if table.band_row {
        regions.push(if row.is_multiple_of(2) {
            TableStyleRegion::Band1Horizontal
        } else {
            TableStyleRegion::Band2Horizontal
        });
    }
    if table.band_col {
        regions.push(if column.is_multiple_of(2) {
            TableStyleRegion::Band1Vertical
        } else {
            TableStyleRegion::Band2Vertical
        });
    }
    let is_first_row = row == 0;
    let is_last_row = row.saturating_add(1) == rows;
    let is_first_column = column == 0;
    let is_last_column = last_column.saturating_add(1) == columns;
    if table.last_col && is_last_column {
        regions.push(TableStyleRegion::LastColumn);
    }
    if table.first_col && is_first_column {
        regions.push(TableStyleRegion::FirstColumn);
    }
    if table.last_row && is_last_row {
        regions.push(TableStyleRegion::LastRow);
    }
    if table.last_row && table.last_col && is_last_row && is_last_column {
        regions.push(TableStyleRegion::SoutheastCell);
    }
    if table.last_row && table.first_col && is_last_row && is_first_column {
        regions.push(TableStyleRegion::SouthwestCell);
    }
    if table.first_row && is_first_row {
        regions.push(TableStyleRegion::FirstRow);
    }
    if table.first_row && table.last_col && is_first_row && is_last_column {
        regions.push(TableStyleRegion::NortheastCell);
    }
    if table.first_row && table.first_col && is_first_row && is_first_column {
        regions.push(TableStyleRegion::NorthwestCell);
    }
    regions
}

fn apply_region(
    target: &mut ResolvedTableCellStyle,
    source: &TableCellStyle,
    row: usize,
    column: usize,
    last_column: usize,
    rows: usize,
    columns: usize,
) {
    if let Some(fill) = &source.fill {
        target.fill = Some(fill.clone());
    }
    if let Some(border) = &source.left {
        target.left = Some(border.clone());
    }
    if let Some(border) = &source.right {
        target.right = Some(border.clone());
    }
    if let Some(border) = &source.top {
        target.top = Some(border.clone());
    }
    if let Some(border) = &source.bottom {
        target.bottom = Some(border.clone());
    }
    if column > 0
        && let Some(border) = &source.inside_vertical
    {
        target.left = Some(border.clone());
    }
    if last_column.saturating_add(1) < columns
        && let Some(border) = &source.inside_vertical
    {
        target.right = Some(border.clone());
    }
    if row > 0
        && let Some(border) = &source.inside_horizontal
    {
        target.top = Some(border.clone());
    }
    if row.saturating_add(1) < rows
        && let Some(border) = &source.inside_horizontal
    {
        target.bottom = Some(border.clone());
    }
    if source.text.font_family.is_some() {
        target.text.font_family.clone_from(&source.text.font_family);
    }
    if source.text.color.is_some() {
        target.text.color.clone_from(&source.text.color);
    }
    if source.text.bold.is_some() {
        target.text.bold = source.text.bold;
    }
    if source.text.italic.is_some() {
        target.text.italic = source.text.italic;
    }
}

fn explicit_style(cell: &TableCell, mut style: ResolvedTableCellStyle) -> ResolvedTableCellStyle {
    if !matches!(cell.fill, Fill::None) {
        style.fill = Some(cell.fill.clone());
    }
    if cell.explicit_borders & 1 != 0 {
        style.left = Some(cell.border_left.clone());
    }
    if cell.explicit_borders & 2 != 0 {
        style.right = Some(cell.border_right.clone());
    }
    if cell.explicit_borders & 4 != 0 {
        style.top = Some(cell.border_top.clone());
    }
    if cell.explicit_borders & 8 != 0 {
        style.bottom = Some(cell.border_bottom.clone());
    }
    style
}
