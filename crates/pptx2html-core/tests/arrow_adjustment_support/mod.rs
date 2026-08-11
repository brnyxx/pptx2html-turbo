use crate::fixtures::MinimalPptx;
use pptx2html_core::convert_bytes;

#[derive(Clone, Copy, Debug)]
pub struct Case<'a> {
    pub preset: &'a str,
    pub key: &'a str,
    pub default: f64,
    pub lower: f64,
    pub upper: f64,
}

pub fn official_cases() -> Vec<Case<'static>> {
    include_str!("../fixtures/arrow_adjustment_contract.tsv")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.starts_with("preset\t"))
        .map(|line| {
            let columns = line.split('\t').collect::<Vec<_>>();
            assert_eq!(columns.len(), 5, "contract row: {line}");
            Case {
                preset: columns[0],
                key: columns[1],
                default: columns[2].parse().expect("default"),
                lower: columns[3].parse().expect("lower"),
                upper: columns[4].parse().expect("upper"),
            }
        })
        .collect()
}

pub fn official_preset_names() -> std::collections::BTreeSet<&'static str> {
    include_str!("../fixtures/arrow_adjustment_contract.tsv")
        .lines()
        .filter_map(|line| line.strip_prefix("# preset_name="))
        .collect()
}

pub fn render_html(
    preset: &str,
    adjustments: &[(&str, f64)],
    width_emu: i64,
    height_emu: i64,
) -> String {
    let adjustment_xml = adjustments
        .iter()
        .map(|(key, value)| format!(r#"<a:gd name="{key}" fmla="val {value}"/>"#))
        .collect::<String>();
    let slide = format!(
        r#"<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="Adjusted shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{width_emu}" cy="{height_emu}"/></a:xfrm>
    <a:prstGeom prst="{preset}"><a:avLst>{adjustment_xml}</a:avLst></a:prstGeom>
    <a:solidFill><a:srgbClr val="336699"/></a:solidFill><a:ln w="12700"><a:solidFill><a:srgbClr val="112233"/></a:solidFill></a:ln>
  </p:spPr></p:sp>"#
    );
    convert_bytes(&MinimalPptx::new(&slide).build()).expect("convert adjusted arrow")
}

pub fn svg_paths(html: &str) -> Vec<&str> {
    html.match_indices("<path ")
        .map(|(start, _)| {
            let element = &html[start..html[start..].find('>').expect("path end") + start];
            let d_start = element.find("d=\"").expect("path d") + 3;
            let d_end = element[d_start..].find('"').expect("path d end") + d_start;
            &element[d_start..d_end]
        })
        .collect()
}

pub fn path_numbers(path: &str) -> Vec<f64> {
    path.split(|character: char| {
        character.is_ascii_alphabetic() || character == ',' || character.is_whitespace()
    })
    .filter(|token| !token.is_empty())
    .map(|token| token.parse::<f64>().expect("SVG number"))
    .collect()
}

pub fn assert_finite_html(html: &str, label: &str) {
    assert!(
        !html.contains("NaN") && !html.to_ascii_lowercase().contains("inf"),
        "{label}: non-finite SVG"
    );
    let paths = svg_paths(html);
    assert!(!paths.is_empty(), "{label}: no path");
    for path in paths {
        let numbers = path_numbers(path);
        assert!(!numbers.is_empty(), "{label}: empty numeric path");
        assert!(
            numbers.iter().all(|value| value.is_finite()),
            "{label}: non-finite number"
        );
    }
}
