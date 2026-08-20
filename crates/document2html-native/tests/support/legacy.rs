use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn convert_fixture_with_libreoffice(
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
