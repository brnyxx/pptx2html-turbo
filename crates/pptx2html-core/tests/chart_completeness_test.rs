mod fixtures;

use std::io::{Cursor, Read, Write};

use fixtures::MinimalPptx;
use pptx2html_core::{convert_bytes_with_metadata, error::PptxError};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

const C: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const CX: &str = "http://schemas.microsoft.com/office/drawing/2014/chartex";
const PREVIEW_CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

fn frame(relationship_id: Option<&str>, uri: &str, prefix: &str) -> String {
    let relationship = relationship_id
        .map(|id| format!(r#" r:id="{id}""#))
        .unwrap_or_default();
    format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="classified chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="100000" y="200000"/><a:ext cx="3000000" cy="2000000"/></p:xfrm><a:graphic><a:graphicData uri="{uri}"><{prefix}:chart xmlns:{prefix}="{uri}"{relationship}/></a:graphicData></a:graphic></p:graphicFrame>"#
    )
}

fn package(
    chart_xml: Option<&str>,
    relationship_id: Option<&str>,
    relationship_target: bool,
) -> Vec<u8> {
    package_with_slide(
        &frame(relationship_id, C, "c"),
        chart_xml,
        relationship_target,
    )
}

fn package_with_slide(slide: &str, chart_xml: Option<&str>, relationship_target: bool) -> Vec<u8> {
    let relationships = if relationship_target {
        r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdChart" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#
    } else {
        r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#
    };
    let mut builder = MinimalPptx::new(slide).with_slide_rels(relationships);
    if let Some(xml) = chart_xml {
        builder = builder.with_extra_file("ppt/charts/chart1.xml", xml.as_bytes());
    }
    builder.build()
}

fn package_entry(package: &[u8], entry_name: &str) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(package)).expect("source package");
    let mut entry = archive.by_name(entry_name).expect("source entry");
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).expect("read source entry");
    bytes
}

fn rewrite_entry(package: &[u8], entry_name: &str, replacement: &[u8]) -> Vec<u8> {
    let mut source = ZipArchive::new(Cursor::new(package)).expect("source package");
    let mut output = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    for index in 0..source.len() {
        let mut entry = source.by_index(index).expect("source entry");
        let name = entry.name().to_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).expect("read source entry");
        output
            .start_file(&name, options)
            .expect("start output entry");
        output
            .write_all(if name == entry_name {
                replacement
            } else {
                &bytes
            })
            .expect("write output entry");
    }
    output.finish().expect("finish package").into_inner()
}

fn chart_space(plot: &str, axes: &str) -> String {
    format!(
        r#"<?xml version="1.0"?><c:chartSpace xmlns:c="{C}"><c:chart><c:plotArea>{plot}{axes}</c:plotArea></c:chart></c:chartSpace>"#
    )
}

fn category_series(values: &str) -> String {
    format!(
        r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:v>S</c:v></c:tx><c:cat><c:strLit><c:ptCount val="2"/><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>B</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>{values}</c:v></c:pt></c:numLit></c:val></c:ser>"#
    )
}

fn xy_series(bubble: bool) -> String {
    let sizes = if bubble {
        r#"<c:bubbleSize><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>2</c:v></c:pt><c:pt idx="1"><c:v>4</c:v></c:pt></c:numLit></c:bubbleSize>"#
    } else {
        ""
    };
    format!(
        r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:xVal><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numLit></c:xVal><c:yVal><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>3</c:v></c:pt><c:pt idx="1"><c:v>4</c:v></c:pt></c:numLit></c:yVal>{sizes}</c:ser>"#
    )
}

fn axes() -> &'static str {
    r#"<c:catAx><c:axId val="10"/><c:crossAx val="20"/></c:catAx><c:valAx><c:axId val="20"/><c:crossAx val="10"/></c:valAx>"#
}

fn numeric_axes() -> &'static str {
    r#"<c:valAx><c:axId val="10"/><c:crossAx val="20"/></c:valAx><c:valAx><c:axId val="20"/><c:crossAx val="10"/></c:valAx>"#
}

