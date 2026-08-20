use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use document2html_core::{DocumentConversionOptions, DocumentFormat, DocumentInput, UnitKind};
use document2html_native::{NativeBackendConfig, NativeDocumentConverter};

#[path = "support/docx.rs"]
mod docx_support;
use docx_support::build_minimal_docx;

#[test]
#[ignore = "requires LibreOffice and Poppler executables"]
fn converts_legacy_doc_through_the_native_public_surface() {
    // Given
    let docx = build_minimal_docx("Universal DOC");
    let data = convert_fixture_with_libreoffice(&docx, "docx", "doc");
    let input = DocumentInput::detect(&data, Some("sample.doc"));
    let converter =
        NativeDocumentConverter::new(NativeBackendConfig::default()).expect("probe runtime");

    // When
    let result = converter
        .convert(&input, &DocumentConversionOptions::default())
        .expect("convert DOC");

    // Then
    assert_eq!(result.format, DocumentFormat::Doc);
    assert_eq!(result.unit_kind, UnitKind::Page);
    assert_eq!(result.unit_count, 1);
    assert!(result.html.contains("Universal"));
    assert!(result.html.contains("DOC"));
}

fn convert_fixture_with_libreoffice(
    source: &[u8],
    source_extension: &str,
    target_extension: &str,
) -> Vec<u8> {
    let temp = tempfile::tempdir().expect("create fixture workspace");
    let profile = temp.path().join("profile");
    let output = temp.path().join("output");
    fs::create_dir(&profile).expect("create fixture profile");
    fs::create_dir(&output).expect("create fixture output");
    let source_path = temp.path().join(format!("input.{source_extension}"));
    fs::write(&source_path, source).expect("write source fixture");
    let result = Command::new(resolve_soffice())
        .arg("--headless")
        .arg(format!("-env:UserInstallation={}", file_uri(&profile)))
        .arg("--convert-to")
        .arg(target_extension)
        .arg("--outdir")
        .arg(&output)
        .arg(&source_path)
        .output()
        .expect("launch LibreOffice fixture conversion");
    assert!(
        result.status.success(),
        "LibreOffice fixture conversion failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    fs::read(output.join(format!("input.{target_extension}"))).expect("read converted fixture")
}

fn resolve_soffice() -> PathBuf {
    std::env::var_os("DOCUMENT2HTML_SOFFICE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("soffice"))
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.to_string_lossy())
}
