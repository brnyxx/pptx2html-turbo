mod fixtures;

use fixtures::MinimalPptx;
use pptx2html_core::model::{
    AnimationEffect, AnimationTrigger, FallbackKind, TimingSourceKind, TransitionKind,
};
use pptx2html_core::{convert_bytes_with_metadata, parser::PptxParser};

const SHAPES: &str = r#"
<p:sp><p:nvSpPr><p:cNvPr id="2" name="first"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1000000" cy="500000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="second"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="1200000" y="100000"/><a:ext cx="1000000" cy="500000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr></p:sp>"#;

const TAIL: &str = r#"
<x:transition xmlns:x="http://schemas.openxmlformats.org/presentationml/2006/main" spd="fast"><x:fade/></x:transition>
<x:timing xmlns:x="http://schemas.openxmlformats.org/presentationml/2006/main"><x:tnLst><x:par><x:cTn id="1" dur="indefinite" nodeType="tmRoot"><x:childTnLst>
<x:par><x:cTn id="10" nodeType="clickEffect"><x:childTnLst><x:animEffect transition="in" filter="fade"><x:cBhvr><x:cTn id="11" dur="120"/><x:tgtEl><x:spTgt spid="2"/></x:tgtEl></x:cBhvr></x:animEffect></x:childTnLst></x:cTn></x:par>
<x:par><x:cTn id="12" nodeType="withEffect"><x:childTnLst><x:set><x:cBhvr><x:cTn id="13" dur="1"/><x:tgtEl><x:spTgt spid="3"/></x:tgtEl></x:cBhvr><x:to><x:strVal val="hidden"/></x:to></x:set></x:childTnLst></x:cTn></x:par>
<x:par><x:cTn id="14" nodeType="afterEffect"><x:childTnLst><x:animEffect transition="out" filter="fade"><x:cBhvr><x:cTn id="15" dur="80"/><x:tgtEl><x:spTgt spid="2"/></x:tgtEl></x:cBhvr></x:animEffect></x:childTnLst></x:cTn></x:par>
<x:par><x:cTn id="20" nodeType="clickEffect"><x:childTnLst><x:set><x:cBhvr><x:cTn id="21" dur="1"/><x:tgtEl><x:spTgt spid="2"/></x:tgtEl></x:cBhvr><x:to><x:strVal val="visible"/></x:to></x:set></x:childTnLst></x:cTn></x:par>
<x:animMotion><x:cBhvr><x:cTn id="30" dur="500"/><x:tgtEl><x:spTgt spid="404"/></x:tgtEl></x:cBhvr></x:animMotion>
</x:childTnLst></x:cTn></x:par></x:tnLst></x:timing>"#;

fn package() -> Vec<u8> {
    MinimalPptx::new("").with_raw_slide(&format!(r#"<?xml version="1.0"?><p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{SHAPES}</p:spTree></p:cSld>{TAIL}</p:sld>"#)).build()
}

#[test]
fn namespace_aware_parser_preserves_order_and_stable_source_identities() {
    let presentation = PptxParser::parse_bytes(&package()).expect("timing fixture parses");
    let timing = &presentation.slides[0].timing;

    assert_eq!(timing.sources.len(), 2);
    assert_eq!(timing.sources[0].identity, "slide-transition-0");
    assert_eq!(timing.sources[0].kind, TimingSourceKind::Transition);
    assert!(timing.sources[0].raw_xml.starts_with("<x:transition"));
    assert_eq!(timing.sources[1].identity, "slide-timing-1");
    assert_eq!(timing.sources[1].kind, TimingSourceKind::Timing);
    assert!(timing.sources[1].raw_xml.starts_with("<x:timing"));
    assert_eq!(
        timing.transition.as_ref().map(|item| item.kind),
        Some(TransitionKind::Fade)
    );
    assert_eq!(timing.groups.len(), 2);
    assert_eq!(timing.groups[0].effects.len(), 3);
    assert_eq!(timing.groups[0].effects[0].identity, "timing-11-effect-0");
    assert_eq!(timing.groups[0].effects[0].trigger, AnimationTrigger::Click);
    assert_eq!(timing.groups[0].effects[0].effect, AnimationEffect::FadeIn);
    assert_eq!(
        timing.groups[0].effects[1].trigger,
        AnimationTrigger::WithPrevious
    );
    assert_eq!(
        timing.groups[0].effects[1].effect,
        AnimationEffect::Disappear
    );
    assert_eq!(
        timing.groups[0].effects[2].trigger,
        AnimationTrigger::AfterPrevious
    );
    assert_eq!(timing.groups[1].effects[0].effect, AnimationEffect::Appear);
    assert_eq!(timing.fallbacks.len(), 1);
    assert_eq!(timing.fallbacks[0].identity, "timing-30-fallback-0");
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
    assert_eq!(fallback.fallback_kind, FallbackKind::TimingMetadata);
    assert_eq!(
        fallback.raw_reference.as_deref(),
        Some("timing-30-fallback-0")
    );
    assert!(result.html.contains("data-pptx-shape-id=\"3\""));
}

#[test]
fn unsupported_namespace_spoofs_do_not_create_timing() {
    let xml = r#"<?xml version="1.0"?><p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:e="urn:evil"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><e:transition><e:fade/></e:transition><e:timing/></p:sld>"#;
    let presentation = PptxParser::parse_bytes(&MinimalPptx::new("").with_raw_slide(xml).build())
        .expect("spoof fixture parses");
    assert!(presentation.slides[0].timing.sources.is_empty());
    assert!(presentation.slides[0].timing.transition.is_none());
}
