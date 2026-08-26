//! Shared XLSX semantic cases.
//!
//! This suite and `evaluate/tests/test_multiformat_portable_spreadsheet_semantics.py`
//! read the same `evaluate/multiformat/xlsx-semantic-cases.v1.json`, so the Rust
//! core and the Python portable extractor cannot drift on which packages are
//! accepted and which are refused.

use std::io::{Cursor, Write};
use std::path::PathBuf;

use document2html_core::parse_xlsx_semantics;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const DOC_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PKG_REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

struct SharedCase {
    name: String,
    outcome: String,
    worksheet_cells: String,
    styles: Option<String>,
    relationship_target: Option<String>,
    relationship_target_mode: Option<String>,
    expected: Vec<ExpectedCell>,
    unattributed: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct ExpectedCell {
    address: String,
    display: String,
    attributable: bool,
}

#[test]
fn shared_cases_agree_with_the_portable_extractor() {
    let cases = load_cases();
    // Guard against a silently truncated or unparsed fixture.
    assert!(
        cases.len() >= 20,
        "expected the full shared case set, parsed {}",
        cases.len()
    );
    for case in &cases {
        let data = package(case);
        let result = parse_xlsx_semantics(&data);
        match case.outcome.as_str() {
            "refuse" => assert!(
                result.is_err(),
                "case {} must be refused by the core extractor",
                case.name
            ),
            "accept" => {
                let semantics = result
                    .unwrap_or_else(|error| panic!("case {} must be accepted: {error}", case.name));
                let actual: Vec<ExpectedCell> = semantics
                    .cells
                    .iter()
                    .filter(|cell| cell.attributable)
                    .map(|cell| ExpectedCell {
                        address: cell.coordinate.clone(),
                        display: cell.displayed_value.clone(),
                        attributable: true,
                    })
                    .collect();
                assert_eq!(actual, case.expected, "case {} value mismatch", case.name);
                // Cells the core keeps for its annotation diagnostic must
                // match the shared unattributable set exactly.
                let refused: Vec<String> = semantics
                    .cells
                    .iter()
                    .filter(|cell| !cell.attributable)
                    .map(|cell| cell.coordinate.clone())
                    .collect();
                assert_eq!(
                    refused, case.unattributed,
                    "case {} unattributable mismatch",
                    case.name
                );
            }
            other => panic!("case {} has unknown outcome {other}", case.name),
        }
    }
}

fn package(case: &SharedCase) -> Vec<u8> {
    let target = case
        .relationship_target
        .clone()
        .unwrap_or_else(|| "worksheets/sheet1.xml".to_owned());
    let mode = case
        .relationship_target_mode
        .as_ref()
        .map(|value| format!(r#" TargetMode="{value}""#))
        .unwrap_or_default();
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    entry(
        &mut zip,
        options,
        "xl/workbook.xml",
        format!(
            r#"<workbook xmlns="{MAIN}" xmlns:r="{DOC_REL}"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>"#
        ),
    );
    entry(
        &mut zip,
        options,
        "xl/_rels/workbook.xml.rels",
        format!(
            r#"<Relationships xmlns="{PKG_REL}"><Relationship Id="rId1" Type="{DOC_REL}/worksheet" Target="{target}"{mode}/></Relationships>"#
        ),
    );
    entry(
        &mut zip,
        options,
        "xl/sharedStrings.xml",
        format!(r#"<sst xmlns="{MAIN}"><si><t>Shared</t></si></sst>"#),
    );
    if let Some(styles) = &case.styles {
        entry(
            &mut zip,
            options,
            "xl/styles.xml",
            format!(r#"<styleSheet xmlns="{MAIN}">{styles}</styleSheet>"#),
        );
    }
    entry(
        &mut zip,
        options,
        "xl/worksheets/sheet1.xml",
        format!(
            r#"<worksheet xmlns="{MAIN}"><sheetData><row r="1">{}</row></sheetData></worksheet>"#,
            case.worksheet_cells
        ),
    );
    zip.finish().expect("finish shared fixture").into_inner()
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

fn cases_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../evaluate/multiformat/xlsx-semantic-cases.v1.json")
}

/// Minimal reader for the shared case file. The core crate intentionally has no
/// JSON dependency, and the fixture shape is fixed and machine-generated, so
/// the few needed fields are extracted directly.
fn load_cases() -> Vec<SharedCase> {
    let path = cases_path();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let mut cases = Vec::new();
    for chunk in raw.split("\"name\":").skip(1) {
        cases.push(SharedCase {
            name: next_string(chunk).expect("case name"),
            outcome: field(chunk, "outcome").expect("case outcome"),
            worksheet_cells: field(chunk, "worksheet_cells").expect("case cells"),
            styles: field(chunk, "styles"),
            relationship_target: field(chunk, "relationship_target"),
            relationship_target_mode: field(chunk, "relationship_target_mode"),
            expected: expected_cells(chunk),
            unattributed: unattributed_cells(chunk),
        });
    }
    cases
}

fn field(chunk: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\":");
    let start = chunk.find(&marker)? + marker.len();
    next_string(&chunk[start..])
}

/// Reads the next JSON string literal, resolving the escapes the fixture uses.
fn next_string(chunk: &str) -> Option<String> {
    let start = chunk.find('"')? + 1;
    let rest = &chunk[start..];
    let mut value = String::new();
    let mut escaped = false;
    for character in rest.chars() {
        if escaped {
            value.push(match character {
                'n' => '\n',
                't' => '\t',
                other => other,
            });
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => return Some(value),
            other => value.push(other),
        }
    }
    None
}

/// Addresses listed under `unattributed` for this case.
fn unattributed_cells(chunk: &str) -> Vec<String> {
    let Some(start) = chunk.find("\"unattributed\":") else {
        return Vec::new();
    };
    let region = &chunk[start..];
    let Some(end) = region.find(']') else {
        return Vec::new();
    };
    region[..end]
        .split("\"address\":")
        .skip(1)
        .filter_map(next_string)
        .collect()
}

fn expected_cells(chunk: &str) -> Vec<ExpectedCell> {
    let Some(start) = chunk.find("\"expected\":") else {
        return Vec::new();
    };
    let region = &chunk[start..];
    let Some(end) = region.find(']') else {
        return Vec::new();
    };
    region[..end]
        .split("\"address\":")
        .skip(1)
        .filter_map(|item| {
            let address = next_string(item)?;
            let display_start = item.find("\"display\":")? + "\"display\":".len();
            let display = next_string(&item[display_start..])?;
            // Absent means attributable; the fixture only spells out false.
            let attributable = !item.contains("\"attributable\": false");
            Some(ExpectedCell {
                address,
                display,
                attributable,
            })
        })
        .collect()
}
