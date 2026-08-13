use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityStage {
    Parsed,
    Resolved,
    Rendered,
    FidelityTested,
}

impl CapabilityStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parsed => "parsed",
            Self::Resolved => "resolved",
            Self::Rendered => "rendered",
            Self::FidelityTested => "fidelity-tested",
        }
    }
}

impl fmt::Display for CapabilityStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportTier {
    Exact,
    Approximate,
    Fallback,
    Unparsed,
}

impl SupportTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Approximate => "approximate",
            Self::Fallback => "fallback",
            Self::Unparsed => "unparsed",
        }
    }
}

impl fmt::Display for SupportTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureFamily {
    Shapes,
    Text,
    Tables,
    Images,
    Layout,
    Charts,
    Media,
    Unsupported,
}

impl FeatureFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shapes => "shapes",
            Self::Text => "text",
            Self::Tables => "tables",
            Self::Images => "images",
            Self::Layout => "layout",
            Self::Charts => "charts",
            Self::Media => "media",
            Self::Unsupported => "unsupported",
        }
    }
}

impl fmt::Display for FeatureFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureCapability {
    pub family: FeatureFamily,
    pub feature: String,
    pub tier: SupportTier,
    pub stage: Option<CapabilityStage>,
    pub notes: Option<String>,
}

