use std::path::Path;

use pyo3::prelude::*;

use pptx2html_core::ConversionOptions;
use pptx2html_core::model::slide::UnresolvedType;
use pptx2html_core::model::{ConversionDiagnostic, DiagnosticLocation};

/// Convert a PPTX file to HTML string
#[pyfunction]
fn convert_file(path: &str) -> PyResult<String> {
    pptx2html_core::convert_file(Path::new(path))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

/// Convert PPTX bytes to HTML string
#[pyfunction]
fn convert_bytes(data: &[u8]) -> PyResult<String> {
    pptx2html_core::convert_bytes(data)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

/// Convert a PPTX file to HTML with options
///
/// Args:
///     path: Path to the PPTX file
///     embed_images: Embed images as base64 data URIs (default: True)
///     include_hidden: Include hidden slides (default: False)
///     slides: List of 1-based slide indices to include (default: all)
///     scale: Whole-slide zoom factor (default: 1.0)
#[pyfunction]
#[pyo3(signature = (path, *, embed_images=true, include_hidden=false, slides=None, scale=1.0))]
fn convert(
    path: &str,
    embed_images: bool,
    include_hidden: bool,
    slides: Option<Vec<usize>>,
    scale: f64,
) -> PyResult<String> {
    let opts = ConversionOptions {
        embed_images,
        include_hidden,
        slide_range: None,
        slide_indices: slides,
        scale,
    };
    pptx2html_core::convert_file_with_options(Path::new(path), &opts)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

/// Convert a PPTX file to HTML with metadata about unresolved elements
///
/// Args:
///     path: Path to the PPTX file
///     embed_images: Embed images as base64 data URIs (default: True)
///     include_hidden: Include hidden slides (default: False)
///     slides: List of 1-based slide indices to include (default: all)
///     scale: Whole-slide zoom factor (default: 1.0)
///
/// Returns:
///     ConversionResult with html, unresolved_elements, and slide_count
#[pyfunction]
#[pyo3(signature = (path, *, embed_images=true, include_hidden=false, slides=None, scale=1.0))]
fn convert_with_metadata(
    path: &str,
    embed_images: bool,
    include_hidden: bool,
    slides: Option<Vec<usize>>,
    scale: f64,
) -> PyResult<PyConversionResult> {
    let opts = ConversionOptions {
        embed_images,
        include_hidden,
        slide_range: None,
        slide_indices: slides,
        scale,
    };
    let result = pptx2html_core::convert_file_with_options_metadata(Path::new(path), &opts)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    let diagnostics_json = result.diagnostics_json();

    Ok(PyConversionResult {
        html: result.html,
        diagnostics: map_diagnostics(result.diagnostics),
        diagnostics_json,
        unresolved_elements: map_unresolved_elements(result.unresolved_elements),
        slide_count: result.slide_count,
    })
}

/// Convert PPTX bytes to HTML with metadata about unresolved elements
///
/// Args:
///     data: PPTX file bytes
///     embed_images: Embed images as base64 data URIs (default: True)
///     include_hidden: Include hidden slides (default: False)
///     slides: List of 1-based slide indices to include (default: all)
///     scale: Whole-slide zoom factor (default: 1.0)
///
/// Returns:
///     ConversionResult with html, unresolved_elements, and slide_count
#[pyfunction]
#[pyo3(signature = (data, *, embed_images=true, include_hidden=false, slides=None, scale=1.0))]
fn convert_bytes_with_metadata(
    data: &[u8],
    embed_images: bool,
    include_hidden: bool,
    slides: Option<Vec<usize>>,
    scale: f64,
) -> PyResult<PyConversionResult> {
    let opts = ConversionOptions {
        embed_images,
        include_hidden,
        slide_range: None,
        slide_indices: slides,
        scale,
    };
    let result = pptx2html_core::convert_bytes_with_options_metadata(data, &opts)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    let diagnostics_json = result.diagnostics_json();

    Ok(PyConversionResult {
        html: result.html,
        diagnostics: map_diagnostics(result.diagnostics),
        diagnostics_json,
        unresolved_elements: map_unresolved_elements(result.unresolved_elements),
        slide_count: result.slide_count,
    })
}

fn map_diagnostics(diagnostics: Vec<ConversionDiagnostic>) -> Vec<PyConversionDiagnostic> {
    diagnostics.into_iter().map(map_diagnostic).collect()
}

fn map_diagnostic(diagnostic: ConversionDiagnostic) -> PyConversionDiagnostic {
    PyConversionDiagnostic {
        code: diagnostic.code,
        family: diagnostic.family.as_str().to_owned(),
        support_tier: diagnostic.support_tier.as_str().to_owned(),
        stage: diagnostic.stage.map(|stage| stage.as_str().to_owned()),
        location: map_diagnostic_location(diagnostic.location),
        raw_reference: diagnostic.raw_reference,
        fallback_kind: diagnostic.fallback_kind.as_str().to_owned(),
        reason: diagnostic.reason,
    }
}

fn map_diagnostic_location(location: DiagnosticLocation) -> PyDiagnosticLocation {
    PyDiagnosticLocation {
        slide_index: location.slide_index,
        part_name: location.part_name,
        relationship_id: location.relationship_id,
        relationship_type: location.relationship_type,
        qualified_element_name: location.qualified_element_name,
        position: location.position.map(|position| PyDiagnosticPosition {
            x: position.x.0,
            y: position.y.0,
        }),
        size: location.size.map(|size| PyDiagnosticSize {
            width: size.width.0,
            height: size.height.0,
        }),
    }
}

fn map_unresolved_elements(
    elements: Vec<pptx2html_core::model::slide::UnresolvedElement>,
) -> Vec<PyUnresolvedElement> {
    elements.into_iter().map(map_unresolved_element).collect()
}

fn map_unresolved_element(
    element: pptx2html_core::model::slide::UnresolvedElement,
) -> PyUnresolvedElement {
    PyUnresolvedElement {
        slide_index: element.slide_index,
        element_type: unresolved_type_name(&element.element_type).to_string(),
        placeholder_id: element.placeholder_id,
        raw_xml: element.raw_xml,
        data_model: element.data_model,
    }
}

fn unresolved_type_name(element_type: &UnresolvedType) -> &'static str {
    match element_type {
        UnresolvedType::SmartArt => "smartart",
        UnresolvedType::OleObject => "ole",
        UnresolvedType::MathEquation => "math",
        UnresolvedType::CustomGeometry => "custom-geometry",
    }
}

/// Get presentation metadata (slide count, size, title)
#[pyfunction]
fn get_info(path: &str) -> PyResult<PresentationInfo> {
    let info = pptx2html_core::get_info(Path::new(path))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    Ok(PresentationInfo {
        slide_count: info.slide_count,
        width_px: info.width_px,
        height_px: info.height_px,
        title: info.title,
    })
}

/// Presentation metadata
#[pyclass]
#[derive(Debug, Clone)]
struct PresentationInfo {
    #[pyo3(get)]
    slide_count: usize,
    #[pyo3(get)]
    width_px: f64,
    #[pyo3(get)]
    height_px: f64,
    #[pyo3(get)]
    title: Option<String>,
}

#[pymethods]
impl PresentationInfo {
    fn __repr__(&self) -> String {
        format!(
            "PresentationInfo(slide_count={}, width_px={:.1}, height_px={:.1}, title={:?})",
            self.slide_count, self.width_px, self.height_px, self.title
        )
    }
}

/// Result of PPTX conversion with metadata
#[pyclass(name = "ConversionResult")]
#[derive(Debug, Clone)]
struct PyConversionResult {
    #[pyo3(get)]
    html: String,
    #[pyo3(get)]
    diagnostics: Vec<PyConversionDiagnostic>,
    #[pyo3(get)]
    diagnostics_json: String,
    #[pyo3(get)]
    unresolved_elements: Vec<PyUnresolvedElement>,
    #[pyo3(get)]
    slide_count: usize,
}

#[pymethods]
impl PyConversionResult {
    fn __repr__(&self) -> String {
        format!(
            "ConversionResult(slide_count={}, diagnostics={}, unresolved_elements={})",
            self.slide_count,
            self.diagnostics.len(),
            self.unresolved_elements.len()
        )
    }
}

/// Position of a conversion diagnostic in integer OOXML EMUs.
#[pyclass(name = "DiagnosticPosition", frozen)]
#[derive(Debug, Clone)]
struct PyDiagnosticPosition {
    #[pyo3(get)]
    x: i64,
    #[pyo3(get)]
    y: i64,
}

/// Size of a conversion diagnostic in integer OOXML EMUs.
#[pyclass(name = "DiagnosticSize", frozen)]
#[derive(Debug, Clone)]
struct PyDiagnosticSize {
    #[pyo3(get)]
    width: i64,
    #[pyo3(get)]
    height: i64,
}

/// Typed source location for a conversion diagnostic.
#[pyclass(name = "DiagnosticLocation", frozen)]
#[derive(Debug, Clone)]
struct PyDiagnosticLocation {
    #[pyo3(get)]
    slide_index: Option<usize>,
    #[pyo3(get)]
    part_name: Option<String>,
    #[pyo3(get)]
    relationship_id: Option<String>,
    #[pyo3(get)]
    relationship_type: Option<String>,
    #[pyo3(get)]
    qualified_element_name: Option<String>,
    #[pyo3(get)]
    position: Option<PyDiagnosticPosition>,
    #[pyo3(get)]
    size: Option<PyDiagnosticSize>,
}

/// Typed, read-only conversion diagnostic.
#[pyclass(name = "ConversionDiagnostic", frozen)]
#[derive(Debug, Clone)]
struct PyConversionDiagnostic {
    #[pyo3(get)]
    code: String,
    #[pyo3(get)]
    family: String,
    #[pyo3(get)]
    support_tier: String,
    #[pyo3(get)]
    stage: Option<String>,
    #[pyo3(get)]
    location: PyDiagnosticLocation,
    #[pyo3(get)]
    raw_reference: Option<String>,
    #[pyo3(get)]
    fallback_kind: String,
    #[pyo3(get)]
    reason: String,
}

/// Metadata about an unresolved element
#[pyclass(name = "UnresolvedElement")]
#[derive(Debug, Clone)]
struct PyUnresolvedElement {
    #[pyo3(get)]
    slide_index: usize,
    #[pyo3(get)]
    element_type: String,
    #[pyo3(get)]
    placeholder_id: String,
    #[pyo3(get)]
    raw_xml: Option<String>,
    #[pyo3(get)]
    data_model: Option<String>,
}

#[pymethods]
impl PyUnresolvedElement {
    fn __repr__(&self) -> String {
        format!(
            "UnresolvedElement(slide={}, type='{}', id='{}')",
            self.slide_index, self.element_type, self.placeholder_id
        )
    }
}

/// Convert PPTX to high-fidelity HTML
#[pymodule]
fn pptx2html(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(convert_file, m)?)?;
    m.add_function(wrap_pyfunction!(convert_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add_function(wrap_pyfunction!(convert_with_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(convert_bytes_with_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(get_info, m)?)?;
    m.add_class::<PresentationInfo>()?;
    m.add_class::<PyConversionResult>()?;
    m.add_class::<PyConversionDiagnostic>()?;
    m.add_class::<PyDiagnosticLocation>()?;
    m.add_class::<PyDiagnosticPosition>()?;
    m.add_class::<PyDiagnosticSize>()?;
    m.add_class::<PyUnresolvedElement>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Cursor, Write};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use pyo3::Python;
    use pyo3::types::{PyAnyMethods, PyModule};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{
        PresentationInfo, PyConversionResult, PyUnresolvedElement, convert, convert_bytes,
        convert_bytes_with_metadata, convert_file, convert_with_metadata, get_info,
        map_unresolved_elements, pptx2html,
    };
    use pptx2html_core::model::slide::{UnresolvedElement, UnresolvedType};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn convert_uses_one_based_slide_indices() {
        let path = write_temp_pptx(build_two_slide_pptx());

        let html = convert(path.to_str().unwrap(), true, false, Some(vec![1]), 1.0)
            .expect("convert should succeed");

        assert!(
            html.contains("Slide One"),
            "expected first slide text in HTML"
        );
        assert!(
            !html.contains("Slide Two"),
            "expected second slide to be filtered out"
        );

        fs::remove_file(path).expect("remove temp pptx");
    }

    #[test]
    fn get_info_reports_slide_count_for_generated_fixture() {
        let path = write_temp_pptx(build_two_slide_pptx());

        let info = get_info(path.to_str().unwrap()).expect("get_info should succeed");

        assert_eq!(info.slide_count, 2);
        assert_eq!(info.title, None);

        fs::remove_file(path).expect("remove temp pptx");
    }

    #[test]
    fn convert_with_metadata_reports_no_unresolved_elements_for_basic_fixture() {
        let path = write_temp_pptx(build_two_slide_pptx());

        let result = convert_with_metadata(path.to_str().unwrap(), true, false, Some(vec![1]), 1.0)
            .expect("convert_with_metadata should succeed");

        assert_eq!(result.slide_count, 1);
        assert!(result.html.contains("Slide One"));
        assert_eq!(result.unresolved_elements.len(), 0);

        fs::remove_file(path).expect("remove temp pptx");
    }

    #[test]
    fn convert_bytes_with_metadata_respects_one_based_slide_filtering() {
        let bytes = build_two_slide_pptx();

        let result = convert_bytes_with_metadata(&bytes, true, false, Some(vec![2]), 1.0)
            .expect("convert_bytes_with_metadata should succeed");

        assert_eq!(result.slide_count, 1);
        assert!(result.html.contains("Slide Two"));
        assert!(!result.html.contains("Slide One"));
        assert_eq!(result.unresolved_elements.len(), 0);
    }

    #[test]
    fn convert_file_and_convert_bytes_cover_basic_public_apis() {
        let bytes = build_two_slide_pptx();
        let path = write_temp_pptx(bytes.clone());

        let file_html = convert_file(path.to_str().unwrap()).expect("convert_file should succeed");
        let bytes_html = convert_bytes(&bytes).expect("convert_bytes should succeed");

        assert!(file_html.contains("Slide One"));
        assert!(file_html.contains("Slide Two"));
        assert!(bytes_html.contains("Slide One"));
        assert!(bytes_html.contains("Slide Two"));

        fs::remove_file(path).expect("remove temp pptx");
    }

    #[test]
    fn python_bindings_map_failures_to_runtime_errors() {
        let missing_file_err = convert_file("/definitely/missing.pptx")
            .expect_err("missing file should map to PyRuntimeError");
        let invalid_bytes_err =
            convert_bytes(b"not-a-pptx").expect_err("invalid bytes should map to PyRuntimeError");

        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            assert!(missing_file_err.is_instance_of::<pyo3::exceptions::PyRuntimeError>(py));
            assert!(invalid_bytes_err.is_instance_of::<pyo3::exceptions::PyRuntimeError>(py));
        });
    }

    #[test]
    fn py_classes_repr_and_module_registration_work() {
        let info = PresentationInfo {
            slide_count: 2,
            width_px: 960.0,
            height_px: 540.0,
            title: Some("Deck".to_string()),
        };
        assert_eq!(
            info.__repr__(),
            "PresentationInfo(slide_count=2, width_px=960.0, height_px=540.0, title=Some(\"Deck\"))"
        );

        let unresolved = PyUnresolvedElement {
            slide_index: 1,
            element_type: "smartart".to_string(),
            placeholder_id: "ph-1".to_string(),
            raw_xml: Some("<dgm/>".to_string()),
            data_model: Some("{\"kind\":\"smartart\"}".to_string()),
        };
        assert_eq!(
            unresolved.__repr__(),
            "UnresolvedElement(slide=1, type='smartart', id='ph-1')"
        );

        let result = PyConversionResult {
            html: "<html/>".to_string(),
            diagnostics: Vec::new(),
            diagnostics_json: "[]".to_string(),
            unresolved_elements: vec![PyUnresolvedElement {
                slide_index: 0,
                element_type: "math".to_string(),
                placeholder_id: "ph-2".to_string(),
                raw_xml: None,
                data_model: None,
            }],
            slide_count: 1,
        };
        assert_eq!(
            result.__repr__(),
            "ConversionResult(slide_count=1, diagnostics=0, unresolved_elements=1)"
        );

        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let module = PyModule::new(py, "pptx2html").expect("create module");
            pptx2html(&module).expect("register module contents");
            let py_info = pyo3::Py::new(py, info.clone()).expect("create PresentationInfo");
            let py_result = pyo3::Py::new(py, result.clone()).expect("create ConversionResult");
            let py_unresolved =
                pyo3::Py::new(py, unresolved.clone()).expect("create UnresolvedElement");

            assert!(module.getattr("convert_file").is_ok());
            assert!(module.getattr("convert_bytes").is_ok());
            assert!(module.getattr("convert").is_ok());
            assert!(module.getattr("convert_with_metadata").is_ok());
            assert!(module.getattr("convert_bytes_with_metadata").is_ok());
            assert!(module.getattr("get_info").is_ok());
            assert!(module.getattr("PresentationInfo").is_ok());
            assert!(module.getattr("ConversionResult").is_ok());
            assert!(module.getattr("ConversionDiagnostic").is_ok());
            assert!(module.getattr("DiagnosticLocation").is_ok());
            assert!(module.getattr("DiagnosticPosition").is_ok());
            assert!(module.getattr("DiagnosticSize").is_ok());
            assert!(module.getattr("UnresolvedElement").is_ok());

            assert_eq!(
                py_info.bind(py).repr().expect("repr").to_string(),
                "PresentationInfo(slide_count=2, width_px=960.0, height_px=540.0, title=Some(\"Deck\"))"
            );
            assert_eq!(
                py_info
                    .bind(py)
                    .getattr("slide_count")
                    .expect("slide_count getter")
                    .extract::<usize>()
                    .expect("slide_count value"),
                2
            );
            assert_eq!(
                py_result.bind(py).repr().expect("repr").to_string(),
                "ConversionResult(slide_count=1, diagnostics=0, unresolved_elements=1)"
            );
            assert_eq!(
                py_result
                    .bind(py)
                    .getattr("slide_count")
                    .expect("slide_count getter")
                    .extract::<usize>()
                    .expect("slide_count value"),
                1
            );
            assert_eq!(
                py_unresolved.bind(py).repr().expect("repr").to_string(),
                "UnresolvedElement(slide=1, type='smartart', id='ph-1')"
            );
            assert_eq!(
                py_unresolved
                    .bind(py)
                    .getattr("element_type")
                    .expect("element_type getter")
                    .extract::<String>()
                    .expect("element_type value"),
                "smartart"
            );
        });
        let mapped = map_unresolved_elements(vec![
            UnresolvedElement {
                slide_index: 0,
                element_type: UnresolvedType::SmartArt,
                placeholder_id: "smartart".to_string(),
                position: None,
                size: None,
                raw_xml: Some("<dgm/>".to_string()),
                data_model: Some("{\"kind\":\"smartart\"}".to_string()),
            },
            UnresolvedElement {
                slide_index: 1,
                element_type: UnresolvedType::OleObject,
                placeholder_id: "ole".to_string(),
                position: None,
                size: None,
                raw_xml: Some("<oleObj/>".to_string()),
                data_model: Some("{\"kind\":\"ole\"}".to_string()),
            },
            UnresolvedElement {
                slide_index: 2,
                element_type: UnresolvedType::MathEquation,
                placeholder_id: "math".to_string(),
                position: None,
                size: None,
                raw_xml: Some("<m:oMath/>".to_string()),
                data_model: Some("{\"kind\":\"math\"}".to_string()),
            },
            UnresolvedElement {
                slide_index: 3,
                element_type: UnresolvedType::CustomGeometry,
                placeholder_id: "custgeom".to_string(),
                position: None,
                size: None,
                raw_xml: Some("<a:custGeom/>".to_string()),
                data_model: Some("{\"kind\":\"custom-geometry\"}".to_string()),
            },
        ]);
        assert_eq!(
            mapped
                .iter()
                .map(|item| item.element_type.as_str())
                .collect::<Vec<_>>(),
            vec!["smartart", "ole", "math", "custom-geometry"]
        );
        assert_eq!(mapped[3].raw_xml.as_deref(), Some("<a:custGeom/>"));
    }

    fn write_temp_pptx(bytes: Vec<u8>) -> PathBuf {
        let unique = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("pptx2html-py-{unique}.pptx"));
        fs::write(&path, bytes).expect("write temp pptx");
        path
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
