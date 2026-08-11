use std::collections::HashSet;

use base64::Engine;
use pptx2html_core::convert_bytes;
use pptx2html_core::model::PatternPreset;
use quick_xml::{events::Event, reader::Reader};

use super::{OFFICIAL_PATTERNS, fixtures::MinimalPptx, shape_with_pattern};

const PERCENTAGES: [(&str, usize); 12] = [
    ("pct5", 5),
    ("pct10", 10),
    ("pct20", 20),
    ("pct25", 25),
    ("pct30", 30),
    ("pct40", 40),
    ("pct50", 50),
    ("pct60", 60),
    ("pct70", 70),
    ("pct75", 75),
    ("pct80", 80),
    ("pct90", 90),
];

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct Cell {
    x: u8,
    y: u8,
    width: u8,
    height: u8,
}

struct TileCoverage {
    width: u8,
    height: u8,
    cells: Vec<Cell>,
}

#[test]
fn classifies_the_exact_official_pattern_inventory() {
    let parsed: Vec<String> = OFFICIAL_PATTERNS
        .iter()
        .map(|name| PatternPreset::from_ooxml(name).as_ooxml().to_owned())
        .collect();

    assert_eq!(parsed, OFFICIAL_PATTERNS);
    assert!(matches!(
        PatternPreset::from_ooxml("futurePattern"),
        PatternPreset::Unknown(raw) if raw == "futurePattern"
    ));
}

#[test]
fn percentage_tiles_use_exact_five_percent_rectangular_cells() {
    for (preset, expected_percent) in PERCENTAGES {
        let svg = rendered_svg(preset);
        let coverage = foreground_cells(&svg);

        assert_eq!((coverage.width, coverage.height), (5, 4), "{preset}");
        assert_eq!(coverage.cells.len() * 5, expected_percent, "{preset}");
        assert_eq!(
            coverage.cells.iter().copied().collect::<HashSet<_>>().len(),
            coverage.cells.len()
        );
        assert!(
            coverage
                .cells
                .iter()
                .all(|cell| (cell.width, cell.height) == (1, 1))
        );
    }
}

#[test]
fn renders_all_official_presets_with_distinct_tile_signatures() {
    let colors = r#"<a:fgClr><a:srgbClr val="224466"/></a:fgClr><a:bgClr><a:srgbClr val="F8F8F8"/></a:bgClr>"#;
    let body: String = OFFICIAL_PATTERNS
        .iter()
        .map(|preset| shape_with_pattern(preset, colors))
        .collect();

    let html = convert_bytes(&MinimalPptx::new(&body).build()).unwrap();
    let signatures: Vec<&str> = html
        .split("data:image/svg+xml;base64,")
        .skip(1)
        .filter_map(|rest| rest.split_once(')').map(|(signature, _)| signature))
        .collect();
    let unique: HashSet<&str> = signatures.iter().copied().collect();

    assert_eq!(signatures.len(), OFFICIAL_PATTERNS.len());
    assert_eq!(unique.len(), OFFICIAL_PATTERNS.len());
}

fn rendered_svg(preset: &str) -> String {
    let colors = r#"<a:fgClr><a:srgbClr val="224466"/></a:fgClr><a:bgClr><a:srgbClr val="F8F8F8"/></a:bgClr>"#;
    let html =
        convert_bytes(&MinimalPptx::new(&shape_with_pattern(preset, colors)).build()).unwrap();
    let encoded = html
        .split_once("data:image/svg+xml;base64,")
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(value, _)| value)
        .unwrap();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    String::from_utf8(decoded).unwrap()
}

fn foreground_cells(svg: &str) -> TileCoverage {
    let mut reader = Reader::from_str(svg);
    let mut tile = (0, 0);
    let mut cells = Vec::new();
    loop {
        match reader.read_event().unwrap() {
            Event::Start(element) if element.name().as_ref() == b"svg" => {
                tile = (
                    attribute(&element, b"width").parse().unwrap(),
                    attribute(&element, b"height").parse().unwrap(),
                );
            }
            Event::Empty(element)
                if element.name().as_ref() == b"rect"
                    && attribute(&element, b"fill") == "#224466" =>
            {
                cells.push(Cell {
                    x: attribute(&element, b"x").parse().unwrap(),
                    y: attribute(&element, b"y").parse().unwrap(),
                    width: attribute(&element, b"width").parse().unwrap(),
                    height: attribute(&element, b"height").parse().unwrap(),
                });
            }
            Event::Eof => break,
            _ => {}
        }
    }
    TileCoverage {
        width: tile.0,
        height: tile.1,
        cells,
    }
}

fn attribute(element: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> String {
    element
        .attributes()
        .filter_map(Result::ok)
        .find(|attribute| attribute.key.as_ref() == name)
        .map(|attribute| String::from_utf8_lossy(attribute.value.as_ref()).into_owned())
        .unwrap_or_default()
}
