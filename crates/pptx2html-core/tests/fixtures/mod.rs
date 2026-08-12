//! Test helper for generating minimal PPTX files

mod package;
mod parts;

pub use package::PackageBuilder;
pub use parts::{FeaturePart, Relationship, SlideXml};

use std::io::{Cursor, Write};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

/// Minimal PPTX builder -- generates various test cases by swapping slide XML
pub struct MinimalPptx {
    slide_xml: String,
    theme_xml: Option<String>,
    master_xml: Option<String>,
    layout_xml: Option<String>,
    layout_rels_xml: Option<String>,
    has_layout_rel: bool,
    presentation_xml: Option<String>,
    presentation_rels_xml: Option<String>,
    custom_theme_xml: Option<String>,
    slide_rels_xml: Option<String>,
    core_properties_xml: Option<String>,
    extra_files: Vec<(String, Vec<u8>)>,
}

impl MinimalPptx {
    pub fn new(slide_body: &str) -> Self {
        Self {
            slide_xml: format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    {slide_body}
  </p:spTree></p:cSld>
</p:sld>"#
            ),
            theme_xml: None,
            master_xml: None,
            layout_xml: None,
            layout_rels_xml: None,
            has_layout_rel: false,
            presentation_xml: None,
            presentation_rels_xml: None,
            custom_theme_xml: None,
            slide_rels_xml: None,
            core_properties_xml: None,
            extra_files: Vec::new(),
        }
    }

    /// Set a custom slide XML (full document, not just body)
    pub fn with_raw_slide(mut self, slide_xml: &str) -> Self {
        self.slide_xml = slide_xml.to_string();
        self
    }

    pub fn with_theme(mut self, color_scheme_body: &str) -> Self {
        self.theme_xml = Some(format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="TestTheme">
  <a:themeElements>
    <a:clrScheme name="TestColors">
      {color_scheme_body}
    </a:clrScheme>
    <a:fontScheme name="TestFonts">
      <a:majorFont><a:latin typeface="Calibri"/></a:majorFont>
      <a:minorFont><a:latin typeface="Calibri"/></a:minorFont>
    </a:fontScheme>
  </a:themeElements>
</a:theme>"#
        ));
        self
    }

    /// Set a full custom theme XML (complete document)
    pub fn with_full_theme(mut self, theme_xml: &str) -> Self {
        self.custom_theme_xml = Some(theme_xml.to_string());
        self
    }

    pub fn with_clr_map(mut self, clr_map_attrs: &str) -> Self {
        self.master_xml = Some(format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap {clr_map_attrs}/>
</p:sldMaster>"#
        ));
        self
    }

    /// Set master XML with shapes and clrMap
    pub fn with_master_shapes(mut self, shapes_xml: &str) -> Self {
        self.master_xml = Some(format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg><p:bgPr><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></p:bgPr></p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      {shapes_xml}
    </p:spTree>
  </p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#
        ));
        self
    }

    /// Set a full custom master XML (complete document)
    pub fn with_full_master(mut self, master_xml: &str) -> Self {
        self.master_xml = Some(master_xml.to_string());
        self
    }

    /// Set layout XML (complete document)
    pub fn with_layout(mut self, layout_xml: &str) -> Self {
        self.layout_xml = Some(layout_xml.to_string());
        self.has_layout_rel = true;
        self
    }

    /// Wire slide -> layout relationships
    pub fn with_slide_layout_rel(mut self) -> Self {
        self.has_layout_rel = true;
        self
    }

    pub fn with_layout_rels(mut self, layout_rels_xml: &str) -> Self {
        self.layout_rels_xml = Some(layout_rels_xml.to_string());
        self
    }

    /// Set custom presentation.xml (complete document)
    pub fn with_presentation_xml(mut self, pres_xml: &str) -> Self {
        self.presentation_xml = Some(pres_xml.to_string());
        self
    }

    pub fn with_presentation_rels(mut self, rels_xml: &str) -> Self {
        self.presentation_rels_xml = Some(rels_xml.to_string());
        self
    }

    pub fn with_slide_rels(mut self, slide_rels_xml: &str) -> Self {
        self.slide_rels_xml = Some(slide_rels_xml.to_string());
        self
    }

    pub fn with_core_properties(mut self, core_xml: &str) -> Self {
        self.core_properties_xml = Some(core_xml.to_string());
        self
    }

    pub fn with_extra_file(mut self, path: &str, data: &[u8]) -> Self {
        self.extra_files.push((path.to_string(), data.to_vec()));
        self
    }

    pub fn build(&self) -> Vec<u8> {
        let buf = Vec::new();
        let cursor = Cursor::new(buf);
        let mut zip = ZipWriter::new(cursor);
        let opts = SimpleFileOptions::default();

        // [Content_Types].xml
        let content_types = if self.layout_xml.is_some() && self.core_properties_xml.is_some() {
            CONTENT_TYPES_WITH_LAYOUT_AND_CORE
        } else if self.layout_xml.is_some() {
            CONTENT_TYPES_WITH_LAYOUT
        } else if self.core_properties_xml.is_some() {
            CONTENT_TYPES_WITH_CORE
        } else {
            CONTENT_TYPES
        };
        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(content_types.as_bytes()).unwrap();

        // _rels/.rels
        zip.start_file("_rels/.rels", opts).unwrap();
        zip.write_all(ROOT_RELS.as_bytes()).unwrap();

        // ppt/presentation.xml
        let pres_xml = self.presentation_xml.as_deref().unwrap_or(PRESENTATION_XML);
        zip.start_file("ppt/presentation.xml", opts).unwrap();
        zip.write_all(pres_xml.as_bytes()).unwrap();

        // ppt/_rels/presentation.xml.rels
        zip.start_file("ppt/_rels/presentation.xml.rels", opts)
            .unwrap();
        let presentation_rels = self
            .presentation_rels_xml
            .as_deref()
            .unwrap_or(PRESENTATION_RELS);
        zip.write_all(presentation_rels.as_bytes()).unwrap();

        // ppt/slides/slide1.xml
        zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
        zip.write_all(self.slide_xml.as_bytes()).unwrap();

        // ppt/slides/_rels/slide1.xml.rels
        let slide_rels = if let Some(ref slide_rels) = self.slide_rels_xml {
            slide_rels.as_str()
        } else if self.has_layout_rel {
            SLIDE_RELS_WITH_LAYOUT
        } else {
            SLIDE_RELS
        };
        zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
            .unwrap();
        zip.write_all(slide_rels.as_bytes()).unwrap();

        // ppt/theme/theme1.xml
        let theme = self
            .custom_theme_xml
            .as_deref()
            .or(self.theme_xml.as_deref())
            .unwrap_or(DEFAULT_THEME);
        zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
        zip.write_all(theme.as_bytes()).unwrap();

        // ppt/slideMasters/slideMaster1.xml (includes ClrMap)
        if let Some(ref master) = self.master_xml {
            zip.start_file("ppt/slideMasters/slideMaster1.xml", opts)
                .unwrap();
            zip.write_all(master.as_bytes()).unwrap();

            // Master rels (references theme and layout)
            let master_rels = if self.layout_xml.is_some() {
                MASTER_RELS_WITH_LAYOUT
            } else {
                MASTER_RELS
            };
            zip.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", opts)
                .unwrap();
            zip.write_all(master_rels.as_bytes()).unwrap();
        }

        // ppt/slideLayouts/slideLayout1.xml
        if let Some(ref layout) = self.layout_xml {
            zip.start_file("ppt/slideLayouts/slideLayout1.xml", opts)
                .unwrap();
            zip.write_all(layout.as_bytes()).unwrap();

            // Layout rels (references master)
            let layout_rels = self.layout_rels_xml.as_deref().unwrap_or(LAYOUT_RELS);
            zip.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", opts)
                .unwrap();
            zip.write_all(layout_rels.as_bytes()).unwrap();
        }

        if let Some(ref core_props) = self.core_properties_xml {
            zip.start_file("docProps/core.xml", opts).unwrap();
            zip.write_all(core_props.as_bytes()).unwrap();
        }

        for (path, data) in &self.extra_files {
            zip.start_file(path, opts).unwrap();
            zip.write_all(data).unwrap();
        }

        zip.finish().unwrap().into_inner()
    }
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

