mod fixtures;

use fixtures::MinimalPptx;
use pptx2html_core::convert_bytes_with_metadata;
use pptx2html_core::model::FallbackKind;

const SHAPES: &str = r#"
<p:sp><p:nvSpPr><p:cNvPr id="2" name="first"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1000000" cy="500000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="second"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="1200000" y="100000"/><a:ext cx="1000000" cy="500000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr></p:sp>"#;

const TAIL: &str = r#"
<x:transition xmlns:x="http://schemas.openxmlformats.org/presentationml/2006/main" spd="fast"><x:fade/></x:transition>
<x:timing xmlns:x="http://schemas.openxmlformats.org/presentationml/2006/main"><x:tnLst><x:par><x:cTn id="1" dur="indefinite" nodeType="tmRoot"><x:childTnLst>
<x:par><x:cTn id="10" nodeType="clickEffect"><x:stCondLst><x:cond delay="25"/></x:stCondLst><x:childTnLst><x:animEffect transition="in" filter="fade"><x:cBhvr><x:cTn id="11" dur="120"/><x:tgtEl><x:spTgt spid="2"/></x:tgtEl></x:cBhvr></x:animEffect></x:childTnLst></x:cTn></x:par>
<x:par><x:cTn id="12" nodeType="withEffect"><x:childTnLst><x:set><x:cBhvr><x:cTn id="13" dur="1"/><x:tgtEl><x:spTgt spid="3"/></x:tgtEl></x:cBhvr><x:to><x:strVal val="hidden"/></x:to></x:set></x:childTnLst></x:cTn></x:par>
<x:par><x:cTn id="14" nodeType="afterEffect"><x:childTnLst><x:animEffect transition="out" filter="fade"><x:cBhvr><x:cTn id="15" dur="80"/><x:tgtEl><x:spTgt spid="2"/></x:tgtEl></x:cBhvr></x:animEffect></x:childTnLst></x:cTn></x:par>
<x:par><x:cTn id="20" nodeType="clickEffect"><x:childTnLst><x:set><x:cBhvr><x:cTn id="21" dur="1"/><x:tgtEl><x:spTgt spid="2"/></x:tgtEl></x:cBhvr><x:to><x:strVal val="visible"/></x:to></x:set></x:childTnLst></x:cTn></x:par>
<x:animMotion path="M 0 0 L 1 1"><x:cBhvr><x:cTn id="30" dur="500"/><x:tgtEl><x:spTgt spid="404"/></x:tgtEl></x:cBhvr></x:animMotion>
<x:par><x:cTn id="31" nodeType="clickEffect"><x:childTnLst><x:set><x:cBhvr><x:cTn id="32" dur="1"/><x:tgtEl/></x:cBhvr><x:to><x:strVal val="visible"/></x:to></x:set></x:childTnLst></x:cTn></x:par>
<x:futureTiming mode="preserve"><x:tgtEl><x:spTgt spid="3"/></x:tgtEl></x:futureTiming>
</x:childTnLst></x:cTn></x:par></x:tnLst></x:timing>"#;

fn package() -> Vec<u8> {
    MinimalPptx::new("").with_raw_slide(&format!(r#"<?xml version="1.0"?><p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{SHAPES}</p:spTree></p:cSld>{TAIL}</p:sld>"#)).build()
}

#[test]
fn namespace_aware_parser_preserves_order_and_stable_source_identities() {
    let result = convert_bytes_with_metadata(&package()).expect("timing fixture converts");
    let timing = result
        .html
        .split("id=\"pptx2html-timing\"")
        .nth(1)
        .expect("timing metadata");
    assert!(timing.contains("slide-transition-0"));
    assert!(timing.contains("timing-10-group-0"));
    assert!(timing.contains("timing-11-effect-0"));
    assert!(timing.contains("\"delay\":25"));
    assert!(timing.contains("\"trigger\":\"with-previous\""));
    assert!(timing.contains("\"trigger\":\"after-previous\""));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|item| item.code == "PRESENTATIONML_TIMING_FALLBACK")
            .count(),
        3
    );
}

#[test]
fn renderer_executes_only_bounded_resolved_subset_and_emits_typed_fallbacks() {
    let result = convert_bytes_with_metadata(&package()).expect("timing fixture converts");

    assert!(result.html.contains("data-pptx-shape-id=\"2\""));
    assert!(result.html.contains("data-timing-initial=\"hidden\""));
    assert!(result.html.contains("id=\"pptx2html-timing\""));
    assert!(result.html.contains("pptx2html:timing-effect"));
    assert!(result.html.contains("pptx2html:timing-group-complete"));
    assert!(result.html.contains("pptx2html:transition-complete"));
    let fallback = result
        .diagnostics
        .iter()
        .find(|item| item.code == "PRESENTATIONML_TIMING_FALLBACK")
        .expect("fallback diagnostic");
    assert_eq!(fallback.fallback_kind, FallbackKind::UnknownElement);
    assert!(fallback.raw_reference.as_deref().is_some_and(|raw| {
        raw.starts_with("<x:animMotion") && raw.contains("<x:spTgt spid=\"404\"/>")
    }));
    assert!(result.diagnostics.iter().any(|item| {
        item.code == "PRESENTATIONML_TIMING_FALLBACK"
            && item
                .raw_reference
                .as_deref()
                .is_some_and(|raw| raw.contains("<x:tgtEl/>"))
    }));
    assert!(result.diagnostics.iter().any(|item| {
        item.code == "PRESENTATIONML_TIMING_FALLBACK"
            && item.location.qualified_element_name.as_deref() == Some("x:futureTiming")
    }));
    assert!(result.html.contains("\"delay\":25"));
    assert!(!result.html.contains("<x:animMotion"));
    assert!(result.html.contains("\\u003Cx:animMotion"));
    assert!(
        result
            .diagnostics
            .iter()
            .filter(|item| {
                item.location
                    .qualified_element_name
                    .as_deref()
                    .is_some_and(|name| {
                        matches!(
                            name,
                            "x:animEffect" | "x:animMotion" | "x:set" | "x:futureTiming"
                        )
                    })
            })
            .all(|item| item.code == "PRESENTATIONML_TIMING_FALLBACK")
    );
    assert!(result.html.contains("data-pptx-shape-id=\"3\""));
}