fn classic_family_xml(family: &str, content: &str, numeric: bool) -> String {
    let axis_xml = if matches!(family, "scatterChart" | "bubbleChart") || numeric {
        numeric_axes()
    } else if matches!(
        family,
        "barChart" | "bar3DChart" | "lineChart" | "line3DChart" | "areaChart" | "area3DChart"
    ) {
        axes()
    } else {
        ""
    };
    let refs = if axis_xml.is_empty() {
        ""
    } else {
        r#"<c:axId val="10"/><c:axId val="20"/>"#
    };
    chart_space(
        &format!("<c:{family}>{content}{refs}</c:{family}>"),
        axis_xml,
    )
}

fn family_plot(family: &str) -> String {
    let (properties, series, axis_refs) = match family {
        "barChart" | "bar3DChart" => (
            r#"<c:barDir val="col"/><c:grouping val="clustered"/>"#,
            category_series("2"),
            r#"<c:axId val="10"/><c:axId val="20"/>"#,
        ),
        "lineChart" | "line3DChart" => (
            "",
            category_series("2"),
            r#"<c:axId val="10"/><c:axId val="20"/>"#,
        ),
        "areaChart" | "area3DChart" => (
            r#"<c:grouping val="standard"/>"#,
            category_series("2"),
            r#"<c:axId val="10"/><c:axId val="20"/>"#,
        ),
        "scatterChart" => (
            r#"<c:scatterStyle val="marker"/>"#,
            xy_series(false),
            r#"<c:axId val="10"/><c:axId val="20"/>"#,
        ),
        "bubbleChart" => (
            "",
            xy_series(true),
            r#"<c:axId val="10"/><c:axId val="20"/>"#,
        ),
        "ofPieChart" => (
            r#"<c:ofPieType val="pie"/><c:splitType val="pos"/><c:splitPos val="1"/>"#,
            category_series("2"),
            "",
        ),
        _ => ("", category_series("2"), ""),
    };
    format!("<c:{family}>{properties}{series}{axis_refs}</c:{family}>")
}

#[test]
fn every_classic_family_has_one_public_deterministic_disposition() {
    let cases = [
        ("areaChart", true, None),
        ("area3DChart", true, None),
        ("barChart", true, None),
        ("bar3DChart", true, None),
        ("bubbleChart", true, None),
        ("doughnutChart", true, None),
        ("lineChart", true, None),
        ("line3DChart", true, None),
        ("ofPieChart", true, None),
        ("pieChart", true, None),
        ("pie3DChart", true, None),
        ("radarChart", true, None),
        ("scatterChart", true, None),
        ("stockChart", false, Some("unsupported-family")),
        ("surfaceChart", false, Some("unsupported-family")),
        ("surface3DChart", false, Some("unsupported-family")),
    ];

    for (family, direct, reason) in cases {
        let axis_xml = if matches!(family, "bubbleChart" | "scatterChart") {
            numeric_axes()
        } else if matches!(
            family,
            "areaChart" | "area3DChart" | "barChart" | "bar3DChart" | "lineChart" | "line3DChart"
        ) {
            axes()
        } else {
            ""
        };
        let xml = chart_space(&family_plot(family), axis_xml);
        let result = convert_bytes_with_metadata(&package(Some(&xml), Some("rIdChart"), true))
            .unwrap_or_else(|error| panic!("{family}: {error}"));
        let diagnostics: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|item| item.code == "DRAWINGML_CHART_FALLBACK")
            .collect();
        assert_eq!(
            result.html.contains("class=\"chart-direct"),
            direct,
            "{family}"
        );
        assert_eq!(
            diagnostics.len(),
            usize::from(!direct),
            "{family}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.diagnostics.len(), usize::from(!direct), "{family}");
        if let Some(reason) = reason {
            assert!(
                diagnostics[0].reason.contains(reason),
                "{family}: {}",
                diagnostics[0].reason
            );
            assert!(
                diagnostics[0]
                    .raw_reference
                    .as_deref()
                    .is_some_and(|raw| raw.contains("series_summary") && raw.contains("raw_xml"))
            );
        }
    }
}

