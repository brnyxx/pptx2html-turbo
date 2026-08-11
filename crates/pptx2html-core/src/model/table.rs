use super::fill::Fill;
use super::style::Border;
use super::table_style::TableStyleReference;
use super::text::{TextBody, VerticalAlign};

/// Table data
#[derive(Debug, Clone, Default)]
pub struct TableData {
    pub rows: Vec<TableRow>,
    pub col_widths: Vec<f64>,
    pub band_row: bool,
    pub band_col: bool,
    pub first_row: bool,
    pub last_row: bool,
    pub first_col: bool,
    pub last_col: bool,
    pub style: Option<TableStyleReference>,
}

#[derive(Debug, Clone, Default)]
pub struct TableRow {
    pub height: f64,
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone)]
pub struct TableCell {
    pub text_body: Option<TextBody>,
    pub fill: Fill,
    pub border_top: Border,
    pub border_bottom: Border,
    pub border_left: Border,
    pub border_right: Border,
    pub col_span: u32,
    pub row_span: u32,
    pub v_merge: bool,
    pub h_merge: bool,
    pub explicit_borders: u8,
    pub margin_left: f64,   // in pt
    pub margin_right: f64,  // in pt
    pub margin_top: f64,    // in pt
    pub margin_bottom: f64, // in pt
    pub vertical_align: VerticalAlign,
}

impl Default for TableCell {
    fn default() -> Self {
        Self {
            text_body: None,
            fill: Fill::None,
            border_top: Border::default(),
            border_bottom: Border::default(),
            border_left: Border::default(),
            border_right: Border::default(),
            col_span: 0,
            row_span: 0,
            v_merge: false,
            h_merge: false,
            explicit_borders: 0,
            margin_left: 7.2,   // OOXML default 91440 EMU
            margin_right: 7.2,  // OOXML default 91440 EMU
            margin_top: 3.6,    // OOXML default 45720 EMU
            margin_bottom: 3.6, // OOXML default 45720 EMU
            vertical_align: VerticalAlign::Top,
        }
    }
}