#[test]
fn non_finite_or_out_of_bounds_start_delay_is_a_static_fallback() {
    let delayed = TAIL.replace("delay=\"25\"", "delay=\"10001\"");
    let xml = format!(
        r#"<?xml version="1.0"?><p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{SHAPES}</p:spTree></p:cSld>{delayed}</p:sld>"#
    );
    let result = convert_bytes_with_metadata(&MinimalPptx::new("").with_raw_slide(&xml).build())
        .expect("delay fixture converts");
    assert!(!result.html.contains("timing-11-effect-0"));
    assert!(result.diagnostics.iter().any(|item| {
        item.code == "PRESENTATIONML_TIMING_FALLBACK"
            && item
                .raw_reference
                .as_deref()
                .is_some_and(|raw| raw.contains("spid=\"2\""))
    }));
}

#[test]
fn adversarial_ancestors_namespaces_targets_groups_and_identities_fall_back_safely() {
    for unsupported in ["repeatCount=\"2\"", "repeatDur=\"500\"", "autoRev=\"1\""] {
        let repeated = TAIL.replace(
            "id=\"10\" nodeType=\"clickEffect\"",
            &format!("id=\"10\" nodeType=\"clickEffect\" {unsupported}"),
        );
        let repeated_result =
            convert_bytes_with_metadata(&package_with_tail(&repeated)).expect("repeat converts");
        assert!(repeated_result.diagnostics.iter().any(|item| {
            item.code == "PRESENTATIONML_TIMING_FALLBACK"
                && item
                    .raw_reference
                    .as_deref()
                    .is_some_and(|raw| raw.contains(unsupported) && raw.contains("spid=\"2\""))
        }));
    }

    let foreign = TAIL.replace(
        "<x:spTgt spid=\"2\"/>",
        "<e:spTgt xmlns:e=\"urn:evil\" spid=\"2\"/>",
    );
    let foreign_result =
        convert_bytes_with_metadata(&package_with_tail(&foreign)).expect("foreign target converts");
    assert!(!foreign_result.html.contains("timing-11-effect-0"));

    let root_target = TAIL.replace("<x:spTgt spid=\"2\"/>", "<x:spTgt spid=\"1\"/>");
    let root_result = convert_bytes_with_metadata(&package_with_tail(&root_target))
        .expect("root target converts");
    assert!(!root_result.html.contains("data-pptx-shape-id=\"1\""));
    assert!(
        root_result
            .diagnostics
            .iter()
            .any(|item| item.reason.contains("rendered slide shape"))
    );

    let concurrent = TAIL.replace("</x:animEffect></x:childTnLst></x:cTn></x:par>", "</x:animEffect><x:set><x:cBhvr><x:cTn id=\"16\" dur=\"1\"/><x:tgtEl><x:spTgt spid=\"3\"/></x:tgtEl></x:cBhvr><x:to><x:strVal val=\"visible\"/></x:to></x:set></x:childTnLst></x:cTn></x:par>");
    let concurrent_result = convert_bytes_with_metadata(&package_with_tail(&concurrent))
        .expect("concurrent group converts");
    assert_eq!(
        concurrent_result.html.matches("timing-10-group-").count(),
        1
    );
    assert!(concurrent_result.html.contains("timing-16-effect-1"));

    let hostile_id = TAIL.replace("id=\"10\"", "id=\"10&quot;&lt;/script&gt;\"");
    let hostile_result = convert_bytes_with_metadata(&package_with_tail(&hostile_id))
        .expect("hostile identity converts");
    assert!(!hostile_result.html.contains("10\"</script>"));
    assert!(hostile_result.html.contains("\\u003C"));
}

fn package_with_tail(tail: &str) -> Vec<u8> {
    MinimalPptx::new("").with_raw_slide(&format!(r#"<?xml version="1.0"?><p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{SHAPES}</p:spTree></p:cSld>{tail}</p:sld>"#)).build()
}

#[test]
fn unsupported_namespace_spoofs_do_not_create_timing() {
    let xml = r#"<?xml version="1.0"?><p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:e="urn:evil"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><e:transition><e:fade/></e:transition><e:timing/></p:sld>"#;
    let result = convert_bytes_with_metadata(&MinimalPptx::new("").with_raw_slide(xml).build())
        .expect("spoof converts");
    assert!(!result.html.contains("id=\"pptx2html-timing\""));
}
