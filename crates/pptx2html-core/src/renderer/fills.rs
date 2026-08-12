use std::fmt::Write;

use base64::Engine;

use crate::resolver::style_ref;

use super::{
    CapabilityStage, ClrMap, ColorScheme, ConversionDiagnostic, DiagnosticLocation, FallbackKind,
    FeatureFamily, Fill, FmtScheme, GlowEffect, GradientType, HtmlRenderer, OuterShadow, Position,
    RenderCtx, Shape, ShapeEffects, Size, SupportTier, push_sep,
};

fn has_explicit_effects(effects: &ShapeEffects) -> bool {
    effects.outer_shadow.is_some()
        || effects.glow.is_some()
        || effects.reflection.is_some()
        || effects.scene_3d.is_some()
        || effects.shape_3d.is_some()
        || !effects.preserved.is_empty()
}

fn bounded(value: Option<f64>, default: f64, minimum: f64, maximum: f64) -> f64 {
    value
        .filter(|value| value.is_finite())
        .unwrap_or(default)
        .clamp(minimum, maximum)
}

fn effect_location(
    slide_index: usize,
    shape_id: u32,
    encounter: usize,
    qualified_name: &str,
    position: Position,
    size: Size,
) -> DiagnosticLocation {
    DiagnosticLocation {
        slide_index: Some(slide_index),
        part_name: Some(format!("ppt/slides/slide{}.xml", slide_index + 1)),
        relationship_id: Some(format!("shape-{shape_id}-effect-{encounter:04}")),
        qualified_element_name: Some(qualified_name.to_owned()),
        position: Some(position),
        size: Some(size),
        ..Default::default()
    }
}

impl HtmlRenderer {
    pub(super) fn resolve_shape_effects(
        shape: &Shape,
        fmt_scheme: Option<&FmtScheme>,
        scheme: Option<&ColorScheme>,
        clr_map: Option<&ClrMap>,
    ) -> Option<ShapeEffects> {
        if has_explicit_effects(&shape.effects) {
            Some(shape.effects.clone())
        } else if let (Some(sr), Some(fmt), Some(cs), Some(cm)) =
            (&shape.style_ref, fmt_scheme, scheme, clr_map)
            && let Some(effect_ref) = &sr.effect_ref
        {
            style_ref::resolve_effect_ref(effect_ref, fmt, cs, cm)
        } else {
            None
        }
    }

    pub(super) fn explicit_shape_effects(shape: &Shape) -> Option<ShapeEffects> {
        if has_explicit_effects(&shape.effects) {
            Some(shape.effects.clone())
        } else {
            None
        }
    }

    pub(super) fn attenuate_shape_effects(effects: &ShapeEffects, factor: f64) -> ShapeEffects {
        let factor = factor.clamp(0.0, 1.0);
        ShapeEffects {
            outer_shadow: effects.outer_shadow.as_ref().map(|shadow| OuterShadow {
                blur_radius: shadow.blur_radius * factor,
                distance: shadow.distance * factor,
                direction: shadow.direction,
                color: shadow.color.clone(),
                alpha: shadow.alpha,
            }),
            glow: effects.glow.as_ref().map(|glow| GlowEffect {
                radius: glow.radius * factor,
                color: glow.color.clone(),
                alpha: glow.alpha,
            }),
            ..effects.clone()
        }
    }

    pub(super) fn effects_to_box_shadows(
        effects: &ShapeEffects,
        ctx: &RenderCtx<'_>,
    ) -> Vec<String> {
        let mut shadows: Vec<String> = Vec::new();

        if let Some(ref shadow) = effects.outer_shadow {
            let angle_rad = shadow.direction.to_radians();
            let offset_x = shadow.distance * angle_rad.cos();
            let offset_y = shadow.distance * angle_rad.sin();
            let blur = shadow.blur_radius;
            let color = ctx
                .color_to_css(&shadow.color)
                .unwrap_or_else(|| "rgba(0,0,0,0.4)".to_string());
            shadows.push(format!(
                "{offset_x:.1}pt {offset_y:.1}pt {blur:.1}pt {color}"
            ));
        }

        if let Some(ref glow) = effects.glow {
            let spread = glow.radius;
            let color = ctx
                .color_to_css(&glow.color)
                .unwrap_or_else(|| "rgba(255,215,0,0.5)".to_string());
            shadows.push(format!("0 0 {spread:.1}pt {spread:.1}pt {color}"));
        }

        shadows
    }

