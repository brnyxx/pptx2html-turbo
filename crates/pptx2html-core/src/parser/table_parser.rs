use quick_xml::events::BytesStart;

use super::master_parser::{is_lvl_ppr, parse_lvl_index};
use super::text_parser::{
    ParagraphBuilder, RunBuilder, append_text as append_run_text,
    apply_paragraph_default_run_properties, assign_spacing_paragraph, assign_typeface_to_paragraph,
    assign_typeface_to_run, parse_bullet, parse_bullet_font, parse_bullet_size,
    parse_paragraph_properties, parse_picture_bullet, parse_picture_bullet_size,
    parse_run_properties, parse_spacing, picture_bullet, start_paragraph as start_text_paragraph,
    start_run as start_text_run,
};
use super::xml_utils;
use crate::model::*;

#[derive(Default)]
pub(crate) struct TableSaxState {
    pub(crate) in_table: bool,
    pub(crate) in_row: bool,
    pub(crate) in_cell: bool,
    pub(crate) in_properties: bool,
    pub(crate) border_side: Option<String>,
    pub(crate) builder: Option<TableBuilder>,
    pub(crate) row: Option<TableRowBuilder>,
    pub(crate) cell: Option<TableCellBuilder>,
    pub(crate) paragraphs: Vec<TextParagraph>,
    pub(crate) paragraph: Option<ParagraphBuilder>,
    pub(crate) run: Option<RunBuilder>,
    pub(crate) in_text: bool,
    pub(crate) in_run_properties: bool,
    pub(crate) in_bullet_color: bool,
    pub(crate) in_picture_bullet: bool,
    pub(crate) in_list_style: bool,
    pub(crate) list_level: Option<usize>,
    pub(crate) list_style: Option<ListStyle>,
    pub(crate) list_picture_size: Option<BulletSize>,
    pub(crate) in_default_run_properties: bool,
    pub(crate) in_line_spacing: bool,
    pub(crate) in_space_before: bool,
    pub(crate) in_space_after: bool,
    pub(crate) in_style_id: bool,
}

impl TableSaxState {
    pub(crate) fn handle_start(
        &mut self,
        local: &str,
        element: &BytesStart<'_>,
        in_graphic_frame: bool,
    ) -> bool {
        match local {
            "tbl" if in_graphic_frame => start_table(&mut self.in_table, &mut self.builder),
            "tblPr" if self.in_table => parse_table_properties(element, &mut self.builder),
            "tableStyleId" if self.in_table => {
                self.in_style_id = true;
            }
            "gridCol" if self.in_table => parse_column(element, &mut self.builder),
            "noFill" if self.in_properties => {
                if let Some(cell) = self.cell.as_mut() {
                    if let Some(side) = self.border_side.as_deref() {
                        if let Some(border) = border_mut(cell, side) {
                            border.no_fill = true;
                        }
                    } else {
                        cell.fill = Fill::NoFill;
                    }
                }
            }
            "tr" if self.in_table => start_row(element, &mut self.in_row, &mut self.row),
            "tc" if self.in_row => {
                self.list_style = None;
                self.list_level = None;
                self.list_picture_size = None;
                start_cell(
                    element,
                    &mut self.in_cell,
                    &mut self.cell,
                    &mut self.paragraphs,
                )
            }
            "tcPr" if self.in_cell => {
                parse_cell_properties(element, &mut self.in_properties, &mut self.cell)
            }
            "lnL" | "lnR" | "lnT" | "lnB" if self.in_properties => {
                start_border(local, element, &mut self.border_side, &mut self.cell)
            }
            "prstDash" if self.in_properties && self.border_side.is_some() => {
                parse_border_dash(element, self.border_side.as_deref(), &mut self.cell)
            }
            "p" if self.in_cell => start_paragraph(&mut self.paragraph),
            "lstStyle" if self.in_cell => self.in_list_style = true,
            tag if self.in_list_style && is_lvl_ppr(tag) => {
                self.list_level = Some(parse_lvl_index(tag));
            }
            "pPr" if self.in_cell && self.paragraph.is_some() => {
                parse_paragraph_properties(element, &mut self.paragraph)
            }
            "defRPr" if self.in_cell && self.paragraph.is_some() && self.run.is_none() => {
                self.in_default_run_properties = true;
                apply_paragraph_default_run_properties(
                    self.paragraph
                        .as_mut()
                        .expect("table paragraph builder for start defRPr"),
                    element,
                );
            }
            "lnSpc" if self.in_cell && self.paragraph.is_some() => self.in_line_spacing = true,
            "spcBef" if self.in_cell && self.paragraph.is_some() => self.in_space_before = true,
            "spcAft" if self.in_cell && self.paragraph.is_some() => self.in_space_after = true,
            "buClr" if self.in_cell && self.paragraph.is_some() => self.in_bullet_color = true,
            "buBlip" if self.in_cell && (self.paragraph.is_some() || self.list_level.is_some()) => {
                self.in_picture_bullet = true
            }
            "blip" if self.in_picture_bullet => self.assign_picture_bullet(element),
            "r" if self.in_cell && self.paragraph.is_some() => start_run(&mut self.run),
            "rPr" if self.in_cell && self.run.is_some() => {
                self.in_run_properties = true;
                parse_run_properties(element, &mut self.run);
            }
            "t" if self.in_cell && self.run.is_some() => self.in_text = true,
            "br" if self.in_cell && self.paragraph.is_some() => self.push_break(),
            _ => return false,
        }
        true
    }

