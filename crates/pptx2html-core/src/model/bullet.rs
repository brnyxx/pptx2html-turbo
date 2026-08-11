use super::color::Color;

pub(crate) const TEXT_FONT_SIZE_MIN_HUNDREDTHS_POINT: f64 = 100.0;
pub(crate) const TEXT_FONT_SIZE_MAX_HUNDREDTHS_POINT: f64 = 400_000.0;

/// Bullet
#[derive(Debug, Clone)]
pub enum Bullet {
    Char(BulletChar),
    AutoNum(BulletAutoNum),
    Picture(PictureBullet),
    None,
}

#[derive(Debug, Clone)]
pub struct PictureBullet {
    pub relationship_id: String,
    pub relationship_mode: Option<PictureBulletRelationshipMode>,
    pub relationship_type: Option<String>,
    pub target_mode: Option<PictureBulletTargetMode>,
    pub image: Option<PictureBulletImage>,
    pub failure: Option<PictureBulletFailure>,
    pub size: Option<BulletSize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureBulletRelationshipMode {
    Embed,
    Link,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PictureBulletTargetMode {
    Internal,
    External,
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureBulletFailure {
    MissingRelationship,
    WrongRelationshipKind,
    WrongTargetMode,
    LinkedExternal,
    MissingContentType,
    UnsupportedContentType,
    MissingPart,
    EmptyImage,
}

impl PictureBulletFailure {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingRelationship => "missing relationship",
            Self::WrongRelationshipKind => "wrong relationship kind",
            Self::WrongTargetMode => "wrong relationship target mode",
            Self::LinkedExternal => "external linked image is not fetched",
            Self::MissingContentType => "missing package content type",
            Self::UnsupportedContentType => "unsupported browser image content type",
            Self::MissingPart => "missing package image part",
            Self::EmptyImage => "empty package image part",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PictureBulletImage {
    pub data: Vec<u8>,
    pub content_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BulletSize {
    Text,
    Percentage(f64),
    Points(f64),
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
