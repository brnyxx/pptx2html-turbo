use std::collections::BTreeSet;
use std::fmt::Write;

use crate::model::timing::{
    AnimationEffect, AnimationTrigger, ParsedTimingInventory, TransitionKind,
};
use crate::model::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    Presentation, Shape, ShapeType, SupportTier,
};

pub(super) fn resolve(
    presentation: &Presentation,
    source: &[ParsedTimingInventory],
    diagnostics: &mut Vec<ConversionDiagnostic>,
) -> Vec<ParsedTimingInventory> {
    let mut resolved = source.to_vec();
    for (slide_index, inventory) in resolved.iter_mut().enumerate() {
        let mut rendered_ids = BTreeSet::new();
        if let Some(slide) = presentation.slides.get(slide_index) {
            collect_rendered_ids(&slide.shapes, &mut rendered_ids);
        }
        for group in &mut inventory.groups {
            group.effects.retain(|effect| {
                if rendered_ids.contains(&effect.shape_id) {
                    return true;
                }
                diagnostics.push(ConversionDiagnostic {
                    code: "PRESENTATIONML_TIMING_FALLBACK".to_owned(),
                    family: FeatureFamily::Unsupported,
                    support_tier: SupportTier::Fallback,
                    stage: Some(CapabilityStage::Resolved),
                    location: DiagnosticLocation {
                        slide_index: Some(slide_index),
                        part_name: Some(format!("ppt/slides/slide{}.xml", slide_index + 1)),
                        relationship_id: Some(effect.source_order.to_string()),
                        qualified_element_name: Some("p:timing-target".to_owned()),
                        ..Default::default()
                    },
                    raw_reference: Some(effect.raw_xml.clone()),
                    fallback_kind: FallbackKind::UnknownElement,
                    reason: "Timing target does not resolve to a rendered slide shape".to_owned(),
                });
                false
            });
        }
        inventory.groups.retain(|group| !group.effects.is_empty());
    }
    resolved
}
fn collect_rendered_ids(shapes: &[Shape], ids: &mut BTreeSet<u32>) {
    for shape in shapes.iter().filter(|shape| !shape.hidden) {
        ids.insert(shape.id);
        if let ShapeType::Group(children, _) = &shape.shape_type {
            collect_rendered_ids(children, ids);
        }
    }
}
pub(super) fn presentation_has_timing(timings: &[ParsedTimingInventory]) -> bool {
    timings.iter().any(ParsedTimingInventory::has_runtime)
}
pub(super) fn metadata(timings: &[ParsedTimingInventory]) -> String {
    let mut json = String::from("{");
    let mut first_slide = true;
    for (slide_index, timing) in timings.iter().enumerate() {
        if !timing.has_runtime() {
            continue;
        }
        if !first_slide {
            json.push(',');
        }
        first_slide = false;
        let _ = write!(json, "\"{}\":{{\"groups\":[", slide_index + 1);
        for (group_index, group) in timing.groups.iter().enumerate() {
            if group_index > 0 {
                json.push(',');
            }
            json.push_str("{\"id\":");
            super::fallback::write_json_string(&mut json, &group.identity);
            json.push_str(",\"effects\":[");
            for (effect_index, effect) in group.effects.iter().enumerate() {
                if effect_index > 0 {
                    json.push(',');
                }
                let trigger = match effect.trigger {
                    AnimationTrigger::Click => "click",
                    AnimationTrigger::WithPrevious => "with-previous",
                    AnimationTrigger::AfterPrevious => "after-previous",
                };
                let kind = match effect.effect {
                    AnimationEffect::Appear => "appear",
                    AnimationEffect::Disappear => "disappear",
                    AnimationEffect::FadeIn => "fade-in",
                    AnimationEffect::FadeOut => "fade-out",
                };
                json.push_str("{\"id\":");
                super::fallback::write_json_string(&mut json, &effect.identity);
                let _ = write!(
                    json,
                    ",\"trigger\":\"{trigger}\",\"effect\":\"{kind}\",\"duration\":{},\"delay\":{},\"shape\":{}}}",
                    effect.duration_ms, effect.delay_ms, effect.shape_id
                );
            }
            json.push_str("]}");
        }
        json.push_str("],\"transition\":");
        if let Some(transition) = timing.transition.as_ref() {
            let kind = match transition.kind {
                TransitionKind::Cut => "cut",
                TransitionKind::Fade => "fade",
            };
            json.push_str("{\"id\":");
            super::fallback::write_json_string(&mut json, &transition.identity);
            let _ = write!(
                json,
                ",\"kind\":\"{kind}\",\"duration\":{}}}",
                transition.duration_ms
            );
        } else {
            json.push_str("null");
        }
        json.push('}');
    }
    json.push('}');
    json
}
pub(super) fn suppress_generic_diagnostics(
    timings: &[ParsedTimingInventory],
    diagnostics: &mut Vec<ConversionDiagnostic>,
) {
    diagnostics.retain(|diagnostic| {
        if diagnostic.code != "OOXML_ELEMENT_UNSUPPORTED" {
            return true;
        }
        let Some(slide_index) = diagnostic.location.slide_index else {
            return true;
        };
        let Some(timing) = timings.get(slide_index) else {
            return true;
        };
        let Some(name) = diagnostic.location.qualified_element_name.as_deref() else {
            return true;
        };
        !timing
            .sources
            .iter()
            .any(|source| source.raw_xml.contains(&format!("<{name}")))
    });
}

