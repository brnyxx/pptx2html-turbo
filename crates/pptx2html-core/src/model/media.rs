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
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaInventory;
