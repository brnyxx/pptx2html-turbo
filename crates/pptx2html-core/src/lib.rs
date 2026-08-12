//! # pptx2html-core
//!
//! Pure Rust library for converting PPTX presentations to self-contained HTML.
//! Built on the ECMA-376 (OOXML) open standard — no Microsoft dependencies.
//!
//! ## Quick Start
//!
//! ```no_run
//! use std::path::Path;
//!
//! // Convert a file
//! let html = pptx2html_core::convert_file(Path::new("input.pptx")).unwrap();
//! std::fs::write("output.html", &html).unwrap();
//!
//! // Convert from bytes
//! let pptx_data = std::fs::read("input.pptx").unwrap();
//! let html = pptx2html_core::convert_bytes(&pptx_data).unwrap();
//! ```
//!
//! ## Features
//!
//! - High-fidelity layout preservation (absolute positioning in EMU coordinates)
//! - Theme color resolution with 12 color modifiers (tint, shade, lumMod, etc.)
//! - Slide master / layout inheritance chain
//! - 30 preset shape SVG rendering
//! - Table, group shape, and connector support
//! - Image embedding (base64) or external references
//! - Text styling: bold, italic, underline, bullets, vertical text, shadows
//! - Graceful fallback for unsupported content (SmartArt, OLE, Math)

pub mod error;
pub mod model;
pub mod parser;
pub mod renderer;
pub mod resolver;

use std::path::Path;

use error::PptxResult;
use model::{ConversionDiagnostic, UnresolvedElement};
use parser::PptxParser;
use renderer::HtmlRenderer;
pub use renderer::provenance::{ProvenanceSource, ProvenanceSubject, RenderedProvenanceEntry};
pub use renderer::text_metrics::{FontResolutionEntry, FontResolutionSource};

/// Options controlling how PPTX content is converted to HTML.
///
/// Use [`Default::default()`] for sensible defaults (embed images, exclude hidden slides).
///
/// ```
/// use pptx2html_core::ConversionOptions;
///
/// let opts = ConversionOptions {
///     embed_images: false,                 // external image refs
///     slide_indices: Some(vec![1, 3, 5]),  // specific slides only
///     scale: 2.0,                          // whole-slide 2x zoom
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ConversionOptions {
    /// Embed images as base64 data URIs (default: true).
    /// When false, outputs `<img src="images/{name}">` references.
    pub embed_images: bool,
    /// Include hidden slides in output (default: false).
    pub include_hidden: bool,
    /// Render only slides in this 1-based inclusive range (e.g. `(2, 5)` = slides 2..=5).
    pub slide_range: Option<(usize, usize)>,
    /// Render only slides at these 1-based indices (e.g. `vec![1, 3, 5]`).
    pub slide_indices: Option<Vec<usize>>,
    /// Uniform whole-slide scale factor. Keeps all coordinates, sizes, and ratios intact.
    /// `1.0` means original size, `2.0` means 2x zoom.
    pub scale: f64,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            embed_images: true,
            include_hidden: false,
            slide_range: None,
            slide_indices: None,
            scale: 1.0,
        }
    }
}

impl ConversionOptions {
    /// Check whether a slide at the given 1-based index should be included.
    pub fn should_include_slide(&self, one_based_index: usize, hidden: bool) -> bool {
        if hidden && !self.include_hidden {
            return false;
        }
        if let Some(ref indices) = self.slide_indices {
            return indices.contains(&one_based_index);
        }
        if let Some((start, end)) = self.slide_range {
            return one_based_index >= start && one_based_index <= end;
        }
        true
    }

    /// Return a safe scale factor for rendering.
    pub fn effective_scale(&self) -> f64 {
        if self.scale.is_finite() && self.scale > 0.0 {
            self.scale
        } else {
            1.0
        }
    }
}

/// Result of PPTX conversion with metadata about unresolved elements.
///
/// Use this with `convert_*_with_metadata()` functions to get both the HTML
/// and structured information about elements that could not be fully rendered.
#[derive(Debug, Clone)]
pub struct ConversionResult {
    /// Generated HTML string.
    pub html: String,
    pub external_assets: Vec<ExternalAsset>,
    pub font_resolution_entries: Vec<FontResolutionEntry>,
    pub provenance_entries: Vec<RenderedProvenanceEntry>,
    /// Ordered, non-fatal preservation and fallback diagnostics.
    pub diagnostics: Vec<ConversionDiagnostic>,
    /// Metadata about elements that were rendered as placeholders.
    pub unresolved_elements: Vec<UnresolvedElement>,
    /// Number of slides processed.
    pub slide_count: usize,
}

impl ConversionResult {
    /// Creates a conversion result with empty metadata collections.
    pub fn new(html: impl Into<String>, slide_count: usize) -> Self {
        Self {
            html: html.into(),
            external_assets: Vec::new(),
            font_resolution_entries: Vec::new(),
            provenance_entries: Vec::new(),
            diagnostics: Vec::new(),
            unresolved_elements: Vec::new(),
            slide_count,
        }
    }