    pub(super) fn effects_to_svg_filter_attr(
        effects: &ShapeEffects,
        ctx: &RenderCtx<'_>,
    ) -> String {
        let mut filters: Vec<String> = Vec::new();

        if let Some(ref shadow) = effects.outer_shadow {
            let angle_rad = shadow.direction.to_radians();
            let offset_x = shadow.distance * angle_rad.cos();
            let offset_y = shadow.distance * angle_rad.sin();
            let blur = shadow.blur_radius;
            let color = ctx
                .color_to_css(&shadow.color)
                .unwrap_or_else(|| "rgba(0,0,0,0.4)".to_string());
            filters.push(format!(
                "drop-shadow({offset_x:.1}pt {offset_y:.1}pt {blur:.1}pt {color})"
            ));
        }

        if let Some(ref glow) = effects.glow {
            let blur = glow.radius;
            let color = ctx
                .color_to_css(&glow.color)
                .unwrap_or_else(|| "rgba(255,215,0,0.5)".to_string());
            filters.push(format!("drop-shadow(0 0 {blur:.1}pt {color})"));
        }

        if filters.is_empty() {
            String::new()
        } else {
            format!(" style=\"filter: {}\"", filters.join(" "))
        }
    }

    pub(super) fn render_reflection_and_record_advanced_effects(
        shape: &Shape,
        resolved_fill: &Fill,
        position: Position,
        size: Size,
        ctx: &RenderCtx<'_>,
        html: &mut String,
    ) {
        let slide_index = ctx.collector.borrow().current_slide_index;
        if let Some(reflection) = shape.effects.reflection.as_ref() {
            let mut reflection_fill = String::new();
            Self::fill_to_css_buf(resolved_fill, ctx, &mut reflection_fill);
            let blur = bounded(reflection.blur_radius, 0.0, 0.0, 20.0);
            let opacity = bounded(reflection.start_alpha, 0.5, 0.0, 1.0);
            let end_opacity = bounded(reflection.end_alpha, 0.0, 0.0, 1.0);
            let start = bounded(reflection.start_position, 0.0, 0.0, 1.0) * 100.0;
            let end = bounded(reflection.end_position, 1.0, 0.0, 1.0) * 100.0;
            let distance = bounded(reflection.distance, 0.0, 0.0, 100.0);
            let direction = bounded(reflection.direction, 90.0, -360.0, 360.0).to_radians();
            let offset_x = (distance * direction.cos()).clamp(-100.0, 100.0);
            let offset_y = (distance * direction.sin()).clamp(-100.0, 100.0);
            let scale_x = bounded(reflection.scale_x.map(f64::abs), 1.0, 0.1, 2.0);
            let scale_y = bounded(reflection.scale_y.map(f64::abs), 1.0, 0.1, 2.0);
            let skew_x = bounded(reflection.skew_x, 0.0, -45.0, 45.0);
            let skew_y = bounded(reflection.skew_y, 0.0, -45.0, 45.0);
            let _ = write!(
                html,
                "<div class=\"shape-reflection\" aria-hidden=\"true\" \
                 aria-label=\"Approximate DrawingML reflection\" \
                 data-support-tier=\"approximate\" style=\"position: absolute; \
                 pointer-events: none; overflow: hidden; left: {offset_x:.2}pt; \
                 top: calc(100% + {offset_y:.2}pt); width: 100%; height: 100%; \
                 transform: scale({scale_x:.3}, -{scale_y:.3}) \
                 skew({skew_x:.2}deg, {skew_y:.2}deg); transform-origin: center center; \
                 opacity: {opacity:.3}; filter: blur({blur:.2}pt); {reflection_fill}; \
                 border-radius: inherit; outline: inherit; \
                 clip-path: inset(0 round 8%); -webkit-mask-image: linear-gradient(to bottom, \
                 rgba(0,0,0,1) {start:.2}%, rgba(0,0,0,{end_opacity:.3}) {end:.2}%); \
                 mask-image: linear-gradient(to bottom, rgba(0,0,0,1) {start:.2}%, \
                 rgba(0,0,0,{end_opacity:.3}) {end:.2}%);\"></div>"
            );
            ctx.collector.borrow_mut().diagnostics.push(ConversionDiagnostic {
                code: "DRAWINGML_REFLECTION_APPROXIMATE".to_owned(),
                family: FeatureFamily::Shapes,
                support_tier: SupportTier::Approximate,
                stage: Some(CapabilityStage::Rendered),
                location: effect_location(
                    slide_index,
                    shape.id,
                    0,
                    "a:reflection",
                    position,
                    size,
                ),
                raw_reference: Some(reflection.raw_xml.clone()),
                fallback_kind: FallbackKind::StyleApproximation,
                reason: "Reflection was rendered with bounded browser blur, transform, and mask primitives; this approximation does not claim PowerPoint fidelity".to_owned(),
            });
        }

        let mut collector = ctx.collector.borrow_mut();
        for (index, effect) in shape.effects.preserved.iter().enumerate() {
            collector.diagnostics.push(ConversionDiagnostic {
                code: "DRAWINGML_3D_FALLBACK".to_owned(),
                family: FeatureFamily::Shapes,
                support_tier: SupportTier::Fallback,
                stage: Some(CapabilityStage::Parsed),
                location: effect_location(
                    slide_index,
                    shape.id,
                    index + 1,
                    effect.kind.qualified_name(),
                    position,
                    size,
                ),
                raw_reference: Some(effect.raw_xml.clone()),
                fallback_kind: FallbackKind::PreservedEffectMetadata,
                reason: format!(
                    "{} was preserved in source order as raw metadata and not rendered as Office 3D",
                    effect.kind.qualified_name()
                ),
            });
        }
    }