pub(super) const CSS: &str = ".shape[data-timing-initial=hidden]{visibility:hidden;opacity:0}";

pub(super) const RUNTIME: &str = r#"<script>(()=>{
const root=document.getElementById('pptx2html-timing');if(!root)return;const spec=JSON.parse(root.textContent),state=new Map(),emit=(name,detail)=>document.dispatchEvent(new CustomEvent(name,{detail}));
const run=async(slide,effect)=>{const node=slide.querySelector('[data-pptx-shape-id="'+effect.shape+'"]');if(!node)return;emit('pptx2html:timing-effect',{phase:'start',slide:Number(slide.dataset.slide),group:effect.group,identity:effect.id,effect:effect.effect,shape:effect.shape,delay:effect.delay});let animation;const options={duration:effect.duration,delay:effect.delay,fill:'both'};if(effect.effect==='appear'){node.style.visibility='visible';node.style.opacity='1';animation=node.animate([{opacity:0,visibility:'hidden'},{opacity:1,visibility:'visible'}],options)}else if(effect.effect==='disappear'){animation=node.animate([{opacity:1},{opacity:1}],options);await animation.finished;node.style.visibility='hidden';node.style.opacity='0'}else if(effect.effect==='fade-in'){node.style.visibility='visible';animation=node.animate([{opacity:0},{opacity:1}],options);await animation.finished;node.style.opacity='1'}else{animation=node.animate([{opacity:1},{opacity:0}],options);await animation.finished;node.style.visibility='hidden';node.style.opacity='0'}if(animation&&effect.effect==='appear')await animation.finished;emit('pptx2html:timing-effect',{phase:'complete',slide:Number(slide.dataset.slide),group:effect.group,identity:effect.id,effect:effect.effect,shape:effect.shape,delay:effect.delay})};
const advance=async slide=>{const key=slide.dataset.slide,cfg=spec[key];if(!cfg)return;const current=state.get(key)||{index:0,busy:false};if(current.busy||current.index>=cfg.groups.length)return;current.busy=true;state.set(key,current);const group=cfg.groups[current.index++];emit('pptx2html:timing-group',{phase:'start',slide:Number(key),identity:group.id});let pending=[];for(const item of group.effects){const effect={...item,group:group.id};if(item.trigger==='after-previous'){await Promise.all(pending);pending=[run(slide,effect)]}else pending.push(run(slide,effect))}await Promise.all(pending);current.busy=false;emit('pptx2html:timing-group-complete',{slide:Number(key),identity:group.id})};
document.addEventListener('click',event=>{if(event.defaultPrevented)return;const slide=event.target.closest('.slide');if(slide)void advance(slide)});
window.addEventListener('hashchange',()=>{const slide=document.querySelector(location.hash);if(!slide)return;const transition=spec[slide.dataset.slide]?.transition;if(!transition)return;emit('pptx2html:transition',{phase:'start',slide:Number(slide.dataset.slide),identity:transition.id,transition:transition.kind});if(transition.kind==='cut'){emit('pptx2html:transition-complete',{slide:Number(slide.dataset.slide),identity:transition.id,transition:transition.kind});return}slide.animate([{opacity:0},{opacity:1}],{duration:transition.duration,fill:'none'}).finished.then(()=>emit('pptx2html:transition-complete',{slide:Number(slide.dataset.slide),identity:transition.id,transition:transition.kind}))});
})();</script>"#;