#[test]
fn graphic_frame_chart_dispatch_requires_exact_namespaces_and_ancestry() {
    let chart_xml = chart_space(&family_plot("barChart"), axes());
    let exact = frame(Some("rIdChart"), C, "c");
    let cases = [
        (
            "foreign-graphic-frame",
            exact
                .replacen(
                    "<p:graphicFrame",
                    "<evil:graphicFrame xmlns:evil=\"urn:evil\"",
                    1,
                )
                .replacen("</p:graphicFrame>", "</evil:graphicFrame>", 1),
            "OOXML_ELEMENT_UNSUPPORTED",
            "evil:graphicFrame",
        ),
        (
            "foreign-graphic-data",
            exact
                .replacen(
                    "<a:graphicData",
                    "<evil:graphicData xmlns:evil=\"urn:evil\"",
                    1,
                )
                .replacen("</a:graphicData>", "</evil:graphicData>", 1),
            "OOXML_ELEMENT_UNSUPPORTED",
            "evil:graphicData",
        ),
        (
            "invalid-official-ancestry",
            exact
                .replace(
                    &format!("<a:graphic><a:graphicData uri=\"{C}\">"),
                    "<a:graphic>",
                )
                .replace("</a:graphicData></a:graphic>", "</a:graphic>"),
            "DRAWINGML_CHART_FALLBACK",
            "c:chart",
        ),
    ];

    for (name, slide, code, qualified_name) in cases {
        let result =
            convert_bytes_with_metadata(&package_with_slide(&slide, Some(&chart_xml), true))
                .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(!result.html.contains("class=\"chart-direct\""), "{name}");
        assert_eq!(
            result.diagnostics.len(),
            1,
            "{name}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.diagnostics[0].code, code, "{name}");
        assert_eq!(
            result.diagnostics[0]
                .location
                .qualified_element_name
                .as_deref(),
            Some(qualified_name),
            "{name}"
        );
        if name == "invalid-official-ancestry" {
            assert!(
                result.diagnostics[0]
                    .reason
                    .contains("invalid-chart-ancestry"),
                "{}",
                result.diagnostics[0].reason
            );
        }
    }
}

#[test]
fn chart_parts_require_exact_family_and_descendant_ancestry() {
    let valid_family = family_plot("barChart");
    let valid_axes = axes();
    let outside_plot_area = format!(
        r#"<c:chartSpace xmlns:c="{C}"><c:chart>{valid_family}<c:plotArea>{valid_axes}</c:plotArea></c:chart></c:chartSpace>"#
    );
    let foreign_wrapped_family = chart_space(
        &format!(r#"<x:wrapper xmlns:x="urn:spoof">{valid_family}</x:wrapper>"#),
        valid_axes,
    );
    let foreign_wrapped_series = chart_space(
        &format!(
            r#"<c:barChart><c:barDir val="col"/><x:wrapper xmlns:x="urn:spoof">{}</x:wrapper><c:axId val="10"/><c:axId val="20"/></c:barChart>"#,
            category_series("2")
        ),
        valid_axes,
    );
    for (name, xml, reason) in [
        (
            "family-outside-plot-area",
            outside_plot_area,
            "unsupported-family",
        ),
        (
            "foreign-wrapper-around-family",
            foreign_wrapped_family,
            "unsupported-family",
        ),
        (
            "foreign-wrapper-around-series-cache",
            foreign_wrapped_series,
            "invalid-cache",
        ),
    ] {
        let result = convert_bytes_with_metadata(&package(Some(&xml), Some("rIdChart"), true))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(!result.html.contains("class=\"chart-direct\""), "{name}");
        let diagnostics = result
            .diagnostics
            .iter()
            .filter(|item| item.code == "DRAWINGML_CHART_FALLBACK")
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 1, "{name}: {:?}", result.diagnostics);
        assert!(
            diagnostics[0].reason.contains(reason),
            "{name}: {}",
            diagnostics[0].reason
        );
        assert!(
            diagnostics[0]
                .raw_reference
                .as_deref()
                .is_some_and(|raw| raw.contains("series=0,cache_points=0")),
            "{name}: {:?}",
            diagnostics[0].raw_reference
        );
    }

    let nested_family = chart_space(
        &format!(
            r#"<c:barChart><c:barDir val="col"/>{}<c:lineChart>{}</c:lineChart><c:axId val="10"/><c:axId val="20"/></c:barChart>"#,
            category_series("2"),
            category_series("999")
        ),
        valid_axes,
    );
    let result =
        convert_bytes_with_metadata(&package(Some(&nested_family), Some("rIdChart"), true))
            .expect("nested family package");
    assert!(result.html.contains("class=\"chart-direct\""));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(!result.html.contains("999"));
}

#[test]
fn combination_axes_series_chartex_and_spoof_namespaces_fall_back_once() {
    let mismatched = r#"<c:barChart><c:barDir val="col"/><c:ser><c:cat><c:strLit><c:ptCount val="2"/><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>B</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser><c:axId val="10"/><c:axId val="20"/></c:barChart>"#;
    let cases = [
        (chart_space(&(family_plot("barChart") + &family_plot("lineChart")), axes()), "combination-chart"),
        (chart_space(&family_plot("barChart"), ""), "incompatible-axes"),
        (
            chart_space(&family_plot("scatterChart"), axes()),
            "incompatible-axes",
        ),
        (chart_space(mismatched, axes()), "incompatible-series"),
        (format!(r#"<cx:chartSpace xmlns:cx="{CX}"><cx:chart><cx:plotArea><cx:plotAreaRegion/></cx:plotArea></cx:chart></cx:chartSpace>"#), "chartex"),
        (r#"<x:chartSpace xmlns:x="urn:spoof"><x:chart><x:plotArea><x:barChart/></x:plotArea></x:chart></x:chartSpace>"#.to_owned(), "unsupported-family"),
    ];
    for (xml, expected) in cases {
        let result = convert_bytes_with_metadata(&package(Some(&xml), Some("rIdChart"), true))
            .expect("public conversion");
        let diagnostic_code = if matches!(
            expected,
            "combination-chart" | "incompatible-axes" | "incompatible-series"
        ) {
            "CHART_STRUCTURE_UNSUPPORTED"
        } else {
            "DRAWINGML_CHART_FALLBACK"
        };
        let diagnostics: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|item| item.code == diagnostic_code)
            .collect();
        assert_eq!(diagnostics.len(), 1, "{expected}: {:?}", result.diagnostics);
        assert_eq!(result.diagnostics.len(), 1, "{expected}");
        assert!(
            diagnostics[0].reason.contains(expected),
            "{}",
            diagnostics[0].reason
        );
        assert!(
            diagnostics[0]
                .raw_reference
                .as_deref()
                .is_some_and(|raw| raw.contains("series_summary") && raw.contains("raw_xml"))
        );
        assert!(result.html.contains("chart-placeholder"));
    }
}

const VALID_PNG: &[u8] = &[
    0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0,
    1, 0, 0, 0, 1, 8, 2, 0, 0, 0, 0x90, 0x77, 0x53, 0xde, 0, 0, 0, 12, b'I', b'D', b'A', b'T', 8,
    0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0, 0, 3, 1, 1, 0, 0x18, 0xdd, 0x8d, 0xb1, 0, 0, 0, 0, b'I', b'E',
    b'N', b'D', 0xae, 0x42, 0x60, 0x82,
];

#[test]
fn chartex_relationship_preserves_xml_summary_and_preview_metadata() {
    let xml = format!(
        r#"<cx:chartSpace xmlns:cx="{CX}"><cx:chart><cx:plotArea><cx:plotAreaRegion><cx:series><cx:dataPt idx="0"/></cx:series></cx:plotAreaRegion></cx:plotArea></cx:chart></cx:chartSpace>"#
    );
    let slide = frame(Some("rIdChartEx"), CX, "cx");
    let slide_relationships = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdChartEx" Type="http://schemas.microsoft.com/office/2014/relationships/chartEx" Target="../charts/chartEx1.xml"/></Relationships>"#;
    let chart_relationships = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdPreview" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/a.png"/></Relationships>"#;
    let package = MinimalPptx::new(&slide)
        .with_slide_rels(slide_relationships)
        .with_content_types(PREVIEW_CONTENT_TYPES)
        .with_extra_file("ppt/charts/chartEx1.xml", xml.as_bytes())
        .with_extra_file(
            "ppt/charts/_rels/chartEx1.xml.rels",
            chart_relationships.as_bytes(),
        )
        .with_extra_file("ppt/media/a.png", VALID_PNG)
        .build();
    let result = convert_bytes_with_metadata(&package).expect("ChartEx conversion");
    let diagnostics = result
        .diagnostics
        .iter()
        .filter(|item| item.code == "DRAWINGML_CHART_FALLBACK")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1, "{:?}", result.diagnostics);
    assert!(
        result
            .diagnostics
            .iter()
            .all(|item| item.code != "OOXML_RELATIONSHIP_UNSUPPORTED"),
        "official ChartEx relationship must be inventoried: {:?}",
        result.diagnostics
    );
    let diagnostic = diagnostics[0];
    assert!(diagnostic.reason.contains("chartex"));
    assert!(!diagnostic.reason.contains("missing-relationship"));
    assert_eq!(diagnostic.fallback_kind.as_str(), "preserved-part");
    assert_eq!(
        diagnostic.location.relationship_type.as_deref(),
        Some("http://schemas.microsoft.com/office/2014/relationships/chartEx")
    );
    let raw = diagnostic
        .raw_reference
        .as_deref()
        .expect("raw ChartEx XML");
    assert!(raw.contains("chartSpace"));
    assert!(raw.contains("series=1"));
    assert!(raw.contains("rIdPreview"));
    assert!(
        raw.contains("\"qualified_type\":\"cx:plotAreaRegion\""),
        "{raw}"
    );
    assert!(
        raw.contains("element_inventory") && raw.contains("cx:series=1"),
        "{raw}"
    );
    assert!(raw.contains("\"chart_fallback_mode\":\"preview\""), "{raw}");
    assert!(result.html.contains("data:image/png;base64,iVBORw0KGgo"));
}

fn preview_package_with_payloads(
    relationships: &str,
    first_payload: &[u8],
    second_payload: &[u8],
) -> Vec<u8> {
    let xml = chart_space(&family_plot("stockChart"), "");
    let slide = frame(Some("rIdChart"), C, "c");
    let slide_relationships = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdChart" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#;
    MinimalPptx::new(&slide)
        .with_slide_rels(slide_relationships)
        .with_content_types(PREVIEW_CONTENT_TYPES)
        .with_extra_file("ppt/charts/chart1.xml", xml.as_bytes())
        .with_extra_file("ppt/charts/_rels/chart1.xml.rels", relationships.as_bytes())
        .with_extra_file("ppt/media/a.png", first_payload)
        .with_extra_file("ppt/media/z.png", second_payload)
        .build()
}

fn preview_package(relationships: &str) -> Vec<u8> {
    preview_package_with_payloads(relationships, VALID_PNG, VALID_PNG)
}

#[test]
fn preview_selection_is_order_independent_and_rejects_unsafe_relationships() {
    let opening = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#;
    let safe_a = r#"<Relationship Id="rIdA" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/a.png"/>"#;
    let safe_z = r#"<Relationship Id="rIdZ" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/z.png"/>"#;
    let unsafe_external = r#"<Relationship Id="rIdExternal" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="https://example.test/tracker.png" TargetMode="External"/>"#;
    let unsafe_encoded = r#"<Relationship Id="rIdEncoded" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="%2e%2e/media/z.png"/>"#;
    let first_rels =
        format!("{opening}{safe_z}{unsafe_external}{safe_a}{unsafe_encoded}</Relationships>");
    let reversed_rels =
        format!("{opening}{unsafe_encoded}{safe_a}{unsafe_external}{safe_z}</Relationships>");

    let first = convert_bytes_with_metadata(&preview_package(&first_rels)).expect("first order");
    let reversed =
        convert_bytes_with_metadata(&preview_package(&reversed_rels)).expect("reverse order");
    assert_eq!(first.html, reversed.html);
    assert!(
        first.html.contains("iVBORw0KGgo"),
        "lexicographically first valid preview is embedded"
    );
    let diagnostics: Vec<_> = first
        .diagnostics
        .iter()
        .filter(|item| item.code == "DRAWINGML_CHART_FALLBACK")
        .collect();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].fallback_kind.as_str(), "preserved-part");
}

