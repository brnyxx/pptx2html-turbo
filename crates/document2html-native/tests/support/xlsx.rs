use std::io::{Cursor, Write};

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub fn build_minimal_xlsx(text: &str) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();
    write_entry(
        &mut zip,
        options,
        "[Content_Types].xml",
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#,
    );
    write_entry(
        &mut zip,
        options,
        "_rels/.rels",
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#,
    );
    write_entry(
        &mut zip,
        options,
        "xl/workbook.xml",
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
 xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#,
    );
    write_entry(
        &mut zip,
        options,
        "xl/_rels/workbook.xml.rels",
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#,
    );
    zip.start_file("xl/worksheets/sheet1.xml", options)
        .expect("start worksheet");
    write!(
        zip,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>{text}</t></is></c></row></sheetData>
</worksheet>"#
    )
    .expect("write worksheet");
    zip.finish().expect("finish XLSX").into_inner()
}

fn write_entry(
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    options: SimpleFileOptions,
    path: &str,
    data: &[u8],
) {
    zip.start_file(path, options).expect("start XLSX entry");
    zip.write_all(data).expect("write XLSX entry");
}
