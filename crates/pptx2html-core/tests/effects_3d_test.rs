mod fixtures;

use fixtures::MinimalPptx;
use pptx2html_core::convert_bytes_with_metadata;
use pptx2html_core::model::{CapabilityStage, FallbackKind, SupportTier};

const EFFECT_SHAPE: &str = r#"
<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="reflection and 3d"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr>
    <a:xfrm><a:off x="914400" y="914400"/><a:ext cx="1828800" cy="914400"/></a:xfrm>
    <a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom>
    <a:solidFill><a:srgbClr val="4472C4"/></a:solidFill>
    <a:effectLst><a:reflection blurRad="40000" stA="50000" endA="0" stPos="0" endPos="100000" dist="25400" dir="5400000" sy="-100000" algn="b" rotWithShape="1"/></a:effectLst>
    <a:scene3d><a:camera prst="perspectiveFront"/><a:lightRig rig="threePt" dir="t"/></a:scene3d>
    <a:effectDag name="ordered"><a:cont name="first"/><a:cont name="second"/></a:effectDag>
    <a:sp3d extrusionH="120000" contourW="12700" prstMaterial="warmMatte"><a:bevelT w="25400" h="12700" prst="circle"/></a:sp3d>
  </p:spPr>
</p:sp>
"#;

#[test]
fn renders_bounded_reflection_and_truthful_stable_diagnostics() {
    let bytes = MinimalPptx::new(EFFECT_SHAPE).build();
    let first = convert_bytes_with_metadata(&bytes).expect("convert effects fixture");
    let second = convert_bytes_with_metadata(&bytes).expect("convert effects fixture twice");

    assert_eq!(first.html, second.html);
    assert_eq!(first.diagnostics, second.diagnostics);
    assert!(first.html.contains("class=\"shape-reflection\""));
    assert!(
        first
            .html
            .contains("aria-label=\"Approximate DrawingML reflection\"")
    );
    assert!(first.html.contains("background-color: #4472C4"));
    assert!(first.html.contains("pointer-events: none"));
    assert!(first.html.contains("overflow: hidden"));
    assert!(!first.html.contains("NaN"));
    assert!(!first.html.contains("inf"));

    let reflection = first
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "DRAWINGML_REFLECTION_APPROXIMATE")
        .expect("reflection approximation diagnostic");
    assert_eq!(reflection.support_tier, SupportTier::Approximate);
    assert_eq!(reflection.stage, Some(CapabilityStage::Rendered));
    assert_eq!(reflection.fallback_kind, FallbackKind::StyleApproximation);
    assert!(
        reflection
            .reason
            .contains("does not claim PowerPoint fidelity")
    );

    let fallbacks = first
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "DRAWINGML_3D_FALLBACK")
        .collect::<Vec<_>>();
    assert_eq!(fallbacks.len(), 3);
    assert_eq!(
        fallbacks
            .iter()
            .filter_map(|diagnostic| diagnostic.location.qualified_element_name.as_deref())
            .collect::<Vec<_>>(),
        vec!["a:scene3d", "a:effectDag", "a:sp3d"]
    );
    assert!(fallbacks.iter().all(|diagnostic| {
        diagnostic.support_tier == SupportTier::Fallback
            && diagnostic.stage == Some(CapabilityStage::Parsed)
            && diagnostic.fallback_kind == FallbackKind::PreservedEffectMetadata
            && diagnostic.location.slide_index == Some(0)
            && diagnostic.location.part_name.as_deref() == Some("ppt/slides/slide1.xml")
            && diagnostic.location.position.is_some()
            && diagnostic.location.size.is_some()
            && diagnostic
                .raw_reference
                .as_deref()
                .is_some_and(|raw| raw.starts_with("<a:"))
            && diagnostic.reason.contains("not rendered as Office 3D")
    }));
}