#[test]
fn declared_png_with_svg_or_script_payload_is_rejected_without_embedding_bytes() {
    let relationships = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdA" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/a.png"/></Relationships>"#;
    for (name, payload, marker) in [
        (
            "svg",
            b"<svg onload=alert(1)>evil</svg>".as_slice(),
            "PHN2Zy",
        ),
        (
            "script",
            b"<script>alert(1)</script>".as_slice(),
            "PHNjcmlwdD",
        ),
    ] {
        let package = preview_package_with_payloads(relationships, payload, VALID_PNG);
        let result = convert_bytes_with_metadata(&package).expect(name);
        let diagnostics = result
            .diagnostics
            .iter()
            .filter(|item| item.code == "DRAWINGML_CHART_FALLBACK")
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 1, "{name}: {:?}", result.diagnostics);
        assert_eq!(result.diagnostics.len(), 1, "{name}");
        assert_eq!(diagnostics[0].fallback_kind.as_str(), "preserved-part");
        assert!(result.html.contains("class=\"chart-placeholder\""));
        assert!(!result.html.contains("data:image/"));
        assert!(!result.html.contains(marker));
        assert!(
            !result
                .html
                .contains(std::str::from_utf8(payload).expect(name))
        );
    }
}

#[test]
fn cache_integrity_failures_fall_back_once_with_raw_xml_and_summary() {
    let cases = [
        (
            "point-count-mismatch",
            classic_family_xml(
                "barChart",
                "<c:barDir val=\"col\"/><c:grouping val=\"clustered\"/><c:ser><c:idx val=\"0\"/><c:order val=\"0\"/><c:cat><c:strLit><c:ptCount val=\"2\"/><c:pt idx=\"0\"><c:v>A</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:ptCount val=\"1\"/><c:pt idx=\"0\"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser>",
                false,
            ),
        ),
        (
            "duplicate-index",
            classic_family_xml(
                "barChart",
                "<c:barDir val=\"col\"/><c:grouping val=\"clustered\"/><c:ser><c:idx val=\"0\"/><c:order val=\"0\"/><c:cat><c:strLit><c:ptCount val=\"2\"/><c:pt idx=\"0\"><c:v>A</c:v></c:pt><c:pt idx=\"0\"><c:v>B</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:ptCount val=\"2\"/><c:pt idx=\"0\"><c:v>1</c:v></c:pt><c:pt idx=\"1\"><c:v>2</c:v></c:pt></c:numLit></c:val></c:ser>",
                false,
            ),
        ),
        (
            "non-finite-numeric",
            classic_family_xml(
                "barChart",
                "<c:barDir val=\"col\"/><c:grouping val=\"clustered\"/><c:ser><c:idx val=\"0\"/><c:order val=\"0\"/><c:cat><c:strLit><c:ptCount val=\"1\"/><c:pt idx=\"0\"><c:v>A</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:ptCount val=\"1\"/><c:pt idx=\"0\"><c:v>NaN</c:v></c:pt></c:numLit></c:val></c:ser>",
                false,
            ),
        ),
        (
            "missing-point-value",
            classic_family_xml(
                "barChart",
                "<c:barDir val=\"col\"/><c:grouping val=\"clustered\"/><c:ser><c:idx val=\"0\"/><c:order val=\"0\"/><c:cat><c:strLit><c:ptCount val=\"1\"/><c:pt idx=\"0\"></c:pt></c:strLit></c:cat><c:val><c:numLit><c:ptCount val=\"1\"/><c:pt idx=\"0\"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser>",
                false,
            ),
        ),
    ];
    for (name, xml) in cases {
        let result =
            convert_bytes_with_metadata(&package(Some(&xml), Some("rIdChart"), true)).expect(name);
        let diagnostics = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "DRAWINGML_CHART_FALLBACK")
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 1, "{name}");
        assert!(diagnostics[0].reason.contains("invalid-cache"), "{name}");
        let raw = diagnostics[0].raw_reference.as_deref().expect(name);
        assert!(
            raw.contains("raw_xml") && raw.contains("series_summary"),
            "{name}"
        );
        assert!(
            raw.contains("\"chart_fallback_mode\":\"placeholder\""),
            "{name}: {raw}"
        );
        assert_eq!(
            diagnostics[0].fallback_kind,
            pptx2html_core::model::FallbackKind::PreservedPart
        );
    }
}

