use super::color::Color;

/// Bullet
#[derive(Debug, Clone)]
pub enum Bullet {
    Char(BulletChar),
    AutoNum(BulletAutoNum),
    None,
}

/// Character bullet with optional font/size/color
#[derive(Debug, Clone)]
pub struct BulletChar {
    pub char: String,
    pub font: Option<String>,
    pub size_pct: Option<f64>, // percentage of text size, e.g. 1.0 = 100%
    pub color: Option<Color>,
}

/// Auto-numbered bullet with optional font/size/color
#[derive(Debug, Clone)]
pub struct BulletAutoNum {
    pub num_type: String, // "arabicPeriod", "alphaLcPeriod", etc.
    pub start_at: Option<i32>,
    pub font: Option<String>,
    pub size_pct: Option<f64>,
    pub color: Option<Color>,
}
