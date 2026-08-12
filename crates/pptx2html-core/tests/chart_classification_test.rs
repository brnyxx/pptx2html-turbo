mod fixtures;

use fixtures::MinimalPptx;
use pptx2html_core::convert_bytes_with_metadata;

const C: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const CX: &str = "http://schemas.microsoft.com/office/drawing/2014/chartex";

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
    let slide = frame(relationship_id, C, "c");
    let relationships = if relationship_target {
        r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdChart" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#
    } else {
        r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#
    };
    let mut builder = MinimalPptx::new(&slide).with_slide_rels(relationships);
    if let Some(xml) = chart_xml {
        builder = builder.with_extra_file("ppt/charts/chart1.xml", xml.as_bytes());
    }
    builder.build()
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
        r#"<c:bubbleSize><c:numLit><c:pt idx="0"><c:v>2</c:v></c:pt><c:pt idx="1"><c:v>4</c:v></c:pt></c:numLit></c:bubbleSize>"#
    } else {
        ""
    };
    format!(
        r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:xVal><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numLit></c:xVal><c:yVal><c:numLit><c:pt idx="0"><c:v>3</c:v></c:pt><c:pt idx="1"><c:v>4</c:v></c:pt></c:numLit></c:yVal>{sizes}</c:ser>"#
    )
}

fn axes() -> &'static str {
    r#"<c:catAx><c:axId val="10"/><c:crossAx val="20"/></c:catAx><c:valAx><c:axId val="20"/><c:crossAx val="10"/></c:valAx>"#
}

fn numeric_axes() -> &'static str {
    r#"<c:valAx><c:axId val="10"/><c:crossAx val="20"/></c:valAx><c:valAx><c:axId val="20"/><c:crossAx val="10"/></c:valAx>"#
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
            result.html.contains("<div class=\"chart-direct\">"),
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
                diagnostics[0].reason.ends_with(reason),
                "{family}: {}",
                diagnostics[0].reason
            );
            assert_eq!(diagnostics[0].raw_reference.as_deref(), Some(xml.as_str()));
        }
    }
}

#[test]
fn combination_axes_series_chartex_and_spoof_namespaces_fall_back_once() {
    let mismatched = r#"<c:barChart><c:barDir val="col"/><c:ser><c:cat><c:strLit><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>B</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser><c:axId val="10"/><c:axId val="20"/></c:barChart>"#;
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
        let diagnostics: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|item| item.code == "DRAWINGML_CHART_FALLBACK")
            .collect();
        assert_eq!(diagnostics.len(), 1, "{expected}: {:?}", result.diagnostics);
        assert_eq!(result.diagnostics.len(), 1, "{expected}");
        assert!(
            diagnostics[0].reason.ends_with(expected),
            "{}",
            diagnostics[0].reason
        );
        assert_eq!(diagnostics[0].raw_reference.as_deref(), Some(xml.as_str()));
        assert!(result.html.contains("chart-placeholder"));
    }
}

fn preview_package(relationships: &str) -> Vec<u8> {
    let xml = chart_space(&family_plot("stockChart"), "");
    let slide = frame(Some("rIdChart"), C, "c");
    let slide_relationships = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdChart" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#;
    MinimalPptx::new(&slide)
        .with_slide_rels(slide_relationships)
        .with_extra_file("ppt/charts/chart1.xml", xml.as_bytes())
        .with_extra_file("ppt/charts/_rels/chart1.xml.rels", relationships.as_bytes())
        .with_extra_file("ppt/media/a.png", b"A_PREVIEW")
        .with_extra_file("ppt/media/z.png", b"Z_PREVIEW")
        .build()
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
        first.html.contains("QV9QUkVWSUVX"),
        "lexicographically first safe preview is embedded"
    );
    let diagnostics: Vec<_> = first
        .diagnostics
        .iter()
        .filter(|item| item.code == "DRAWINGML_CHART_FALLBACK")
        .collect();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].fallback_kind.as_str(), "chart-preview");
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
        assert!(diagnostics[0].reason.ends_with(expected));
        assert!(result.html.contains("chart-placeholder"));
    }
}