#[test]
fn content_types_and_chart_relationships_require_exact_namespaces_and_types() {
    let xml = classic_family_xml("barChart", &category_series(""), false);
    let fixture = package(Some(&xml), Some("rIdChart"), true);
    let spoof_content_types = rewrite_entry(
        &fixture,
        "[Content_Types].xml",
        br#"<?xml version="1.0"?><Types xmlns="urn:spoof"><Default Extension="png" ContentType="image/png"/></Types>"#,
    );
    let result = convert_bytes_with_metadata(&spoof_content_types).expect("spoof content types");
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "DRAWINGML_CHART_FALLBACK")
            .count(),
        1
    );

    let wrong_chart_rel = rewrite_entry(
        &fixture,
        "ppt/slides/_rels/slide1.xml.rels",
        br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://example.test/chart" Target="../charts/chart1.xml"/></Relationships>"#,
    );
    let result = convert_bytes_with_metadata(&wrong_chart_rel).expect("wrong chart relationship");
    let diagnostics = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "DRAWINGML_CHART_FALLBACK")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].reason.contains("missing-relationship"));

    let spoof_relationship_namespace = rewrite_entry(
        &fixture,
        "ppt/slides/_rels/slide1.xml.rels",
        br#"<?xml version="1.0"?><Relationships xmlns="urn:spoof"><Relationship Id="rIdChart" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#,
    );
    let result = convert_bytes_with_metadata(&spoof_relationship_namespace)
        .expect("spoof relationship namespace");
    let diagnostics = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "DRAWINGML_CHART_FALLBACK")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].reason.contains("missing-relationship"));

    let slide_xml =
        String::from_utf8(package_entry(&fixture, "ppt/slides/slide1.xml")).expect("slide XML");
    let spoof_id_xml = slide_xml.replace(
        "<c:chart xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" r:id=\"rIdChart\"/>",
        "<c:chart xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:r=\"urn:spoof\" r:id=\"rIdChart\"/>",
    );
    assert_ne!(slide_xml, spoof_id_xml);
    let spoof_id = rewrite_entry(&fixture, "ppt/slides/slide1.xml", spoof_id_xml.as_bytes());
    let result = convert_bytes_with_metadata(&spoof_id).expect("spoof relationship ID namespace");
    let diagnostics = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "DRAWINGML_CHART_FALLBACK")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].reason.contains("missing-relationship-id"));
    assert!(!result.html.contains("class=\"chart-direct\""));
}

