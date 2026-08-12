from typing import Literal, Optional

SupportTier = Literal["exact", "approximate", "fallback", "unparsed"]
CapabilityStage = Literal["parsed", "resolved", "rendered", "fidelity-tested"]
FeatureFamily = Literal["shapes", "text", "tables", "images", "layout", "charts", "media", "unsupported"]
FallbackKind = Literal[
    "smartart-placeholder",
    "ole-placeholder",
    "math-placeholder",
    "custom-geometry-placeholder",
    "preserved-part",
    "ignored-relationship",
    "unknown-element",
    "table-style-definition-unavailable",
    "action-metadata",
]

class PresentationInfo:
    """Presentation metadata."""
    slide_count: int
    width_px: float
    height_px: float
    title: Optional[str]

class UnresolvedElement:
    """Metadata about an element rendered as a placeholder."""
    slide_index: int
    element_type: str  # "smartart" | "ole" | "math" | "custom-geometry"
    placeholder_id: str
    raw_xml: Optional[str]
    data_model: Optional[str]

class DiagnosticPosition:
    """Position in integer OOXML EMUs."""
    x: int
    y: int

class DiagnosticSize:
    """Size in integer OOXML EMUs."""
    width: int
    height: int

class DiagnosticLocation:
    """Typed source location for a conversion diagnostic."""
    slide_index: Optional[int]
    part_name: Optional[str]
    relationship_id: Optional[str]
    relationship_type: Optional[str]
    qualified_element_name: Optional[str]
    position: Optional[DiagnosticPosition]
    size: Optional[DiagnosticSize]

class ConversionDiagnostic:
    """Read-only conversion diagnostic."""
    code: str
    family: FeatureFamily
    support_tier: SupportTier
    stage: Optional[CapabilityStage]
    location: DiagnosticLocation
    raw_reference: Optional[str]
    fallback_kind: FallbackKind
    reason: str

class ConversionResult:
    """Result of PPTX conversion with metadata."""
    html: str
    diagnostics: list[ConversionDiagnostic]
    diagnostics_json: str
    unresolved_elements: list[UnresolvedElement]
    slide_count: int

def convert_file(path: str) -> str:
    """Convert a PPTX file to an HTML string."""
    ...

def convert_bytes(data: bytes) -> str:
    """Convert PPTX bytes to an HTML string."""
    ...

def convert(
    path: str,
    *,
    embed_images: bool = True,
    include_hidden: bool = False,
    slides: Optional[list[int]] = None,
    scale: float = 1.0,
) -> str:
    """Convert a PPTX file to HTML with options.

    Args:
        path: Path to the PPTX file.
        embed_images: Embed images as base64 data URIs (default: True).
        include_hidden: Include hidden slides (default: False).
        slides: List of 1-based slide indices to include (default: all).
        scale: Whole-slide zoom factor (default: 1.0).
    """
    ...

def convert_with_metadata(
    path: str,
    *,
    embed_images: bool = True,
    include_hidden: bool = False,
    slides: Optional[list[int]] = None,
    scale: float = 1.0,
) -> ConversionResult:
    """Convert a PPTX file to HTML with metadata about unresolved elements.

    Args:
        path: Path to the PPTX file.
        embed_images: Embed images as base64 data URIs (default: True).
        include_hidden: Include hidden slides (default: False).
        slides: List of 1-based slide indices to include (default: all).
        scale: Whole-slide zoom factor (default: 1.0).

    Returns:
        ConversionResult with html, unresolved_elements, and slide_count.
    """
    ...

def convert_bytes_with_metadata(
    data: bytes,
    *,
    embed_images: bool = True,
    include_hidden: bool = False,
    slides: Optional[list[int]] = None,
    scale: float = 1.0,
) -> ConversionResult:
    """Convert PPTX bytes to HTML with metadata about unresolved elements.

    Args:
        data: PPTX file bytes.
        embed_images: Embed images as base64 data URIs (default: True).
        include_hidden: Include hidden slides (default: False).
        slides: List of 1-based slide indices to include (default: all).
        scale: Whole-slide zoom factor (default: 1.0).

    Returns:
        ConversionResult with html, unresolved_elements, and slide_count.
    """
    ...

def get_info(path: str) -> PresentationInfo:
    """Get presentation metadata (slide count, size, title)."""
    ...
