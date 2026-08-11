use crate::model::*;

#[derive(Default)]
pub(crate) struct TableBuilder {
    pub(crate) col_widths: Vec<f64>,
    pub(crate) rows: Vec<TableRow>,
    pub(crate) band_row: bool,
    pub(crate) band_col: bool,
    pub(crate) first_row: bool,
    pub(crate) last_row: bool,
    pub(crate) first_col: bool,
    pub(crate) last_col: bool,
}

impl TableBuilder {
    pub(crate) fn build(self) -> TableData {
        TableData {
            rows: self.rows,
            col_widths: self.col_widths,
            band_row: self.band_row,
            band_col: self.band_col,
            first_row: self.first_row,
            last_row: self.last_row,
            first_col: self.first_col,
            last_col: self.last_col,
        }
    }
}

#[derive(Default)]
pub(crate) struct TableRowBuilder {
    pub(crate) height: f64,
    pub(crate) cells: Vec<TableCell>,
}

impl TableRowBuilder {
    pub(crate) fn build(self) -> TableRow {
        TableRow {
            height: self.height,
            cells: self.cells,
        }
    }
}

pub(crate) struct TableCellBuilder {
    pub(crate) text_body: Option<TextBody>,
    pub(crate) fill: Fill,
    pub(crate) border_left: Border,
    pub(crate) border_right: Border,
    pub(crate) border_top: Border,
    pub(crate) border_bottom: Border,
    pub(crate) col_span: u32,
    pub(crate) row_span: u32,
    pub(crate) v_merge: bool,
    pub(crate) margin_left: f64,
    pub(crate) margin_right: f64,
    pub(crate) margin_top: f64,
    pub(crate) margin_bottom: f64,
    pub(crate) vertical_align: VerticalAlign,
}

impl Default for TableCellBuilder {
    fn default() -> Self {
        let cell = TableCell::default();
        Self {
            text_body: cell.text_body,
            fill: cell.fill,
            border_left: cell.border_left,
            border_right: cell.border_right,
            border_top: cell.border_top,
            border_bottom: cell.border_bottom,
            col_span: cell.col_span,
            row_span: cell.row_span,
            v_merge: cell.v_merge,
            margin_left: cell.margin_left,
            margin_right: cell.margin_right,
            margin_top: cell.margin_top,
            margin_bottom: cell.margin_bottom,
            vertical_align: cell.vertical_align,
        }
    }
}

impl TableCellBuilder {
    pub(crate) fn build(self) -> TableCell {
        TableCell {
            text_body: self.text_body,
            fill: self.fill,
            border_left: self.border_left,
            border_right: self.border_right,
            border_top: self.border_top,
            border_bottom: self.border_bottom,
            col_span: self.col_span,
            row_span: self.row_span,
            v_merge: self.v_merge,
            margin_left: self.margin_left,
            margin_right: self.margin_right,
            margin_top: self.margin_top,
            margin_bottom: self.margin_bottom,
            vertical_align: self.vertical_align,
        }
    }
}

pub(crate) fn assign_cell_color(
    color: Color,
    border_side: &Option<String>,
    cell: &mut Option<TableCellBuilder>,
) {
    let Some(cell) = cell.as_mut() else {
        return;
    };
    match border_side.as_deref() {
        Some("lnL") => assign_border_color(&mut cell.border_left, color),
        Some("lnR") => assign_border_color(&mut cell.border_right, color),
        Some("lnT") => assign_border_color(&mut cell.border_top, color),
        Some("lnB") => assign_border_color(&mut cell.border_bottom, color),
        None => cell.fill = Fill::Solid(SolidFill { color }),
        _ => {}
    }
}

fn assign_border_color(border: &mut Border, color: Color) {
    border.color = color;
    if matches!(border.style, BorderStyle::None) && border.width > 0.0 {
        border.style = BorderStyle::Solid;
    }
}
