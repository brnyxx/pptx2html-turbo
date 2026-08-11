use quick_xml::events::BytesStart;

use super::text_parser::{
    ParagraphBuilder, RunBuilder, append_text as append_run_text,
    start_paragraph as start_text_paragraph, start_run as start_text_run,
};
use super::xml_utils;
use crate::model::*;

pub(crate) fn start_table(in_table: &mut bool, builder: &mut Option<TableBuilder>) {
    *in_table = true;
    *builder = Some(TableBuilder::default());
}

pub(crate) fn parse_table_properties(element: &BytesStart<'_>, builder: &mut Option<TableBuilder>) {
    let Some(builder) = builder.as_mut() else {
        return;
    };
    let parse_bool = |value: &str| value == "1" || value == "true";
    for (name, target) in [
        ("bandRow", &mut builder.band_row),
        ("bandCol", &mut builder.band_col),
        ("firstRow", &mut builder.first_row),
        ("lastRow", &mut builder.last_row),
        ("firstCol", &mut builder.first_col),
        ("lastCol", &mut builder.last_col),
    ] {
        if let Some(value) = xml_utils::attr_str(element, name) {
            *target = parse_bool(&value);
        }
    }
}

pub(crate) fn parse_column(element: &BytesStart<'_>, builder: &mut Option<TableBuilder>) {
    if let Some(builder) = builder.as_mut() {
        let width = xml_utils::attr_str(element, "w")
            .map(|value| Emu::parse_emu(&value).to_px())
            .unwrap_or(0.0);
        builder.col_widths.push(width);
    }
}

pub(crate) fn start_row(
    element: &BytesStart<'_>,
    in_row: &mut bool,
    row: &mut Option<TableRowBuilder>,
) {
    *in_row = true;
    let height = xml_utils::attr_str(element, "h")
        .map(|value| Emu::parse_emu(&value).to_px())
        .unwrap_or(0.0);
    *row = Some(TableRowBuilder {
        height,
        cells: Vec::new(),
    });
}

pub(crate) fn start_cell(
    element: &BytesStart<'_>,
    in_cell: &mut bool,
    cell: &mut Option<TableCellBuilder>,
    paragraphs: &mut Vec<TextParagraph>,
) {
    *in_cell = true;
    let col_span = xml_utils::attr_str(element, "gridSpan")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    let row_span = xml_utils::attr_str(element, "rowSpan")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    let v_merge =
        xml_utils::attr_str(element, "vMerge").is_some_and(|value| value == "1" || value == "true");
    *cell = Some(TableCellBuilder {
        text_body: None,
        fill: Fill::None,
        border_left: Border::default(),
        border_right: Border::default(),
        border_top: Border::default(),
        border_bottom: Border::default(),
        col_span,
        row_span,
        v_merge,
        margin_left: 7.2,
        margin_right: 7.2,
        margin_top: 3.6,
        margin_bottom: 3.6,
        vertical_align: VerticalAlign::Top,
    });
    paragraphs.clear();
}

pub(crate) fn start_paragraph(paragraph: &mut Option<ParagraphBuilder>) {
    start_text_paragraph(paragraph);
}

pub(crate) fn start_run(run: &mut Option<RunBuilder>) {
    start_text_run(run);
}

pub(crate) fn append_text(run: &mut Option<RunBuilder>, text: &str) {
    append_run_text(run, text);
}

pub(crate) fn parse_cell_properties(
    element: &BytesStart<'_>,
    in_properties: &mut bool,
    cell: &mut Option<TableCellBuilder>,
) {
    *in_properties = true;
    let Some(cell) = cell.as_mut() else {
        return;
    };
    for (name, target) in [
        ("marL", &mut cell.margin_left),
        ("marR", &mut cell.margin_right),
        ("marT", &mut cell.margin_top),
        ("marB", &mut cell.margin_bottom),
    ] {
        if let Some(value) = xml_utils::attr_str(element, name) {
            *target = Emu::parse_emu(&value).to_pt();
        }
    }
    if let Some(anchor) = xml_utils::attr_str(element, "anchor") {
        cell.vertical_align = VerticalAlign::from_ooxml(&anchor);
    }
}

pub(crate) fn start_border(
    side: &str,
    element: &BytesStart<'_>,
    border_side: &mut Option<String>,
    cell: &mut Option<TableCellBuilder>,
) {
    *border_side = Some(side.to_owned());
    let Some(width) = xml_utils::attr_str(element, "w") else {
        return;
    };
    let width = Emu::parse_emu(&width).to_pt();
    if let Some(border) = cell.as_mut().and_then(|cell| border_mut(cell, side)) {
        border.width = width;
    }
}

pub(crate) fn parse_border_dash(
    element: &BytesStart<'_>,
    side: Option<&str>,
    cell: &mut Option<TableCellBuilder>,
) {
    let Some(value) = xml_utils::attr_str(element, "val") else {
        return;
    };
    let Some(side) = side else {
        return;
    };
    let Some(border) = cell.as_mut().and_then(|cell| border_mut(cell, side)) else {
        return;
    };
    border.style = match value.as_str() {
        "solid" => BorderStyle::Solid,
        "dash" | "lgDash" | "sysDash" => BorderStyle::Dashed,
        "dot" | "sysDot" | "lgDashDot" | "lgDashDotDot" => BorderStyle::Dotted,
        _ => BorderStyle::Solid,
    };
    border.dash_style = match value.as_str() {
        "dash" => DashStyle::Dash,
        "dot" => DashStyle::Dot,
        "dashDot" => DashStyle::DashDot,
        "lgDash" => DashStyle::LongDash,
        "lgDashDot" => DashStyle::LongDashDot,
        "lgDashDotDot" => DashStyle::LongDashDotDot,
        "sysDash" => DashStyle::SystemDash,
        "sysDot" => DashStyle::SystemDot,
        _ => DashStyle::Solid,
    };
}

fn border_mut<'a>(cell: &'a mut TableCellBuilder, side: &str) -> Option<&'a mut Border> {
    match side {
        "lnL" => Some(&mut cell.border_left),
        "lnR" => Some(&mut cell.border_right),
        "lnT" => Some(&mut cell.border_top),
        "lnB" => Some(&mut cell.border_bottom),
        _ => None,
    }
}

pub(crate) fn finish_run(run: &mut Option<RunBuilder>, paragraph: &mut Option<ParagraphBuilder>) {
    if let Some(run) = run.take()
        && let Some(paragraph) = paragraph.as_mut()
    {
        paragraph.runs.push(run.build());
    }
}

pub(crate) fn finish_paragraph(
    paragraph: &mut Option<ParagraphBuilder>,
    paragraphs: &mut Vec<TextParagraph>,
) {
    if let Some(paragraph) = paragraph.take() {
        paragraphs.push(paragraph.build());
    }
}

pub(crate) fn finish_cell(
    cell: &mut Option<TableCellBuilder>,
    paragraphs: &mut Vec<TextParagraph>,
    row: &mut Option<TableRowBuilder>,
) {
    if let Some(mut cell) = cell.take() {
        if !paragraphs.is_empty() {
            cell.text_body = Some(TextBody {
                paragraphs: std::mem::take(paragraphs),
                list_style: None,
                ..Default::default()
            });
        }
        if let Some(row) = row.as_mut() {
            row.cells.push(cell.build());
        }
    }
}

pub(crate) fn finish_row(row: &mut Option<TableRowBuilder>, builder: &mut Option<TableBuilder>) {
    if let Some(row) = row.take()
        && let Some(builder) = builder.as_mut()
    {
        builder.rows.push(row.build());
    }
}

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