    pub(crate) fn handle_end(&mut self, local: &str, current_color: &mut Option<Color>) -> bool {
        match local {
            "t" if self.in_text => self.in_text = false,
            "rPr" if self.in_run_properties => self.in_run_properties = false,
            "defRPr" if self.in_default_run_properties => {
                if let Some(color) = current_color.take()
                    && let Some(paragraph) = self.paragraph.as_mut()
                {
                    paragraph.def_rpr_color = Some(color);
                }
                self.in_default_run_properties = false;
            }
            "r" if self.in_cell && self.paragraph.is_some() => {
                finish_run(&mut self.run, &mut self.paragraph)
            }
            "p" if self.in_cell => finish_paragraph(&mut self.paragraph, &mut self.paragraphs),
            "buClr" if self.in_bullet_color => self.in_bullet_color = false,
            "buBlip" if self.in_picture_bullet => self.in_picture_bullet = false,
            tag if self.in_list_style && is_lvl_ppr(tag) => self.list_level = None,
            "lstStyle" if self.in_list_style => self.in_list_style = false,
            "lnL" | "lnR" | "lnT" | "lnB" if self.in_properties => self.border_side = None,
            "tcPr" => self.in_properties = false,
            "tc" => {
                finish_cell(
                    &mut self.cell,
                    &mut self.paragraphs,
                    &mut self.list_style,
                    &mut self.row,
                );
                self.in_cell = false;
                self.paragraph = None;
                self.run = None;
                self.in_text = false;
                self.in_run_properties = false;
                self.in_list_style = false;
                self.list_level = None;
                self.list_picture_size = None;
            }
            "tr" => {
                finish_row(&mut self.row, &mut self.builder);
                self.in_row = false;
            }
            "tbl" => self.in_table = false,
            "tableStyleId" if self.in_style_id => self.in_style_id = false,
            "lnSpc" if self.in_line_spacing => self.in_line_spacing = false,
            "spcBef" if self.in_space_before => self.in_space_before = false,
            "spcAft" if self.in_space_after => self.in_space_after = false,
            _ => return false,
        }
        true
    }

    pub(crate) fn handle_empty(&mut self, local: &str, element: &BytesStart<'_>) -> bool {
        match local {
            "gridCol" if self.in_table => parse_column(element, &mut self.builder),
            "noFill" if self.in_properties => {
                if let Some(cell) = self.cell.as_mut() {
                    if let Some(side) = self.border_side.as_deref() {
                        if let Some(border) = border_mut(cell, side) {
                            border.no_fill = true;
                        }
                    } else {
                        cell.fill = Fill::NoFill;
                    }
                }
            }
            "tableStyleId" if self.in_table => {
                if let Some(builder) = self.builder.as_mut() {
                    builder.style_id = Some(String::new());
                }
            }
            "pPr" if self.in_cell && self.paragraph.is_some() => {
                parse_paragraph_properties(element, &mut self.paragraph)
            }
            "defRPr" if self.in_cell && self.paragraph.is_some() && self.run.is_none() => {
                apply_paragraph_default_run_properties(
                    self.paragraph
                        .as_mut()
                        .expect("table paragraph builder for defRPr"),
                    element,
                )
            }
            "rPr" if self.in_cell && self.run.is_some() => {
                parse_run_properties(element, &mut self.run)
            }
            "br" if self.in_cell && self.paragraph.is_some() => self.push_break(),
            "latin" | "ea" | "cs" if self.in_cell => {
                if let Some(typeface) = xml_utils::attr_str(element, "typeface") {
                    if self.in_default_run_properties {
                        if let Some(paragraph) = self.paragraph.as_mut() {
                            assign_typeface_to_paragraph(paragraph, local, typeface);
                        }
                    } else if let Some(run) = self.run.as_mut() {
                        assign_typeface_to_run(run, local, typeface);
                    }
                }
            }
            "spcPct" | "spcPts" if self.in_cell => {
                if let Some(spacing) = parse_spacing(local, element) {
                    assign_spacing_paragraph(
                        self.paragraph.as_mut(),
                        spacing,
                        self.in_line_spacing,
                        self.in_space_before,
                        self.in_space_after,
                    );
                }
            }
            "buFont" if self.in_cell => parse_bullet_font(element, &mut self.paragraph),
            "buSzPct" | "buSzPts" | "buSzTx" if self.in_cell => {
                if self.in_list_style {
                    self.list_picture_size = parse_picture_bullet_size(local, element);
                } else {
                    parse_bullet_size(local, element, &mut self.paragraph);
                }
            }
            "buNone" | "buChar" | "buAutoNum" if self.in_cell => {
                parse_bullet(local, element, &mut self.paragraph)
            }
            "blip" if self.in_picture_bullet => self.assign_picture_bullet(element),
            _ => return false,
        }
        true
    }

