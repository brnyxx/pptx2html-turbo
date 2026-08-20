#![forbid(unsafe_code)]

use std::path::Path;

use document2html_core::{
    AssetMode, CoreDocumentConverter, DocumentConversionOptions, DocumentConversionResult,
    DocumentFormat, DocumentInput, detect_format as detect_document_format,
};
use document2html_native::{NativeBackendConfig, NativeDocumentConverter, ProcessIsolation};
use pyo3::prelude::*;

#[pyclass(name = "DocumentConversionResult")]
#[derive(Debug, Clone)]
pub struct PyDocumentConversionResult {
    #[pyo3(get)]
    pub html: String,
    #[pyo3(get)]
    pub format: String,
    #[pyo3(get)]
    pub unit_count: usize,
    #[pyo3(get)]
    pub unit_kind: String,
    #[pyo3(get)]
    pub backend_name: String,
    #[pyo3(get)]
    pub backend_version: String,
    #[pyo3(get)]
    pub diagnostics_json: String,
}

#[pymethods]
impl PyDocumentConversionResult {
    fn __repr__(&self) -> String {
        format!(
            "DocumentConversionResult(format='{}', units={}, backend='{}')",
            self.format, self.unit_count, self.backend_name
        )
    }
}

#[pyfunction(signature = (data, filename=None))]
fn detect_format(data: &[u8], filename: Option<&str>) -> PyResult<String> {
    let input = DocumentInput::detect(data, filename);
    detect_document_format(&input)
        .map(|format| format.as_str().to_owned())
        .map_err(runtime_error)
}

#[pyfunction(signature = (path, *, allow_unisolated=false))]
fn convert_file(path: &str, allow_unisolated: bool) -> PyResult<PyDocumentConversionResult> {
    let data = std::fs::read(path).map_err(runtime_error)?;
    let filename = Path::new(path).file_name().and_then(|name| name.to_str());
    convert_bytes(&data, filename, allow_unisolated)
}

#[pyfunction(signature = (data, filename=None, *, allow_unisolated=false))]
fn convert_bytes(
    data: &[u8],
    filename: Option<&str>,
    allow_unisolated: bool,
) -> PyResult<PyDocumentConversionResult> {
    let input = DocumentInput::detect(data, filename);
    let format = detect_document_format(&input).map_err(runtime_error)?;
    let options = DocumentConversionOptions {
        asset_mode: AssetMode::Embed,
    };
    let result = if format == DocumentFormat::Pptx {
        CoreDocumentConverter::convert(&input, &options).map_err(runtime_error)?
    } else {
        NativeDocumentConverter::new(NativeBackendConfig {
            process_isolation: if allow_unisolated {
                ProcessIsolation::AllowUnisolated
            } else {
                ProcessIsolation::StrictAuto
            },
            ..Default::default()
        })
        .and_then(|converter| converter.convert(&input, &options))
        .map_err(runtime_error)?
    };
    Ok(map_result(result))
}

#[pyfunction]
fn supported_formats() -> Vec<&'static str> {
    vec!["pptx", "docx", "doc", "xlsx", "xls", "ppt", "pdf"]
}

fn map_result(result: DocumentConversionResult) -> PyDocumentConversionResult {
    let diagnostics_json = result.diagnostics_json();
    PyDocumentConversionResult {
        html: result.html,
        format: result.format.as_str().to_owned(),
        unit_count: result.unit_count,
        unit_kind: result.unit_kind.as_str().to_owned(),
        backend_name: result.backend.name,
        backend_version: result.backend.version,
        diagnostics_json,
    }
}

fn runtime_error(error: impl std::fmt::Display) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())
}

#[pymodule]
fn document2html(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(detect_format, module)?)?;
    module.add_function(wrap_pyfunction!(convert_file, module)?)?;
    module.add_function(wrap_pyfunction!(convert_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(supported_formats, module)?)?;
    module.add_class::<PyDocumentConversionResult>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use pyo3::Python;
    use pyo3::types::{PyAnyMethods, PyModule};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{convert_bytes, detect_format, document2html, supported_formats};

    const SINGLE_SLIDE_PPTX: &[u8] =
        include_bytes!("../../pptx2html-cli/tests/fixtures/single-slide.pptx");

    #[test]
    fn detects_and_converts_pptx_without_native_runtime() {
        // Given
        let filename = Some("single-slide.pptx");

        // When
        let format = detect_format(SINGLE_SLIDE_PPTX, filename).expect("detect PPTX");
        let result = convert_bytes(SINGLE_SLIDE_PPTX, filename, false).expect("convert PPTX bytes");

        // Then
        assert_eq!(format, "pptx");
        assert_eq!(result.format, "pptx");
        assert_eq!(result.unit_count, 1);
        assert!(result.html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn module_registers_generic_functions_and_result_class() {
        // Given
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|python| {
            let module = PyModule::new(python, "document2html").expect("create module");

            // When
            document2html(&module).expect("register module");

            // Then
            for name in [
                "detect_format",
                "convert_file",
                "convert_bytes",
                "supported_formats",
                "DocumentConversionResult",
            ] {
                assert!(module.getattr(name).is_ok(), "missing {name}");
            }
        });
        assert_eq!(supported_formats().len(), 7);
    }

    #[test]
    #[ignore = "requires LibreOffice and Poppler executables"]
    fn converts_docx_through_python_binding_contract() {
        // Given
        let data = build_minimal_docx("Python DOCX");

        // When
        let result = convert_bytes(&data, Some("sample.docx"), false).expect("convert DOCX bytes");

        // Then
        assert_eq!(result.format, "docx");
        assert_eq!(result.unit_count, 1);
        assert!(result.html.contains("Python"));
        assert!(result.html.contains("DOCX"));
    }

    fn build_minimal_docx(text: &str) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();
        zip.start_file("[Content_Types].xml", options)
            .expect("start content types");
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
        )
        .expect("write content types");
        zip.start_file("_rels/.rels", options)
            .expect("start relationships");
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
        )
        .expect("write relationships");
        zip.start_file("word/document.xml", options)
            .expect("start document");
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>{text}</w:t></w:r></w:p><w:sectPr/></w:body>
</w:document>"#
        )
        .expect("write document");
        zip.finish().expect("finish DOCX").into_inner()
    }
}