    /// Render shape with resolved properties from inheritance cascade
    pub(super) fn fill_to_css_buf(fill: &Fill, ctx: &RenderCtx<'_>, buf: &mut String) {
        match fill {
            Fill::None | Fill::NoFill => {}
            Fill::Solid(sf) => {
                if let Some(color_css) = ctx.color_to_css(&sf.color) {
                    push_sep(buf);
                    let _ = write!(buf, "background-color: {color_css}");
                }
            }
            Fill::Gradient(gf) => {
                let mut has_stops = false;
                let mut stops_buf = String::with_capacity(64);
                for s in &gf.stops {
                    if let Some(c) = ctx.color_to_css(&s.color) {
                        if has_stops {
                            stops_buf.push_str(", ");
                        }
                        let _ = write!(stops_buf, "{c} {:.0}%", s.position * 100.0);
                        has_stops = true;
                    }
                }
                if has_stops {
                    push_sep(buf);
                    match gf.gradient_type {
                        GradientType::Linear => {
                            let _ = write!(
                                buf,
                                "background: linear-gradient({:.0}deg, {stops_buf})",
                                gf.angle
                            );
                        }
                        GradientType::Radial => {
                            let _ = write!(buf, "background: radial-gradient(circle, {stops_buf})");
                        }
                        GradientType::Rectangular => {
                            let _ =
                                write!(buf, "background: radial-gradient(ellipse, {stops_buf})");
                        }
                        GradientType::Shape => {
                            let _ = write!(
                                buf,
                                "background: radial-gradient(closest-side, {stops_buf})"
                            );
                        }
                    }
                }
            }
            Fill::Image(img_fill) => {
                if !img_fill.data.is_empty() {
                    let mime = if img_fill.content_type.is_empty() {
                        "image/png"
                    } else {
                        &img_fill.content_type
                    };
                    push_sep(buf);
                    if ctx.embed_images {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&img_fill.data);
                        let _ = write!(
                            buf,
                            "background-image: url(data:{mime};base64,{b64}); \
                             background-size: cover; background-position: center; \
                             background-repeat: no-repeat"
                        );
                    } else {
                        let url = ctx.register_external_asset("background", mime, &img_fill.data);
                        let _ = write!(
                            buf,
                            "background-image: url({url}); \
                             background-size: cover; background-position: center; \
                             background-repeat: no-repeat"
                        );
                    }
                }
            }
            Fill::Pattern(pattern) => {
                if let Some((foreground, background)) = ctx.pattern_colors(pattern)
                    && let Some(css) = super::patterns::css(pattern, &foreground, &background)
                {
                    push_sep(buf);
                    buf.push_str(&css);
                }
            }
        }
    }

    /// Convert Fill to CSS (theme-aware)
    pub(super) fn fill_to_css(fill: &Fill, ctx: &RenderCtx<'_>) -> String {
        match fill {
            Fill::None | Fill::NoFill => String::new(),
            Fill::Solid(sf) => {
                if let Some(color_css) = ctx.color_to_css(&sf.color) {
                    format!("background-color: {color_css}")
                } else {
                    String::new()
                }
            }
            Fill::Gradient(gf) => {
                let stops: Vec<String> = gf
                    .stops
                    .iter()
                    .filter_map(|s| {
                        ctx.color_to_css(&s.color)
                            .map(|c| format!("{c} {:.0}%", s.position * 100.0))
                    })
                    .collect();
                if stops.is_empty() {
                    String::new()
                } else {
                    let joined = stops.join(", ");
                    match gf.gradient_type {
                        GradientType::Linear => {
                            format!("background: linear-gradient({:.0}deg, {joined})", gf.angle)
                        }
                        GradientType::Radial => {
                            format!("background: radial-gradient(circle, {joined})")
                        }
                        GradientType::Rectangular => {
                            format!("background: radial-gradient(ellipse, {joined})")
                        }
                        GradientType::Shape => {
                            format!("background: radial-gradient(closest-side, {joined})")
                        }
                    }
                }
            }
            Fill::Image(img_fill) => {
                if !img_fill.data.is_empty() {
                    let mime = if img_fill.content_type.is_empty() {
                        "image/png"
                    } else {
                        &img_fill.content_type
                    };
                    let url = if ctx.embed_images {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&img_fill.data);
                        format!("data:{mime};base64,{b64}")
                    } else {
                        ctx.register_external_asset("background", mime, &img_fill.data)
                    };
                    format!(
                        "background-image: url({url}); background-size: cover; background-position: center; background-repeat: no-repeat"
                    )
                } else {
                    String::new()
                }
            }
            Fill::Pattern(pattern) => ctx
                .pattern_colors(pattern)
                .and_then(|(foreground, background)| {
                    super::patterns::css(pattern, &foreground, &background)
                })
                .unwrap_or_default(),
        }
    }
}