    /// Returns ordered, non-fatal preservation and fallback diagnostics.
    pub fn diagnostics(&self) -> &[ConversionDiagnostic] {
        &self.diagnostics
    }

    /// Serializes the authoritative diagnostics using the same canonical,
    /// script-safe schema embedded in [`Self::html`] by the core renderer.
    /// Empty diagnostics serialize as `[]`.
    pub fn diagnostics_json(&self) -> String {
        renderer::diagnostics_json(&self.diagnostics)
    }
}

#[derive(Debug, Clone)]
pub struct ExternalAsset {
    pub relative_path: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

/// Convert a PPTX file at `path` to a self-contained HTML string.
///
/// Images are embedded as base64 data URIs by default.
/// Returns [`PptxError`](error::PptxError) on I/O, ZIP, or XML errors.
pub fn convert_file(path: &Path) -> PptxResult<String> {
    Ok(convert_file_with_metadata(path)?.html)
}

/// Convert PPTX byte data to a self-contained HTML string.
///
/// Useful when the PPTX is already loaded in memory (e.g. from a network request).
pub fn convert_bytes(data: &[u8]) -> PptxResult<String> {
    Ok(convert_bytes_with_metadata(data)?.html)
}

/// Convert a PPTX file to HTML with custom [`ConversionOptions`].
pub fn convert_file_with_options(path: &Path, opts: &ConversionOptions) -> PptxResult<String> {
    Ok(convert_file_with_options_metadata(path, opts)?.html)
}

/// Convert PPTX byte data to HTML with custom [`ConversionOptions`].
pub fn convert_bytes_with_options(data: &[u8], opts: &ConversionOptions) -> PptxResult<String> {
    Ok(convert_bytes_with_options_metadata(data, opts)?.html)
}

/// Convert a PPTX file to HTML with metadata about unresolved elements.
pub fn convert_file_with_metadata(path: &Path) -> PptxResult<ConversionResult> {
    convert_file_with_options_metadata(path, &ConversionOptions::default())
}

/// Convert PPTX byte data to HTML with metadata about unresolved elements.
pub fn convert_bytes_with_metadata(data: &[u8]) -> PptxResult<ConversionResult> {
    convert_bytes_with_options_metadata(data, &ConversionOptions::default())
}

/// Convert a PPTX file to HTML with options and metadata about unresolved elements.
pub fn convert_file_with_options_metadata(
    path: &Path,
    opts: &ConversionOptions,
) -> PptxResult<ConversionResult> {
    let data = std::fs::read(path)?;
    convert_bytes_with_options_metadata(&data, opts)
}

/// Convert PPTX byte data to HTML with options and metadata about unresolved elements.
pub fn convert_bytes_with_options_metadata(
    data: &[u8],
    opts: &ConversionOptions,
) -> PptxResult<ConversionResult> {
    let (presentation, notes_comments, timings, extensions) =
        PptxParser::parse_bytes_with_metadata(data)?;
    let mut diagnostics = parser::collect_package_diagnostics(data)?;
    diagnostics.extend(parser::presentation_extension_diagnostics(&extensions));
    HtmlRenderer::render_with_options_diagnostics(
        &presentation,
        opts,
        diagnostics,
        &notes_comments,
        &timings,
    )
}

/// Lightweight presentation metadata (no rendering performed).
#[derive(Debug, Clone)]
pub struct PresentationInfo {
    /// Number of slides in the presentation.
    pub slide_count: usize,
    /// Slide width in CSS pixels (96 DPI).
    pub width_px: f64,
    /// Slide height in CSS pixels (96 DPI).
    pub height_px: f64,
    /// Presentation title from core properties, if present.
    pub title: Option<String>,
}

/// Extract metadata from a PPTX file without rendering.
pub fn get_info(path: &Path) -> PptxResult<PresentationInfo> {
    let presentation = PptxParser::parse_file(path)?;
    Ok(PresentationInfo {
        slide_count: presentation.slides.len(),
        width_px: presentation.slide_size.width.to_px(),
        height_px: presentation.slide_size.height.to_px(),
        title: presentation.title.clone(),
    })
}

/// Extract metadata from in-memory PPTX byte data without rendering.
pub fn get_info_from_bytes(data: &[u8]) -> PptxResult<PresentationInfo> {
    let presentation = PptxParser::parse_bytes(data)?;
    Ok(PresentationInfo {
        slide_count: presentation.slides.len(),
        width_px: presentation.slide_size.width.to_px(),
        height_px: presentation.slide_size.height.to_px(),
        title: presentation.title.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Cursor, Write};

    use tempfile::tempdir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{
        ConversionOptions, convert_bytes, convert_bytes_with_metadata, convert_bytes_with_options,
        convert_bytes_with_options_metadata, convert_file, convert_file_with_metadata,
        convert_file_with_options, get_info, get_info_from_bytes,
    };

    #[test]
    fn conversion_options_should_include_slide_respects_hidden_indices_and_ranges() {
        let default_opts = ConversionOptions::default();
        assert!(default_opts.should_include_slide(1, false));
        assert!(!default_opts.should_include_slide(1, true));

        let indexed = ConversionOptions {
            include_hidden: true,
            slide_indices: Some(vec![1, 3]),
            ..Default::default()
        };
        assert!(indexed.should_include_slide(1, false));
        assert!(!indexed.should_include_slide(2, false));

        let ranged = ConversionOptions {
            slide_range: Some((2, 4)),
            ..Default::default()
        };
        assert!(!ranged.should_include_slide(1, false));
        assert!(ranged.should_include_slide(3, false));
        assert!(!ranged.should_include_slide(5, false));
    }

    #[test]
    fn file_and_bytes_wrappers_convert_generated_fixture_and_report_metadata() {
        let bytes = build_basic_pptx();
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("fixture.pptx");
        fs::write(&path, &bytes).expect("write fixture");

        let html_from_file = convert_file(&path).expect("convert_file should succeed");
        assert!(html_from_file.contains("Coverage Slide"));

        let html_from_bytes = convert_bytes(&bytes).expect("convert_bytes should succeed");
        assert!(html_from_bytes.contains("Coverage Slide"));

        let opts = ConversionOptions {
            slide_indices: Some(vec![1]),
            ..Default::default()
        };
        assert!(
            convert_file_with_options(&path, &opts)
                .expect("convert_file_with_options should succeed")
                .contains("Coverage Slide")
        );
        assert!(
            convert_bytes_with_options(&bytes, &opts)
                .expect("convert_bytes_with_options should succeed")
                .contains("Coverage Slide")
        );

        let file_result =
            convert_file_with_metadata(&path).expect("convert_file_with_metadata should succeed");
        assert_eq!(file_result.slide_count, 1);
        assert_eq!(file_result.unresolved_elements.len(), 0);

        let bytes_result = convert_bytes_with_metadata(&bytes)
            .expect("convert_bytes_with_metadata should succeed");
        assert_eq!(bytes_result.slide_count, 1);
        assert_eq!(bytes_result.unresolved_elements.len(), 0);

        let bytes_opts_result = convert_bytes_with_options_metadata(&bytes, &opts)
            .expect("convert_bytes_with_options_metadata should succeed");
        assert_eq!(bytes_opts_result.slide_count, 1);

        let file_info = get_info(&path).expect("get_info should succeed");
        assert_eq!(file_info.slide_count, 1);
        assert_eq!(file_info.width_px, 960.0);
        assert_eq!(file_info.height_px, 720.0);

        let bytes_info = get_info_from_bytes(&bytes).expect("get_info_from_bytes should succeed");
        assert_eq!(bytes_info.slide_count, 1);
        assert_eq!(bytes_info.width_px, 960.0);
        assert_eq!(bytes_info.height_px, 720.0);
    }

    fn build_basic_pptx() -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(content_types().as_bytes()).unwrap();

        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(root_rels().as_bytes()).unwrap();

        zip.start_file("ppt/presentation.xml", options).unwrap();
        zip.write_all(presentation_xml().as_bytes()).unwrap();

        zip.start_file("ppt/_rels/presentation.xml.rels", options)
            .unwrap();
        zip.write_all(presentation_rels().as_bytes()).unwrap();

        zip.start_file("ppt/slides/slide1.xml", options).unwrap();
        zip.write_all(slide_xml("Coverage Slide").as_bytes())
            .unwrap();

        zip.start_file("ppt/slides/_rels/slide1.xml.rels", options)
            .unwrap();
        zip.write_all(empty_relationships().as_bytes()).unwrap();

        zip.start_file("ppt/theme/theme1.xml", options).unwrap();
        zip.write_all(theme_xml().as_bytes()).unwrap();

        zip.finish().unwrap().into_inner()
    }

    fn content_types() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#
    }

    fn root_rels() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#
    }

    fn presentation_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId1"/>
  </p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#
    }

    fn presentation_rels() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#
    }

    fn slide_xml(text: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="TextBox 1"/>
          <p:cNvSpPr txBox="1"/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
          <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr lang="en-US" sz="1800"/>
              <a:t>{text}</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#
        )
    }

    fn empty_relationships() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#
    }

    fn theme_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="TestTheme">
  <a:themeElements>
    <a:clrScheme name="TestColors">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="1F1F1F"/></a:dk2>
      <a:lt2><a:srgbClr val="F7F7F7"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="TestFonts">
      <a:majorFont><a:latin typeface="Calibri"/></a:majorFont>
      <a:minorFont><a:latin typeface="Calibri"/></a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="TestFmt"/>
  </a:themeElements>
</a:theme>"#
    }
}