const CONTENT_TYPES_WITH_CORE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>"#;

const CONTENT_TYPES_WITH_LAYOUT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

const CONTENT_TYPES_WITH_LAYOUT_AND_CORE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#;

const PRESENTATION_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#;

const PRESENTATION_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#;

const SLIDE_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
</Relationships>"#;

const SLIDE_RELS_WITH_LAYOUT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#;

const MASTER_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#;

const MASTER_RELS_WITH_LAYOUT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#;

const LAYOUT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#;

const DEFAULT_THEME: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="DefaultTheme">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="Office">
      <a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont>
      <a:minorFont><a:latin typeface="Calibri"/></a:minorFont>
    </a:fontScheme>
  </a:themeElements>
</a:theme>"#;

#[cfg(test)]
mod fixture_api_contract {
    use super::{FeaturePart, MinimalPptx, PackageBuilder, Relationship, SlideXml};

    #[test]
    fn minimal_fixture_custom_xml_options_build_a_package() {
        let package = MinimalPptx::new("")
            .with_raw_slide("<p:sld/>")
            .with_theme("<a:dk1/>")
            .with_full_theme("<a:theme/>")
            .with_clr_map("")
            .with_master_shapes("")
            .with_full_master("<p:sldMaster/>")
            .build();

        assert!(!package.is_empty());
    }

