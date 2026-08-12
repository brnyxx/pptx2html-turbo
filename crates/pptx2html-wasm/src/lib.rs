use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Backward-compatible API (v0.5)
// ---------------------------------------------------------------------------

/// Convert PPTX bytes to an HTML string.
#[wasm_bindgen]
pub fn convert(data: &[u8]) -> Result<String, JsError> {
    pptx2html_core::convert_bytes(data).map_err(to_js_error)
}

/// Convert PPTX bytes to HTML with slide filtering (0-based indices).
#[wasm_bindgen]
pub fn convert_slides(data: &[u8], slides: &[usize]) -> Result<String, JsError> {
    let slide_indices = slides
        .iter()
        .copied()
        .map(to_one_based_index)
        .collect::<Result<Vec<_>, _>>()?;
    let opts = pptx2html_core::ConversionOptions {
        slide_indices: Some(slide_indices),
        ..Default::default()
    };
    pptx2html_core::convert_bytes_with_options(data, &opts).map_err(to_js_error)
}

/// Get the number of slides in a PPTX file.
#[wasm_bindgen]
pub fn get_slide_count(data: &[u8]) -> Result<usize, JsError> {
    let info = pptx2html_core::get_info_from_bytes(data).map_err(to_js_error)?;
    Ok(info.slide_count)
}

/// Get presentation metadata as JSON string.
///
/// Kept for backward compatibility. Prefer `get_presentation_info()` for typed access.
#[wasm_bindgen]
pub fn get_info(data: &[u8]) -> Result<String, JsError> {
    let info = pptx2html_core::get_info_from_bytes(data).map_err(to_js_error)?;
    Ok(format!(
        r#"{{"slide_count":{},"width_px":{:.1},"height_px":{:.1},"title":{}}}"#,
        info.slide_count,
        info.width_px,
        info.height_px,
        match &info.title {
            Some(t) => format!("\"{}\"", escape_json_string(t)),
            None => "null".to_string(),
        }
    ))
}

// ---------------------------------------------------------------------------
// Enhanced API (v0.6) — typed return values, full ConversionOptions support
// ---------------------------------------------------------------------------

/// Presentation metadata returned as a structured JS object.
#[wasm_bindgen]
pub struct PresentationInfo {
    slide_count: usize,
    width_px: f64,
    height_px: f64,
    title: Option<String>,
}

#[wasm_bindgen]
impl PresentationInfo {
    #[wasm_bindgen(getter, js_name = "slideCount")]
    pub fn slide_count(&self) -> usize {
        self.slide_count
    }

    #[wasm_bindgen(getter, js_name = "widthPx")]
    pub fn width_px(&self) -> f64 {
        self.width_px
    }

    #[wasm_bindgen(getter, js_name = "heightPx")]
    pub fn height_px(&self) -> f64 {
        self.height_px
    }

    #[wasm_bindgen(getter)]
    pub fn title(&self) -> Option<String> {
        self.title.clone()
    }
}

/// Get presentation metadata as a typed object.
///
/// ```js
/// const info = get_presentation_info(data);
/// console.log(info.slideCount, info.widthPx, info.heightPx, info.title);
/// ```
#[wasm_bindgen]
pub fn get_presentation_info(data: &[u8]) -> Result<PresentationInfo, JsError> {
    let info = pptx2html_core::get_info_from_bytes(data).map_err(to_js_error)?;
    Ok(PresentationInfo {
        slide_count: info.slide_count,
        width_px: info.width_px,
        height_px: info.height_px,
        title: info.title,
    })
}

/// Conversion result with HTML and metadata about unresolved elements.
#[wasm_bindgen]
pub struct ConversionResult {
    html: String,
    diagnostics_json: String,
    unresolved_json: String,
    slide_count: usize,
}

#[wasm_bindgen]
impl ConversionResult {
    /// The generated HTML string.
    #[wasm_bindgen(getter)]
    pub fn html(&self) -> String {
        self.html.clone()
    }

    /// Canonical core conversion diagnostics JSON.
    #[wasm_bindgen(getter, js_name = "diagnosticsJson")]
    pub fn diagnostics_json(&self) -> String {
        self.diagnostics_json.clone()
    }

    /// Canonical core conversion diagnostics JSON (compatibly named projection).
    #[wasm_bindgen(getter)]
    pub fn diagnostics(&self) -> String {
        self.diagnostics_json.clone()
    }

    /// JSON array of unresolved elements (SmartArt, OLE, Math).
    ///
    /// Each element: `{ slideIndex, elementType, placeholderId, rawXml?, dataModel? }`
    #[wasm_bindgen(getter, js_name = "unresolvedElements")]
    pub fn unresolved_elements(&self) -> String {
        self.unresolved_json.clone()
    }

