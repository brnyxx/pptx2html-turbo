//! Data model representing PPTX document structure
//! Based on ECMA-376 Part 1 (PresentationML)

pub mod action;
pub mod bullet;
pub mod capabilities;
pub mod chart;
pub mod color;
pub mod effects;
pub mod embedded;
pub mod fill;
mod geometry;
pub mod hierarchy;
pub mod media;
pub mod notes_comments;
mod pattern;
pub mod presentation;
pub mod preserved;
pub mod shape;
pub mod slide;
mod style;
pub mod table;
pub mod table_style;
pub mod text;
pub mod timing;

pub use action::{
    Action, ActionIssue, ActionSet, ActionTarget, ActionTrigger, is_safe_external_uri,
};
pub use capabilities::{
    CapabilityMatrix, CapabilityStage, FeatureCapability, FeatureFamily, SupportTier,
};
pub use color::{Color, ColorKind, ColorModifier, ResolvedColor};
pub use geometry::{
    AdjustHandle, ConnectionSite, CustomGeometry, CustomGeometryIssue, CustomGuide, Emu, GeomRect,
    GeometryPath, GuideFormulaError, PathCommand, PathFill, PolarAdjustHandle, Position, Size,
    XYAdjustHandle,
};
pub use hierarchy::{
    ClrMapOverride, EffectStyle, FmtScheme, FontRef, ListStyle, ParagraphDefaults, PlaceholderInfo,
    PlaceholderType, RunDefaults, ShapeStyleRef, SlideLayout, SlideMaster, SpacingValue, StyleRef,
    TxStyles,
};
pub use media::{MediaData, MediaFailure, MediaKind};
pub use notes_comments::{
    AnnotationIssue, AnnotationIssueCode, CommentAuthor, CommentKind, NotesCommentsInventory,
    SlideComment, SlideNote,
};
pub use pattern::PatternPreset;
pub use presentation::{ClrMap, FontScheme, Presentation};
pub use preserved::{ConversionDiagnostic, DiagnosticLocation, FallbackKind};
pub use slide::{
    AutoFit, Bullet, BulletAutoNum, BulletChar, BulletSize, ChartBubbleSizeRepresents, ChartData,
    ChartDataLabelPosition, ChartDataLabelSettings, ChartGrouping, ChartMarkerSpec, ChartOfPieType,
    ChartRadarStyle, ChartScatterStyle, ChartSeries, ChartSpec, ChartSplitType, ChartType,
    ConnectionRef, CropRect, GroupData, ParagraphDefRPr, PictureBullet, PictureBulletFailure,
    PictureBulletImage, PictureBulletRelationshipMode, PictureBulletTargetMode, PictureData, Shape,
    ShapeType, Slide, TableCell, TableData, TableRow, TextBody, TextMargins, TextParagraph,
    TextRun, UnresolvedElement, UnresolvedType, UnsupportedData, VerticalAlign,
};
pub use style::{
    Alignment, Border, BorderStyle, CompoundLine, DashStyle, Fill, FontStyle, GlowEffect,
    GradientFill, GradientStop, GradientType, ImageFill, LineAlignment, LineCap, LineEnd,
    LineEndSize, LineEndType, LineJoin, OuterShadow, PatternFill, ShapeEffects, SolidFill,
    StrikethroughType, TextCapitalization, TextShadow, TextStyle, UnderlineType,
};
pub use table_style::{
    TableCellStyle, TableStyle, TableStyleIssue, TableStylePrimitiveReference, TableStyleReference,
    TableStyleRegion, TableStyleSourceKind, TableTextStyle,
};
