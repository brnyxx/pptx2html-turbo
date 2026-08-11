use super::{Border, Color, Fill};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableStyleSourceKind {
    Package,
    BuiltIn,
    Invalid,
}

impl TableStyleSourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::BuiltIn => "built_in",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableStyleRegion {
    WholeTable,
    Band1Horizontal,
    Band2Horizontal,
    Band1Vertical,
    Band2Vertical,
    LastColumn,
    FirstColumn,
    LastRow,
    SoutheastCell,
    SouthwestCell,
    FirstRow,
    NortheastCell,
    NorthwestCell,
}

impl TableStyleRegion {
    pub const fn as_ooxml(self) -> &'static str {
        match self {
            Self::WholeTable => "wholeTbl",
            Self::Band1Horizontal => "band1H",
            Self::Band2Horizontal => "band2H",
            Self::Band1Vertical => "band1V",
            Self::Band2Vertical => "band2V",
            Self::LastColumn => "lastCol",
            Self::FirstColumn => "firstCol",
            Self::LastRow => "lastRow",
            Self::SoutheastCell => "seCell",
            Self::SouthwestCell => "swCell",
            Self::FirstRow => "firstRow",
            Self::NortheastCell => "neCell",
            Self::NorthwestCell => "nwCell",
        }
    }

    pub fn from_ooxml(value: &str) -> Option<Self> {
        match value {
            "wholeTbl" => Some(Self::WholeTable),
            "band1H" => Some(Self::Band1Horizontal),
            "band2H" => Some(Self::Band2Horizontal),
            "band1V" => Some(Self::Band1Vertical),
            "band2V" => Some(Self::Band2Vertical),
            "lastCol" => Some(Self::LastColumn),
            "firstCol" => Some(Self::FirstColumn),
            "lastRow" => Some(Self::LastRow),
            "seCell" => Some(Self::SoutheastCell),
            "swCell" => Some(Self::SouthwestCell),
            "firstRow" => Some(Self::FirstRow),
            "neCell" => Some(Self::NortheastCell),
            "nwCell" => Some(Self::NorthwestCell),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TableTextStyle {
    pub font_family: Option<String>,
    pub color: Option<Color>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct TableCellStyle {
    pub fill: Option<Fill>,
    pub left: Option<Border>,
    pub right: Option<Border>,
    pub top: Option<Border>,
    pub bottom: Option<Border>,
    pub inside_horizontal: Option<Border>,
    pub inside_vertical: Option<Border>,
    pub text: TableTextStyle,
}

#[derive(Debug, Clone, Default)]
pub struct TableStyle {
    pub id: String,
    pub name: Option<String>,
    pub regions: Vec<(TableStyleRegion, TableCellStyle)>,
    pub table_background: Option<Fill>,
    pub unsupported_primitives: Vec<String>,
}

impl TableStyle {
    pub fn region(&self, region: TableStyleRegion) -> Option<&TableCellStyle> {
        self.regions
            .iter()
            .rev()
            .find_map(|(candidate, style)| (*candidate == region).then_some(style))
    }
}

#[derive(Debug, Clone)]
pub struct TableStyleReference {
    pub id: String,
    pub source_kind: TableStyleSourceKind,
    pub definition: Option<TableStyle>,
    pub issues: Vec<TableStyleIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableStyleIssue {
    DuplicateId,
    InvalidBoolean { name: String, value: String },
}
