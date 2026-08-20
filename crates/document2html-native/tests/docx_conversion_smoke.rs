use std::io::{Cursor, Write};

use document2html_core::{
    AssetMode, DocumentConversionOptions, DocumentFormat, DocumentInput, UnitKind,
};
use document2html_native::{NativeBackendConfig, NativeDocumentConverter};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[test]
#[ignore = "requires LibreOffice and Poppler executables"]
fn converts_docx_through_the_native_public_surface() {
    // Given
    let data = build_minimal_docx("Universal DOCX");
    let input = DocumentInput::detect(&data, Some("sample.docx"));
    let config = NativeBackendConfig::default();
    let converter = NativeDocumentConverter::new(config).expect("probe native runtime");

    // When
    let result = converter
        .convert(&input, &DocumentConversionOptions::default())
        .expect("convert DOCX");
    let repeated = converter
        .convert(&input, &DocumentConversionOptions::default())
        .expect("repeat DOCX conversion");
    let external = converter
        .convert(
            &input,
            &DocumentConversionOptions {
                asset_mode: AssetMode::External,
            },
        )
        .expect("convert DOCX with external assets");

    // Then
    assert_eq!(result.format, DocumentFormat::Docx);
    assert_eq!(result.unit_kind, UnitKind::Page);
    assert_eq!(result.unit_count, 1);
    assert!(result.html.contains("Universal"));
    assert!(result.html.contains("DOCX"));
    assert!(result.html.contains("data:image/png;base64,"));
    assert!(result.external_assets.is_empty());
    assert_eq!(result.html, repeated.html);
    assert_eq!(result.external_assets, repeated.external_assets);
    assert!(external.html.contains("assets/asset-0001.png"));
    assert!(!external.html.contains("data:image/png;base64,"));
    assert_eq!(external.external_assets.len(), 1);
    assert_eq!(
        external.external_assets[0].relative_path,
        "assets/asset-0001.png"
    );
    assert_eq!(result.backend.name, "libreoffice+poppler");
    assert_eq!(result.diagnostics[0].code, "NATIVE_BACKEND_OPAQUE");
    assert!(
        result
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "NATIVE_NETWORK_ISOLATION_DISABLED")
    );
}

fn build_minimal_docx(text: &str) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();
    zip.start_file("[Content_Types].xml", options)
        .expect("start content types");
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
    )
    .expect("write content types");
    zip.start_file("_rels/.rels", options)
        .expect("start root relationships");
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
    )
    .expect("write root relationships");
    zip.start_file("word/document.xml", options)
        .expect("start Word document");
    write!(
        zip,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>{text}</w:t></w:r></w:p><w:sectPr/></w:body>
</w:document>"#
    )
    .expect("write Word document");
    zip.finish().expect("finish DOCX").into_inner()
}