    /// Number of slides processed.
    #[wasm_bindgen(getter, js_name = "slideCount")]
    pub fn slide_count(&self) -> usize {
        self.slide_count
    }
}

/// Convert PPTX with explicit options.
///
/// ```js
/// const html = convert_with_options(
///   data,
///   false,
///   true,
///   new Uint32Array([1, 3, 5]),
///   2.0,
/// );
/// ```
///
/// `slide_indices` uses 1-based indexing. Pass an empty array to include all slides.
#[wasm_bindgen]
pub fn convert_with_options(
    data: &[u8],
    embed_images: bool,
    include_hidden: bool,
    slide_indices: &[usize],
    scale: f64,
) -> Result<String, JsError> {
    let opts = pptx2html_core::ConversionOptions {
        embed_images,
        include_hidden,
        slide_indices: optional_slide_indices(slide_indices),
        scale,
        ..Default::default()
    };
    pptx2html_core::convert_bytes_with_options(data, &opts).map_err(to_js_error)
}

/// Convert PPTX and return both HTML and metadata about unresolved elements.
///
/// ```js
/// const result = convert_with_metadata(data);
/// document.body.innerHTML = result.html;
/// const unresolved = JSON.parse(result.unresolvedElements);
/// ```
#[wasm_bindgen]
pub fn convert_with_metadata(data: &[u8]) -> Result<ConversionResult, JsError> {
    let result = pptx2html_core::convert_bytes_with_metadata(data).map_err(to_js_error)?;
    let diagnostics_json = result.diagnostics_json();
    Ok(ConversionResult {
        html: result.html,
        diagnostics_json,
        unresolved_json: serialize_unresolved(&result.unresolved_elements),
        slide_count: result.slide_count,
    })
}

