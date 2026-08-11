use super::color::Color;

/// Shape-level effects (outerShdw, glow from <a:effectLst>)
#[derive(Debug, Clone, Default)]
pub struct ShapeEffects {
    pub outer_shadow: Option<OuterShadow>,
    pub glow: Option<GlowEffect>,
}

/// Outer shadow effect (<a:outerShdw>)
#[derive(Debug, Clone)]
pub struct OuterShadow {
    pub blur_radius: f64, // in pt (EMU / 12700)
    pub distance: f64,    // in pt
    pub direction: f64,   // in degrees (from 60000ths)
    pub color: Color,
    pub alpha: f64, // 0.0-1.0
}

/// Glow effect (<a:glow>)
#[derive(Debug, Clone)]
pub struct GlowEffect {
    pub radius: f64, // in pt
    pub color: Color,
    pub alpha: f64, // 0.0-1.0
}
