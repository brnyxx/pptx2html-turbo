use std::fmt::Write;

use base64::Engine;

use crate::resolver::style_ref;

use super::{
    ClrMap, ColorScheme, Fill, FmtScheme, GlowEffect, GradientType, HtmlRenderer, OuterShadow,
    RenderCtx, Shape, ShapeEffects, push_sep,
};

impl HtmlRenderer {
    pub(super) fn resolve_shape_effects(
        shape: &Shape,
        fmt_scheme: Option<&FmtScheme>,
        scheme: Option<&ColorScheme>,
        clr_map: Option<&ClrMap>,
    ) -> Option<ShapeEffects> {
        if shape.effects.outer_shadow.is_some() || shape.effects.glow.is_some() {
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
        if shape.effects.outer_shadow.is_some() || shape.effects.glow.is_some() {
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