#[test]
fn theme_and_non_shape_advanced_effects_emit_specialized_raw_fallbacks() {
    let theme = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="AdversarialTheme">
  <a:themeElements>
    <a:clrScheme name="colors"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1></a:clrScheme>
    <a:fontScheme name="fonts"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
    <a:fmtScheme name="fmt"><a:fillStyleLst/><a:lnStyleLst/><a:effectStyleLst>
      <a:effectStyle><a:effectLst><a:reflection blurRad="12700" stA="40000"/></a:effectLst><a:scene3d><a:camera prst="orthographicFront"/></a:scene3d></a:effectStyle>
    </a:effectStyleLst><a:bgFillStyleLst/></a:fmtScheme>
  </a:themeElements>
</a:theme>"#;
    let shape = r#"<p:sp><p:nvSpPr><p:cNvPr id="7" name="theme effect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr><p:style><a:lnRef idx="0"/><a:fillRef idx="0"/><a:effectRef idx="1"/><a:fontRef idx="minor"/></p:style></p:sp>"#;
    let result =
        convert_bytes_with_metadata(&MinimalPptx::new(shape).with_full_theme(theme).build())
            .expect("convert adversarial theme effects");

    let theme_effects = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "DRAWINGML_THEME_EFFECT_FALLBACK")
        .collect::<Vec<_>>();
    assert_eq!(theme_effects.len(), 2);
    assert_eq!(
        theme_effects
            .iter()
            .filter_map(|diagnostic| diagnostic.location.qualified_element_name.as_deref())
            .collect::<Vec<_>>(),
        vec!["a:reflection", "a:scene3d"]
    );
    assert!(theme_effects.iter().all(|diagnostic| {
        diagnostic.support_tier == SupportTier::Fallback
            && diagnostic.stage == Some(CapabilityStage::Parsed)
            && diagnostic
                .raw_reference
                .as_deref()
                .is_some_and(|raw| raw.starts_with("<a:"))
            && diagnostic.reason.contains("theme effect style")
    }));
    assert!(result.diagnostics.iter().all(|diagnostic| {
        diagnostic.location.qualified_element_name.as_deref() != Some("a:reflection")
            || diagnostic.code != "OOXML_ELEMENT_UNSUPPORTED"
    }));
}

#[test]
fn leading_zero_shape_id_keeps_one_layer_and_one_truthful_diagnostic() {
    let shape = EFFECT_SHAPE.replace("id=\"2\"", "id=\"0017\"");
    let result = convert_bytes_with_metadata(&MinimalPptx::new(&shape).build())
        .expect("convert leading-zero shape identity");

    assert_eq!(result.html.matches("class=\"shape-reflection\"").count(), 1);
    let reflection_diagnostics = result
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.code.as_str(),
                "DRAWINGML_REFLECTION_APPROXIMATE" | "DRAWINGML_REFLECTION_FALLBACK"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(reflection_diagnostics.len(), 1);
    assert_eq!(
        reflection_diagnostics[0].code,
        "DRAWINGML_REFLECTION_APPROXIMATE"
    );
    assert_eq!(
        reflection_diagnostics[0]
            .location
            .relationship_id
            .as_deref(),
        Some("shape-17-effect-0000")
    );
    assert_eq!(
        reflection_diagnostics[0].support_tier,
        SupportTier::Approximate
    );
    assert_eq!(
        reflection_diagnostics[0].stage,
        Some(CapabilityStage::Rendered)
    );
}

#[test]
fn hostile_reflection_values_are_clamped_to_finite_bounded_css() {
    let shape = EFFECT_SHAPE
        .replace("blurRad=\"40000\"", "blurRad=\"999999999999999999999\"")
        .replace("dist=\"25400\"", "dist=\"999999999999999999999\"")
        .replace("stA=\"50000\"", "stA=\"999999999\"")
        .replace("sy=\"-100000\"", "sy=\"-999999999\"");
    let result = convert_bytes_with_metadata(&MinimalPptx::new(&shape).build())
        .expect("convert bounded hostile reflection");

    let reflection_start = result
        .html
        .find("<div class=\"shape-reflection\"")
        .expect("reflection layer starts");
    let reflection_end = result.html[reflection_start..]
        .find("</div>")
        .map(|offset| reflection_start + offset)
        .expect("reflection layer ends");
    let reflection_html = &result.html[reflection_start..reflection_end];
    assert!(reflection_html.contains("filter: blur(20.00pt)"));
    assert!(reflection_html.contains("opacity: 1.000"));
    assert!(!reflection_html.contains("999999999"));
    assert!(!reflection_html.contains("NaN"));
    assert!(!reflection_html.contains("inf"));
}
