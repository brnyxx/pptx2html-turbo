use document2html_native::{NativeBackendConfig, NativeRuntime};

#[test]
#[ignore = "requires LibreOffice and Poppler executables"]
fn probes_installed_native_runtime_versions() {
    // Given
    let runtime = NativeRuntime::new(NativeBackendConfig::default());

    // When
    let info = runtime
        .probe()
        .expect("installed native runtime should probe");

    // Then
    assert!(info.libreoffice.version.starts_with("LibreOffice "));
    assert!(info.pdftohtml.version.starts_with("pdftohtml version "));
    assert!(info.pdfinfo.version.starts_with("pdfinfo version "));
}