    #[test]
    fn minimal_fixture_relationship_and_extra_file_options_build_a_package() {
        let package = MinimalPptx::new("")
            .with_layout("<p:sldLayout/>")
            .with_slide_layout_rel()
            .with_layout_rels("<Relationships/>")
            .with_presentation_xml("<p:presentation/>")
            .with_presentation_rels("<Relationships/>")
            .with_slide_rels("<Relationships/>")
            .with_core_properties("<cp:coreProperties/>")
            .with_extra_file("ppt/media/fixture.bin", b"fixture")
            .build();

        assert!(!package.is_empty());
    }

    #[test]
    fn feature_fixture_builders_create_and_validate_a_package() {
        let slide = SlideXml::from_body("")
            .with_alternate_content("<mc:AlternateContent/>")
            .build();

        let package = PackageBuilder::new(slide)
            .with_slide_relationship(Relationship::internal(
                "rId1",
                "chart",
                "../charts/chart1.xml",
            ))
            .with_slide_relationship(Relationship::external(
                "rId2",
                "hyperlink",
                "https://example.test",
            ))
            .with_part(FeaturePart::notes("<p:notes/>"))
            .with_part(FeaturePart::comments("<p:cmLst/>"))
            .with_part(FeaturePart::media(
                "fixture.bin",
                "application/octet-stream",
                b"media",
            ))
            .with_part(FeaturePart::chart("<c:chartSpace/>"))
            .with_part(FeaturePart::extra(
                "ppt/embeddings/fixture.bin",
                "application/xml",
                b"chart",
            ));
        package
            .validate()
            .expect("declared feature package is valid");
        assert!(!package.build().expect("feature package builds").is_empty());
        let temporary = package
            .write_to_temp("fixture-api-contract")
            .expect("feature package writes to an isolated path");
        assert!(temporary.path().is_file());

        let error = PackageBuilder::new(SlideXml::from_body("").build())
            .with_slide_relationship(Relationship::internal(
                "rIdMissing",
                "image",
                "../media/missing.png",
            ))
            .validate()
            .expect_err("missing relationship target is rejected");
        assert_eq!(error.code(), "DANGLING_RELATIONSHIP");
        assert_eq!(error.target(), Some("ppt/media/missing.png"));
    }
}