impl FeatureCapability {
    pub fn new(
        family: FeatureFamily,
        feature: impl Into<String>,
        tier: SupportTier,
        stage: Option<CapabilityStage>,
    ) -> Result<Self, &'static str> {
        let capability = Self {
            family,
            feature: feature.into(),
            tier,
            stage,
            notes: None,
        };
        capability.validate()?;
        Ok(capability)
    }

    pub fn highest_completed_stage(&self) -> Option<CapabilityStage> {
        self.stage
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        match (self.tier, self.stage) {
            (SupportTier::Unparsed, None) => Ok(()),
            (SupportTier::Unparsed, Some(_)) => {
                Err("unparsed capabilities must not declare a completed stage")
            }
            (SupportTier::Exact, Some(CapabilityStage::FidelityTested)) => Ok(()),
            (SupportTier::Exact, _) => Err("exact capabilities must be fidelity-tested"),
            (SupportTier::Approximate | SupportTier::Fallback, Some(_)) => Ok(()),
            (SupportTier::Approximate | SupportTier::Fallback, None) => {
                Err("approximate and fallback capabilities must declare a completed stage")
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityMatrix {
    pub features: Vec<FeatureCapability>,
}

impl CapabilityMatrix {
    pub fn new(features: Vec<FeatureCapability>) -> Self {
        Self { features }
    }
}

// BEGIN EXACTNESS CAPABILITY REGISTRATIONS
pub const CAPABILITY_PRESENTATION: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "p:presentation",
    "direct",
);
pub const CAPABILITY_PRESENTATION_PROPERTIES: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "p:presentationPr",
    "direct",
);
pub const CAPABILITY_SLIDE_MASTER: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Resolved),
    "p:sldMaster",
    "direct",
);
pub const CAPABILITY_SLIDE_LAYOUT: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Resolved),
    "p:sldLayout",
    "direct",
);
pub const CAPABILITY_SLIDE: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "p:sld",
    "direct",
);
pub const CAPABILITY_THEME: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Resolved),
    "a:theme",
    "direct",
);
pub const CAPABILITY_NOTES_MASTER: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "p:notesMaster",
    "fallback",
);
pub const CAPABILITY_NOTES: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "p:notes",
    "fallback",
);
pub const CAPABILITY_HANDOUT_MASTER: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "p:handoutMaster",
    "fallback",
);
pub const CAPABILITY_COMMENTS: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "p:cmLst",
    "fallback",
);
pub const CAPABILITY_COMMENT_AUTHORS: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "p:cmAuthorLst",
    "fallback",
);
pub const CAPABILITY_SHAPE_TREE: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "p:spTree",
    "direct",
);
pub const CAPABILITY_PRESET_SHAPE: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "a:prstGeom",
    "direct",
);
pub const CAPABILITY_CUSTOM_GEOMETRY: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "a:custGeom",
    "direct",
);
pub const CAPABILITY_CONNECTOR: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "p:cxnSp",
    "direct",
);
pub const CAPABILITY_GROUP_SHAPE: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "p:grpSp",
    "direct",
);
pub const CAPABILITY_PICTURE: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "p:pic",
    "direct",
);
pub const CAPABILITY_TEXT_BODY: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "p:txBody",
    "direct",
);
pub const CAPABILITY_RTL_TEXT: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "a:pPr",
    "direct",
);
pub const CAPABILITY_BULLETS: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "a:buChar",
    "direct",
);
pub const CAPABILITY_PICTURE_BULLETS: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "a:buBlip",
    "fallback",
);
pub const CAPABILITY_FILLS: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "a:solidFill",
    "direct",
);
pub const CAPABILITY_PATTERN_FILL: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "a:pattFill",
    "fallback",
);
pub const CAPABILITY_EFFECTS: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "a:outerShdw",
    "direct",
);
pub const CAPABILITY_REFLECTION_AND_3D: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "a:reflection",
    "fallback",
);
pub const CAPABILITY_TABLE: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "a:tbl",
    "direct",
);
pub const CAPABILITY_TABLE_STYLE: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "PPTX_COMPLETENESS_FALLBACK",
    "fallback",
);
pub const CAPABILITY_IMAGE: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image",
    "direct",
);
pub const CAPABILITY_CHART_DIRECT_SUBSET: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart",
    "direct",
);
pub const CAPABILITY_CHART_PREVIEW_FALLBACK: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart",
    "fallback",
);
pub const CAPABILITY_CHART_PLACEHOLDER_FALLBACK: (
    SupportTier,
    Option<CapabilityStage>,
    &str,
    &str,
) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart",
    "fallback",
);
pub const CAPABILITY_DIAGRAM: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData",
    "fallback",
);
pub const CAPABILITY_DIAGRAM_DATA: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "dgm:dataModel",
    "fallback",
);
pub const CAPABILITY_DIAGRAM_LAYOUT: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "dgm:layoutDef",
    "fallback",
);
pub const CAPABILITY_DIAGRAM_STYLES: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "dgm:styleDef",
    "fallback",
);
pub const CAPABILITY_DIAGRAM_COLORS: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "dgm:colorsDef",
    "fallback",
);
pub const CAPABILITY_OLE_EMBEDDED_OBJECT: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "p:oleObj",
    "fallback",
);
pub const CAPABILITY_MATH: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "m:oMath",
    "fallback",
);
pub const CAPABILITY_MEDIA_AUDIO: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "a:audioFile",
    "direct",
);
pub const CAPABILITY_MEDIA_VIDEO: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "a:videoFile",
    "direct",
);
pub const CAPABILITY_HYPERLINK_RUN_AND_CELL: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
    "direct",
);
pub const CAPABILITY_SHAPE_HYPERLINK_AND_ACTION: (
    SupportTier,
    Option<CapabilityStage>,
    &str,
    &str,
) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
    "fallback",
);
pub const CAPABILITY_TIMING_AND_ANIMATION: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "p:timing",
    "direct",
);
pub const CAPABILITY_TRANSITIONS: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Approximate,
    Some(CapabilityStage::Parsed),
    "p:transition",
    "direct",
);
pub const CAPABILITY_EXTENSIONS: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "p:extLst",
    "fallback",
);
pub const CAPABILITY_ALTERNATE_CONTENT: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "mc:AlternateContent",
    "fallback",
);
pub const CAPABILITY_BIBLIOGRAPHY: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "b:Sources",
    "fallback",
);
pub const CAPABILITY_ADDITIONAL_CHARACTERISTICS: (
    SupportTier,
    Option<CapabilityStage>,
    &str,
    &str,
) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "ac:AdditionalCharacteristics",
    "fallback",
);
pub const CAPABILITY_CUSTOM_XML: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml",
    "fallback",
);
pub const CAPABILITY_THUMBNAIL: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/package/2006/relationships/metadata/thumbnail",
    "fallback",
);
pub const CAPABILITY_THEME_OVERRIDE: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/themeOverride",
    "fallback",
);
pub const CAPABILITY_SLIDE_SYNCHRONIZATION: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "p:sldSyncPr",
    "fallback",
);
pub const CAPABILITY_CONTENT_PART: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "p:contentPart",
    "fallback",
);
pub const CAPABILITY_EMBEDDED_PACKAGE: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/package",
    "fallback",
);
pub const CAPABILITY_EMBEDDED_CONTROL_PERSISTENCE: (
    SupportTier,
    Option<CapabilityStage>,
    &str,
    &str,
) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/control",
    "fallback",
);
pub const CAPABILITY_USER_DEFINED_TAGS: (SupportTier, Option<CapabilityStage>, &str, &str) = (
    SupportTier::Fallback,
    Some(CapabilityStage::Parsed),
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/tags",
    "fallback",
);
// END EXACTNESS CAPABILITY REGISTRATIONS

