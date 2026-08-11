use pptx2html_core::{convert_bytes, convert_bytes_with_metadata};

use super::{fixtures::MinimalPptx, shape_with_pattern};

#[test]
fn renders_known_pattern_as_a_deterministic_repeated_tile() {
    let colors = r#"<a:fgClr><a:srgbClr val="336699"/></a:fgClr><a:bgClr><a:srgbClr val="F2F2F2"/></a:bgClr>"#;
    let bytes = MinimalPptx::new(&shape_with_pattern("diagCross", colors)).build();

    let first = convert_bytes(&bytes).unwrap();
    let second = convert_bytes(&bytes).unwrap();

    assert_eq!(first, second);
    assert!(first.contains("data:image/svg+xml;base64,"));
    assert!(first.contains("background-repeat: repeat"));
    assert!(!first.contains("background-color: #336699"));
}

#[test]
fn preserves_unknown_pattern_semantics_without_a_solid_fallback() {
    let hostile = "futurePattern&lt;/script&gt;";
    let colors = r#"<a:fgClr><a:sysClr val="windowText" lastClr="112233"><a:shade val="50000"/></a:sysClr></a:fgClr>"#;
    let bytes = MinimalPptx::new(&shape_with_pattern(hostile, colors)).build();

    let result = convert_bytes_with_metadata(&bytes).unwrap();
    let diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "DRAWINGML_PATTERN_UNSUPPORTED")
        .collect();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].location.qualified_element_name.as_deref(),
        Some("a:pattFill")
    );
    assert!(
        diagnostics[0]
            .raw_reference
            .as_deref()
            .unwrap()
            .contains(r"futurePattern\u003C/script\u003E")
    );
    assert!(!result.html.contains("background-color: #112233"));
    assert!(!result.html.contains("</script><script"));
}

#[test]
fn renders_svg_table_cell_and_direct_background_patterns() {
    let pattern = r#"<a:pattFill prst="cross"><a:fgClr><a:srgbClr val="336699"/></a:fgClr><a:bgClr><a:srgbClr val="F2F2F2"/></a:bgClr></a:pattFill>"#;
    let slide = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:bg><p:bgPr>{pattern}</p:bgPr></p:bg><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Ellipse"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="ellipse"/>{pattern}</p:spPr></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Custom"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="914400" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l="0" t="0" r="100" b="100"/><a:pathLst><a:path w="100" h="100"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="100" y="0"/></a:lnTo><a:lnTo><a:pt x="50" y="100"/></a:lnTo><a:close/></a:path></a:pathLst></a:custGeom>{pattern}</p:spPr></p:sp>
<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="4" name="Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="914400"/><a:ext cx="1828800" cy="914400"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="1828800"/></a:tblGrid><a:tr h="914400"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>Pattern</a:t></a:r></a:p></a:txBody><a:tcPr>{pattern}</a:tcPr></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>
</p:spTree></p:cSld></p:sld>"#
    );
    let bytes = MinimalPptx::new("").with_raw_slide(&slide).build();

    let html = convert_bytes(&bytes).unwrap();

    assert!(html.matches("<pattern id=\"pattern-s1-cross-").count() >= 2);
    assert!(html.matches("data:image/svg+xml;base64,").count() >= 2);
    assert!(html.contains("data-table-cell=\"r0c0\" style=\"background-image:"));
}

#[test]
fn keeps_identical_unknown_pattern_diagnostics_distinct() {
    let colors = r#"<a:fgClr><a:srgbClr val="112233"/></a:fgClr><a:bgClr><a:srgbClr val="FFFFFF"/></a:bgClr>"#;
    let body = format!(
        "{}{}",
        shape_with_pattern("sameUnknown", colors),
        shape_with_pattern("sameUnknown", colors)
    );
    let result = convert_bytes_with_metadata(&MinimalPptx::new(&body).build()).unwrap();
    let diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "DRAWINGML_PATTERN_UNSUPPORTED")
        .collect();

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(
        diagnostics[0].location.relationship_id.as_deref(),
        Some("pattern-s0-e0")
    );
    assert_eq!(
        diagnostics[1].location.relationship_id.as_deref(),
        Some("pattern-s0-e1")
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        !diagnostic
            .location
            .relationship_id
            .as_deref()
            .unwrap()
            .contains("sameUnknown")
    }));
}

#[test]
fn reports_missing_pattern_colors_without_nonfinite_or_solid_output() {
    let bytes = MinimalPptx::new(&shape_with_pattern("wave", "")).build();
    let result = convert_bytes_with_metadata(&bytes).unwrap();

    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "DRAWINGML_PATTERN_UNSUPPORTED")
            .count(),
        1
    );
    assert!(!result.html.contains("data:image/svg+xml;base64,"));
    assert!(!result.html.contains("NaN"));
    assert!(!result.html.contains("Infinity"));
}
