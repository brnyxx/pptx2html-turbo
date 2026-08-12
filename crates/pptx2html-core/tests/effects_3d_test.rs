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
    <a:scene3d><a:camera prst="perspectiveFront" fov="1800000" zoom="90000"><a:rot lat="100" lon="200" rev="300"/></a:camera><a:lightRig rig="threePt" dir="t"><a:rot lat="400" lon="500" rev="600"/></a:lightRig></a:scene3d>
    <a:effectDag name="ordered"><a:cont name="first"/><a:cont name="second"/></a:effectDag>
    <a:sp3d z="5000" extrusionH="120000" contourW="12700" prstMaterial="warmMatte"><a:bevelT w="25400" h="12700" prst="circle"/><a:bevelB w="12700" h="6350" prst="angle"/></a:sp3d>
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
    assert!(reflection.raw_reference.as_deref().is_some_and(|metadata| {
        metadata.contains("\"kind\":\"reflection\"")
            && metadata.contains("\"blur_radius_emu\":\"40000\"")
            && metadata.contains("\"scale_y\":\"-100000\"")
            && metadata.contains("\"raw_xml\":\"")
    }));
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
                .is_some_and(|metadata| metadata.contains("\"raw_xml\":\"<a:"))
            && diagnostic.reason.contains("not rendered as Office 3D")
    }));
    assert!(
        fallbacks[0]
            .raw_reference
            .as_deref()
            .is_some_and(|metadata| {
                metadata.contains("\"camera_preset\":\"perspectiveFront\"")
                    && metadata.contains("\"camera_fov\":\"1800000\"")
                    && metadata.contains("\"camera_zoom\":\"90000\"")
                    && metadata.contains("\"camera_latitude\":\"100\"")
                    && metadata.contains("\"camera_longitude\":\"200\"")
                    && metadata.contains("\"camera_revolution\":\"300\"")
                    && metadata.contains("\"light_rig\":\"threePt\"")
                    && metadata.contains("\"light_direction\":\"t\"")
                    && metadata.contains("\"light_latitude\":\"400\"")
                    && metadata.contains("\"light_longitude\":\"500\"")
                    && metadata.contains("\"light_revolution\":\"600\"")
            })
    );
    assert!(
        fallbacks[2]
            .raw_reference
            .as_deref()
            .is_some_and(|metadata| {
                metadata.contains("\"material\":\"warmMatte\"")
                    && metadata.contains("\"shape_depth_emu\":\"5000\"")
                    && metadata.contains("\"extrusion_height_emu\":\"120000\"")
                    && metadata.contains("\"contour_width_emu\":\"12700\"")
                    && metadata.contains("\"top_bevel_preset\":\"circle\"")
                    && metadata.contains("\"top_bevel_width_emu\":\"25400\"")
                    && metadata.contains("\"top_bevel_height_emu\":\"12700\"")
                    && metadata.contains("\"bottom_bevel_preset\":\"angle\"")
                    && metadata.contains("\"bottom_bevel_width_emu\":\"12700\"")
                    && metadata.contains("\"bottom_bevel_height_emu\":\"6350\"")
            })
    );
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
                .is_some_and(|metadata| metadata.contains("\"raw_xml\":\"<a:"))
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
fn spoofed_shape_ancestors_never_claim_a_rendered_reflection() {
    let slide = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:spoof="urn:not-presentationml">
  <p:cSld><p:spTree><p:nvGrpSpPr/><p:grpSpPr/>
    <p:sp><p:nvSpPr><p:cNvPr id="9" name="spoof"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <spoof:spPr><a:effectLst><a:reflection blurRad="12700"/></a:effectLst></spoof:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;
    let result = convert_bytes_with_metadata(&MinimalPptx::new("").with_raw_slide(slide).build())
        .expect("convert namespace spoof fixture");

    assert_eq!(result.html.matches("class=\"shape-reflection\"").count(), 0);
    let reflection = result
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.location.qualified_element_name.as_deref() == Some("a:reflection")
        })
        .expect("spoofed reflection remains observable");
    assert_eq!(reflection.code, "DRAWINGML_REFLECTION_FALLBACK");
    assert_eq!(reflection.support_tier, SupportTier::Fallback);
    assert_eq!(reflection.stage, Some(CapabilityStage::Parsed));
}

#[test]
fn effect_encounter_order_is_numeric_past_four_digits() {
    let effects = (0..10_002)
        .map(|index| {
            format!("<a:scene3d><a:camera prst=\"legacyObliqueTopLeft{index}\"/></a:scene3d>")
        })
        .collect::<String>();
    let shape = format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"12\" name=\"many effects\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr>{effects}</p:spPr></p:sp>"
    );
    let result = convert_bytes_with_metadata(&MinimalPptx::new(&shape).build())
        .expect("convert five-digit effect ordering fixture");
    let identities = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "DRAWINGML_3D_FALLBACK")
        .filter_map(|diagnostic| diagnostic.location.relationship_id.as_deref())
        .collect::<Vec<_>>();

    assert_eq!(identities.len(), 10_002);
    assert_eq!(
        &identities[9_998..=10_000],
        [
            "shape-12-effect-9998",
            "shape-12-effect-9999",
            "shape-12-effect-10000",
        ]
    );
}

#[test]
fn huge_effect_dag_has_bounded_raw_payload_with_loss_metadata() {
    let payload = "x".repeat(200_000);
    let shape = format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"22\" name=\"huge dag\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:effectDag name=\"{payload}\"><a:cont name=\"tail\"/></a:effectDag></p:spPr></p:sp>"
    );
    let result = convert_bytes_with_metadata(&MinimalPptx::new(&shape).build())
        .expect("convert bounded huge DAG fixture");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "DRAWINGML_3D_FALLBACK")
        .expect("huge DAG diagnostic");
    let metadata = diagnostic
        .raw_reference
        .as_deref()
        .expect("effect metadata");

    assert!(
        metadata.len() <= 70_000,
        "bounded metadata was {} bytes",
        metadata.len()
    );
    assert!(metadata.contains("\"raw_xml_limit_bytes\":65536"));
    assert!(metadata.contains("\"raw_xml_truncated\":true"));
    assert!(metadata.contains("\"raw_xml_original_bytes\":200"));
    assert!(metadata.contains("\"raw_xml_hash_fnv1a64\":\""));
    assert!(diagnostic.reason.contains("truncated to 65536 bytes"));
    assert!(diagnostic.reason.contains("full length and FNV-1a hash"));
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
