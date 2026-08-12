use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("pptx2html-cli-{name}-{nanos}"))
}

fn write_temp_file(name: &str, bytes: &[u8]) -> PathBuf {
    let path = unique_temp_path(name).with_extension("pptx");
    fs::write(&path, bytes).expect("write temp pptx");
    path
}

fn mutate_slides(bytes: &[u8], mutate: impl Fn(&str) -> String) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("fixture opens");
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(DateTime::default());
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("fixture entry opens");
        let name = entry.name().to_owned();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).expect("fixture entry reads");
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            let xml = String::from_utf8(data).expect("slide XML is UTF-8");
            data = mutate(&xml).into_bytes();
        }
        writer.start_file(name, options).expect("entry starts");
        writer.write_all(&data).expect("entry writes");
    }
    writer.finish().expect("fixture finishes").into_inner()
}

fn fallback_fixture(bytes: &[u8]) -> Vec<u8> {
    mutate_slides(bytes, |xml| {
        xml.replace(
            "</p:spTree>",
            "<future:widget xmlns:future=\"urn:example:future\" id=\"fallback\"/></p:spTree>",
        )
    })
}

#[test]
fn info_command_outputs_json_metadata() {
    let input = write_temp_file("info", include_bytes!("fixtures/single-slide.pptx"));

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--info")
        .output()
        .expect("run cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"slide_count\":1"));
    assert!(stdout.contains("\"width_px\":960.0"));

    fs::remove_file(input).ok();
}

#[test]
fn single_file_conversion_writes_requested_output() {
    let input = write_temp_file("single", include_bytes!("fixtures/single-slide.pptx"));
    let output_path = unique_temp_path("single-output").with_extension("html");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--slides")
        .arg("1")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("run cli");

    assert!(output.status.success(), "{output:?}");
    let html = fs::read_to_string(&output_path).expect("read output html");
    assert!(html.contains("Slide One"));

    fs::remove_file(input).ok();
    fs::remove_file(output_path).ok();
}

#[test]
fn diagnostics_sidecar_and_strict_exit_preserve_outputs() {
    let input = write_temp_file(
        "diagnostics-strict",
        &fallback_fixture(include_bytes!("fixtures/single-slide.pptx")),
    );
    let output_path = unique_temp_path("diagnostics-output").with_extension("html");
    let sidecar = unique_temp_path("diagnostics-sidecar").with_extension("json");

    let default = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--output")
        .arg(&output_path)
        .arg("--diagnostics")
        .arg(&sidecar)
        .output()
        .expect("run default CLI");
    assert!(default.status.success(), "{default:?}");
    let first_json = fs::read_to_string(&sidecar).expect("sidecar exists");
    assert!(first_json.starts_with("[{\"code\":"), "{first_json}");
    let html = fs::read_to_string(&output_path).expect("HTML exists");
    let marker = "<script type=\"application/json\" id=\"pptx2html-diagnostics\">";
    let embedded = html
        .split_once(marker)
        .and_then(|(_, remainder)| remainder.split_once("</script>"))
        .map(|(payload, _)| payload)
        .expect("embedded diagnostics exist");
    assert_eq!(first_json, embedded);
    let default_stdout = String::from_utf8(default.stdout).unwrap();
    let default_stderr = String::from_utf8(default.stderr).unwrap();
    assert!(
        default_stdout.starts_with("Conversion complete:"),
        "{default_stdout}"
    );
    assert!(!default_stdout.contains("OOXML_ELEMENT_UNSUPPORTED"));
    assert!(
        default_stderr.contains("Conversion diagnostics (1): OOXML_ELEMENT_UNSUPPORTED=1"),
        "{default_stderr}"
    );

    let strict = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--output")
        .arg(&output_path)
        .arg("--diagnostics")
        .arg(&sidecar)
        .arg("--fail-on-fallback")
        .output()
        .expect("run strict CLI");
    assert_eq!(strict.status.code(), Some(2), "{strict:?}");
    assert!(
        output_path.exists(),
        "HTML must be written before strict exit"
    );
    assert_eq!(fs::read_to_string(&sidecar).unwrap(), first_json);

    fs::remove_file(input).ok();
    fs::remove_file(output_path).ok();
    fs::remove_file(sidecar).ok();
}