#[test]
fn chart_reference_failures_are_visible_and_typed() {
    let valid = chart_space(&family_plot("pieChart"), "");
    let cases = [
        (package(Some(&valid), None, true), "missing-relationship-id"),
        (
            package(Some(&valid), Some("rIdChart"), false),
            "missing-relationship",
        ),
        (package(None, Some("rIdChart"), true), "missing-chart-part"),
        (
            package(Some("<c:chartSpace"), Some("rIdChart"), true),
            "invalid-chart-xml",
        ),
    ];
    for (bytes, expected) in cases {
        let result = convert_bytes_with_metadata(&bytes).expect("reference failure is non-fatal");
        let diagnostics: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|item| item.code == "DRAWINGML_CHART_FALLBACK")
            .collect();
        assert_eq!(diagnostics.len(), 1, "{expected}: {:?}", result.diagnostics);
        assert_eq!(result.diagnostics.len(), 1, "{expected}");
        assert!(diagnostics[0].reason.contains(expected));
        assert!(result.html.contains("chart-placeholder"));
    }
}

#[test]
fn chart_doctype_is_rejected_before_invalid_xml_fallback() {
    let bytes = package(
        Some("<!DOCTYPE chartSpace><c:chartSpace"),
        Some("rIdChart"),
        true,
    );
    let error = convert_bytes_with_metadata(&bytes).expect_err("chart DOCTYPE should fail");
    assert!(
        matches!(
            error,
            PptxError::UnsupportedFormat(ref message)
                if message == "XML document type declarations are forbidden: ppt/charts/chart1.xml"
        ),
        "{error}"
    );
}
