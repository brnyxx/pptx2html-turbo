use std::fmt::Write;

use crate::model::{AnimationEffect, AnimationTrigger, Presentation, TransitionKind};

pub(super) fn presentation_has_timing(presentation: &Presentation) -> bool {
    presentation
        .slides
        .iter()
        .any(|slide| slide.timing.has_runtime())
}

pub(super) fn metadata(presentation: &Presentation) -> String {
    let mut json = String::from("{");
    let mut first_slide = true;
    for (slide_index, slide) in presentation.slides.iter().enumerate() {
        if !slide.timing.has_runtime() {
            continue;
        }
        if !first_slide {
            json.push(',');
        }
        first_slide = false;
        let _ = write!(json, "\"{}\":{{\"groups\":[", slide_index + 1);
        for (group_index, group) in slide.timing.groups.iter().enumerate() {
            if group_index > 0 {
                json.push(',');
            }
            let _ = write!(json, "{{\"id\":\"{}\",\"effects\":[", group.identity);
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
                let _ = write!(
                    json,
                    "{{\"id\":\"{}\",\"trigger\":\"{trigger}\",\"effect\":\"{kind}\",\"duration\":{},\"shape\":{}}}",
                    effect.identity, effect.duration_ms, effect.shape_id
                );
            }
            json.push_str("]}");
        }
        json.push_str("],\"transition\":");
        if let Some(transition) = slide.timing.transition.as_ref() {
            let kind = match transition.kind {
                TransitionKind::Cut => "cut",
                TransitionKind::Fade => "fade",
            };
            let _ = write!(
                json,
                "{{\"id\":\"{}\",\"kind\":\"{kind}\",\"duration\":{}}}",
                transition.identity, transition.duration_ms
            );
        } else {
            json.push_str("null");
        }
        json.push('}');
    }
    json.push('}');
    json
}

pub(super) const CSS: &str = ".shape[data-timing-initial=hidden]{visibility:hidden;opacity:0}";

pub(super) const RUNTIME: &str = r#"<script>(()=>{
const root=document.getElementById('pptx2html-timing');if(!root)return;const spec=JSON.parse(root.textContent),state=new Map(),emit=(name,detail)=>document.dispatchEvent(new CustomEvent(name,{detail}));
const run=async(slide,effect)=>{const node=slide.querySelector('[data-pptx-shape-id="'+effect.shape+'"]');if(!node)return;emit('pptx2html:timing-effect',{phase:'start',slide:Number(slide.dataset.slide),group:effect.group,identity:effect.id,effect:effect.effect,shape:effect.shape});let animation;if(effect.effect==='appear'){node.style.visibility='visible';node.style.opacity='1';animation=node.animate([{opacity:1},{opacity:1}],{duration:effect.duration,fill:'forwards'})}else if(effect.effect==='disappear'){animation=node.animate([{opacity:1},{opacity:1}],{duration:effect.duration,fill:'forwards'});await animation.finished;node.style.visibility='hidden';node.style.opacity='0'}else if(effect.effect==='fade-in'){node.style.visibility='visible';animation=node.animate([{opacity:0},{opacity:1}],{duration:effect.duration,fill:'forwards'});await animation.finished;node.style.opacity='1'}else{animation=node.animate([{opacity:1},{opacity:0}],{duration:effect.duration,fill:'forwards'});await animation.finished;node.style.visibility='hidden';node.style.opacity='0'}if(animation&&effect.effect==='appear')await animation.finished;emit('pptx2html:timing-effect',{phase:'complete',slide:Number(slide.dataset.slide),group:effect.group,identity:effect.id,effect:effect.effect,shape:effect.shape})};
const advance=async slide=>{const key=slide.dataset.slide,cfg=spec[key];if(!cfg)return;const current=state.get(key)||{index:0,busy:false};if(current.busy||current.index>=cfg.groups.length)return;current.busy=true;state.set(key,current);const group=cfg.groups[current.index++];emit('pptx2html:timing-group',{phase:'start',slide:Number(key),identity:group.id});let pending=[];for(const item of group.effects){const effect={...item,group:group.id};if(item.trigger==='after-previous'){await Promise.all(pending);pending=[run(slide,effect)]}else pending.push(run(slide,effect))}await Promise.all(pending);current.busy=false;emit('pptx2html:timing-group-complete',{slide:Number(key),identity:group.id})};
document.addEventListener('click',event=>{const slide=event.target.closest('.slide');if(slide)void advance(slide)});
window.addEventListener('hashchange',()=>{const slide=document.querySelector(location.hash);if(!slide)return;const transition=spec[slide.dataset.slide]?.transition;if(!transition)return;emit('pptx2html:transition',{phase:'start',slide:Number(slide.dataset.slide),identity:transition.id,transition:transition.kind});if(transition.kind==='cut'){emit('pptx2html:transition-complete',{slide:Number(slide.dataset.slide),identity:transition.id,transition:transition.kind});return}slide.animate([{opacity:0},{opacity:1}],{duration:transition.duration,fill:'none'}).finished.then(()=>emit('pptx2html:transition-complete',{slide:Number(slide.dataset.slide),identity:transition.id,transition:transition.kind}))});
})();</script>"#;