    fn push_break(&mut self) {
        let run = RunBuilder {
            is_break: true,
            text: "\n".to_owned(),
            ..Default::default()
        };
        if let Some(paragraph) = self.paragraph.as_mut() {
            paragraph.runs.push(run.build());
        }
    }

    fn assign_picture_bullet(&mut self, element: &BytesStart<'_>) {
        if let Some(level) = self.list_level {
            let defaults = self
                .list_style
                .get_or_insert_with(ListStyle::default)
                .levels[level]
                .get_or_insert_with(ParagraphDefaults::default);
            defaults.bullet = picture_bullet(element, self.list_picture_size.take());
        } else {
            parse_picture_bullet(element, &mut self.paragraph);
        }
    }

    pub(crate) fn handle_text(&mut self, text: &str) -> bool {
        if self.in_style_id {
            if let Some(builder) = self.builder.as_mut() {
                builder.style_id.get_or_insert_default().push_str(text);
            }
            return true;
        }
        if !self.in_text {
            return false;
        }

        append_run_text(&mut self.run, text);
        true
    }
}

pub(crate) fn start_table(in_table: &mut bool, builder: &mut Option<TableBuilder>) {
    *in_table = true;
    *builder = Some(TableBuilder::default());
}

pub(crate) fn parse_table_properties(element: &BytesStart<'_>, builder: &mut Option<TableBuilder>) {
    let Some(builder) = builder.as_mut() else {
        return;
    };
    for (name, target) in [
        ("bandRow", &mut builder.band_row),
        ("bandCol", &mut builder.band_col),
        ("firstRow", &mut builder.first_row),
        ("lastRow", &mut builder.last_row),
        ("firstCol", &mut builder.first_col),
        ("lastCol", &mut builder.last_col),
    ] {
        if let Some(value) = xml_utils::attr_str(element, name) {
            match value.as_str() {
                "1" | "true" => *target = true,
                "0" | "false" => *target = false,
                _ => builder.style_issues.push(TableStyleIssue::InvalidBoolean {
                    name: name.to_owned(),
                    value,
                }),
            }
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
    let h_merge =
        xml_utils::attr_str(element, "hMerge").is_some_and(|value| value == "1" || value == "true");
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
        h_merge,
        explicit_borders: 0,
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
    if let Some(border) = cell.as_mut().and_then(|cell| border_mut(cell, side)) {
        border.no_fill = true;
        if let Some(cell) = cell.as_mut() {
            cell.explicit_borders |= border_bit(side);
        }
    }
    let Some(width) = xml_utils::attr_str(element, "w") else {
        return;
    };
    let width = Emu::parse_emu(&width).to_pt();
    if let Some(border) = cell.as_mut().and_then(|cell| border_mut(cell, side)) {
        border.width = width;
        border.no_fill = false;
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

fn border_bit(side: &str) -> u8 {
    match side {
        "lnL" => 1,
        "lnR" => 2,
        "lnT" => 4,
        "lnB" => 8,
        _ => 0,
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
    list_style: &mut Option<ListStyle>,
    row: &mut Option<TableRowBuilder>,
) {
    if let Some(mut cell) = cell.take() {
        if !paragraphs.is_empty() || list_style.is_some() {
            cell.text_body = Some(TextBody {
                paragraphs: std::mem::take(paragraphs),
                list_style: list_style.take(),
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
    pub(crate) style_id: Option<String>,
    pub(crate) style_issues: Vec<TableStyleIssue>,
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
            style: self.style_id.map(|id| TableStyleReference {
                id,
                source_kind: TableStyleSourceKind::Invalid,
                definition: None,
                issues: self.style_issues,
            }),
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
    pub(crate) h_merge: bool,
    pub(crate) explicit_borders: u8,
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
            h_merge: cell.h_merge,
            explicit_borders: cell.explicit_borders,
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
            h_merge: self.h_merge,
            explicit_borders: self.explicit_borders,
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
