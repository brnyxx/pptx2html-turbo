use super::color::Color;

/// Shape-level DrawingML effects.
#[derive(Debug, Clone, Default)]
pub struct ShapeEffects {
    pub outer_shadow: Option<OuterShadow>,
    pub glow: Option<GlowEffect>,
    pub reflection: Option<ReflectionEffect>,
    pub scene_3d: Option<Scene3d>,
    pub shape_3d: Option<Shape3d>,
    /// Advanced effects in source order. These remain raw fallback metadata.
    pub preserved: Vec<PreservedEffect>,
}

/// Reflection parameters from `<a:reflection>`.
///
/// Values remain in their DrawingML-derived units; renderers must bound them
/// before producing browser styles.
#[derive(Debug, Clone, Default)]
pub struct ReflectionEffect {
    pub blur_radius: Option<f64>,
    pub start_alpha: Option<f64>,
    pub end_alpha: Option<f64>,
    pub start_position: Option<f64>,
    pub end_position: Option<f64>,
    pub distance: Option<f64>,
    pub direction: Option<f64>,
    pub scale_x: Option<f64>,
    pub scale_y: Option<f64>,
    pub skew_x: Option<f64>,
    pub skew_y: Option<f64>,
    pub alignment: Option<String>,
    pub rotate_with_shape: Option<bool>,
    pub raw_xml: String,
}

/// Known scene fields from `<a:scene3d>`; full XML remains preserved separately.
#[derive(Debug, Clone, Default)]
pub struct Scene3d {
    pub camera_preset: Option<String>,
    pub light_rig: Option<String>,
    pub light_direction: Option<String>,
}

/// Known shape fields from `<a:sp3d>`; full XML remains preserved separately.
#[derive(Debug, Clone, Default)]
pub struct Shape3d {
    pub extrusion_height: Option<f64>,
    pub contour_width: Option<f64>,
    pub preset_material: Option<String>,
    pub top_bevel: Option<Bevel3d>,
    pub bottom_bevel: Option<Bevel3d>,
}

#[derive(Debug, Clone, Default)]
pub struct Bevel3d {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub preset: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreservedEffectKind {
    Scene3d,
    Shape3d,
    EffectDag,
}

impl PreservedEffectKind {
    pub fn qualified_name(self) -> &'static str {
        match self {
            Self::Scene3d => "a:scene3d",
            Self::Shape3d => "a:sp3d",
            Self::EffectDag => "a:effectDag",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreservedEffect {
    pub kind: PreservedEffectKind,
    pub raw_xml: String,
}

/// Outer shadow effect (`<a:outerShdw>`).
#[derive(Debug, Clone)]
pub struct OuterShadow {
    pub blur_radius: f64, // in pt (EMU / 12700)
    pub distance: f64,    // in pt
    pub direction: f64,   // in degrees (from 60000ths)
    pub color: Color,
    pub alpha: f64, // 0.0-1.0
}

/// Glow effect (`<a:glow>`).
#[derive(Debug, Clone)]
pub struct GlowEffect {
    pub radius: f64, // in pt
    pub color: Color,
    pub alpha: f64, // 0.0-1.0
}
