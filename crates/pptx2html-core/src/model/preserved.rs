use super::geometry::{CustomGeometry, Position, Size};
use super::{CapabilityStage, FeatureFamily, SupportTier};

#[derive(Debug, Clone, Default)]
pub struct DiagnosticLocation {
    pub slide_index: Option<usize>,
    pub part_name: Option<String>,
    pub relationship_id: Option<String>,
    pub relationship_type: Option<String>,
    pub qualified_element_name: Option<String>,
    pub position: Option<Position>,
    pub size: Option<Size>,
}

impl PartialEq for DiagnosticLocation {
    fn eq(&self, other: &Self) -> bool {
        self.slide_index == other.slide_index
            && self.part_name == other.part_name
            && self.relationship_id == other.relationship_id
            && self.relationship_type == other.relationship_type
            && self.qualified_element_name == other.qualified_element_name
            && positions_equal(self.position, other.position)
            && sizes_equal(self.size, other.size)
    }
}

impl Eq for DiagnosticLocation {}

fn positions_equal(left: Option<Position>, right: Option<Position>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.x.0 == right.x.0 && left.y.0 == right.y.0,
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
    }
}

fn sizes_equal(left: Option<Size>, right: Option<Size>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => {
            left.width.0 == right.width.0 && left.height.0 == right.height.0
        }
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackKind {
    SmartArtPlaceholder,
    OlePlaceholder,
    MathPlaceholder,
    CustomGeometryPlaceholder,
    PreservedPart,
    IgnoredRelationship,
    UnknownElement,
    TableStyleDefinitionUnavailable,
    ActionMetadata,
}

impl FallbackKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SmartArtPlaceholder => "smartart-placeholder",
            Self::OlePlaceholder => "ole-placeholder",
            Self::MathPlaceholder => "math-placeholder",
            Self::CustomGeometryPlaceholder => "custom-geometry-placeholder",
            Self::PreservedPart => "preserved-part",
            Self::IgnoredRelationship => "ignored-relationship",
            Self::UnknownElement => "unknown-element",
            Self::TableStyleDefinitionUnavailable => "table-style-definition-unavailable",
            Self::ActionMetadata => "action-metadata",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionDiagnostic {
    pub code: String,
    pub family: FeatureFamily,
    pub support_tier: SupportTier,
    pub stage: Option<CapabilityStage>,
    pub location: DiagnosticLocation,
    pub raw_reference: Option<String>,
    pub fallback_kind: FallbackKind,
    pub reason: String,
}

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
    pub custom_geometry: Option<CustomGeometry>,
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
