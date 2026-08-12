#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimingInventory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimingSourceKind {
    Transition,
    Timing,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TimingSource {
    pub identity: String,
    pub kind: TimingSourceKind,
    pub raw_xml: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransitionKind {
    Cut,
    Fade,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlideTransition {
    pub identity: String,
    pub kind: TransitionKind,
    pub duration_ms: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnimationTrigger {
    Click,
    WithPrevious,
    AfterPrevious,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnimationEffect {
    Appear,
    Disappear,
    FadeIn,
    FadeOut,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TimingEffect {
    pub identity: String,
    pub source_order: usize,
    pub trigger: AnimationTrigger,
    pub effect: AnimationEffect,
    pub duration_ms: u32,
    pub delay_ms: u32,
    pub shape_id: u32,
    pub raw_xml: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TimingGroup {
    pub identity: String,
    pub source_order: usize,
    pub effects: Vec<TimingEffect>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TimingFallback {
    pub identity: String,
    pub source_order: usize,
    pub qualified_name: String,
    pub raw_xml: String,
    pub reason: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ParsedTimingInventory {
    pub sources: Vec<TimingSource>,
    pub transition: Option<SlideTransition>,
    pub groups: Vec<TimingGroup>,
    pub fallbacks: Vec<TimingFallback>,
}
impl ParsedTimingInventory {
    pub fn has_runtime(&self) -> bool {
        self.transition.is_some() || self.groups.iter().any(|group| !group.effects.is_empty())
    }
    pub fn initially_hidden(&self, shape_id: u32) -> bool {
        self.groups
            .iter()
            .flat_map(|group| &group.effects)
            .find(|effect| effect.shape_id == shape_id)
            .is_some_and(|effect| {
                matches!(
                    effect.effect,
                    AnimationEffect::Appear | AnimationEffect::FadeIn
                )
            })
    }
}
