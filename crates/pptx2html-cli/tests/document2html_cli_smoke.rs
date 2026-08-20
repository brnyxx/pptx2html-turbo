use std::fs;
use std::io::{Cursor, Write};
use std::process::Command;

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[test]
#[ignore = "requires LibreOffice and Poppler executables"]
fn document2html_cli_handles_help_bad_input_and_docx() {
    // Given
    let binary = env!("CARGO_BIN_EXE_document2html");

    // When
    let help = Command::new(binary)
        .arg("--help")
        .output()
        .expect("run document2html --help");
    let missing = Command::new(binary)
        .arg("missing.docx")
        .output()
        .expect("run document2html with missing input");

    // Then
    assert!(help.status.success());
    assert!(
        String::from_utf8_lossy(&help.stdout).contains("PPTX, DOCX, DOC, XLSX, XLS, PPT, or PDF")
    );
    assert_eq!(missing.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("failed to read"));

    // Given
    let temp = tempfile::tempdir().expect("create CLI workspace");
    let input = temp.path().join("sample.docx");
    let output = temp.path().join("sample.html");
    fs::write(&input, build_minimal_docx("CLI DOCX")).expect("write DOCX fixture");

    // When
    let conversion = Command::new(binary)
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .output()
        .expect("run document2html conversion");

    // Then
    assert!(
        conversion.status.success(),
        "{}",
        String::from_utf8_lossy(&conversion.stderr)
    );
    assert!(String::from_utf8_lossy(&conversion.stdout).contains("Conversion complete"));
    let html = fs::read_to_string(output).expect("read CLI HTML");
    assert!(html.contains("CLI"));
    assert!(html.contains("DOCX"));
}

fn build_minimal_docx(text: &str) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();
    write_entry(
        &mut zip,
        options,
        "[Content_Types].xml",
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
    );
    write_entry(
        &mut zip,
        options,
        "_rels/.rels",
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
    );
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

fn write_entry(
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    options: SimpleFileOptions,
    path: &str,
    data: &[u8],
) {
    zip.start_file(path, options).expect("start DOCX entry");
    zip.write_all(data).expect("write DOCX entry");
}
