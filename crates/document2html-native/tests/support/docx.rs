use std::io::{Cursor, Write};

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub fn build_minimal_docx(text: &str) -> Vec<u8> {
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
