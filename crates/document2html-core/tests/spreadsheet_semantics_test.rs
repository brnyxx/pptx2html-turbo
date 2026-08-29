use std::io::{Cursor, Write};

use document2html_core::{SpreadsheetCell, parse_xlsx_semantics};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[test]
fn extracts_sheets_repeated_unicode_formulas_and_skips_empty_cells() {
    let data = workbook_fixture(
        r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>반복 &amp; café</t></si></sst>"#,
        &[
            (
                "sheet1.xml",
                r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="inlineStr"><is><t>반복 &amp; café</t></is></c><c r="C1"><f>1+1</f><v>2</v></c><c r="D1"/></row></sheetData></worksheet>"#,
            ),
            (
                "sheet2.xml",
                r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="3"><c r="B3" t="str"><f>CONCAT(&quot;東&quot;,&quot;京&quot;)</f><v>東京</v></c><c r="C3" t="b"><v>1</v></c></row></sheetData></worksheet>"#,
            ),
        ],
    );

    let semantics = parse_xlsx_semantics(&data).expect("parse workbook semantics");

    assert_eq!(
        semantics.cells,
        vec![
            cell("First & <sheet>", "A1", "반복 & café"),
            cell("First & <sheet>", "B1", "반복 & café"),
            cell("First & <sheet>", "C1", "2"),
            cell("東京", "B3", "東京"),
            cell("東京", "C3", "TRUE"),
        ]
    );
}

#[test]
fn malformed_or_untrusted_workbook_identity_fails_closed() {
    let malformed_coordinate = workbook_fixture(
        "<sst/>",
        &[(
            "sheet1.xml",
            r#"<worksheet><sheetData><row><c r="not-A1"><v>7</v></c></row></sheetData></worksheet>"#,
        )],
    );
    assert!(parse_xlsx_semantics(&malformed_coordinate).is_err());

    assert!(parse_xlsx_semantics(b"not an XLSX package").is_err());
}

#[test]
fn empty_workbook_has_no_attributable_cells() {
    let data = workbook_fixture("<sst/>", &[]);

    let semantics = parse_xlsx_semantics(&data).expect("empty workbook should remain renderable");

    assert!(semantics.cells.is_empty());
}

fn cell(worksheet: &str, coordinate: &str, displayed_value: &str) -> SpreadsheetCell {
    SpreadsheetCell {
        worksheet: worksheet.to_owned(),
        coordinate: coordinate.to_owned(),
        displayed_value: displayed_value.to_owned(),
        attributable: true,
    }
}

fn workbook_fixture(shared_strings: &str, sheets: &[(&str, &str)]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    entry(&mut zip, options, "[Content_Types].xml", content_types());
    entry(&mut zip, options, "_rels/.rels", root_relationships());
    entry(&mut zip, options, "xl/workbook.xml", workbook_xml(sheets));
    entry(
        &mut zip,
        options,
        "xl/_rels/workbook.xml.rels",
        workbook_relationships(sheets),
    );
    entry(
        &mut zip,
        options,
        "xl/sharedStrings.xml",
        shared_strings.to_owned(),
    );
    for (name, xml) in sheets {
        entry(
            &mut zip,
            options,
            &format!("xl/worksheets/{name}"),
            (*xml).to_owned(),
        );
    }
    zip.finish().expect("finish fixture").into_inner()
}

fn content_types() -> String {
    r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/></Types>"#.to_owned()
}

fn root_relationships() -> String {
    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.to_owned()
}

fn workbook_xml(sheets: &[(&str, &str)]) -> String {
    let names = ["First &amp; &lt;sheet&gt;", "東京"];
    let items = sheets
        .iter()
        .enumerate()
        .map(|(index, _)| {
            format!(
                r#"<sheet name="{}" sheetId="{}" r:id="rId{}"/>"#,
                names[index],
                index + 1,
                index + 1
            )
        })
        .collect::<String>();
    format!(
        r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>{items}</sheets></workbook>"#
    )
}

fn workbook_relationships(sheets: &[(&str, &str)]) -> String {
    let items = sheets
        .iter()
        .enumerate()
        .map(|(index, (name, _))| {
            format!(
                r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/{name}"/>"#,
                index + 1
            )
        })
        .collect::<String>();
    format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{items}</Relationships>"#
    )
}

fn entry(
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    options: SimpleFileOptions,
    name: &str,
    value: String,
) {
    zip.start_file(name, options).expect("start fixture part");
    zip.write_all(value.as_bytes()).expect("write fixture part");
}

/// An unsupported number format must never fail the conversion. The cell is
/// preserved as unattributable so the document still converts.
#[test]
fn unsupported_number_format_converts_as_unattributable() {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    entry(&mut zip, options, "[Content_Types].xml", content_types());
    entry(&mut zip, options, "_rels/.rels", root_relationships());
    let sheets: &[(&str, &str)] = &[(
        "sheet1.xml",
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" s="1"><v>12.5</v></c><c r="B1"><v>7</v></c></row></sheetData></worksheet>"#,
    )];
    entry(&mut zip, options, "xl/workbook.xml", workbook_xml(sheets));
    entry(
        &mut zip,
        options,
        "xl/_rels/workbook.xml.rels",
        workbook_relationships(sheets),
    );
    entry(
        &mut zip,
        options,
        "xl/sharedStrings.xml",
        "<sst/>".to_owned(),
    );
    entry(
        &mut zip,
        options,
        "xl/styles.xml",
        r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cellXfs count="2"><xf numFmtId="0"/><xf numFmtId="44"/></cellXfs></styleSheet>"#.to_owned(),
    );
    for (name, xml) in sheets {
        entry(
            &mut zip,
            options,
            &format!("xl/worksheets/{name}"),
            (*xml).to_owned(),
        );
    }
    let data = zip.finish().expect("finish fixture").into_inner();

    let semantics = parse_xlsx_semantics(&data).expect("conversion must not fail");

    assert_eq!(semantics.cells.len(), 2);
    assert!(!semantics.cells[0].attributable);
    assert_eq!(semantics.cells[0].coordinate, "A1");
    assert!(semantics.cells[1].attributable);
    assert_eq!(semantics.cells[1].displayed_value, "7");
}
