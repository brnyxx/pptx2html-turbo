#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Audio,
    Video,
}

impl MediaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFailure {
    MissingRelationship,
    DuplicateRelationship,
    WrongRelationshipType,
    ExternalTarget,
    UnsafeTarget,
    MissingContentType,
    UnsupportedContentType,
    MissingPart,
    EmptyAsset,
    AssetTooLarge,
    UnsupportedCodec,
}

impl MediaFailure {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingRelationship => "missing-relationship",
            Self::DuplicateRelationship => "duplicate-relationship",
            Self::WrongRelationshipType => "wrong-relationship-type",
            Self::ExternalTarget => "external-target",
            Self::UnsafeTarget => "unsafe-target",
            Self::MissingContentType => "missing-content-type",
            Self::UnsupportedContentType => "unsupported-content-type",
            Self::MissingPart => "missing-part",
            Self::EmptyAsset => "empty-asset",
            Self::AssetTooLarge => "asset-too-large",
            Self::UnsupportedCodec => "unsupported-codec",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::MissingRelationship => "DRAWINGML_MEDIA_RELATIONSHIP_MISSING",
            Self::DuplicateRelationship => "DRAWINGML_MEDIA_RELATIONSHIP_DUPLICATE",
            Self::WrongRelationshipType => "DRAWINGML_MEDIA_RELATIONSHIP_TYPE",
            Self::ExternalTarget => "DRAWINGML_MEDIA_EXTERNAL_TARGET",
            Self::UnsafeTarget => "DRAWINGML_MEDIA_TARGET_UNSAFE",
            Self::MissingContentType => "DRAWINGML_MEDIA_CONTENT_TYPE_MISSING",
            Self::UnsupportedContentType => "DRAWINGML_MEDIA_CONTENT_TYPE_UNSUPPORTED",
            Self::MissingPart => "DRAWINGML_MEDIA_PART_MISSING",
            Self::EmptyAsset => "DRAWINGML_MEDIA_ASSET_EMPTY",
            Self::AssetTooLarge => "DRAWINGML_MEDIA_ASSET_TOO_LARGE",
            Self::UnsupportedCodec => "DRAWINGML_MEDIA_CODEC_UNSUPPORTED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaData {
    pub kind: MediaKind,
    pub relationship_id: String,
    pub relationship_type: Option<String>,
    pub content_type: Option<String>,
    pub data: Vec<u8>,
    pub failure: Option<MediaFailure>,
    pub trim_start: Option<u64>,
    pub trim_end: Option<u64>,
    pub loop_requested: Option<bool>,
    pub volume: Option<u32>,
    pub autoplay_requested: Option<bool>,
}

impl MediaData {
    pub fn unresolved(kind: MediaKind, relationship_id: String) -> Self {
        Self {
            kind,
            relationship_id,
            relationship_type: None,
            content_type: None,
            data: Vec::new(),
            failure: Some(MediaFailure::MissingRelationship),
            trim_start: None,
            trim_end: None,
            loop_requested: None,
            volume: None,
            autoplay_requested: None,
        }
    }

    pub(crate) fn encode_marker(&self) -> String {
        format!(
            "application/x-pptx2html-media;kind={};rid={};rtype={};failure={};trim_start={};trim_end={};loop={};volume={};autoplay={}",
            self.kind.as_str(),
            self.relationship_id,
            self.relationship_type.as_deref().unwrap_or(""),
            self.failure.map(MediaFailure::as_str).unwrap_or("none"),
            optional_number(self.trim_start),
            optional_number(self.trim_end),
            optional_bool(self.loop_requested),
            optional_number(self.volume),
            optional_bool(self.autoplay_requested),
        )
    }

    pub(crate) fn decode_marker(marker: &str) -> Option<Self> {
        let fields = marker.strip_prefix("application/x-pptx2html-media;")?;
        let value = |name: &str| {
            fields.split(';').find_map(|field| {
                let (key, value) = field.split_once('=')?;
                (key == name).then_some(value)
            })
        };
        let kind = match value("kind")? {
            "audio" => MediaKind::Audio,
            "video" => MediaKind::Video,
            _ => return None,
        };
        let failure = match value("failure")? {
            "none" => None,
            raw => Some(MediaFailure::from_str(raw)?),
        };
        Some(Self {
            kind,
            relationship_id: value("rid")?.to_owned(),
            relationship_type: value("rtype")
                .filter(|raw| !raw.is_empty())
                .map(str::to_owned),
            content_type: None,
            data: Vec::new(),
            failure,
            trim_start: parse_optional_number(value("trim_start")),
            trim_end: parse_optional_number(value("trim_end")),
            loop_requested: parse_optional_bool(value("loop")),
            volume: parse_optional_number(value("volume")),
            autoplay_requested: parse_optional_bool(value("autoplay")),
        })
    }
}

impl MediaFailure {
    fn from_str(value: &str) -> Option<Self> {
        [
            Self::MissingRelationship,
            Self::DuplicateRelationship,
            Self::WrongRelationshipType,
            Self::ExternalTarget,
            Self::UnsafeTarget,
            Self::MissingContentType,
            Self::UnsupportedContentType,
            Self::MissingPart,
            Self::EmptyAsset,
            Self::AssetTooLarge,
            Self::UnsupportedCodec,
        ]
        .into_iter()
        .find(|failure| failure.as_str() == value)
    }
}

fn optional_number<T: ToString>(value: Option<T>) -> String {
    value.map(|number| number.to_string()).unwrap_or_default()
}

fn optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "",
    }
}

fn parse_optional_number<T: std::str::FromStr>(value: Option<&str>) -> Option<T> {
    value.filter(|raw| !raw.is_empty())?.parse().ok()
}

fn parse_optional_bool(value: Option<&str>) -> Option<bool> {
    match value? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaInventory;
