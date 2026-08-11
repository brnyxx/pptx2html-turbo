use super::geometry::{Position, Size};

/// Type of content that could not be fully rendered
#[derive(Debug, Clone, PartialEq)]
pub enum UnresolvedType {
    SmartArt,
    OleObject,
    MathEquation,
    CustomGeometry,
}

/// Data carried by an Unsupported shape variant
#[derive(Debug, Clone)]
pub struct UnsupportedData {
    /// Human-readable label (e.g. "SmartArt", "OLE Object")
    pub label: String,
    /// Typed classification for programmatic use
    pub element_type: UnresolvedType,
    /// Raw XML snippet captured from the original PPTX
    pub raw_xml: Option<String>,
}

/// Metadata about an element that was rendered as a placeholder
#[derive(Debug, Clone)]
pub struct UnresolvedElement {
    /// 0-based slide index
    pub slide_index: usize,
    /// Type of unresolved content
    pub element_type: UnresolvedType,
    /// Unique ID matching the HTML placeholder element
    pub placeholder_id: String,
    /// Bounding box position in EMU
    pub position: Option<Position>,
    /// Bounding box size in EMU
    pub size: Option<Size>,
    /// Raw XML snippet from the original PPTX
    pub raw_xml: Option<String>,
    /// Structured data model as JSON string (reserved for LLM post-processing)
    pub data_model: Option<String>,
}
