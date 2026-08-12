use std::collections::HashMap;

use super::action::ActionSet;
use super::chart::ChartData;
use super::effects::ShapeEffects;
use super::fill::Fill;
use super::geometry::{CustomGeometry, Position, Size};
use super::hierarchy::{PlaceholderInfo, ShapeStyleRef};
use super::media::MediaData;
use super::preserved::UnsupportedData;
use super::style::Border;
use super::table::TableData;
use super::text::TextBody;

/// Shape type
#[derive(Debug, Clone, Default)]
pub enum ShapeType {
    #[default]
    Rectangle,
    RoundedRectangle,
    Ellipse,
    Triangle,
    Arrow,
    Line,
    TextBox,
    Picture(PictureData),
    Table(TableData),
    Group(Vec<Shape>, GroupData),
    Chart(ChartData),
    Custom(String), // preset shape name
    CustomGeom(CustomGeometry),
    /// Unsupported content placeholder (SmartArt, OLE, Math, etc.)
    Unsupported(UnsupportedData),
}

/// Shape
#[derive(Debug, Clone, Default)]
pub struct Shape {
    pub id: u32,
    pub name: String,
    pub actions: ActionSet,
    /// Shape-owned DrawingML audio or video, when declared in `p:nvPr`.
    pub media: Option<MediaData>,
    pub shape_type: ShapeType,
    pub position: Position,
    pub size: Size,
    pub rotation: f64, // in degrees
    pub flip_h: bool,
    pub flip_v: bool,
    pub fill: Fill,
    pub border: Border,
    pub text_body: Option<TextBody>,
    pub hidden: bool,
    pub placeholder: Option<PlaceholderInfo>,
    pub style_ref: Option<ShapeStyleRef>,
    pub adjust_values: Option<HashMap<String, f64>>,
    pub start_connection: Option<ConnectionRef>,
    pub end_connection: Option<ConnectionRef>,
    pub vertical_text: Option<String>, // "vert", "vert270", "wordArtVert", etc.
    pub vertical_text_explicit: bool,
    pub effects: ShapeEffects,
}

#[derive(Debug, Clone)]
pub struct ConnectionRef {
    pub shape_id: u32,
    pub site_idx: usize,
}

/// Picture data
#[derive(Debug, Clone, Default)]
pub struct PictureData {
    pub rel_id: String,
    pub content_type: String,
    pub data: Vec<u8>,
    pub crop: Option<CropRect>,
}

/// Image crop rectangle (values 0.0-1.0, representing percentage from each edge)
#[derive(Debug, Clone, Default)]
pub struct CropRect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

/// Group shape data (child offset/extent for coordinate remapping)
#[derive(Debug, Clone, Default)]
pub struct GroupData {
    pub child_offset: Position,
    pub child_extent: Size,
}
