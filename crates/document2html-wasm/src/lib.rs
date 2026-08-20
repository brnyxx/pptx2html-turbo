#![forbid(unsafe_code)]

use document2html_core::{
    CoreDocumentConverter, DocumentConversionOptions, DocumentInput, core_runtime_capabilities,
    detect_format,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn detect_document_format(data: &[u8], filename: Option<String>) -> Result<String, JsError> {
    let input = DocumentInput::detect(data, filename.as_deref());
    detect_format(&input)
        .map(|format| format.as_str().to_owned())
        .map_err(to_js_error)
}

#[wasm_bindgen]
pub fn convert_document(data: &[u8], filename: Option<String>) -> Result<String, JsError> {
    let input = DocumentInput::detect(data, filename.as_deref());
    CoreDocumentConverter::convert(&input, &DocumentConversionOptions::default())
        .map(|result| result.html)
        .map_err(to_js_error)
}

#[wasm_bindgen]
pub fn runtime_capabilities_json() -> String {
    let entries = core_runtime_capabilities()
        .into_iter()
        .map(|capability| {
            let backend = capability
                .backend
                .map(|backend| format!("\"{backend}\""))
                .unwrap_or_else(|| "null".to_owned());
            format!(
                concat!(
                    "{{",
                    "\"format\":\"{}\",",
                    "\"support\":\"{}\",",
                    "\"backend\":{}",
                    "}}"
                ),
                capability.format.as_str(),
                capability.support.as_str(),
                backend
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{entries}]")
}

fn to_js_error(error: impl std::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{convert_document, detect_document_format, runtime_capabilities_json};

    const SINGLE_SLIDE_PPTX: &[u8] =
        include_bytes!("../../pptx2html-cli/tests/fixtures/single-slide.pptx");

    #[test]
    fn detects_and_converts_pptx_in_core_only_runtime() {
        // Given
        let filename = Some("single-slide.pptx".to_owned());

        // When
        let format =
            detect_document_format(SINGLE_SLIDE_PPTX, filename.clone()).expect("detect PPTX");
        let html = convert_document(SINGLE_SLIDE_PPTX, filename).expect("convert PPTX");

        // Then
        assert_eq!(format, "pptx");
        assert!(html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn capability_json_marks_native_formats_unavailable() {
        // Given and When
        let json = runtime_capabilities_json();

        // Then
        assert!(json.contains(r#""format":"pptx","support":"available""#));
        assert!(json.contains(r#""format":"docx","support":"backend-unavailable""#));
        assert!(json.contains(r#""format":"pdf","support":"backend-unavailable""#));
    }
}