#[cfg(test)]
mod tests {
    use super::{CapabilityStage, FeatureCapability, FeatureFamily, SupportTier};

    #[test]
    fn capability_support_tier_display_is_stable() {
        assert_eq!(SupportTier::Exact.to_string(), "exact");
        assert_eq!(SupportTier::Approximate.to_string(), "approximate");
        assert_eq!(SupportTier::Fallback.to_string(), "fallback");
        assert_eq!(SupportTier::Unparsed.to_string(), "unparsed");
    }

    #[test]
    fn capability_stage_display_is_stable() {
        assert_eq!(CapabilityStage::Parsed.to_string(), "parsed");
        assert_eq!(CapabilityStage::Resolved.to_string(), "resolved");
        assert_eq!(CapabilityStage::Rendered.to_string(), "rendered");
        assert_eq!(
            CapabilityStage::FidelityTested.to_string(),
            "fidelity-tested"
        );
    }

    #[test]
    fn capability_highest_completed_stage_uses_latest_completed_checkpoint() {
        let mut capability = FeatureCapability::new(
            FeatureFamily::Text,
            "paragraph spacing",
            SupportTier::Approximate,
            Some(CapabilityStage::Resolved),
        )
        .expect("capability should be valid");
        capability.notes = Some("Needs PowerPoint-reference verification".to_string());

        assert_eq!(
            capability.highest_completed_stage(),
            Some(CapabilityStage::Resolved)
        );
    }

    #[test]
    fn capability_rejects_invalid_tier_stage_pairs() {
        let unparsed = FeatureCapability::new(
            FeatureFamily::Charts,
            "chart renderer",
            SupportTier::Unparsed,
            Some(CapabilityStage::Parsed),
        );
        assert!(unparsed.is_err());

        let exact = FeatureCapability::new(
            FeatureFamily::Shapes,
            "preset shape svg",
            SupportTier::Exact,
            Some(CapabilityStage::Rendered),
        );
        assert!(exact.is_err());
    }

    #[test]
    fn feature_family_display_is_stable_for_all_families() {
        assert_eq!(FeatureFamily::Shapes.to_string(), "shapes");
        assert_eq!(FeatureFamily::Text.to_string(), "text");
        assert_eq!(FeatureFamily::Tables.to_string(), "tables");
        assert_eq!(FeatureFamily::Images.to_string(), "images");
        assert_eq!(FeatureFamily::Layout.to_string(), "layout");
        assert_eq!(FeatureFamily::Charts.to_string(), "charts");
        assert_eq!(FeatureFamily::Media.to_string(), "media");
        assert_eq!(FeatureFamily::Unsupported.to_string(), "unsupported");
    }

    #[test]
    fn capability_matrix_and_valid_combinations_cover_remaining_paths() {
        let exact = FeatureCapability::new(
            FeatureFamily::Shapes,
            "preset shape svg",
            SupportTier::Exact,
            Some(CapabilityStage::FidelityTested),
        )
        .expect("exact + fidelity tested should be valid");
        let approximate = FeatureCapability::new(
            FeatureFamily::Layout,
            "placeholder inheritance",
            SupportTier::Approximate,
            Some(CapabilityStage::Resolved),
        )
        .expect("approximate + stage should be valid");
        let fallback = FeatureCapability::new(
            FeatureFamily::Unsupported,
            "smartart placeholder",
            SupportTier::Fallback,
            Some(CapabilityStage::Rendered),
        )
        .expect("fallback + stage should be valid");
        let unparsed = FeatureCapability::new(
            FeatureFamily::Media,
            "embedded video",
            SupportTier::Unparsed,
            None,
        )
        .expect("unparsed without stage should be valid");

        let matrix = super::CapabilityMatrix::new(vec![
            exact.clone(),
            approximate.clone(),
            fallback.clone(),
            unparsed.clone(),
        ]);

        assert_eq!(
            matrix.features,
            vec![exact, approximate, fallback, unparsed]
        );
        assert!(
            FeatureCapability::new(FeatureFamily::Media, "video", SupportTier::Fallback, None)
                .is_err()
        );
    }
}