#[test]
fn diagnostics_rejects_input_alias_without_changing_input_or_html() {
    let dir = unique_temp_path("diagnostics-input-collision");
    fs::create_dir_all(dir.join("alias")).unwrap();
    let input = dir.join("input.pptx");
    let output_path = dir.join("output.html");
    let input_bytes = include_bytes!("fixtures/single-slide.pptx");
    fs::write(&input, input_bytes).unwrap();
    fs::write(&output_path, b"existing html").unwrap();
    let sidecar_alias = dir.join("alias").join("..").join("input.pptx");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--output")
        .arg(&output_path)
        .arg("--diagnostics")
        .arg(&sidecar_alias)
        .output()
        .expect("run input collision CLI");

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("input PPTX"), "{stderr}");
    assert!(stderr.contains("same resolved path"), "{stderr}");
    assert_eq!(fs::read(&input).unwrap(), input_bytes);
    assert_eq!(fs::read(&output_path).unwrap(), b"existing html");

    fs::remove_dir_all(dir).ok();
}

#[test]
fn single_file_conversion_applies_uniform_slide_scale() {
    let input = write_temp_file("single-scale", include_bytes!("fixtures/single-slide.pptx"));
    let output_path = unique_temp_path("single-scale-output").with_extension("html");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--scale")
        .arg("2")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("run cli");

    assert!(output.status.success(), "{output:?}");
    let html = fs::read_to_string(&output_path).expect("read scaled output html");
    assert!(html.contains("class=\"slide-shell\""));
    assert!(html.contains("width: 1920.0px; height: 1440.0px;"));
    assert!(html.contains("transform: scale(2.0000);"));

    fs::remove_file(input).ok();
    fs::remove_file(output_path).ok();
}

#[test]
fn multi_file_conversion_writes_per_slide_outputs() {
    let input = write_temp_file("multi", include_bytes!("fixtures/two-slides.pptx"));
    let output_dir = unique_temp_path("multi-output");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--format")
        .arg("multi")
        .arg("--output")
        .arg(&output_dir)
        .output()
        .expect("run cli");

    assert!(output.status.success(), "{output:?}");
    let slide_one = fs::read_to_string(output_dir.join("slide-1.html")).expect("slide 1 html");
    let slide_two = fs::read_to_string(output_dir.join("slide-2.html")).expect("slide 2 html");
    assert!(slide_one.contains("Slide One"));
    assert!(slide_two.contains("Slide Two"));

    fs::remove_file(input).ok();
    fs::remove_dir_all(output_dir).ok();
}