/// Convert PPTX with options and return both HTML and metadata.
///
/// ```js
/// const result = convert_with_options_metadata(
///   data,
///   false,
///   true,
///   new Uint32Array([1, 3, 5]),
///   2.0,
/// );
/// ```
#[wasm_bindgen]
pub fn convert_with_options_metadata(
    data: &[u8],
    embed_images: bool,
    include_hidden: bool,
    slide_indices: &[usize],
    scale: f64,
) -> Result<ConversionResult, JsError> {
    let opts = pptx2html_core::ConversionOptions {
        embed_images,
        include_hidden,
        slide_indices: optional_slide_indices(slide_indices),
        scale,
        ..Default::default()
    };
    let result =
        pptx2html_core::convert_bytes_with_options_metadata(data, &opts).map_err(to_js_error)?;
    let diagnostics_json = result.diagnostics_json();
    Ok(ConversionResult {
        html: result.html,
        diagnostics_json,
        unresolved_json: serialize_unresolved(&result.unresolved_elements),
        slide_count: result.slide_count,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn to_js_error(error: impl std::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

fn to_one_based_index(index: usize) -> Result<usize, JsError> {
    index
        .checked_add(1)
        .ok_or_else(|| JsError::new("slide index overflow"))
}

fn optional_slide_indices(slide_indices: &[usize]) -> Option<Vec<usize>> {
    (!slide_indices.is_empty()).then(|| slide_indices.to_vec())
}

fn serialize_unresolved(elements: &[pptx2html_core::model::UnresolvedElement]) -> String {
    let mut json = String::from("[");
    for (i, elem) in elements.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        let element_type = match elem.element_type {
            pptx2html_core::model::UnresolvedType::SmartArt => "SmartArt",
            pptx2html_core::model::UnresolvedType::OleObject => "OleObject",
            pptx2html_core::model::UnresolvedType::MathEquation => "MathEquation",
            pptx2html_core::model::UnresolvedType::CustomGeometry => "CustomGeometry",
        };
        let raw_xml = match &elem.raw_xml {
            Some(xml) => format!("\"{}\"", escape_json_string(xml)),
            None => "null".to_string(),
        };
        let data_model = match &elem.data_model {
            Some(dm) => format!("\"{}\"", escape_json_string(dm)),
            None => "null".to_string(),
        };
        json.push_str(&format!(
            r#"{{"slideIndex":{},"elementType":"{}","placeholderId":"{}","rawXml":{},"dataModel":{}}}"#,
            elem.slide_index,
            element_type,
            escape_json_string(&elem.placeholder_id),
            raw_xml,
            data_model,
        ));
    }
    json.push(']');
    json
}

fn escape_json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0C}' => escaped.push_str("\\f"),
            control if control < '\u{20}' => {
                escaped.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use pptx2html_core::model::{UnresolvedElement, UnresolvedType};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{
        ConversionResult, PresentationInfo, convert, convert_slides, convert_with_metadata,
        convert_with_options, convert_with_options_metadata, escape_json_string, get_info,
        get_presentation_info, get_slide_count, optional_slide_indices, serialize_unresolved,
    };

    #[test]
    fn convert_slides_uses_zero_based_indices() {
        let data = build_two_slide_pptx();

        let html = convert_slides(&data, &[0]).expect("convert_slides should succeed");

        assert!(
            html.contains("Slide One"),
            "expected first slide text in HTML"
        );
        assert!(
            !html.contains("Slide Two"),
            "expected second slide to be filtered out"
        );
    }

    #[test]
    fn convert_with_options_keeps_one_based_indices() {
        let data = build_two_slide_pptx();

        let html = convert_with_options(&data, true, false, &[1], 1.0)
            .expect("convert_with_options works");

        assert!(
            html.contains("Slide One"),
            "expected first slide text in HTML"
        );
        assert!(
            !html.contains("Slide Two"),
            "expected second slide to be filtered out"
        );
    }

    #[test]
    fn serialize_unresolved_escapes_placeholder_id_control_characters() {
        let json = serialize_unresolved(&[
            UnresolvedElement {
                slide_index: 0,
                element_type: UnresolvedType::SmartArt,
                placeholder_id: "placeholder\n\t\u{8}\"\\id".to_string(),
                position: None,
                size: None,
                raw_xml: Some("<node>line\nvalue</node>".to_string()),
                data_model: Some("{\"line\":\"value\r\"}".to_string()),
            },
            UnresolvedElement {
                slide_index: 1,
                element_type: UnresolvedType::OleObject,
                placeholder_id: "ole\u{01}".to_string(),
                position: None,
                size: None,
                raw_xml: None,
                data_model: None,
            },
            UnresolvedElement {
                slide_index: 2,
                element_type: UnresolvedType::MathEquation,
                placeholder_id: "math".to_string(),
                position: None,
                size: None,
                raw_xml: None,
                data_model: None,
            },
            UnresolvedElement {
                slide_index: 3,
                element_type: UnresolvedType::CustomGeometry,
                placeholder_id: "custom".to_string(),
                position: None,
                size: None,
                raw_xml: None,
                data_model: None,
            },
        ]);

        assert!(json.contains("placeholder\\n\\t\\b\\\"\\\\id"));
        assert!(!json.contains("placeholder\n\t\u{8}\"\\id"));
        assert!(json.contains("<node>line\\nvalue</node>"));
        assert!(json.contains("{\\\"line\\\":\\\"value\\r\\\"}"));
        assert!(json.contains("\"elementType\":\"OleObject\""));
        assert!(json.contains("\"elementType\":\"MathEquation\""));
        assert!(json.contains("\"elementType\":\"CustomGeometry\""));
        assert!(json.contains("ole\\u0001"));
        assert!(json.contains("\"rawXml\":null"));
    }

    #[test]
    fn wasm_public_apis_cover_metadata_and_error_paths() {
        let data = build_two_slide_pptx();

        let html = convert(&data).expect("convert should succeed");
        assert!(html.contains("Slide One"));
        assert!(html.contains("Slide Two"));

        assert_eq!(get_slide_count(&data).expect("slide count"), 2);

        let info_json = get_info(&data).expect("get_info should succeed");
        assert!(info_json.contains("\"slide_count\":2"));
        assert!(info_json.contains("\"title\":null"));

        let info = get_presentation_info(&data).expect("typed info should succeed");
        assert_eq!(info.slide_count(), 2);
        assert_eq!(info.width_px(), 960.0);
        assert_eq!(info.height_px(), 720.0);
        assert_eq!(info.title(), None);

        let metadata = convert_with_metadata(&data).expect("metadata conversion should succeed");
        assert!(metadata.html().contains("Slide One"));
        assert_eq!(metadata.slide_count(), 2);
        assert_eq!(metadata.diagnostics_json(), "[]");
        assert_eq!(metadata.diagnostics(), "[]");
        assert_eq!(metadata.unresolved_elements(), "[]");

        let filtered = convert_with_options_metadata(&data, true, false, &[2], 1.0)
            .expect("filtered conversion should succeed");
        assert!(filtered.html().contains("Slide Two"));
        assert!(!filtered.html().contains("Slide One"));
        assert_eq!(filtered.slide_count(), 1);
        assert_eq!(filtered.unresolved_elements(), "[]");
    }

    #[test]
    fn wasm_public_apis_cover_title_and_empty_selection_paths() {
        let data = build_titled_single_slide_pptx("Quarterly \"Deck\"");

        let info_json = get_info(&data).expect("titled info should succeed");
        assert!(info_json.contains("\"title\":\"Quarterly \\\"Deck\\\"\""));

        let info = get_presentation_info(&data).expect("typed titled info should succeed");
        assert_eq!(info.title(), Some("Quarterly \"Deck\"".to_string()));

        let html = convert_with_options(&data, true, false, &[], 1.0)
            .expect("empty selection keeps slides");
        assert!(html.contains("Slide One"));

        let metadata = convert_with_options_metadata(&data, true, false, &[], 1.0)
            .expect("empty selection metadata keeps slides");
        assert!(metadata.html().contains("Slide One"));
        assert_eq!(metadata.slide_count(), 1);

        assert_eq!(optional_slide_indices(&[]), None);
        assert_eq!(optional_slide_indices(&[1, 3]), Some(vec![1, 3]));
    }

    #[test]
    fn wasm_struct_getters_and_escape_helper_cover_remaining_paths() {
        let info = PresentationInfo {
            slide_count: 3,
            width_px: 1024.0,
            height_px: 768.0,
            title: Some("Deck".to_string()),
        };
        assert_eq!(info.slide_count(), 3);
        assert_eq!(info.width_px(), 1024.0);
        assert_eq!(info.height_px(), 768.0);
        assert_eq!(info.title(), Some("Deck".to_string()));

        let result = ConversionResult {
            html: "<html/>".to_string(),
            diagnostics_json: "[]".to_string(),
            unresolved_json: "[{\"slideIndex\":1}]".to_string(),
            slide_count: 1,
        };
        assert_eq!(result.html(), "<html/>");
        assert_eq!(result.diagnostics_json(), "[]");
        assert_eq!(result.diagnostics(), "[]");
        assert_eq!(result.unresolved_elements(), "[{\"slideIndex\":1}]");
        assert_eq!(result.slide_count(), 1);

        assert_eq!(
            escape_json_string("line\nquote\"slash\\tab\tbackspace\u{08}formfeed\u{0C}"),
            "line\\nquote\\\"slash\\\\tab\\tbackspace\\bformfeed\\f"
        );
    }

    fn build_two_slide_pptx() -> Vec<u8> {
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
        zip.write_all(slide_xml("Slide One").as_bytes()).unwrap();

        zip.start_file("ppt/slides/slide2.xml", options).unwrap();
        zip.write_all(slide_xml("Slide Two").as_bytes()).unwrap();

        zip.start_file("ppt/slides/_rels/slide1.xml.rels", options)
            .unwrap();
        zip.write_all(empty_relationships().as_bytes()).unwrap();

        zip.start_file("ppt/slides/_rels/slide2.xml.rels", options)
            .unwrap();
        zip.write_all(empty_relationships().as_bytes()).unwrap();

        zip.start_file("ppt/theme/theme1.xml", options).unwrap();
        zip.write_all(theme_xml().as_bytes()).unwrap();

        zip.finish().unwrap().into_inner()
    }

    fn build_titled_single_slide_pptx(title: &str) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>"#,
        )
        .unwrap();

        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(root_rels().as_bytes()).unwrap();

        zip.start_file("ppt/presentation.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst/>
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId1"/>
  </p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
        )
        .unwrap();

        zip.start_file("ppt/_rels/presentation.xml.rels", options)
            .unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#,
        )
        .unwrap();

        zip.start_file("ppt/slides/slide1.xml", options).unwrap();
        zip.write_all(slide_xml("Slide One").as_bytes()).unwrap();

        zip.start_file("ppt/slides/_rels/slide1.xml.rels", options)
            .unwrap();
        zip.write_all(empty_relationships().as_bytes()).unwrap();

        zip.start_file("ppt/theme/theme1.xml", options).unwrap();
        zip.write_all(theme_xml().as_bytes()).unwrap();

        zip.start_file("docProps/core.xml", options).unwrap();
        zip.write_all(
            format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
  <dc:title>{title}</dc:title>
</cp:coreProperties>"#
            )
            .as_bytes(),
        )
        .unwrap();

        zip.finish().unwrap().into_inner()
    }

    fn content_types() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/slides/slide2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
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
  <p:sldMasterIdLst/>
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId1"/>
    <p:sldId id="257" r:id="rId2"/>
  </p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#
    }

    fn presentation_rels() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
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