#[test]
fn invalid_slide_selection_returns_nonzero_exit_code() {
    let input = write_temp_file(
        "invalid-slides",
        include_bytes!("fixtures/single-slide.pptx"),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--slides")
        .arg("3-1")
        .output()
        .expect("run cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("Invalid --slides value"));

    fs::remove_file(input).ok();
}

#[test]
fn missing_input_returns_nonzero_exit_code() {
    let missing = unique_temp_path("missing").with_extension("pptx");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&missing)
        .arg("--info")
        .output()
        .expect("run cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("Failed to read presentation"));
}

#[test]
fn single_file_conversion_reports_conversion_failures_for_missing_input() {
    let missing = unique_temp_path("missing-convert").with_extension("pptx");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&missing)
        .output()
        .expect("run cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("Conversion failed"));
}

#[test]
fn info_command_escapes_title_strings() {
    let input = write_temp_file("info-title", &build_titled_single_slide_pptx());

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--info")
        .output()
        .expect("run cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"title\":\"Quarterly \\\\\\\"Deck\\\\\\\" \\\\\\\\ Notes\""));

    fs::remove_file(input).ok();
}

#[test]
fn multi_file_conversion_reports_output_directory_creation_failure() {
    let input = write_temp_file(
        "multi-dir-fail",
        include_bytes!("fixtures/single-slide.pptx"),
    );
    let output_path = unique_temp_path("multi-dir-target");
    fs::write(&output_path, b"not-a-directory").expect("seed output path as file");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--format")
        .arg("multi")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("run cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("Failed to create output directory"));

    fs::remove_file(input).ok();
    fs::remove_file(output_path).ok();
}

#[test]
fn multi_file_conversion_reports_slide_write_failures() {
    let input = write_temp_file(
        "multi-write-fail",
        include_bytes!("fixtures/single-slide.pptx"),
    );
    let output_dir = unique_temp_path("multi-write-target");
    fs::create_dir_all(output_dir.join("slide-1.html")).expect("seed slide html path as dir");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--format")
        .arg("multi")
        .arg("--output")
        .arg(&output_dir)
        .output()
        .expect("run cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("Failed to write"));

    fs::remove_file(input).ok();
    fs::remove_dir_all(output_dir).ok();
}

#[test]
fn single_file_conversion_reports_output_write_failures() {
    let input = write_temp_file(
        "single-write-fail",
        include_bytes!("fixtures/single-slide.pptx"),
    );
    let output_dir = unique_temp_path("single-output-dir");
    fs::create_dir_all(&output_dir).expect("create output dir");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--output")
        .arg(&output_dir)
        .output()
        .expect("run cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("Failed to write output file"));

    fs::remove_file(input).ok();
    fs::remove_dir_all(output_dir).ok();
}

#[test]
fn multi_file_conversion_reports_info_failures_for_missing_input() {
    let missing = unique_temp_path("missing-multi").with_extension("pptx");
    let output_dir = unique_temp_path("missing-multi-output");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&missing)
        .arg("--format")
        .arg("multi")
        .arg("--output")
        .arg(&output_dir)
        .output()
        .expect("run cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("Failed to read presentation"));

    fs::remove_dir_all(output_dir).ok();
}

#[test]
fn multi_file_conversion_reports_slide_conversion_failures() {
    let input = write_temp_file("missing-slide", &build_missing_slide_pptx());
    let output_dir = unique_temp_path("missing-slide-output");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--format")
        .arg("multi")
        .arg("--output")
        .arg(&output_dir)
        .output()
        .expect("run cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("Failed to read presentation")
            || stderr.contains("Failed to convert slide 1"),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(input).ok();
    fs::remove_dir_all(output_dir).ok();
}

#[test]
fn multi_file_conversion_reports_external_asset_write_failures() {
    let input = write_temp_file("multi-assets-fail", &build_background_image_pptx());
    let output_dir = unique_temp_path("multi-assets-output");
    fs::create_dir_all(&output_dir).expect("create output dir");
    fs::write(output_dir.join("images"), b"blocking-file").expect("create blocking file");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--format")
        .arg("multi")
        .arg("--no-embed")
        .arg("--output")
        .arg(&output_dir)
        .output()
        .expect("run cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("Failed to write external assets"));

    fs::remove_file(input).ok();
    fs::remove_file(output_dir.join("images")).ok();
    fs::remove_dir_all(output_dir).ok();
}

#[test]
fn single_file_conversion_reports_external_asset_write_failures() {
    let input = write_temp_file("single-assets-fail", &build_background_image_pptx());
    let output_dir = unique_temp_path("single-assets-output");
    fs::create_dir_all(&output_dir).expect("create output dir");
    fs::write(output_dir.join("images"), b"blocking-file").expect("create blocking file");
    let output_path = output_dir.join("deck.html");

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--no-embed")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("run cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("Failed to write external assets"));

    fs::remove_file(input).ok();
    fs::remove_file(output_dir.join("images")).ok();
    fs::remove_file(output_path).ok();
    fs::remove_dir_all(output_dir).ok();
}

#[cfg(unix)]
#[test]
fn diagnostics_rejects_input_hard_link_without_changing_input_or_html() {
    let dir = unique_temp_path("diagnostics-input-hard-link");
    fs::create_dir_all(&dir).unwrap();
    let input = dir.join("input.pptx");
    let output_path = dir.join("output.html");
    let diagnostics_sidecar = dir.join("diagnostics.json");
    fs::write(&input, include_bytes!("fixtures/single-slide.pptx")).unwrap();
    fs::write(&output_path, b"existing html").unwrap();
    fs::hard_link(&input, &diagnostics_sidecar).unwrap();
    let input_before = fs::read(&input).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_pptx2html"))
        .arg(&input)
        .arg("--output")
        .arg(&output_path)
        .arg("--diagnostics")
        .arg(&diagnostics_sidecar)
        .output()
        .expect("run hard link collision CLI");

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("input PPTX"), "{stderr}");
    assert!(stderr.contains("same resolved path"), "{stderr}");
    assert_eq!(fs::read(&input).unwrap(), input_before);
    assert_eq!(fs::read(&output_path).unwrap(), b"existing html");
    assert!(!dir.join("diagnostics.html").exists());

    fs::remove_dir_all(dir).ok();
}

fn build_titled_single_slide_pptx() -> Vec<u8> {
    let cursor = std::io::Cursor::new(Vec::new());
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
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
    )
    .unwrap();

    zip.start_file("ppt/presentation.xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst/>
  <p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#,
    )
    .unwrap();

    zip.start_file("ppt/_rels/presentation.xml.rels", options)
        .unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#,
    )
    .unwrap();

    zip.start_file("ppt/slides/slide1.xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm></p:spPr>
        <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Titled Slide</a:t></a:r></a:p></p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#,
    )
    .unwrap();

    zip.start_file("ppt/slides/_rels/slide1.xml.rels", options)
        .unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#,
    )
    .unwrap();

    zip.start_file("ppt/theme/theme1.xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="TestTheme">
  <a:themeElements>
    <a:clrScheme name="TestColors">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="000000"/></a:dk2>
      <a:lt2><a:srgbClr val="FFFFFF"/></a:lt2>
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
  </a:themeElements>
</a:theme>"#,
    )
    .unwrap();

    zip.start_file("docProps/core.xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
  <dc:title>Quarterly \"Deck\" \\ Notes</dc:title>
</cp:coreProperties>"#,
    )
    .unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_missing_slide_pptx() -> Vec<u8> {
    let cursor = std::io::Cursor::new(Vec::new());
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
  <Override PartName="/ppt/slides/slide2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
</Types>"#,
    )
    .unwrap();

    zip.start_file("_rels/.rels", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
    )
    .unwrap();

    zip.start_file("ppt/presentation.xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId1"/>
    <p:sldId id="257" r:id="rId2"/>
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
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
</Relationships>"#,
    )
    .unwrap();

    zip.start_file("ppt/slides/slide1.xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sld>"#,
    )
    .unwrap();

    zip.start_file("ppt/slides/slide2.xml", options).unwrap();
    zip.write_all(b"<not-xml").unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_background_image_pptx() -> Vec<u8> {
    let cursor = std::io::Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();

    zip.start_file("[Content_Types].xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#,
    )
    .unwrap();

    zip.start_file("_rels/.rels", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
    )
    .unwrap();

    zip.start_file("ppt/presentation.xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();

    zip.start_file("ppt/_rels/presentation.xml.rels", options)
        .unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#,
    )
    .unwrap();

    zip.start_file("ppt/slides/slide1.xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:blipFill>
          <a:blip r:embed="rId2"/>
          <a:stretch><a:fillRect/></a:stretch>
        </a:blipFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sld>"#,
    )
    .unwrap();

    zip.start_file("ppt/slides/_rels/slide1.xml.rels", options)
        .unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
</Relationships>"#,
    )
    .unwrap();

    zip.start_file("ppt/slideMasters/slideMaster1.xml", options)
        .unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sldMaster>"#,
    )
    .unwrap();

    zip.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", options)
        .unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#,
    )
    .unwrap();

    zip.start_file("ppt/media/image1.png", options).unwrap();
    zip.write_all(&[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ])
    .unwrap();

    zip.start_file("ppt/theme/theme1.xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="TestTheme">
  <a:themeElements>
    <a:clrScheme name="TestColors">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
    </a:clrScheme>
    <a:fontScheme name="TestFonts">
      <a:majorFont><a:latin typeface="Calibri"/></a:majorFont>
      <a:minorFont><a:latin typeface="Calibri"/></a:minorFont>
    </a:fontScheme>
  </a:themeElements>
</a:theme>"#,
    )
    .unwrap();

    zip.finish().unwrap().into_inner()
}
