//! HTML/CSS renderer
//! Presentation model -> self-contained HTML string generation

mod actions;
mod bullets;
mod charts;
mod custom_geometry_diagnostic;
mod embedded_fallback;
mod fallback;
mod fills;
mod geometry;
mod media;
mod pattern_tiles;
mod patterns;
mod picture_bullets;
pub mod provenance;
mod table_style_diagnostics;
mod table_styles;
mod tables;
pub mod text_metrics;

use std::collections::HashMap;
use std::fmt::Write;

use base64::Engine;

use crate::ConversionOptions;
use crate::ConversionResult;
use crate::ExternalAsset;
use crate::error::PptxResult;
use crate::model::presentation::{ClrMap, ColorScheme};
use crate::model::*;
use crate::resolver::inheritance;
use crate::resolver::placeholder;
use provenance::{ProvenanceSource, ProvenanceSubject, RenderedProvenanceEntry};
use text_metrics::{
    FontResolutionEntry, FontResolutionSource, TextWrapPolicy, classify_wrap_policy,
};

use std::cell::RefCell;

/// Mutable state for collecting unresolved elements during rendering
struct UnresolvedCollector {
    elements: Vec<UnresolvedElement>,
    diagnostics: Vec<ConversionDiagnostic>,
    external_assets: Vec<ExternalAsset>,
    font_resolution_entries: Vec<FontResolutionEntry>,
    provenance_entries: Vec<RenderedProvenanceEntry>,
    counter: usize,
    current_slide_index: usize,
    gradient_counter: usize,
    pattern_counter: usize,
    marker_counter: usize,
    asset_counter: usize,
    action_counter: usize,
}

/// Rendering context -- propagates theme/ClrMap references and full presentation
struct RenderCtx<'a> {
    pres: &'a Presentation,
    slide: Option<&'a Slide>,
    scheme: Option<&'a ColorScheme>,
    clr_map: Option<&'a ClrMap>,
    embed_images: bool,
    collector: &'a RefCell<UnresolvedCollector>,
}

struct RunRenderDefaults<'a> {
    para_def_rpr: Option<&'a ParagraphDefRPr>,
    run_defaults: Option<&'a RunDefaults>,
    font_ref_font: Option<&'a str>,
    font_ref_color: Option<&'a ResolvedColor>,
    font_scale: Option<f64>,
}

const DEFAULT_FONT_SIZE_PT: f64 = 18.0;

impl<'a> RenderCtx<'a> {
    fn resolve_color(&self, color: &Color) -> Option<ResolvedColor> {
        color.resolve(self.scheme, self.clr_map)
    }

    fn color_to_css(&self, color: &Color) -> Option<String> {
        self.resolve_color(color)
            .map(|c| c.to_css())
            .or_else(|| color.to_css())
    }

    fn pattern_colors(&self, pattern: &PatternFill) -> Option<(String, String)> {
        if matches!(pattern.preset, PatternPreset::Unknown(_)) {
            self.record_pattern_diagnostic(pattern);
            return None;
        }
        let colors = pattern.foreground.as_ref().and_then(|foreground| {
            pattern.background.as_ref().and_then(|background| {
                Some((
                    self.resolve_color(foreground)?.to_css(),
                    self.resolve_color(background)?.to_css(),
                ))
            })
        });
        if colors.is_none() {
            self.record_pattern_diagnostic(pattern);
        }
        colors
    }

    fn record_pattern_diagnostic(&self, pattern: &PatternFill) {
        let mut coll = self.collector.borrow_mut();
        let encounter = coll.counter;
        coll.counter += 1;
        let slide_index = coll.current_slide_index;
        coll.diagnostics.push(ConversionDiagnostic {
            code: "DRAWINGML_PATTERN_UNSUPPORTED".to_owned(),
            family: FeatureFamily::Shapes,
            support_tier: SupportTier::Fallback,
            stage: Some(CapabilityStage::Rendered),
            location: DiagnosticLocation {
                slide_index: Some(slide_index),
                part_name: Some(format!("ppt/slides/slide{}.xml", slide_index + 1)),
                relationship_id: Some(format!("pattern-s{slide_index}-e{encounter}")),
                qualified_element_name: Some("a:pattFill".to_owned()),
                ..Default::default()
            },
            raw_reference: Some(patterns::raw_semantics(pattern)),
            fallback_kind: FallbackKind::UnknownElement,
            reason:
                "Pattern preset or color cannot be rendered without inventing fallback semantics"
                    .to_owned(),
        });
    }

    fn next_gradient_id(&self) -> String {
        let mut coll = self.collector.borrow_mut();
        let id = coll.gradient_counter;
        coll.gradient_counter += 1;
        format!("grad{id}")
    }

    fn next_pattern_id(&self, preset: &PatternPreset) -> String {
        let mut coll = self.collector.borrow_mut();
        let id = coll.pattern_counter;
        coll.pattern_counter += 1;
        let slide = coll.current_slide_index + 1;
        let safe_preset: String = preset
            .as_ooxml()
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .collect();
        format!("pattern-s{slide}-{safe_preset}-{id}")
    }

    fn next_marker_id(&self, suffix: &str) -> String {
        let mut coll = self.collector.borrow_mut();
        let id = coll.marker_counter;
        coll.marker_counter += 1;
        format!("marker-{suffix}-{id}")
    }

    fn register_external_asset(&self, prefix: &str, mime: &str, data: &[u8]) -> String {
        let mut coll = self.collector.borrow_mut();
        let slide_number = coll.current_slide_index + 1;
        let asset_number = coll.asset_counter;
        coll.asset_counter += 1;

        let ext = match mime {
            "image/jpeg" => "jpg",
            "image/gif" => "gif",
            "image/svg+xml" => "svg",
            "image/webp" => "webp",
            _ => "png",
        };

        let relative_path = format!("images/slide-{slide_number}/{prefix}-{asset_number}.{ext}");
        coll.external_assets.push(ExternalAsset {
            relative_path: relative_path.clone(),
            content_type: mime.to_string(),
            data: data.to_vec(),
        });
        relative_path
    }

    fn push_provenance(&self, entry: RenderedProvenanceEntry) {
        self.collector.borrow_mut().provenance_entries.push(entry);
    }

    fn push_font_resolution(&self, entry: FontResolutionEntry) {
        self.collector
            .borrow_mut()
            .font_resolution_entries
            .push(entry);
    }

    /// Create a slide-scoped context with resolved ClrMap and per-master theme
    fn for_slide(
        &self,
        slide_clr_map: Option<&'a ClrMap>,
        master_theme_idx: Option<usize>,
    ) -> RenderCtx<'a> {
        let scheme = master_theme_idx
            .and_then(|idx| self.pres.themes.get(idx))
            .map(|t| &t.color_scheme)
            .or(self.scheme);
        RenderCtx {
            pres: self.pres,
            slide: self.slide,
            scheme,
            clr_map: slide_clr_map.or(self.clr_map),
            embed_images: self.embed_images,
            collector: self.collector,
        }
    }
}

pub struct HtmlRenderer;

impl HtmlRenderer {
    /// Render entire Presentation to HTML
    pub fn render(pres: &Presentation) -> PptxResult<String> {
        Self::render_with_options(pres, &ConversionOptions::default())
    }

    /// Render entire Presentation to HTML with conversion options
    pub fn render_with_options(
        pres: &Presentation,
        opts: &ConversionOptions,
    ) -> PptxResult<String> {
        Ok(Self::render_with_options_metadata(pres, opts)?.html)
    }

    /// Render entire Presentation to HTML with metadata about unresolved elements
    pub fn render_with_options_metadata(
        pres: &Presentation,
        opts: &ConversionOptions,
    ) -> PptxResult<ConversionResult> {
        Self::render_with_options_diagnostics(pres, opts, Vec::new())
    }

    pub(crate) fn render_with_options_diagnostics(
        pres: &Presentation,
        opts: &ConversionOptions,
        diagnostics: Vec<ConversionDiagnostic>,
    ) -> PptxResult<ConversionResult> {
        let slide_w = pres.slide_size.width.to_px();
        let slide_h = pres.slide_size.height.to_px();
        let slide_scale = opts.effective_scale();

        let collector = RefCell::new(UnresolvedCollector {
            elements: Vec::new(),
            diagnostics,
            external_assets: Vec::new(),
            font_resolution_entries: Vec::new(),
            provenance_entries: Vec::new(),
            counter: 0,
            current_slide_index: 0,
            gradient_counter: 0,
            pattern_counter: 0,
            marker_counter: 0,
            asset_counter: 0,
            action_counter: 0,
        });

        let ctx = RenderCtx {
            pres,
            slide: None,
            scheme: pres.primary_theme().map(|t| &t.color_scheme),
            clr_map: if pres.clr_map.is_empty() {
                None
            } else {
                Some(&pres.clr_map)
            },
            embed_images: opts.embed_images,
            collector: &collector,
        };

        let mut html = String::with_capacity(4096);

        html.push_str("<!DOCTYPE html>\n<html lang=\"ko\">\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str(
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n",
        );
        html.push_str("<meta name=\"generator\" content=\"pptx2html-turbo\">\n");
        if let Some(ref title) = pres.title {
            let _ = writeln!(html, "<title>{}</title>", escape_html(title));
        } else {
            html.push_str("<title>Presentation</title>\n");
        }
        html.push_str("<style>\n");
        html.push_str(&Self::global_css(slide_w, slide_h));
        let has_actions = actions::presentation_has_actions(pres);
        if has_actions {
            html.push_str(actions::CSS);
        }
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");
        html.push_str("<div class=\"pptx-container\">\n");

        let mut slide_count = 0;
        for (i, slide) in pres.slides.iter().enumerate() {
            let one_based = i + 1;
            if !opts.should_include_slide(one_based, slide.hidden) {
                continue;
            }
            collector.borrow_mut().current_slide_index = i;
            Self::render_slide(
                slide,
                one_based,
                slide_w,
                slide_h,
                slide_scale,
                &ctx,
                &mut html,
            );
            slide_count += 1;
        }

        html.push_str("</div>\n");
        media::append_diagnostics(pres, &collector);
        embedded_fallback::append_diagnostics(pres, &collector);
        let mut coll = collector.into_inner();
        fallback::sort_and_deduplicate(&mut coll.diagnostics);
        html.push_str("<script type=\"application/json\" id=\"pptx2html-diagnostics\">");
        html.push_str(&fallback::diagnostics_json(&coll.diagnostics));
        html.push_str("</script>\n");
        if has_actions {
            html.push_str(actions::RUNTIME);
        }
        html.push_str("\n</body>\n</html>");
        Ok(ConversionResult {
            html,
            diagnostics: coll.diagnostics,
            external_assets: coll.external_assets,
            font_resolution_entries: coll.font_resolution_entries,
            provenance_entries: coll.provenance_entries,
            unresolved_elements: coll.elements,
            slide_count,
        })
    }

    fn global_css(slide_w: f64, slide_h: f64) -> String {
        format!(
            r#"* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{ background: #f0f0f0; font-family: 'Calibri', 'Malgun Gothic', sans-serif; }}
.pptx-container {{ display: flex; flex-direction: column; align-items: center; gap: 20px; padding: 20px; }}
.slide-shell {{
  position: relative;
  flex: 0 0 auto;
  overflow: hidden;
}}
.slide {{
  position: relative;
  width: {slide_w:.1}px;
  height: {slide_h:.1}px;
  background: #fff;
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(0,0,0,0.15);
}}
.shape {{
  position: absolute;
  overflow: visible;
}}
.text-body {{
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow-wrap: break-word;
  word-wrap: break-word;
}}
.text-body.v-top {{ justify-content: flex-start; }}
.text-body.v-middle {{ justify-content: center; }}
.text-body.v-bottom {{ justify-content: flex-end; }}
.text-body.h-center {{ align-items: center; }}
.paragraph {{ margin: 0; }}
.run {{ white-space: pre-wrap; word-break: normal; overflow-wrap: normal; }}
.text-body.emergency-wrap .run {{ word-break: break-word; overflow-wrap: anywhere; }}
.text-body.nowrap .run {{ white-space: inherit; word-break: normal; overflow-wrap: normal; }}
img.shape-image {{ width: 100%; height: 100%; object-fit: cover; display: block; }}
.shape-svg {{ position: absolute; top: 0; left: 0; width: 100%; height: 100%; }}
.shape-svg + .text-body {{ position: relative; z-index: 1; }}
.chart-placeholder {{ display: flex; align-items: center; justify-content: center; width: 100%; height: 100%; background: #f8f8f8; border: 1px dashed #ccc; color: #888; font-size: 14px; }}
.chart-direct {{ width: 100%; height: 100%; display: flex; flex-direction: column; justify-content: stretch; gap: 8px; color: #333; }}
.chart-series-label {{ font-size: 12px; font-weight: 600; color: #555; }}
.chart-plot-area {{ display: flex; flex: 1 1 auto; align-items: stretch; gap: 8px; min-height: 0; }}
.chart-plot-main {{ display: flex; flex: 1 1 auto; flex-direction: column; min-width: 0; min-height: 0; }}
.chart-svg {{ width: 100%; height: 100%; }}
.chart-axis-labels {{ display: flex; justify-content: space-between; font-size: 11px; color: #666; gap: 8px; }}
.chart-axis-title {{ font-size: 11px; color: #666; }}
.chart-axis-title-y {{ writing-mode: vertical-rl; transform: rotate(180deg); display: flex; align-items: center; justify-content: center; min-width: 18px; text-align: center; }}
.chart-axis-title-x {{ text-align: center; padding-top: 2px; }}
.chart-data-label {{ font-size: 10px; fill: #444; text-anchor: middle; dominant-baseline: middle; }}
.chart-legend {{ display: flex; flex-wrap: wrap; gap: 10px; font-size: 11px; color: #555; }}
.chart-legend-item {{ display: inline-flex; align-items: center; gap: 4px; }}
.chart-legend-swatch {{ width: 10px; height: 10px; border-radius: 2px; display: inline-block; }}
.chart-bar {{ fill: #4472C4; }}
.chart-bar-horizontal {{ fill: #4472C4; }}
.chart-bar-stacked {{ fill: #4472C4; }}
.chart-line {{ fill: none; stroke-width: 2; }}
.chart-area {{ stroke: none; opacity: 0.35; }}
.chart-point {{ stroke: none; }}
.chart-pie-slice {{ stroke: #fff; stroke-width: 1; }}
.unresolved-element {{ display: flex; align-items: center; justify-content: center; width: 100%; height: 100%; background: #f8f8f8; border: 1px dashed #ccc; color: #888; font-size: 14px; }}
"#
        )
    }

    fn render_slide(
        slide: &Slide,
        num: usize,
        slide_w: f64,
        slide_h: f64,
        slide_scale: f64,
        ctx: &RenderCtx<'_>,
        html: &mut String,
    ) {
        // Look up layout and master for this slide
        let layout = slide.layout_idx.and_then(|idx| ctx.pres.layouts.get(idx));
        let master = layout
            .map(|l| l.master_idx)
            .and_then(|idx| ctx.pres.masters.get(idx));

        // Resolve ClrMap per slide (considering overrides) and per-master theme
        let slide_ctx = if let Some(m) = master {
            let resolved_cm = inheritance::resolve_clr_map(slide, layout, m);
            ctx.for_slide(
                if resolved_cm.is_empty() {
                    None
                } else {
                    Some(resolved_cm)
                },
                Some(m.theme_idx),
            )
        } else {
            ctx.for_slide(None, None)
        };
        let slide_ctx = RenderCtx {
            slide: Some(slide),
            ..slide_ctx
        };

        // Resolve background via inheritance
        let bg = inheritance::resolve_background(slide, layout, master);
        let bg_style = Self::fill_to_css(&bg, &slide_ctx);
        let shell_w = slide_w * slide_scale;
        let shell_h = slide_h * slide_scale;
        let slide_style = if (slide_scale - 1.0).abs() < f64::EPSILON {
            bg_style
        } else {
            format!("{bg_style}; transform: scale({slide_scale:.4}); transform-origin: top left")
        };
        let _ = writeln!(
            html,
            "<div class=\"slide-shell\" data-slide=\"{num}\" style=\"width: {shell_w:.1}px; height: {shell_h:.1}px;\">"
        );
        let _ = writeln!(
            html,
            "<div class=\"slide\" id=\"slide-{num}\" data-slide=\"{num}\" style=\"{slide_style}\">"
        );
        slide_ctx.push_provenance(RenderedProvenanceEntry {
            slide_index: num,
            subject: ProvenanceSubject::SlideBackground,
            shape_name: None,
            fill_source: None,
            border_source: None,
            text_source: None,
            background_source: Some(inheritance::background_source(slide, layout, master)),
        });

        // Render master shapes if show_master_sp is true.
        // Only non-placeholder master shapes (decorative elements) are rendered
        // directly. Placeholder shapes from the master are property-inheritance
        // sources only -- they must never appear as standalone HTML elements.
        let show_master = slide.show_master_sp && layout.is_none_or(|l| l.show_master_sp);
        if show_master && let Some(m) = master {
            for master_shape in &m.shapes {
                if master_shape.hidden {
                    continue;
                }
                // Skip ALL placeholder shapes -- they are property templates,
                // not renderable content.  Slide shapes inherit from them via
                // the layout/master cascade; rendering them here produces
                // duplicate shapes with template text (e.g. "Click to edit Master title style").
                if master_shape.placeholder.is_some() {
                    continue;
                }
                Self::render_shape_resolved(master_shape, None, None, &slide_ctx, html);
            }
        }

        // Render slide shapes with inheritance
        for shape in &slide.shapes {
            if shape.hidden {
                continue;
            }
            // Find matching placeholder in layout/master
            let layout_match = shape.placeholder.as_ref().and_then(|ph| {
                layout.and_then(|l| placeholder::find_matching_placeholder(ph, &l.shapes))
            });
            let master_match = shape.placeholder.as_ref().and_then(|ph| {
                master.and_then(|m| placeholder::find_matching_placeholder(ph, &m.shapes))
            });

            Self::render_shape_resolved(shape, layout_match, master_match, &slide_ctx, html);
        }

        html.push_str("</div>\n</div>\n");
    }

    fn same_shape_kind(lhs: &ShapeType, rhs: &ShapeType) -> bool {
        use ShapeType::*;

        match (lhs, rhs) {
            (Custom(a), Custom(b)) => a == b,
            (Picture(_), Picture(_))
            | (Table(_), Table(_))
            | (Group(_, _), Group(_, _))
            | (Chart(_), Chart(_))
            | (CustomGeom(_), CustomGeom(_))
            | (Unsupported(_), Unsupported(_)) => true,
            _ => std::mem::discriminant(lhs) == std::mem::discriminant(rhs),
        }
    }

    fn inherited_geometry_source<'a>(
        shape: &Shape,
        layout_match: Option<&'a Shape>,
        master_match: Option<&'a Shape>,
    ) -> Option<&'a Shape> {
        shape.placeholder.as_ref()?;

        [layout_match, master_match]
            .into_iter()
            .flatten()
            .find(|candidate| {
                !matches!(
                    candidate.shape_type,
                    ShapeType::Rectangle | ShapeType::TextBox
                ) || candidate.adjust_values.is_some()
                    || candidate.rotation != 0.0
                    || candidate.flip_h
                    || candidate.flip_v
            })
    }

    fn render_shape_resolved(
        shape: &Shape,
        layout_match: Option<&Shape>,
        master_match: Option<&Shape>,
        ctx: &RenderCtx<'_>,
        html: &mut String,
    ) {
        // Resolve position/size via inheritance
        let (pos, size) = inheritance::resolve_position(shape, layout_match, master_match);
        let mut x = pos.x.to_px();
        let mut y = pos.y.to_px();
        let mut w = size.width.to_px();
        let mut h = size.height.to_px();
        let inherited_geometry = Self::inherited_geometry_source(shape, layout_match, master_match);
        let effective_shape_type = if matches!(shape.shape_type, ShapeType::TextBox) {
            inherited_geometry
                .map(|candidate| &candidate.shape_type)
                .unwrap_or(&shape.shape_type)
        } else {
            &shape.shape_type
        };
        let geometry_matches_candidate = inherited_geometry.is_some_and(|candidate| {
            Self::same_shape_kind(effective_shape_type, &candidate.shape_type)
        });
        let effective_adjust_values = shape.adjust_values.as_ref().or_else(|| {
            inherited_geometry.and_then(|candidate| {
                if matches!(shape.shape_type, ShapeType::TextBox) || geometry_matches_candidate {
                    candidate.adjust_values.as_ref()
                } else {
                    None
                }
            })
        });
        let effective_rotation = if (matches!(shape.shape_type, ShapeType::TextBox)
            || geometry_matches_candidate)
            && shape.rotation == 0.0
        {
            inherited_geometry
                .map(|candidate| candidate.rotation)
                .unwrap_or(shape.rotation)
        } else {
            shape.rotation
        };
        let effective_flip_h = if (matches!(shape.shape_type, ShapeType::TextBox)
            || geometry_matches_candidate)
            && !shape.flip_h
        {
            inherited_geometry
                .map(|candidate| candidate.flip_h)
                .unwrap_or(shape.flip_h)
        } else {
            shape.flip_h
        };
        let effective_flip_v = if (matches!(shape.shape_type, ShapeType::TextBox)
            || geometry_matches_candidate)
            && !shape.flip_v
        {
            inherited_geometry
                .map(|candidate| candidate.flip_v)
                .unwrap_or(shape.flip_v)
        } else {
            shape.flip_v
        };

        let anchored_connector = connector_anchor_geometry(shape, ctx);
        if let Some((ax1, ay1, ax2, ay2)) = anchored_connector {
            x = ax1.min(ax2);
            y = ay1.min(ay2);
            w = (ax2 - ax1).abs();
            h = (ay2 - ay1).abs();
        }

        let mut style_buf = String::with_capacity(256);
        let _ = write!(
            style_buf,
            "left: {x:.1}px; top: {y:.1}px; width: {w:.1}px; height: {h:.1}px"
        );

        // Determine SVG preset name early so we know whether to skip CSS fill/border
        let svg_preset_name = match effective_shape_type {
            ShapeType::Ellipse => Some("ellipse"),
            ShapeType::RoundedRectangle => Some("roundRect"),
            ShapeType::Triangle => Some("triangle"),
            ShapeType::Custom(name) => Some(name.as_str()),
            _ => None,
        };

        // For connector shapes with rotation, swap width/height in the CSS box
        // and handle flip via SVG path transform instead of CSS transform.
        // OOXML connectors use rotation to reorient the path (e.g., 270° to make
        // a horizontal connector into a vertical one) — this is a layout hint,
        // not a visual rotation.
        let is_connector = svg_preset_name.is_some_and(|pn| {
            matches!(
                pn,
                "line"
                    | "lineInv"
                    | "straightConnector1"
                    | "bentConnector2"
                    | "bentConnector3"
                    | "bentConnector4"
                    | "bentConnector5"
                    | "curvedConnector2"
                    | "curvedConnector3"
                    | "curvedConnector4"
                    | "curvedConnector5"
            )
        });
        let connector_needs_swap = is_connector
            && ((effective_rotation - 90.0).abs() < 1.0
                || (effective_rotation - 270.0).abs() < 1.0);

        let (w, h) = if connector_needs_swap { (h, w) } else { (w, h) };
        if connector_needs_swap {
            // Rewrite CSS position with swapped dimensions, adjusting offset
            // so the center of the bounding box stays in the same place
            let dx = (size.width.to_px() - size.height.to_px()) / 2.0;
            let dy = (size.height.to_px() - size.width.to_px()) / 2.0;
            style_buf.clear();
            let _ = write!(
                style_buf,
                "left: {:.1}px; top: {:.1}px; width: {w:.1}px; height: {h:.1}px",
                x + dx,
                y + dy
            );
        }

        // Build transform: flip + rotation (skip for connectors with swap)
        if !connector_needs_swap
            && (effective_rotation != 0.0 || effective_flip_h || effective_flip_v)
        {
            let sx = if effective_flip_h { -1 } else { 1 };
            let sy = if effective_flip_v { -1 } else { 1 };
            if effective_flip_h || effective_flip_v {
                if effective_rotation != 0.0 {
                    let _ = write!(
                        style_buf,
                        "; transform: scale({sx},{sy}) rotate({:.1}deg)",
                        effective_rotation
                    );
                } else {
                    let _ = write!(style_buf, "; transform: scale({sx},{sy})");
                }
            } else {
                let _ = write!(
                    style_buf,
                    "; transform: rotate({:.1}deg)",
                    effective_rotation
                );
            }
        }

        // Line shapes with zero width or height need a minimum CSS dimension
        // so the shape div is visible (otherwise browser collapses it)
        if let Some(pn) = svg_preset_name {
            let is_line = matches!(
                pn,
                "line"
                    | "lineInv"
                    | "straightConnector1"
                    | "bentConnector2"
                    | "bentConnector3"
                    | "bentConnector4"
                    | "bentConnector5"
                    | "curvedConnector2"
                    | "curvedConnector3"
                    | "curvedConnector4"
                    | "curvedConnector5"
            );
            if is_line {
                if w < 0.5 {
                    // Vertical line: give minimum width for stroke visibility
                    style_buf.clear();
                    let _ = write!(
                        style_buf,
                        "left: {:.1}px; top: {y:.1}px; width: 2px; height: {h:.1}px",
                        x - 1.0
                    );
                } else if h < 0.5 {
                    // Horizontal line: give minimum height for stroke visibility
                    style_buf.clear();
                    let _ = write!(
                        style_buf,
                        "left: {x:.1}px; top: {:.1}px; width: {w:.1}px; height: 2px",
                        y - 1.0
                    );
                }
            }
        }
        let uses_svg =
            svg_preset_name.is_some() || matches!(effective_shape_type, ShapeType::CustomGeom(_));

        // Resolve fill via inheritance (with style_ref fallback)
        let fmt_scheme = ctx.pres.primary_theme().map(|t| &t.fmt_scheme);
        let resolved_fill = inheritance::resolve_shape_fill_with_theme(
            shape,
            layout_match,
            master_match,
            fmt_scheme,
            ctx.scheme,
            ctx.clr_map,
        );
        // Only emit CSS background for non-SVG shapes; SVG shapes use the fill attribute
        // on the <path> element directly, so CSS background would leak outside the shape path
        if !uses_svg {
            Self::fill_to_css_buf(&resolved_fill, ctx, &mut style_buf);
        }

        // Resolve border via inheritance (with style_ref fallback)
        let resolved_border = inheritance::resolve_border_with_theme(
            shape,
            layout_match,
            master_match,
            fmt_scheme,
            ctx.scheme,
            ctx.clr_map,
        );

        // Only apply CSS outline for non-SVG shapes; SVG shapes use stroke instead.
        // Use outline instead of border to avoid box-sizing: border-box shrinking
        // the content area (text insets should not compete with border thickness).
        if resolved_border.width > 0.0 && !uses_svg {
            let border_color = ctx
                .color_to_css(&resolved_border.color)
                .unwrap_or_else(|| "#000".to_string());
            let border_style = match resolved_border.style {
                BorderStyle::Solid => "solid",
                BorderStyle::Dashed => "dashed",
                BorderStyle::Dotted => "dotted",
                BorderStyle::None => "none",
            };
            let _ = write!(
                style_buf,
                "; outline: {:.1}pt {border_style} {border_color}; outline-offset: {:.1}pt",
                resolved_border.width,
                -(resolved_border.width / 2.0)
            );
        }

        let effective_effects = if uses_svg {
            if !svg_uses_style_ref_effect_fallback(svg_preset_name) {
                Self::explicit_shape_effects(shape)
            } else if let Some(explicit) = Self::explicit_shape_effects(shape) {
                Some(explicit)
            } else {
                Self::resolve_shape_effects(shape, fmt_scheme, ctx.scheme, ctx.clr_map).map(
                    |effects| {
                        Self::attenuate_shape_effects(
                            &effects,
                            svg_style_effect_factor(svg_preset_name),
                        )
                    },
                )
            }
        } else {
            Self::resolve_shape_effects(shape, fmt_scheme, ctx.scheme, ctx.clr_map)
        };
        let empty_svg_adj: HashMap<String, f64> = HashMap::new();
        let svg_adj_values = effective_adjust_values.unwrap_or(&empty_svg_adj);
        let svg_effects = if uses_svg {
            effective_effects.as_ref().map(|effects| {
                scale_svg_effect_blur(
                    effects,
                    svg_preset_shadow_blur_factor(svg_preset_name, svg_adj_values),
                )
            })
        } else {
            None
        };
        let svg_effect_attr = effective_effects
            .as_ref()
            .map(|effects| {
                let adjusted = svg_effects.as_ref().unwrap_or(effects);
                Self::effects_to_svg_filter_attr(adjusted, ctx)
            })
            .unwrap_or_default();

        // Shape-level effects on non-SVG shapes can use CSS box-shadow directly.
        if !uses_svg && let Some(ref effects) = effective_effects {
            let shadows = Self::effects_to_box_shadows(effects, ctx);
            if !shadows.is_empty() {
                let _ = write!(style_buf, "; box-shadow: {}", shadows.join(", "));
            }
        }

        // Cropped images need overflow:hidden on the shape container
        if let ShapeType::Picture(pic) = &shape.shape_type
            && pic.crop.is_some()
        {
            style_buf.push_str("; overflow: hidden");
        }

        let _ = writeln!(html, "<div class=\"shape\" style=\"{style_buf}\">");
        actions::render_shape_surface(&shape.actions, shape.id, ctx, html);

        // Table
        if let ShapeType::Table(ref table) = shape.shape_type {
            Self::render_table(table, shape.id, ctx, html);
            html.push_str("</div>\n");
            return;
        }

        // Group
        if let ShapeType::Group(ref children, ref group_data) = shape.shape_type {
            Self::render_group(children, shape, group_data, ctx, html);
            html.push_str("</div>\n");
            return;
        }

        if let ShapeType::Unsupported(ref data) = shape.shape_type {
            fallback::render_unsupported(data, pos, size, ctx, html);
            return;
        }

        if let ShapeType::Chart(ref chart_data) = shape.shape_type {
            charts::render_chart(chart_data, ctx, w, h, html);
            return;
        }

        // SVG preset shape rendering
        if let Some(preset_name) = svg_preset_name {
            let empty_adj: HashMap<String, f64> = HashMap::new();
            let adj_values = effective_adjust_values.unwrap_or(&empty_adj);
            // Connector/line shapes need a default visible stroke
            let is_line_shape = matches!(
                preset_name,
                "line"
                    | "lineInv"
                    | "straightConnector1"
                    | "bentConnector2"
                    | "bentConnector3"
                    | "bentConnector4"
                    | "bentConnector5"
                    | "curvedConnector2"
                    | "curvedConnector3"
                    | "curvedConnector4"
                    | "curvedConnector5"
            );
            // For line shapes with zero dimension, use a fixed viewBox and custom path
            let svg_w = if is_line_shape && w < 0.5 { 2.0 } else { w };
            let svg_h = if is_line_shape && h < 0.5 { 2.0 } else { h };
            // Generate path: for zero-dim lines, create centered line path directly
            let line_svg_override = if let Some((ax1, ay1, ax2, ay2)) = anchored_connector {
                let aw = (ax2 - ax1).abs().max(0.0);
                let ah = (ay2 - ay1).abs().max(0.0);
                match preset_name {
                    "bentConnector2" => Some(format!("M0,0 L0,{ah:.1} L{aw:.1},{ah:.1}")),
                    "bentConnector3" => {
                        let adj1 = adj_values.get("adj1").copied().unwrap_or(50000.0);
                        let mid = ah * adj1 / 100_000.0;
                        Some(format!(
                            "M0,0 L0,{mid:.1} L{aw:.1},{mid:.1} L{aw:.1},{ah:.1}"
                        ))
                    }
                    _ => Some(format!("M0,0 L{aw:.1},{ah:.1}")),
                }
            } else if is_line_shape && (w < 0.5 || h < 0.5) {
                if w < 0.5 {
                    Some(format!("M1.0,0 L1.0,{svg_h:.1}"))
                } else {
                    Some(format!("M0,1.0 L{svg_w:.1},1.0"))
                }
            } else if connector_needs_swap {
                // Connectors with 90°/270° rotation need rotated path variants.
                // After dimension swap (w↔h), the original path direction is wrong.
                // Generate the correct path based on rotation + flip.
                let flip_h = shape.flip_h;
                let flip_v = shape.flip_v;
                match preset_name {
                    "line" | "lineInv" | "straightConnector1" => {
                        // Straight line: always diagonal or centered, rotation doesn't change path
                        None
                    }
                    "bentConnector2" => {
                        // Original: RIGHT→DOWN. After 270° rotation:
                        // +flipH → DOWN→RIGHT; +flipV → UP→LEFT; no flip → DOWN→LEFT
                        let path = if flip_h {
                            format!("M0,0 L0,{h:.1} L{w:.1},{h:.1}", w = svg_w, h = svg_h)
                        } else if flip_v {
                            format!("M{w:.1},{h:.1} L{w:.1},0 L0,0", w = svg_w, h = svg_h)
                        } else {
                            format!("M{w:.1},0 L{w:.1},{h:.1} L0,{h:.1}", w = svg_w, h = svg_h)
                        };
                        Some(path)
                    }
                    "bentConnector3" => {
                        // Original: RIGHT→DOWN→RIGHT with adj midpoint.
                        // After 270° rotation + flipH → DOWN→RIGHT→DOWN
                        let adj1 = adj_values.get("adj1").copied().unwrap_or(50000.0);
                        let mid = svg_h * adj1 / 100_000.0;
                        let path = if flip_h {
                            format!(
                                "M0,0 L0,{mid:.1} L{w:.1},{mid:.1} L{w:.1},{h:.1}",
                                w = svg_w,
                                mid = mid,
                                h = svg_h
                            )
                        } else {
                            format!(
                                "M{w:.1},0 L{w:.1},{mid:.1} L0,{mid:.1} L0,{h:.1}",
                                w = svg_w,
                                mid = mid,
                                h = svg_h
                            )
                        };
                        Some(path)
                    }
                    _ => None, // Other connectors: fall through to default path + transform
                }
            } else {
                None
            };
            let has_override = line_svg_override.is_some();
            let svg_multi_opt = if is_line_shape {
                None
            } else {
                geometry::preset_shape_multi_svg(preset_name, svg_w, svg_h, adj_values)
            };
            let svg_path_opt = line_svg_override
                .or_else(|| geometry::preset_shape_svg(preset_name, svg_w, svg_h, adj_values));
            if let Some(svg_multi) = svg_multi_opt {
                let (stroke_color, stroke_width) = if resolved_border.width > 0.0 {
                    let c = ctx
                        .color_to_css(&resolved_border.color)
                        .unwrap_or_else(|| "#000".to_string());
                    (c, resolved_border.width * 4.0 / 3.0)
                } else if is_line_shape {
                    let c = ctx
                        .color_to_css(&resolved_border.color)
                        .unwrap_or_else(|| "#000".to_string());
                    (c, 1.0)
                } else {
                    ("none".to_string(), 0.0)
                };
                let stroke_width =
                    stroke_width * svg_preset_stroke_width_factor(Some(preset_name), adj_values);
                let dash_attr = dash_style_to_svg(&resolved_border.dash_style, stroke_width);
                let cap_attr = line_cap_to_svg(&resolved_border.cap);
                let join_attr = line_join_to_svg(&resolved_border.join);
                let miter_limit_attr = line_miter_limit_to_svg(&resolved_border);
                let _ = write!(
                    html,
                    "<svg viewBox=\"0 0 {svg_w:.1} {svg_h:.1}\" class=\"shape-svg\" preserveAspectRatio=\"none\">"
                );
                let grad_id = match &resolved_fill {
                    Fill::Pattern(pattern) => ctx.next_pattern_id(&pattern.preset),
                    _ => ctx.next_gradient_id(),
                };
                let mut defs_buf = String::new();
                let gradient_fill_ref =
                    svg_gradient_def(&resolved_fill, &grad_id, ctx, &mut defs_buf);
                if !defs_buf.is_empty() {
                    html.push_str("<defs>");
                    html.push_str(&defs_buf);
                    html.push_str("</defs>");
                }
                let default_fill = if let Some(ref grad_ref) = gradient_fill_ref {
                    grad_ref.clone()
                } else {
                    ctx.color_to_css(&resolved_fill.color_ref())
                        .unwrap_or_else(|| "none".to_string())
                };
                let base_fill = ctx.resolve_color(&resolved_fill.color_ref());
                let _ = write!(html, "<g{svg_effect_attr}>");
                for path_svg in &svg_multi.paths {
                    let fill = svg_path_fill_to_css(&path_svg.fill, &default_fill, base_fill);
                    let stroke_attr = if path_svg.stroke {
                        format!(
                            "stroke=\"{stroke_color}\" stroke-width=\"{stroke_width:.1}\"\
                             {dash_attr}{cap_attr}{join_attr}{miter_limit_attr}"
                        )
                    } else {
                        "stroke=\"none\"".to_string()
                    };
                    let _ = write!(
                        html,
                        "<path d=\"{}\" fill=\"{fill}\" {stroke_attr}/>",
                        path_svg.d
                    );
                }
                html.push_str("</g></svg>");
            } else if let Some(svg_path) = svg_path_opt {
                // Convert border width from pt to px for SVG (viewBox is in px)
                let (stroke_color, stroke_width) = if resolved_border.width > 0.0 {
                    let c = ctx
                        .color_to_css(&resolved_border.color)
                        .unwrap_or_else(|| "#000".to_string());
                    (c, resolved_border.width * 4.0 / 3.0)
                } else if is_line_shape {
                    // Default 0.75pt stroke for connectors with no explicit line;
                    // still respect parsed color if available
                    let c = ctx
                        .color_to_css(&resolved_border.color)
                        .unwrap_or_else(|| "#000".to_string());
                    (c, 1.0) // 0.75pt = 1.0px
                } else {
                    ("none".to_string(), 0.0)
                };
                let stroke_width = if is_line_shape && preset_name == "lineInv" {
                    stroke_width * 3.2
                } else {
                    stroke_width * svg_preset_stroke_width_factor(Some(preset_name), adj_values)
                };
                let dash_attr = dash_style_to_svg(&resolved_border.dash_style, stroke_width);
                let cap_attr = line_cap_to_svg(&resolved_border.cap);
                let join_attr = line_join_to_svg(&resolved_border.join);
                let miter_limit_attr = line_miter_limit_to_svg(&resolved_border);
                let _ = write!(
                    html,
                    "<svg viewBox=\"0 0 {svg_w:.1} {svg_h:.1}\" class=\"shape-svg\" preserveAspectRatio=\"none\">"
                );
                // Build <defs> for gradient and/or marker definitions
                let grad_id = match &resolved_fill {
                    Fill::Pattern(pattern) => ctx.next_pattern_id(&pattern.preset),
                    _ => ctx.next_gradient_id(),
                };
                let mut defs_buf = String::new();
                let gradient_fill_ref =
                    svg_gradient_def(&resolved_fill, &grad_id, ctx, &mut defs_buf);
                // Emit marker defs for line endings with unique IDs.
                // OOXML tailEnd decorates the start of the path and headEnd
                // decorates the end of the path.
                let mut marker_start_attr = String::new();
                let mut marker_end_attr = String::new();
                if resolved_border.head_end.is_some() || resolved_border.tail_end.is_some() {
                    if let Some(ref te) = resolved_border.tail_end {
                        let mid = ctx.next_marker_id("tail");
                        emit_marker_def(&mut defs_buf, &mid, te, &stroke_color, stroke_width, true);
                        marker_start_attr = format!(" marker-start=\"url(#{mid})\"");
                    }
                    if let Some(ref he) = resolved_border.head_end {
                        let mid = ctx.next_marker_id("head");
                        emit_marker_def(
                            &mut defs_buf,
                            &mid,
                            he,
                            &stroke_color,
                            stroke_width,
                            false,
                        );
                        marker_end_attr = format!(" marker-end=\"url(#{mid})\"");
                    }
                }
                if !defs_buf.is_empty() {
                    html.push_str("<defs>");
                    html.push_str(&defs_buf);
                    html.push_str("</defs>");
                }
                // Determine fill attribute: gradient url > solid color > none
                let fill_attr = if is_line_shape {
                    "none".to_string()
                } else if let Some(ref grad_ref) = gradient_fill_ref {
                    grad_ref.clone()
                } else {
                    ctx.color_to_css(&resolved_fill.color_ref())
                        .unwrap_or_else(|| "none".to_string())
                };
                // Shapes with holes (donut, frame, etc.) need evenodd fill rule
                let fill_rule_attr = if geometry::needs_evenodd_fill(preset_name) {
                    " fill-rule=\"evenodd\""
                } else {
                    ""
                };
                // For connectors with swapped dimensions where the path was NOT
                // directly generated (fallback case), apply flip via SVG transform
                let svg_transform = if connector_needs_swap
                    && !has_override
                    && (effective_flip_h || effective_flip_v)
                {
                    let sx = if effective_flip_h { -1.0 } else { 1.0 };
                    let sy = if effective_flip_v { -1.0 } else { 1.0 };
                    let tx = if effective_flip_h { svg_w } else { 0.0 };
                    let ty = if effective_flip_v { svg_h } else { 0.0 };
                    format!(" transform=\"translate({tx:.1},{ty:.1}) scale({sx},{sy})\"")
                } else {
                    String::new()
                };
                // non-scaling-stroke prevents stroke distortion when viewBox
                // and CSS dimensions have different aspect ratios.
                // Ensure minimum 1.5px for visibility at screen resolution.
                let (non_scaling, stroke_width) = if is_line_shape {
                    (
                        " vector-effect=\"non-scaling-stroke\"",
                        stroke_width.max(1.5),
                    )
                } else {
                    ("", stroke_width)
                };
                let svg_path = if is_line_shape && preset_name == "lineInv" {
                    line_inverse_path_with_overshoot(svg_w, svg_h, 0.8)
                } else {
                    svg_path
                };
                let _ = write!(html, "<g{svg_effect_attr}>");
                if preset_name == "actionButtonInformation"
                    && let Some(base_fill) = ctx.resolve_color(&resolved_fill.color_ref())
                {
                    let button_path = geometry::preset_shape_svg(
                        "actionButtonBlank",
                        svg_w,
                        svg_h,
                        &HashMap::new(),
                    )
                    .unwrap_or_else(|| svg_path.clone());
                    let (circle_path, mark_path) =
                        geometry::action_button_information_icon_paths(svg_w, svg_h);
                    let dark_fill = shade_resolved_color(base_fill, 0.59).to_css();
                    let light_fill = tint_resolved_color(base_fill, 0.60).to_css();
                    let common_attrs = format!(
                        "stroke=\"{stroke_color}\" stroke-width=\"{stroke_width:.1}\"{non_scaling}{dash_attr}{cap_attr}{join_attr}{miter_limit_attr}{svg_transform}"
                    );
                    let _ = writeln!(
                        html,
                        "<path d=\"{button_path}\" fill=\"{fill_attr}\" {common_attrs}/>"
                    );
                    let _ = writeln!(
                        html,
                        "<path d=\"{circle_path}\" fill=\"{dark_fill}\" {common_attrs}/>"
                    );
                    let _ = writeln!(
                        html,
                        "<path d=\"{mark_path}\" fill=\"{light_fill}\" {common_attrs}/>"
                    );
                    html.push_str("</g></svg>");
                } else {
                    let _ = writeln!(
                        html,
                        "<path d=\"{svg_path}\" fill=\"{fill_attr}\"{fill_rule_attr} \
                         stroke=\"{stroke_color}\" stroke-width=\"{stroke_width:.1}\"\
                         {non_scaling}{dash_attr}{cap_attr}{join_attr}{miter_limit_attr}{marker_start_attr}{marker_end_attr}{svg_transform}/>\
                         </g></svg>"
                    );
                }
            }
        }

        // Custom geometry SVG rendering
        if let ShapeType::CustomGeom(geom) = effective_shape_type
            && let Some(svg_geom) = geometry::custom_geometry_svg(geom, w, h)
        {
            // Convert border width from pt to px for SVG (viewBox is in px)
            let (stroke_color, stroke_width) = if resolved_border.width > 0.0 {
                let c = ctx
                    .color_to_css(&resolved_border.color)
                    .unwrap_or_else(|| "#000".to_string());
                (c, resolved_border.width * 4.0 / 3.0)
            } else {
                ("none".to_string(), 0.0)
            };
            let dash_attr = dash_style_to_svg(&resolved_border.dash_style, stroke_width);
            let cap_attr = line_cap_to_svg(&resolved_border.cap);
            let join_attr = line_join_to_svg(&resolved_border.join);
            let miter_limit_attr = line_miter_limit_to_svg(&resolved_border);
            let _ = write!(
                html,
                "<svg viewBox=\"0 0 {w:.1} {h:.1}\" class=\"shape-svg\" preserveAspectRatio=\"none\">"
            );
            // Gradient fill support for custom geometry
            let grad_id = match &resolved_fill {
                Fill::Pattern(pattern) => ctx.next_pattern_id(&pattern.preset),
                _ => ctx.next_gradient_id(),
            };
            let mut defs_buf = String::new();
            let gradient_fill_ref = svg_gradient_def(&resolved_fill, &grad_id, ctx, &mut defs_buf);
            // Emit marker defs for custom geometry arrows.
            // OOXML tailEnd decorates the start of the path and headEnd
            // decorates the end of the path.
            let mut marker_start_attr = String::new();
            let mut marker_end_attr = String::new();
            if resolved_border.head_end.is_some() || resolved_border.tail_end.is_some() {
                if let Some(ref te) = resolved_border.tail_end {
                    let mid = ctx.next_marker_id("tail");
                    emit_marker_def(&mut defs_buf, &mid, te, &stroke_color, stroke_width, true);
                    marker_start_attr = format!(" marker-start=\"url(#{mid})\"");
                }
                if let Some(ref he) = resolved_border.head_end {
                    let mid = ctx.next_marker_id("head");
                    emit_marker_def(&mut defs_buf, &mid, he, &stroke_color, stroke_width, false);
                    marker_end_attr = format!(" marker-end=\"url(#{mid})\"");
                }
            }
            if !defs_buf.is_empty() {
                html.push_str("<defs>");
                html.push_str(&defs_buf);
                html.push_str("</defs>");
            }
            let _ = write!(html, "<g{svg_effect_attr}>");
            let default_fill = if let Some(ref grad_ref) = gradient_fill_ref {
                grad_ref.clone()
            } else {
                ctx.color_to_css(&resolved_fill.color_ref())
                    .unwrap_or_else(|| "none".to_string())
            };
            let base_fill = ctx.resolve_color(&resolved_fill.color_ref());
            for path_svg in &svg_geom.paths {
                let fill = svg_path_fill_to_css(&path_svg.fill, &default_fill, base_fill);
                let stroke_attr = if path_svg.stroke {
                    format!(
                        "stroke=\"{stroke_color}\" stroke-width=\"{stroke_width:.1}\"\
                         {dash_attr}{cap_attr}{join_attr}{miter_limit_attr}{marker_start_attr}{marker_end_attr}"
                    )
                } else {
                    "stroke=\"none\"".to_string()
                };
                let _ = write!(
                    html,
                    "<path d=\"{}\" fill=\"{fill}\" {stroke_attr}/>",
                    path_svg.d
                );
            }
            html.push_str("</g>\n");
            html.push_str("</svg>\n");
        }

        // Image
        if let ShapeType::Picture(pic) = &shape.shape_type
            && !pic.data.is_empty()
        {
            let mime = if pic.content_type.is_empty() {
                "image/png"
            } else {
                &pic.content_type
            };
            let src = if ctx.embed_images {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&pic.data);
                format!("data:{mime};base64,{b64}")
            } else {
                ctx.register_external_asset("image", mime, &pic.data)
            };
            if let Some(ref crop) = pic.crop {
                // OOXML srcRect: l/t/r/b are fractions (0..1) to crop from each
                // edge of the SOURCE image.  The shape bounding box is the final
                // visible area.  We scale the <img> beyond 100% so the full source
                // fills more than the shape, then shift it so the crop region's
                // top-left aligns with the shape origin.  overflow:hidden on the
                // parent div clips the excess.
                let l = crop.left * 100.0; // left crop %
                let t = crop.top * 100.0; // top crop %
                let r = crop.right * 100.0; // right crop %
                let b = crop.bottom * 100.0; // bottom crop %
                let vis_w = 100.0 - l - r;
                let vis_h = 100.0 - t - b;
                if vis_w > 0.001 && vis_h > 0.001 {
                    let img_w_pct = 100.0 / vis_w * 100.0;
                    let img_h_pct = 100.0 / vis_h * 100.0;
                    // Use absolute px offsets for positioning (margin-%
                    // in CSS is always relative to container width, even
                    // for vertical — that gives wrong results).
                    let off_x_px = -(l / 100.0) * w * (img_w_pct / 100.0);
                    let off_y_px = -(t / 100.0) * h * (img_h_pct / 100.0);
                    let _ = writeln!(
                        html,
                        "<img class=\"shape-image\" src=\"{src}\" alt=\"\" style=\"\
                         object-fit: fill; \
                         width: {img_w_pct:.2}%; height: {img_h_pct:.2}%; \
                         margin-left: {off_x_px:.2}px; margin-top: {off_y_px:.2}px\">"
                    );
                } else {
                    // Degenerate crop — show the whole image
                    let _ = writeln!(html, "<img class=\"shape-image\" src=\"{src}\" alt=\"\">");
                }
            } else {
                let _ = writeln!(html, "<img class=\"shape-image\" src=\"{src}\" alt=\"\">");
            }
        }

        // Resolve text style source for this shape's placeholder type
        let text_style_ctx = Self::build_text_style_ctx(shape, layout_match, master_match, ctx);
        let provenance_slide_index = ctx.collector.borrow().current_slide_index + 1;
        ctx.push_provenance(RenderedProvenanceEntry {
            slide_index: provenance_slide_index,
            subject: ProvenanceSubject::Shape,
            shape_name: (!shape.name.is_empty()).then(|| shape.name.clone()),
            fill_source: inheritance::shape_fill_source(
                shape,
                layout_match,
                master_match,
                shape
                    .style_ref
                    .as_ref()
                    .and_then(|s| s.fill_ref.as_ref())
                    .is_some(),
            ),
            border_source: inheritance::border_source(
                shape,
                layout_match,
                master_match,
                shape
                    .style_ref
                    .as_ref()
                    .and_then(|s| s.ln_ref.as_ref())
                    .is_some(),
            ),
            text_source: text_style_ctx.primary_source(),
            background_source: None,
        });

        // Resolve fontRef from <p:style> for font-family and color fallback
        let (font_ref_font, font_ref_color) = Self::resolve_font_ref_font(shape, ctx)
            .map(|(f, c)| (Some(f), c))
            .unwrap_or((None, None));

        // Text
        if let Some(ref text_body) = shape.text_body {
            let effective_auto_fit =
                Self::resolve_text_auto_fit(text_body, layout_match, master_match);
            let effective_vertical_align =
                Self::resolve_text_vertical_align(text_body, layout_match, master_match);
            let effective_word_wrap =
                Self::resolve_text_word_wrap(text_body, layout_match, master_match);
            let effective_margins =
                Self::resolve_text_margins(text_body, layout_match, master_match);
            let effective_anchor_center =
                Self::resolve_text_anchor_center(text_body, layout_match, master_match);
            let effective_text_rotation =
                Self::resolve_text_rotation(text_body, layout_match, master_match);
            let effective_vertical_text =
                Self::resolve_vertical_text(shape, layout_match, master_match);
            let v_class = match effective_vertical_align {
                VerticalAlign::Top => "v-top",
                VerticalAlign::Middle => "v-middle",
                VerticalAlign::Bottom => "v-bottom",
            };
            let rect_insets = custom_geom_text_rect_insets(shape, w, h);
            let mut tb_style = String::with_capacity(128);
            let _ = write!(
                tb_style,
                "padding: {:.1}pt {:.1}pt {:.1}pt {:.1}pt",
                effective_margins.top + rect_insets.0,
                effective_margins.right + rect_insets.1,
                effective_margins.bottom + rect_insets.2,
                effective_margins.left + rect_insets.3,
            );
            if matches!(effective_auto_fit, AutoFit::Shrink) {
                tb_style.push_str("; height: auto; min-height: 100%");
            }
            // Extract auto-fit scaling factors
            let (font_scale, ln_spc_reduction) = match effective_auto_fit {
                AutoFit::Normal {
                    font_scale,
                    line_spacing_reduction,
                } => (
                    font_scale.filter(|value| value.is_finite() && (0.0..=1.0).contains(value)),
                    line_spacing_reduction
                        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value)),
                ),
                _ => (None, None),
            };
            let content_width_px = (w
                - ((effective_margins.left
                    + effective_margins.right
                    + rect_insets.1
                    + rect_insets.3)
                    * (96.0 / 72.0)))
                .max(1.0);
            let wrap_policy = if effective_word_wrap {
                if matches!(effective_auto_fit, AutoFit::Shrink) {
                    TextWrapPolicy::Normal
                } else {
                    let inherited_font_sizes: Vec<Option<f64>> = text_body
                        .paragraphs
                        .iter()
                        .map(|para| {
                            text_style_ctx
                                .get_level_defaults(para.level as usize)
                                .and_then(|defaults| defaults.def_run_props.as_ref())
                                .and_then(|run_defaults| run_defaults.font_size)
                        })
                        .collect();
                    classify_wrap_policy(
                        &text_body.paragraphs,
                        &inherited_font_sizes,
                        content_width_px,
                        font_scale,
                    )
                }
            } else {
                TextWrapPolicy::Normal
            };
            // Text wrapping control
            if !effective_word_wrap {
                tb_style.push_str("; white-space: nowrap");
            } else if matches!(wrap_policy, TextWrapPolicy::Emergency) {
                tb_style.push_str("; overflow-wrap: anywhere");
            }
            // Vertical text rendering
            let mut has_vert270 = false;
            if let Some(vert) = effective_vertical_text {
                match vert.as_str() {
                    "vert" | "wordArtVert" | "eaVert" => {
                        tb_style.push_str("; writing-mode: vertical-rl");
                    }
                    "vert270" => {
                        tb_style.push_str("; writing-mode: vertical-lr");
                        has_vert270 = true;
                    }
                    "mongolianVert" => {
                        tb_style.push_str("; writing-mode: vertical-lr");
                    }
                    _ => {}
                }
            }
            if effective_text_rotation != 0.0 {
                let _ = write!(
                    tb_style,
                    "; transform: rotate({effective_text_rotation:.1}deg)"
                );
            }
            // Add overflow:hidden when text is auto-fitted with fontScale
            if font_scale.is_some() {
                tb_style.push_str("; overflow: hidden");
            }
            // Build combined transform for text-body: vert270 rotate + flip counter-scale
            // PowerPoint flips the shape geometry but keeps text left-to-right,
            // so we counter-flip the text container.
            if has_vert270 || shape.flip_h || shape.flip_v {
                let mut transforms = Vec::new();
                if shape.flip_h || shape.flip_v {
                    let tx = if shape.flip_h { -1 } else { 1 };
                    let ty = if shape.flip_v { -1 } else { 1 };
                    transforms.push(format!("scale({tx},{ty})"));
                }
                if has_vert270 {
                    transforms.push("rotate(180deg)".to_string());
                }
                let _ = write!(tb_style, "; transform: {}", transforms.join(" "));
            }
            let _ = writeln!(
                html,
                "<div class=\"text-body {v_class}{}{}{}\" style=\"{tb_style}\">",
                if effective_word_wrap { "" } else { " nowrap" },
                if matches!(wrap_policy, TextWrapPolicy::Emergency) {
                    " emergency-wrap"
                } else {
                    ""
                },
                if effective_anchor_center {
                    " h-center"
                } else {
                    ""
                }
            );
            // Track auto-number counters per level for this text body
            let mut auto_num_counters: [i32; 9] = [0; 9];
            for para in &text_body.paragraphs {
                Self::render_paragraph_with_defaults(
                    para,
                    ctx,
                    &mut auto_num_counters,
                    bullets::ParagraphRenderContext {
                        text_style: &text_style_ctx,
                        font_ref_font: font_ref_font.as_deref(),
                        font_ref_color: font_ref_color.as_ref(),
                        font_scale,
                        line_spacing_reduction: ln_spc_reduction,
                    },
                    html,
                );
            }
            html.push_str("</div>\n");
        }

        html.push_str("</div>\n");
    }

    /// Build text style context from placeholder type and master txStyles / defaultTextStyle
    fn render_group(
        children: &[Shape],
        parent: &Shape,
        group_data: &GroupData,
        ctx: &RenderCtx<'_>,
        html: &mut String,
    ) {
        // Group coordinate transform:
        // Child coords are in child coordinate space (chOff/chExt).
        // We need to map them to the group's actual bounding box.
        let (parent_pos, parent_size) =
            crate::resolver::inheritance::resolve_position(parent, None, None);
        let ch_off_x = group_data.child_offset.x.to_px();
        let ch_off_y = group_data.child_offset.y.to_px();
        let ch_ext_w = group_data.child_extent.width.to_px();
        let ch_ext_h = group_data.child_extent.height.to_px();
        let grp_w = parent_size.width.to_px();
        let grp_h = parent_size.height.to_px();

        for child in children {
            if child.hidden {
                continue;
            }
            // Transform child position from child coordinate space to group-relative pixels
            let child_x = child.position.x.to_px();
            let child_y = child.position.y.to_px();
            let child_w = child.size.width.to_px();
            let child_h = child.size.height.to_px();

            let (rel_x, rel_y, rel_w, rel_h) = if ch_ext_w > 0.0 && ch_ext_h > 0.0 {
                let scale_x = grp_w / ch_ext_w;
                let scale_y = grp_h / ch_ext_h;
                (
                    (child_x - ch_off_x) * scale_x,
                    (child_y - ch_off_y) * scale_y,
                    child_w * scale_x,
                    child_h * scale_y,
                )
            } else {
                // Fallback: use child coords relative to parent position
                (
                    child_x - parent_pos.x.to_px(),
                    child_y - parent_pos.y.to_px(),
                    child_w,
                    child_h,
                )
            };

            // Create a modified child shape with group-relative coordinates
            let mut child_clone = child.clone();
            child_clone.position = Position {
                x: Emu((rel_x / 96.0 * 914400.0) as i64),
                y: Emu((rel_y / 96.0 * 914400.0) as i64),
            };
            child_clone.size = Size {
                width: Emu((rel_w / 96.0 * 914400.0) as i64),
                height: Emu((rel_h / 96.0 * 914400.0) as i64),
            };
            Self::render_shape_resolved(&child_clone, None, None, ctx, html);
        }
    }
}

fn connector_anchor_geometry(shape: &Shape, ctx: &RenderCtx<'_>) -> Option<(f64, f64, f64, f64)> {
    let slide = ctx.slide?;
    let start = shape.start_connection.as_ref()?;
    let end = shape.end_connection.as_ref()?;
    let start_shape = slide.shapes.iter().find(|s| s.id == start.shape_id)?;
    let end_shape = slide.shapes.iter().find(|s| s.id == end.shape_id)?;
    let (sx, sy) = shape_connection_point(start_shape, start.site_idx)?;
    let (ex, ey) = shape_connection_point(end_shape, end.site_idx)?;
    Some((sx, sy, ex, ey))
}

fn shape_connection_point(shape: &Shape, site_idx: usize) -> Option<(f64, f64)> {
    let ShapeType::CustomGeom(ref geom) = shape.shape_type else {
        return None;
    };
    let site = geom.connection_sites.get(site_idx)?;
    let path = geom
        .paths
        .iter()
        .find(|p| p.width > 0.0 && p.height > 0.0)?;

    let width_px = shape.size.width.to_px();
    let height_px = shape.size.height.to_px();
    let mut local_x = width_px * (site.x / path.width);
    let mut local_y = height_px * (site.y / path.height);

    if shape.flip_h {
        local_x = width_px - local_x;
    }
    if shape.flip_v {
        local_y = height_px - local_y;
    }

    if shape.rotation != 0.0 {
        let cx = width_px / 2.0;
        let cy = height_px / 2.0;
        let rad = shape.rotation.to_radians();
        let dx = local_x - cx;
        let dy = local_y - cy;
        local_x = cx + dx * rad.cos() - dy * rad.sin();
        local_y = cy + dx * rad.sin() + dy * rad.cos();
    }

    Some((
        shape.position.x.to_px() + local_x,
        shape.position.y.to_px() + local_y,
    ))
}

fn custom_geom_text_rect_insets(
    shape: &Shape,
    width_px: f64,
    height_px: f64,
) -> (f64, f64, f64, f64) {
    let ShapeType::CustomGeom(ref geom) = shape.shape_type else {
        return (0.0, 0.0, 0.0, 0.0);
    };
    let Some(ref rect) = geom.text_rect else {
        return (0.0, 0.0, 0.0, 0.0);
    };
    let Some(path) = geom.paths.iter().find(|p| p.width > 0.0 && p.height > 0.0) else {
        return (0.0, 0.0, 0.0, 0.0);
    };

    let left_px = width_px * (rect.left / path.width);
    let top_px = height_px * (rect.top / path.height);
    let right_px = width_px * ((path.width - rect.right) / path.width);
    let bottom_px = height_px * ((path.height - rect.bottom) / path.height);

    (
        px_to_pt(top_px.max(0.0)),
        px_to_pt(right_px.max(0.0)),
        px_to_pt(bottom_px.max(0.0)),
        px_to_pt(left_px.max(0.0)),
    )
}

fn px_to_pt(px: f64) -> f64 {
    px * 3.0 / 4.0
}

/// Append a "; " separator to the style buffer if it's non-empty
#[inline]
fn push_sep(buf: &mut String) {
    if !buf.is_empty() {
        buf.push_str("; ");
    }
}

/// Format auto-numbered bullet label based on OOXML numbering type
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Convert DashStyle to SVG stroke-dasharray attribute string (including leading space)
fn dash_style_to_svg(style: &DashStyle, stroke_width: f64) -> String {
    let sw = if stroke_width > 0.0 {
        stroke_width
    } else {
        1.0
    };
    match style {
        DashStyle::Solid => String::new(),
        DashStyle::Dash => format!(" stroke-dasharray=\"{:.1} {:.1}\"", 8.0 * sw, 4.0 * sw),
        DashStyle::Dot => format!(" stroke-dasharray=\"{:.1} {:.1}\"", 2.0 * sw, 2.0 * sw),
        DashStyle::DashDot => format!(
            " stroke-dasharray=\"{:.1} {:.1} {:.1} {:.1}\"",
            8.0 * sw,
            4.0 * sw,
            2.0 * sw,
            4.0 * sw
        ),
        DashStyle::LongDash => format!(" stroke-dasharray=\"{:.1} {:.1}\"", 12.0 * sw, 4.0 * sw),
        DashStyle::LongDashDot => format!(
            " stroke-dasharray=\"{:.1} {:.1} {:.1} {:.1}\"",
            12.0 * sw,
            4.0 * sw,
            2.0 * sw,
            4.0 * sw
        ),
        DashStyle::LongDashDotDot => format!(
            " stroke-dasharray=\"{:.1} {:.1} {:.1} {:.1} {:.1} {:.1}\"",
            12.0 * sw,
            4.0 * sw,
            2.0 * sw,
            4.0 * sw,
            2.0 * sw,
            4.0 * sw
        ),
        DashStyle::SystemDash => format!(" stroke-dasharray=\"{:.1} {:.1}\"", 6.0 * sw, 3.0 * sw),
        DashStyle::SystemDot => format!(" stroke-dasharray=\"{:.1} {:.1}\"", 1.0 * sw, 2.0 * sw),
        DashStyle::SystemDashDot => format!(
            " stroke-dasharray=\"{:.1} {:.1} {:.1} {:.1}\"",
            3.0 * sw,
            1.0 * sw,
            1.0 * sw,
            1.0 * sw
        ),
        DashStyle::SystemDashDotDot => format!(
            " stroke-dasharray=\"{:.1} {:.1} {:.1} {:.1} {:.1} {:.1}\"",
            3.0 * sw,
            1.0 * sw,
            1.0 * sw,
            1.0 * sw,
            1.0 * sw,
            1.0 * sw
        ),
    }
}

/// Convert DashStyle to CSS border-style keyword
fn dash_style_to_css(style: &DashStyle) -> &'static str {
    match style {
        DashStyle::Solid => "solid",
        DashStyle::Dash | DashStyle::LongDash | DashStyle::SystemDash => "dashed",
        DashStyle::Dot | DashStyle::SystemDot => "dotted",
        DashStyle::DashDot
        | DashStyle::LongDashDot
        | DashStyle::LongDashDotDot
        | DashStyle::SystemDashDot
        | DashStyle::SystemDashDotDot => "dashed",
    }
}

/// Convert LineCap to SVG stroke-linecap attribute string (including leading space).
/// Returns empty string for Flat (SVG default "butt").
fn svg_style_effect_factor(preset_name: Option<&str>) -> f64 {
    match preset_name {
        Some("sun") => 0.65,
        Some(
            "rightArrow" | "upDownArrow" | "notchedRightArrow" | "stripedRightArrow"
            | "quadArrowCallout" | "upDownArrowCallout",
        ) => 0.63,
        Some("cornerTabs") => 0.35,
        Some("curvedRightArrow" | "curvedUpArrow") => 0.35,
        _ => 0.55,
    }
}

fn svg_preset_stroke_width_factor(
    preset_name: Option<&str>,
    adjust_values: &HashMap<String, f64>,
) -> f64 {
    match preset_name {
        Some("circularArrow") => 2.1,
        Some("leftUpArrow")
            if matches_svg_adjust_profile(
                adjust_values,
                &[("adj1", 15_000.0), ("adj2", 15_000.0), ("adj3", 15_000.0)],
            ) =>
        {
            1.2
        }
        Some("bentArrow")
            if matches_svg_adjust_profile(
                adjust_values,
                &[
                    ("adj1", 15_000.0),
                    ("adj2", 15_000.0),
                    ("adj3", 15_000.0),
                    ("adj4", 35_000.0),
                ],
            ) =>
        {
            1.4
        }
        Some("bentUpArrow")
            if matches_svg_adjust_profile(
                adjust_values,
                &[("adj1", 15_000.0), ("adj2", 15_000.0), ("adj3", 15_000.0)],
            ) =>
        {
            1.3
        }
        Some("stripedRightArrow")
            if matches_svg_adjust_profile(
                adjust_values,
                &[("adj1", 15_000.0), ("adj2", 15_000.0)],
            ) =>
        {
            1.5
        }
        Some("leftRightArrowCallout")
            if matches_svg_adjust_profile(
                adjust_values,
                &[
                    ("adj1", 15_000.0),
                    ("adj2", 15_000.0),
                    ("adj3", 15_000.0),
                    ("adj4", 15_000.0),
                ],
            ) =>
        {
            1.26
        }
        Some("upDownArrowCallout")
            if matches_svg_adjust_profile(
                adjust_values,
                &[
                    ("adj1", 15_000.0),
                    ("adj2", 15_000.0),
                    ("adj3", 15_000.0),
                    ("adj4", 15_000.0),
                ],
            ) =>
        {
            1.3
        }
        Some("quadArrowCallout")
            if matches_svg_adjust_profile(
                adjust_values,
                &[
                    ("adj1", 15_000.0),
                    ("adj2", 15_000.0),
                    ("adj3", 15_000.0),
                    ("adj4", 15_000.0),
                ],
            ) =>
        {
            1.3
        }
        Some("leftRightUpArrow")
            if matches_svg_adjust_profile(
                adjust_values,
                &[("adj1", 15_000.0), ("adj2", 15_000.0), ("adj3", 15_000.0)],
            ) =>
        {
            1.3
        }
        Some("curvedRightArrow" | "curvedLeftArrow") => 1.8,
        Some("curvedDownArrow") => 1.35,
        Some("curvedUpArrow") => 1.5,
        _ => 1.0,
    }
}

fn matches_svg_adjust_profile(
    adjust_values: &HashMap<String, f64>,
    expected: &[(&str, f64)],
) -> bool {
    expected.iter().all(|(name, value)| {
        (adjust_values.get(*name).copied().unwrap_or_default() - *value).abs() < 0.5
    })
}

fn svg_preset_shadow_blur_factor(
    preset_name: Option<&str>,
    adjust_values: &HashMap<String, f64>,
) -> f64 {
    match preset_name {
        Some("leftUpArrow")
            if matches_svg_adjust_profile(
                adjust_values,
                &[("adj1", 15_000.0), ("adj2", 15_000.0), ("adj3", 15_000.0)],
            ) =>
        {
            1.1
        }
        Some("leftRightUpArrow")
            if matches_svg_adjust_profile(
                adjust_values,
                &[("adj1", 15_000.0), ("adj2", 15_000.0), ("adj3", 15_000.0)],
            ) =>
        {
            1.1
        }
        _ => 1.0,
    }
}

fn scale_svg_effect_blur(effects: &ShapeEffects, factor: f64) -> ShapeEffects {
    let factor = factor.clamp(0.0, 4.0);
    ShapeEffects {
        outer_shadow: effects.outer_shadow.as_ref().map(|shadow| OuterShadow {
            blur_radius: shadow.blur_radius * factor,
            distance: shadow.distance,
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

fn svg_uses_style_ref_effect_fallback(preset_name: Option<&str>) -> bool {
    !matches!(
        preset_name,
        Some(
            "squareTabs"
                | "plaqueTabs"
                | "lineInv"
                | "circularArrow"
                | "leftCircularArrow"
                | "leftRightCircularArrow"
                | "arc"
                | "mathNotEqual"
                | "leftRightArrowCallout"
                | "frame"
        )
    )
}

fn line_cap_to_svg(cap: &LineCap) -> &'static str {
    match cap {
        LineCap::Flat => "",
        LineCap::Square => " stroke-linecap=\"square\"",
        LineCap::Round => " stroke-linecap=\"round\"",
    }
}

fn shade_resolved_color(color: ResolvedColor, factor: f64) -> ResolvedColor {
    let factor = factor.clamp(0.0, 1.0);
    ResolvedColor::new(
        (f64::from(color.r) * factor).round() as u8,
        (f64::from(color.g) * factor).round() as u8,
        (f64::from(color.b) * factor).round() as u8,
    )
}

fn tint_resolved_color(color: ResolvedColor, factor: f64) -> ResolvedColor {
    let factor = factor.clamp(0.0, 1.0);
    let tint = |channel: u8| (f64::from(channel) * factor + 255.0 * (1.0 - factor)).round() as u8;
    ResolvedColor::new(tint(color.r), tint(color.g), tint(color.b))
}

fn svg_path_fill_to_css(
    path_fill: &PathFill,
    default_fill: &str,
    base_fill: Option<ResolvedColor>,
) -> String {
    match path_fill {
        PathFill::None => "none".to_string(),
        PathFill::Norm => default_fill.to_string(),
        PathFill::Darken => base_fill
            .map(|color| shade_resolved_color(color, 0.59).to_css())
            .unwrap_or_else(|| default_fill.to_string()),
        PathFill::DarkenLess => base_fill
            .map(|color| shade_resolved_color(color, 0.78).to_css())
            .unwrap_or_else(|| default_fill.to_string()),
        PathFill::Lighten => base_fill
            .map(|color| tint_resolved_color(color, 0.60).to_css())
            .unwrap_or_else(|| default_fill.to_string()),
        PathFill::LightenLess => base_fill
            .map(|color| tint_resolved_color(color, 0.82).to_css())
            .unwrap_or_else(|| default_fill.to_string()),
    }
}

fn line_inverse_path_with_overshoot(w: f64, h: f64, overshoot: f64) -> String {
    format!(
        "M{start_x:.1},{start_y:.1} L{end_x:.1},{end_y:.1}",
        start_x = -overshoot,
        start_y = h + overshoot,
        end_x = w + overshoot,
        end_y = -overshoot
    )
}

/// Convert LineJoin to SVG stroke-linejoin attribute string (including leading space).
/// Returns empty string for Miter (SVG default).
fn line_join_to_svg(join: &LineJoin) -> &'static str {
    match join {
        LineJoin::Miter => "",
        LineJoin::Bevel => " stroke-linejoin=\"bevel\"",
        LineJoin::Round => " stroke-linejoin=\"round\"",
    }
}

fn line_miter_limit_to_svg(border: &Border) -> String {
    if matches!(border.join, LineJoin::Miter)
        && let Some(limit) = border.miter_limit
    {
        return format!(" stroke-miterlimit=\"{limit:.1}\"");
    }
    String::new()
}

/// Emit an SVG gradient definition (`<linearGradient>` or `<radialGradient>`)
/// and return the fill attribute string.
/// Returns `None` if the fill is not a gradient or has no resolvable stops.
fn svg_gradient_def(
    fill: &Fill,
    grad_id: &str,
    ctx: &RenderCtx<'_>,
    html: &mut String,
) -> Option<String> {
    if let Fill::Pattern(pattern) = fill {
        let (foreground, background) = ctx.pattern_colors(pattern)?;
        return patterns::svg_def(pattern, grad_id, &foreground, &background, html);
    }
    if let Fill::Gradient(gf) = fill {
        let stops: Vec<(f64, String)> = gf
            .stops
            .iter()
            .filter_map(|s| ctx.color_to_css(&s.color).map(|c| (s.position, c)))
            .collect();
        if stops.is_empty() {
            return None;
        }
        match gf.gradient_type {
            GradientType::Linear => {
                // Convert OOXML angle (clockwise from top) to SVG linearGradient coordinates.
                // SVG linearGradient uses x1,y1 -> x2,y2 as the gradient vector.
                let angle_rad = (gf.angle - 90.0_f64).to_radians();
                let x1 = 50.0 - 50.0 * angle_rad.cos();
                let y1 = 50.0 - 50.0 * angle_rad.sin();
                let x2 = 50.0 + 50.0 * angle_rad.cos();
                let y2 = 50.0 + 50.0 * angle_rad.sin();
                let _ = write!(
                    html,
                    "<linearGradient id=\"{grad_id}\" \
                     x1=\"{x1:.1}%\" y1=\"{y1:.1}%\" x2=\"{x2:.1}%\" y2=\"{y2:.1}%\">"
                );
            }
            GradientType::Radial | GradientType::Rectangular | GradientType::Shape => {
                let _ = write!(
                    html,
                    "<radialGradient id=\"{grad_id}\" \
                     cx=\"50%\" cy=\"50%\" r=\"50%\">"
                );
            }
        }
        for (pos, color) in &stops {
            let _ = write!(
                html,
                "<stop offset=\"{:.0}%\" stop-color=\"{color}\"/>",
                pos * 100.0
            );
        }
        match gf.gradient_type {
            GradientType::Linear => html.push_str("</linearGradient>"),
            _ => html.push_str("</radialGradient>"),
        }
        return Some(format!("url(#{grad_id})"));
    }
    None
}

/// Emit an SVG <marker> definition for a line ending (arrowhead)
fn emit_marker_def(
    html: &mut String,
    marker_id: &str,
    line_end: &LineEnd,
    color: &str,
    stroke_width: f64,
    is_start: bool,
) {
    let w_mult = line_end.width.multiplier();
    let l_mult = line_end.length.multiplier();
    let marker_w = w_mult * stroke_width;
    let marker_h = l_mult * stroke_width;
    let half_w = marker_w / 2.0;

    let (path, fill_attr) = match line_end.end_type {
        LineEndType::Arrow => (
            format!("M0,0 L{marker_h:.1},{half_w:.1} L0,{marker_w:.1}"),
            "none".to_string(),
        ),
        LineEndType::Triangle => (
            format!("M0,0 L{marker_h:.1},{half_w:.1} L0,{marker_w:.1} Z"),
            color.to_string(),
        ),
        LineEndType::Stealth => (
            format!(
                "M0,0 L{marker_h:.1},{half_w:.1} L0,{marker_w:.1} L{back:.1},{half_w:.1} Z",
                back = marker_h * 0.35,
            ),
            color.to_string(),
        ),
        LineEndType::Diamond => {
            let mid_h = marker_h / 2.0;
            (
                format!(
                    "M0,{half_w:.1} L{mid_h:.1},0 L{marker_h:.1},{half_w:.1} L{mid_h:.1},{marker_w:.1} Z",
                ),
                color.to_string(),
            )
        }
        LineEndType::Oval => {
            let rx = marker_h / 2.0;
            let ry = half_w;
            let cx = rx;
            let cy = ry;
            (
                format!(
                    "M{start:.1},{cy:.1} A{rx:.1},{ry:.1} 0 1,1 {end:.1},{cy:.1} A{rx:.1},{ry:.1} 0 1,1 {start:.1},{cy:.1} Z",
                    start = cx - rx,
                    end = cx + rx,
                ),
                color.to_string(),
            )
        }
        LineEndType::None => return,
    };

    // For marker-start the reference point is at the base (refX=0) so the
    // marker sits at the start of the line.  For marker-end the reference
    // point is at the tip (refX=marker_h).  orient="auto-start-reverse"
    // handles directional flipping automatically.
    let ref_x = if is_start { 0.0 } else { marker_h };

    let _ = write!(
        html,
        "<marker id=\"{marker_id}\" viewBox=\"0 0 {marker_h:.1} {marker_w:.1}\" \
         refX=\"{ref_x:.1}\" refY=\"{half_w:.1}\" \
         markerWidth=\"{marker_h:.1}\" markerHeight=\"{marker_w:.1}\" \
         markerUnits=\"userSpaceOnUse\" \
         orient=\"auto-start-reverse\">\
         <path d=\"{path}\" fill=\"{fill_attr}\" stroke=\"{color}\" stroke-width=\"0.5\"/>\
         </marker>"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::presentation::{ClrMap, ColorScheme, Presentation, Theme};

    fn test_ctx(_embed_images: bool) -> (Presentation, RefCell<UnresolvedCollector>) {
        let mut pres = Presentation::default();
        pres.themes.push(Theme {
            name: "Theme".to_string(),
            color_scheme: ColorScheme {
                accent1: "4472C4".to_string(),
                accent2: "ED7D31".to_string(),
                ..Default::default()
            },
            ..Default::default()
        });
        let collector = RefCell::new(UnresolvedCollector {
            elements: Vec::new(),
            diagnostics: Vec::new(),
            external_assets: Vec::new(),
            font_resolution_entries: Vec::new(),
            provenance_entries: Vec::new(),
            counter: 0,
            current_slide_index: 0,
            gradient_counter: 0,
            pattern_counter: 0,
            marker_counter: 0,
            asset_counter: 0,
            action_counter: 0,
        });
        (pres, collector)
    }

    #[test]
    fn global_css_and_helper_formatters_cover_supported_variants() {
        let css = HtmlRenderer::global_css(960.0, 540.0);
        assert!(css.contains(".pptx-container"));
        assert!(css.contains(".slide-shell"));
        assert!(css.contains("width: 960.0px"));
        assert!(css.contains("height: 540.0px"));

        assert_eq!(bullets::format_auto_num("arabicPeriod", 3), "3.");
        assert_eq!(bullets::format_auto_num("alphaLcParenBoth", 27), "(aa)");
        assert_eq!(bullets::format_auto_num("alphaUcParenR", 2), "B)");
        assert_eq!(bullets::format_auto_num("romanLcPeriod", 14), "xiv.");
        assert_eq!(bullets::format_auto_num("romanUcParenBoth", 9), "(IX)");
        assert_eq!(bullets::format_auto_num("unknown", 5), "5.");
        assert_eq!(bullets::to_alpha_lc(0), "a");
        assert_eq!(bullets::to_alpha_uc(28), "AB");
        assert_eq!(bullets::to_roman_lc(4), "iv");
        assert_eq!(bullets::to_roman_uc(4000), "4000");
        assert_eq!(
            escape_html("<tag attr=\"1\">&"),
            "&lt;tag attr=&quot;1&quot;&gt;&amp;"
        );
    }

    #[test]
    fn dash_cap_join_miter_and_marker_helpers_cover_variants() {
        assert_eq!(dash_style_to_svg(&DashStyle::Solid, 2.0), "");
        assert!(dash_style_to_svg(&DashStyle::Dash, 2.0).contains("16.0 8.0"));
        assert!(dash_style_to_svg(&DashStyle::Dot, 2.0).contains("4.0 4.0"));
        assert!(dash_style_to_svg(&DashStyle::DashDot, 1.0).contains("8.0 4.0 2.0 4.0"));
        assert!(dash_style_to_svg(&DashStyle::LongDash, 1.0).contains("12.0 4.0"));
        assert!(dash_style_to_svg(&DashStyle::LongDashDot, 1.0).contains("12.0 4.0 2.0 4.0"));
        assert!(
            dash_style_to_svg(&DashStyle::LongDashDotDot, 1.0).contains("12.0 4.0 2.0 4.0 2.0 4.0")
        );
        assert!(dash_style_to_svg(&DashStyle::SystemDash, 1.0).contains("6.0 3.0"));
        assert!(dash_style_to_svg(&DashStyle::SystemDot, 1.0).contains("1.0 2.0"));
        assert!(dash_style_to_svg(&DashStyle::SystemDashDot, 1.0).contains("3.0 1.0 1.0 1.0"));
        assert!(
            dash_style_to_svg(&DashStyle::SystemDashDotDot, 1.0)
                .contains("3.0 1.0 1.0 1.0 1.0 1.0")
        );

        assert_eq!(dash_style_to_css(&DashStyle::Solid), "solid");
        assert_eq!(dash_style_to_css(&DashStyle::Dash), "dashed");
        assert_eq!(dash_style_to_css(&DashStyle::Dot), "dotted");
        assert_eq!(dash_style_to_css(&DashStyle::SystemDashDotDot), "dashed");

        assert_eq!(line_cap_to_svg(&LineCap::Flat), "");
        assert_eq!(
            line_cap_to_svg(&LineCap::Square),
            " stroke-linecap=\"square\""
        );
        assert_eq!(
            line_cap_to_svg(&LineCap::Round),
            " stroke-linecap=\"round\""
        );
        assert_eq!(line_join_to_svg(&LineJoin::Miter), "");
        assert_eq!(
            line_join_to_svg(&LineJoin::Bevel),
            " stroke-linejoin=\"bevel\""
        );
        assert_eq!(
            line_join_to_svg(&LineJoin::Round),
            " stroke-linejoin=\"round\""
        );

        let border = Border {
            join: LineJoin::Miter,
            miter_limit: Some(2.5),
            ..Default::default()
        };
        assert_eq!(
            line_miter_limit_to_svg(&border),
            " stroke-miterlimit=\"2.5\""
        );
        assert_eq!(line_miter_limit_to_svg(&Border::default()), "");

        let mut html = String::new();
        for (suffix, end_type) in [
            ("arrow", LineEndType::Arrow),
            ("triangle", LineEndType::Triangle),
            ("stealth", LineEndType::Stealth),
            ("diamond", LineEndType::Diamond),
            ("oval", LineEndType::Oval),
        ] {
            emit_marker_def(
                &mut html,
                suffix,
                &LineEnd {
                    end_type,
                    width: LineEndSize::Medium,
                    length: LineEndSize::Large,
                },
                "#112233",
                2.0,
                suffix == "arrow",
            );
        }
        assert!(html.contains("marker id=\"arrow\""));
        assert!(html.contains("marker id=\"triangle\""));
        assert!(html.contains("marker id=\"stealth\""));
        assert!(html.contains("marker id=\"diamond\""));
        assert!(html.contains("marker id=\"oval\""));
    }

    #[test]
    fn fill_helpers_cover_gradient_and_image_branches() {
        let gradient_fill = Fill::Gradient(GradientFill {
            gradient_type: GradientType::Linear,
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: Color::theme("accent1"),
                },
                GradientStop {
                    position: 1.0,
                    color: Color::rgb("00FF00"),
                },
            ],
            angle: 135.0,
        });

        let (pres_embed, collector_embed) = test_ctx(true);
        let ctx_embed = RenderCtx {
            pres: &pres_embed,
            slide: None,
            scheme: pres_embed.primary_theme().map(|t| &t.color_scheme),
            clr_map: None,
            embed_images: true,
            collector: &collector_embed,
        };
        let mut buf = String::new();
        HtmlRenderer::fill_to_css_buf(&gradient_fill, &ctx_embed, &mut buf);
        assert!(buf.contains("linear-gradient(135deg"));
        assert!(HtmlRenderer::fill_to_css(&gradient_fill, &ctx_embed).contains("linear-gradient"));

        let mut defs = String::new();
        let fill_attr = svg_gradient_def(&gradient_fill, "grad-test", &ctx_embed, &mut defs)
            .expect("svg gradient should be emitted");
        assert_eq!(fill_attr, "url(#grad-test)");
        assert!(defs.contains("<linearGradient id=\"grad-test\""));

        let image_fill = Fill::Image(ImageFill {
            rel_id: "rId1".to_string(),
            data: vec![1, 2, 3, 4],
            content_type: "image/png".to_string(),
        });
        let mut embed_buf = String::new();
        HtmlRenderer::fill_to_css_buf(&image_fill, &ctx_embed, &mut embed_buf);
        assert!(embed_buf.contains("background-image: url(data:image/png;base64,"));

        let (pres_external, collector_external) = test_ctx(false);
        let ctx_external = RenderCtx {
            pres: &pres_external,
            slide: None,
            scheme: pres_external.primary_theme().map(|t| &t.color_scheme),
            clr_map: None,
            embed_images: false,
            collector: &collector_external,
        };
        let mut external_buf = String::new();
        HtmlRenderer::fill_to_css_buf(&image_fill, &ctx_external, &mut external_buf);
        assert!(external_buf.contains("background-image: url(images/slide-1/background-0.png)"));
        let assets = &collector_external.borrow().external_assets;
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].relative_path, "images/slide-1/background-0.png");
    }

    #[test]
    fn render_table_and_paragraph_cover_borders_spans_and_bullets() {
        let (pres, collector) = test_ctx(true);
        let ctx = RenderCtx {
            pres: &pres,
            slide: None,
            scheme: pres.primary_theme().map(|t| &t.color_scheme),
            clr_map: None,
            embed_images: true,
            collector: &collector,
        };

        let paragraph = TextParagraph {
            runs: vec![TextRun {
                text: "Cell Text".to_string(),
                ..Default::default()
            }],
            bullet: Some(Bullet::AutoNum(BulletAutoNum {
                num_type: "romanUcPeriod".to_string(),
                start_at: Some(1),
                font: Some("Calibri".to_string()),
                size_pct: Some(1.2),
                color: Some(Color::rgb("FF0000")),
            })),
            ..Default::default()
        };
        let char_bullet_para = TextParagraph {
            runs: vec![TextRun {
                text: "Bullet Text".to_string(),
                ..Default::default()
            }],
            bullet: Some(Bullet::Char(BulletChar {
                char: "•".to_string(),
                font: Some("Symbol".to_string()),
                size_pct: Some(0.9),
                color: Some(Color::theme("accent2")),
            })),
            ..Default::default()
        };
        let mut para_html = String::new();
        let mut counters = [0; 9];
        HtmlRenderer::render_paragraph(&paragraph, &ctx, &mut counters, &mut para_html);
        HtmlRenderer::render_paragraph(&char_bullet_para, &ctx, &mut counters, &mut para_html);
        assert!(para_html.contains("I."));
        assert!(para_html.contains("•"));
        assert!(para_html.contains("Cell Text"));
        assert!(para_html.contains("Bullet Text"));

        let table = TableData {
            rows: vec![TableRow {
                height: 24.0,
                cells: vec![TableCell {
                    text_body: Some(TextBody {
                        paragraphs: vec![paragraph],
                        ..Default::default()
                    }),
                    fill: Fill::Solid(SolidFill {
                        color: Color::rgb("00FF00"),
                    }),
                    border_left: Border {
                        width: 1.0,
                        color: Color::rgb("FF0000"),
                        dash_style: DashStyle::Dash,
                        ..Default::default()
                    },
                    border_right: Border {
                        width: 1.0,
                        color: Color::rgb("0000FF"),
                        dash_style: DashStyle::Dot,
                        ..Default::default()
                    },
                    border_top: Border {
                        width: 1.0,
                        color: Color::rgb("123456"),
                        dash_style: DashStyle::Solid,
                        ..Default::default()
                    },
                    border_bottom: Border {
                        width: 1.0,
                        color: Color::rgb("654321"),
                        dash_style: DashStyle::SystemDash,
                        ..Default::default()
                    },
                    col_span: 2,
                    row_span: 3,
                    v_merge: false,
                    h_merge: false,
                    explicit_borders: 15,
                    margin_left: 7.2,
                    margin_right: 7.2,
                    margin_top: 3.6,
                    margin_bottom: 3.6,
                    vertical_align: VerticalAlign::Middle,
                }],
            }],
            col_widths: vec![120.0],
            band_row: true,
            band_col: false,
            first_row: true,
            last_row: false,
            first_col: true,
            last_col: false,
            style: None,
        };
        let mut html = String::new();
        HtmlRenderer::render_table(&table, 0, &ctx, &mut html);
        assert!(html.contains("<table"));
        assert!(html.contains("colspan=\"2\""));
        assert!(html.contains("rowspan=\"3\""));
        assert!(html.contains("background-color: #00FF00"));
        assert!(html.contains("border-left: 1.0pt dashed #FF0000"));
        assert!(html.contains("border-right: 1.0pt dotted #0000FF"));
        assert!(html.contains("vertical-align: middle"));
    }

    #[test]
    fn render_slide_covers_hidden_shapes_rotation_min_line_size_crop_and_unresolved_custom_geometry()
     {
        let (mut pres, collector) = test_ctx(true);
        pres.masters.push(SlideMaster {
            theme_idx: 0,
            shapes: vec![
                Shape {
                    name: "hidden-master".to_string(),
                    hidden: true,
                    ..Default::default()
                },
                Shape {
                    name: "placeholder-master".to_string(),
                    placeholder: Some(PlaceholderInfo::default()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        pres.layouts.push(SlideLayout {
            master_idx: 0,
            show_master_sp: true,
            ..Default::default()
        });

        let slide = Slide {
            layout_idx: Some(0),
            show_master_sp: true,
            shapes: vec![
                Shape {
                    name: "hidden-slide".to_string(),
                    hidden: true,
                    ..Default::default()
                },
                Shape {
                    name: "rotated-rect".to_string(),
                    shape_type: ShapeType::Rectangle,
                    size: Size {
                        width: Emu(457_200),
                        height: Emu(228_600),
                    },
                    rotation: 30.0,
                    fill: Fill::Solid(SolidFill {
                        color: Color::rgb("CCCCCC"),
                    }),
                    ..Default::default()
                },
                Shape {
                    name: "rotated-line".to_string(),
                    shape_type: ShapeType::Custom("line".to_string()),
                    position: Position {
                        x: Emu(9_144),
                        y: Emu(18_288),
                    },
                    size: Size {
                        width: Emu(0),
                        height: Emu(914_400),
                    },
                    rotation: 30.0,
                    border: Border {
                        width: 1.0,
                        color: Color::rgb("112233"),
                        style: BorderStyle::Solid,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Shape {
                    name: "cropped-picture".to_string(),
                    shape_type: ShapeType::Picture(PictureData {
                        rel_id: "rId1".to_string(),
                        content_type: "image/png".to_string(),
                        data: vec![1, 2, 3, 4],
                        crop: Some(CropRect {
                            left: 0.1,
                            top: 0.0,
                            right: 0.1,
                            bottom: 0.0,
                        }),
                    }),
                    position: Position {
                        x: Emu(0),
                        y: Emu(0),
                    },
                    size: Size {
                        width: Emu(914_400),
                        height: Emu(457_200),
                    },
                    ..Default::default()
                },
                Shape {
                    name: "group".to_string(),
                    shape_type: ShapeType::Group(
                        vec![Shape {
                            name: "group-child".to_string(),
                            shape_type: ShapeType::Rectangle,
                            size: Size {
                                width: Emu(457_200),
                                height: Emu(228_600),
                            },
                            fill: Fill::Solid(SolidFill {
                                color: Color::rgb("00FF00"),
                            }),
                            ..Default::default()
                        }],
                        GroupData::default(),
                    ),
                    size: Size {
                        width: Emu(914_400),
                        height: Emu(457_200),
                    },
                    ..Default::default()
                },
                Shape {
                    name: "custom-geometry-placeholder".to_string(),
                    shape_type: ShapeType::Unsupported(UnsupportedData {
                        label: "Custom Geometry".to_string(),
                        element_type: UnresolvedType::CustomGeometry,
                        raw_xml: Some("<custGeom/>".to_string()),
                        custom_geometry: None,
                    }),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let ctx = RenderCtx {
            pres: &pres,
            slide: None,
            scheme: pres.primary_theme().map(|t| &t.color_scheme),
            clr_map: None,
            embed_images: true,
            collector: &collector,
        };
        let mut html = String::new();
        HtmlRenderer::render_slide(&slide, 1, 960.0, 540.0, 1.0, &ctx, &mut html);

        assert!(html.contains("transform: rotate(30.0deg)"));
        assert!(html.contains("width: 2px"));
        assert!(html.contains("overflow: hidden"));
        assert!(html.contains("data-type=\"custom-geometry\""));
        assert!(html.contains("background-color: #00FF00"));
        assert!(!html.contains("hidden-master"));
        assert!(!html.contains("hidden-slide"));
        assert!(
            collector
                .borrow()
                .elements
                .iter()
                .any(|elem| matches!(elem.element_type, UnresolvedType::CustomGeometry))
        );
    }

    #[test]
    fn render_shape_resolved_covers_outline_none_and_effect_ref_fallback() {
        let (mut pres, collector) = test_ctx(true);
        pres.themes[0]
            .fmt_scheme
            .effect_style_lst
            .push(EffectStyle {
                outer_shadow: Some(OuterShadow {
                    blur_radius: 2.0,
                    distance: 3.0,
                    direction: 45.0,
                    color: Color::theme("accent1"),
                    alpha: 1.0,
                }),
                glow: Some(GlowEffect {
                    radius: 1.5,
                    color: Color::rgb("ABCDEF"),
                    alpha: 1.0,
                }),
            });
        let clr_map = ClrMap::default();

        let shape = Shape {
            name: "outlined-rect".to_string(),
            shape_type: ShapeType::Rectangle,
            size: Size {
                width: Emu(914_400),
                height: Emu(457_200),
            },
            border: Border {
                width: 1.0,
                style: BorderStyle::None,
                ..Default::default()
            },
            style_ref: Some(ShapeStyleRef {
                effect_ref: Some(StyleRef {
                    idx: 1,
                    color: Color::none(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let ctx = RenderCtx {
            pres: &pres,
            slide: None,
            scheme: pres.primary_theme().map(|t| &t.color_scheme),
            clr_map: Some(&clr_map),
            embed_images: true,
            collector: &collector,
        };
        let mut html = String::new();
        HtmlRenderer::render_shape_resolved(&shape, None, None, &ctx, &mut html);

        assert!(html.contains("outline: 1.0pt none"));
        assert!(html.contains("box-shadow:"));
    }

    #[test]
    fn render_shape_resolved_routes_svg_effects_to_filter_instead_of_box_shadow() {
        let (pres, collector) = test_ctx(true);
        let clr_map = ClrMap::default();
        let shape = Shape {
            name: "arrow-with-effect-ref".to_string(),
            shape_type: ShapeType::Custom("rightArrow".to_string()),
            size: Size {
                width: Emu(914_400),
                height: Emu(457_200),
            },
            fill: Fill::Solid(SolidFill {
                color: Color::rgb("336699"),
            }),
            effects: ShapeEffects {
                outer_shadow: Some(OuterShadow {
                    blur_radius: 2.0,
                    distance: 3.0,
                    direction: 45.0,
                    color: Color::theme("accent1"),
                    alpha: 1.0,
                }),
                glow: Some(GlowEffect {
                    radius: 1.5,
                    color: Color::rgb("ABCDEF"),
                    alpha: 1.0,
                }),
            },
            ..Default::default()
        };

        let ctx = RenderCtx {
            pres: &pres,
            slide: None,
            scheme: pres.primary_theme().map(|t| &t.color_scheme),
            clr_map: Some(&clr_map),
            embed_images: true,
            collector: &collector,
        };
        let mut html = String::new();
        HtmlRenderer::render_shape_resolved(&shape, None, None, &ctx, &mut html);

        assert!(html.contains("filter: drop-shadow("));
        assert!(!html.contains("box-shadow:"));
    }

    #[test]
    fn render_shape_resolved_applies_style_ref_effect_fallback_for_svg_shapes() {
        let (mut pres, collector) = test_ctx(true);
        pres.themes[0]
            .fmt_scheme
            .effect_style_lst
            .push(EffectStyle {
                outer_shadow: Some(OuterShadow {
                    blur_radius: 2.0,
                    distance: 3.0,
                    direction: 45.0,
                    color: Color::theme("accent1"),
                    alpha: 1.0,
                }),
                glow: Some(GlowEffect {
                    radius: 1.5,
                    color: Color::rgb("ABCDEF"),
                    alpha: 1.0,
                }),
            });
        let clr_map = ClrMap::default();

        let shape = Shape {
            name: "arrow-style-ref-only".to_string(),
            shape_type: ShapeType::Custom("rightArrow".to_string()),
            size: Size {
                width: Emu(914_400),
                height: Emu(457_200),
            },
            fill: Fill::Solid(SolidFill {
                color: Color::rgb("336699"),
            }),
            style_ref: Some(ShapeStyleRef {
                effect_ref: Some(StyleRef {
                    idx: 1,
                    color: Color::none(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let ctx = RenderCtx {
            pres: &pres,
            slide: None,
            scheme: pres.primary_theme().map(|t| &t.color_scheme),
            clr_map: Some(&clr_map),
            embed_images: true,
            collector: &collector,
        };
        let mut html = String::new();
        HtmlRenderer::render_shape_resolved(&shape, None, None, &ctx, &mut html);

        assert!(html.contains("filter: drop-shadow("));
        assert!(!html.contains("box-shadow:"));
    }

    #[test]
    fn render_shape_resolved_covers_chart_preview_assets_and_label_positions() {
        let (pres_external, collector_external) = test_ctx(false);
        let ctx_external = RenderCtx {
            pres: &pres_external,
            slide: None,
            scheme: pres_external.primary_theme().map(|t| &t.color_scheme),
            clr_map: None,
            embed_images: false,
            collector: &collector_external,
        };

        let preview_shape = Shape {
            shape_type: ShapeType::Chart(ChartData {
                rel_id: "rIdChart".to_string(),
                preview_image: Some(vec![1, 2, 3, 4]),
                preview_mime: Some("image/png".to_string()),
                direct_spec: None,
            }),
            size: Size {
                width: Emu(914_400),
                height: Emu(457_200),
            },
            ..Default::default()
        };
        let mut preview_html = String::new();
        HtmlRenderer::render_shape_resolved(
            &preview_shape,
            None,
            None,
            &ctx_external,
            &mut preview_html,
        );
        assert!(preview_html.contains("images/slide-1/chart-0.png"));
        let assets = &collector_external.borrow().external_assets;
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].relative_path, "images/slide-1/chart-0.png");

        let (pres_embed, collector_embed) = test_ctx(true);
        let ctx_embed = RenderCtx {
            pres: &pres_embed,
            slide: None,
            scheme: pres_embed.primary_theme().map(|t| &t.color_scheme),
            clr_map: None,
            embed_images: true,
            collector: &collector_embed,
        };

        let make_chart_shape = |spec: ChartSpec| Shape {
            shape_type: ShapeType::Chart(ChartData {
                rel_id: "rIdChart".to_string(),
                preview_image: None,
                preview_mime: None,
                direct_spec: Some(spec),
            }),
            size: Size {
                width: Emu(1_828_800),
                height: Emu(914_400),
            },
            ..Default::default()
        };

        let mut bar_html = String::new();
        HtmlRenderer::render_shape_resolved(
            &make_chart_shape(ChartSpec {
                chart_type: ChartType::Column,
                grouping: ChartGrouping::Clustered,
                data_labels: Some(ChartDataLabelSettings {
                    show_value: true,
                    position: Some(ChartDataLabelPosition::Center),
                    ..Default::default()
                }),
                series: vec![ChartSeries {
                    name: Some("Revenue".to_string()),
                    categories: vec!["Q1".to_string()],
                    values: vec![10.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            None,
            None,
            &ctx_embed,
            &mut bar_html,
        );
        assert!(bar_html.contains("data-label-position=\"ctr\""));

        let mut line_html = String::new();
        HtmlRenderer::render_shape_resolved(
            &make_chart_shape(ChartSpec {
                chart_type: ChartType::Line,
                data_labels: Some(ChartDataLabelSettings {
                    show_value: true,
                    position: Some(ChartDataLabelPosition::InEnd),
                    ..Default::default()
                }),
                series: vec![ChartSeries {
                    name: Some("Series".to_string()),
                    categories: vec!["A".to_string()],
                    values: vec![5.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            None,
            None,
            &ctx_embed,
            &mut line_html,
        );
        assert!(line_html.contains("data-label-position=\"inEnd\""));

        let mut scatter_html = String::new();
        HtmlRenderer::render_shape_resolved(
            &make_chart_shape(ChartSpec {
                chart_type: ChartType::Scatter,
                scatter_style: Some(ChartScatterStyle::Marker),
                data_labels: Some(ChartDataLabelSettings {
                    show_value: true,
                    position: Some(ChartDataLabelPosition::InEnd),
                    ..Default::default()
                }),
                series: vec![ChartSeries {
                    name: Some("Points".to_string()),
                    x_values: vec![1.0],
                    values: vec![5.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            None,
            None,
            &ctx_embed,
            &mut scatter_html,
        );
        assert!(scatter_html.contains("data-label-position=\"inEnd\""));

        let mut pie_html = String::new();
        HtmlRenderer::render_shape_resolved(
            &make_chart_shape(ChartSpec {
                chart_type: ChartType::Pie,
                data_labels: Some(ChartDataLabelSettings {
                    show_value: true,
                    position: Some(ChartDataLabelPosition::InEnd),
                    ..Default::default()
                }),
                series: vec![ChartSeries {
                    name: Some("Series".to_string()),
                    categories: vec!["Slice".to_string()],
                    values: vec![20.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            None,
            None,
            &ctx_embed,
            &mut pie_html,
        );
        assert!(pie_html.contains("data-label-position=\"inEnd\""));
    }

    #[test]
    fn render_context_helpers_cover_asset_extensions_theme_override_and_wrappers() {
        let (mut pres, collector) = test_ctx(false);
        pres.themes.push(Theme {
            name: "Alt".to_string(),
            color_scheme: ColorScheme {
                accent1: "00FF00".to_string(),
                ..Default::default()
            },
            ..Default::default()
        });

        let clr_map = ClrMap::default();
        let slide_ctx = RenderCtx {
            pres: &pres,
            slide: None,
            scheme: pres.primary_theme().map(|t| &t.color_scheme),
            clr_map: Some(&clr_map),
            embed_images: false,
            collector: &collector,
        };

        assert!(
            slide_ctx
                .register_external_asset("img", "image/jpeg", &[1, 2, 3])
                .ends_with(".jpg")
        );
        assert!(
            slide_ctx
                .register_external_asset("img", "image/gif", &[1, 2, 3])
                .ends_with(".gif")
        );
        assert!(
            slide_ctx
                .register_external_asset("img", "image/svg+xml", &[1, 2, 3])
                .ends_with(".svg")
        );
        assert!(
            slide_ctx
                .register_external_asset("img", "image/webp", &[1, 2, 3])
                .ends_with(".webp")
        );
        assert!(
            slide_ctx
                .register_external_asset("img", "image/png", &[1, 2, 3])
                .ends_with(".png")
        );

        let alt_ctx = slide_ctx.for_slide(None, Some(1));
        assert_eq!(
            alt_ctx.scheme.map(|scheme| scheme.accent1.as_str()),
            Some("00FF00")
        );
        let inherited_ctx = slide_ctx.for_slide(None, None);
        assert_eq!(inherited_ctx.clr_map.map(|_| true), Some(true));

        let mut presentation = Presentation {
            slide_size: Size {
                width: Emu(914_400),
                height: Emu(457_200),
            },
            ..Default::default()
        };
        presentation.slides.push(Slide {
            shapes: vec![Shape {
                shape_type: ShapeType::Rectangle,
                size: Size {
                    width: Emu(457_200),
                    height: Emu(228_600),
                },
                fill: Fill::Solid(SolidFill {
                    color: Color::rgb("CCCCCC"),
                }),
                ..Default::default()
            }],
            ..Default::default()
        });
        assert!(
            HtmlRenderer::render(&presentation)
                .expect("render wrapper")
                .contains("pptx-container")
        );
        assert!(
            HtmlRenderer::render_with_options(&presentation, &ConversionOptions::default())
                .expect("render_with_options wrapper")
                .contains("pptx-container")
        );
    }

    #[test]
    fn render_with_scale_wraps_slide_in_scaled_shell() {
        let mut presentation = Presentation {
            slide_size: Size {
                width: Emu(914_400),
                height: Emu(457_200),
            },
            ..Default::default()
        };
        presentation.slides.push(Slide {
            shapes: vec![Shape {
                shape_type: ShapeType::Rectangle,
                size: Size {
                    width: Emu(228_600),
                    height: Emu(114_300),
                },
                fill: Fill::Solid(SolidFill {
                    color: Color::rgb("CCCCCC"),
                }),
                ..Default::default()
            }],
            ..Default::default()
        });

        let html = HtmlRenderer::render_with_options(
            &presentation,
            &ConversionOptions {
                scale: 2.0,
                ..Default::default()
            },
        )
        .expect("scaled render");

        assert!(html.contains("class=\"slide-shell\""));
        assert!(html.contains("width: 192.0px; height: 96.0px;"));
        assert!(html.contains("transform: scale(2.0000); transform-origin: top left"));
        assert!(html.contains("class=\"slide\""));
    }

    #[test]
    fn render_line_shape_places_tail_marker_at_start_and_head_marker_at_end() {
        let (pres, collector) = test_ctx(true);
        let ctx = RenderCtx {
            pres: &pres,
            slide: None,
            scheme: pres.primary_theme().map(|t| &t.color_scheme),
            clr_map: None,
            embed_images: true,
            collector: &collector,
        };
        let shape = Shape {
            shape_type: ShapeType::Custom("line".to_string()),
            size: Size {
                width: Emu(914_400),
                height: Emu(457_200),
            },
            border: Border {
                width: 1.0,
                color: Color::rgb("112233"),
                head_end: Some(LineEnd {
                    end_type: LineEndType::Triangle,
                    width: LineEndSize::Large,
                    length: LineEndSize::Small,
                }),
                tail_end: Some(LineEnd {
                    end_type: LineEndType::Diamond,
                    width: LineEndSize::Small,
                    length: LineEndSize::Large,
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut html = String::new();
        HtmlRenderer::render_shape_resolved(&shape, None, None, &ctx, &mut html);

        assert!(html.contains("marker-start=\"url(#marker-tail-0)\""));
        assert!(html.contains("marker-end=\"url(#marker-head-1)\""));
        assert!(html.contains("marker id=\"marker-tail-0\""));
        assert!(html.contains("marker id=\"marker-head-1\""));
    }

    #[test]
    fn render_line_inverse_boosts_default_stroke_width_for_reference_fidelity() {
        let (pres, collector) = test_ctx(true);
        let ctx = RenderCtx {
            pres: &pres,
            slide: None,
            scheme: pres.primary_theme().map(|t| &t.color_scheme),
            clr_map: None,
            embed_images: true,
            collector: &collector,
        };
        let shape = Shape {
            shape_type: ShapeType::Custom("lineInv".to_string()),
            size: Size {
                width: Emu(1_051_560),
                height: Emu(1_691_640),
            },
            border: Border {
                width: 1.5,
                color: Color::rgb("202020"),
                ..Default::default()
            },
            fill: Fill::None,
            ..Default::default()
        };

        let mut html = String::new();
        HtmlRenderer::render_shape_resolved(&shape, None, None, &ctx, &mut html);

        assert!(html.contains("d=\"M-0.8,178.4 L111.2,-0.8\""));
        assert!(html.contains("stroke-width=\"6.4\""));
    }

    #[test]
    fn svg_style_effect_factor_uses_sun_override() {
        assert_eq!(svg_style_effect_factor(Some("sun")), 0.65);
        assert_eq!(svg_style_effect_factor(Some("rightArrow")), 0.63);
        assert_eq!(svg_style_effect_factor(Some("upDownArrow")), 0.63);
        assert_eq!(svg_style_effect_factor(Some("notchedRightArrow")), 0.63);
        assert_eq!(svg_style_effect_factor(Some("stripedRightArrow")), 0.63);
        assert_eq!(svg_style_effect_factor(Some("quadArrowCallout")), 0.63);
        assert_eq!(svg_style_effect_factor(Some("upDownArrowCallout")), 0.63);
        assert_eq!(svg_style_effect_factor(Some("cornerTabs")), 0.35);
        assert_eq!(svg_style_effect_factor(Some("curvedRightArrow")), 0.35);
        assert_eq!(svg_style_effect_factor(Some("curvedUpArrow")), 0.35);
        assert_eq!(svg_style_effect_factor(Some("cloud")), 0.55);
        assert_eq!(svg_style_effect_factor(None), 0.55);
    }

    #[test]
    fn svg_preset_stroke_width_factor_uses_arrow_overrides() {
        let tight = HashMap::from([
            ("adj1".to_string(), 15_000.0),
            ("adj2".to_string(), 15_000.0),
            ("adj3".to_string(), 15_000.0),
        ]);
        let empty = HashMap::new();
        assert_eq!(
            svg_preset_stroke_width_factor(Some("circularArrow"), &empty),
            2.1
        );
        assert_eq!(
            svg_preset_stroke_width_factor(Some("curvedRightArrow"), &empty),
            1.8
        );
        assert_eq!(
            svg_preset_stroke_width_factor(Some("curvedLeftArrow"), &empty),
            1.8
        );
        assert_eq!(
            svg_preset_stroke_width_factor(Some("curvedUpArrow"), &empty),
            1.5
        );
        assert_eq!(
            svg_preset_stroke_width_factor(Some("curvedDownArrow"), &empty),
            1.35
        );
        assert_eq!(
            svg_preset_stroke_width_factor(Some("leftUpArrow"), &tight),
            1.2
        );
        let bent_tight = HashMap::from([
            ("adj1".to_string(), 15_000.0),
            ("adj2".to_string(), 15_000.0),
            ("adj3".to_string(), 15_000.0),
            ("adj4".to_string(), 35_000.0),
        ]);
        assert_eq!(
            svg_preset_stroke_width_factor(Some("bentArrow"), &bent_tight),
            1.4
        );
        assert_eq!(
            svg_preset_stroke_width_factor(Some("bentUpArrow"), &tight),
            1.3
        );
        let striped_tight = HashMap::from([
            ("adj1".to_string(), 15_000.0),
            ("adj2".to_string(), 15_000.0),
        ]);
        assert_eq!(
            svg_preset_stroke_width_factor(Some("stripedRightArrow"), &striped_tight),
            1.5
        );
        let tight_callout = HashMap::from([
            ("adj1".to_string(), 15_000.0),
            ("adj2".to_string(), 15_000.0),
            ("adj3".to_string(), 15_000.0),
            ("adj4".to_string(), 15_000.0),
        ]);
        assert_eq!(
            svg_preset_stroke_width_factor(Some("leftRightArrowCallout"), &tight_callout),
            1.26
        );
        assert_eq!(
            svg_preset_stroke_width_factor(Some("upDownArrowCallout"), &tight_callout),
            1.3
        );
        assert_eq!(
            svg_preset_stroke_width_factor(Some("quadArrowCallout"), &tight_callout),
            1.3
        );
        assert_eq!(
            svg_preset_stroke_width_factor(Some("leftRightUpArrow"), &tight),
            1.3
        );
        assert_eq!(
            svg_preset_stroke_width_factor(Some("rightArrow"), &empty),
            1.0
        );
        assert_eq!(svg_preset_stroke_width_factor(None, &empty), 1.0);
    }

    #[test]
    fn svg_preset_shadow_blur_factor_uses_left_up_arrow_tight_override() {
        let tight = HashMap::from([
            ("adj1".to_string(), 15_000.0),
            ("adj2".to_string(), 15_000.0),
            ("adj3".to_string(), 15_000.0),
        ]);
        let wide = HashMap::from([
            ("adj1".to_string(), 35_000.0),
            ("adj2".to_string(), 35_000.0),
            ("adj3".to_string(), 45_000.0),
        ]);
        assert_eq!(
            svg_preset_shadow_blur_factor(Some("leftUpArrow"), &tight),
            1.1
        );
        assert_eq!(
            svg_preset_shadow_blur_factor(Some("leftUpArrow"), &wide),
            1.0
        );
        assert_eq!(
            svg_preset_shadow_blur_factor(Some("leftRightUpArrow"), &tight),
            1.1
        );
        assert_eq!(
            svg_preset_shadow_blur_factor(Some("rightArrow"), &tight),
            1.0
        );
    }

    #[test]
    fn render_action_button_information_uses_two_tone_icon_layers() {
        let (pres, collector) = test_ctx(true);
        let ctx = RenderCtx {
            pres: &pres,
            slide: None,
            scheme: pres.primary_theme().map(|t| &t.color_scheme),
            clr_map: None,
            embed_images: true,
            collector: &collector,
        };
        let shape = Shape {
            shape_type: ShapeType::Custom("actionButtonInformation".to_string()),
            size: Size {
                width: Emu(1_889_760),
                height: Emu(1_889_760),
            },
            border: Border {
                width: 1.5,
                color: Color::rgb("202020"),
                ..Default::default()
            },
            fill: Fill::Solid(SolidFill {
                color: Color::rgb("4472C4"),
            }),
            ..Default::default()
        };

        let mut html = String::new();
        HtmlRenderer::render_shape_resolved(&shape, None, None, &ctx, &mut html);

        assert!(html.contains("fill=\"#4472C4\""));
        assert!(html.contains("fill=\"#284374\""));
        assert!(html.contains("fill=\"#8FAADC\""));
        assert!(html.matches("<path d=").count() >= 3);
    }

    #[test]
    fn render_shape_resolved_covers_chart_edge_case_branches() {
        let (pres, collector) = test_ctx(true);
        let ctx = RenderCtx {
            pres: &pres,
            slide: None,
            scheme: pres.primary_theme().map(|t| &t.color_scheme),
            clr_map: None,
            embed_images: true,
            collector: &collector,
        };

        let make_chart_shape = |spec: ChartSpec| Shape {
            shape_type: ShapeType::Chart(ChartData {
                rel_id: "rIdChart".to_string(),
                preview_image: None,
                preview_mime: None,
                direct_spec: Some(spec),
            }),
            size: Size {
                width: Emu(1_828_800),
                height: Emu(914_400),
            },
            ..Default::default()
        };

        let mut scatter_html = String::new();
        HtmlRenderer::render_shape_resolved(
            &make_chart_shape(ChartSpec {
                chart_type: ChartType::Scatter,
                scatter_style: Some(ChartScatterStyle::Line),
                data_labels: Some(ChartDataLabelSettings {
                    show_value: true,
                    position: Some(ChartDataLabelPosition::Center),
                    ..Default::default()
                }),
                series: vec![ChartSeries {
                    name: Some("Scatter".to_string()),
                    x_values: vec![f64::NAN],
                    values: vec![5.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            None,
            None,
            &ctx,
            &mut scatter_html,
        );
        assert!(scatter_html.contains("chart-line"));
        assert!(scatter_html.contains("data-label-position=\"ctr\""));

        let mut bubble_html = String::new();
        HtmlRenderer::render_shape_resolved(
            &make_chart_shape(ChartSpec {
                chart_type: ChartType::Bubble,
                bubble_scale: Some(150.0),
                series: vec![ChartSeries {
                    name: Some("Bubbles".to_string()),
                    x_values: vec![f64::NAN],
                    values: vec![f64::NAN],
                    bubble_sizes: vec![4.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            None,
            None,
            &ctx,
            &mut bubble_html,
        );
        assert!(bubble_html.contains("chart-bubble"));

        let mut area_html = String::new();
        HtmlRenderer::render_shape_resolved(
            &make_chart_shape(ChartSpec {
                chart_type: ChartType::Area,
                data_labels: Some(ChartDataLabelSettings {
                    show_value: true,
                    position: Some(ChartDataLabelPosition::Center),
                    ..Default::default()
                }),
                series: vec![ChartSeries {
                    name: Some("Area".to_string()),
                    categories: vec!["Only".to_string()],
                    values: vec![0.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            None,
            None,
            &ctx,
            &mut area_html,
        );
        assert!(area_html.contains("chart-area"));

        let mut of_pie_html = String::new();
        HtmlRenderer::render_shape_resolved(
            &make_chart_shape(ChartSpec {
                chart_type: ChartType::OfPie,
                of_pie_type: Some(ChartOfPieType::Pie),
                split_type: Some(ChartSplitType::Pos),
                split_pos: Some(1.0),
                series: vec![ChartSeries {
                    name: Some("Split".to_string()),
                    categories: vec!["A".to_string(), "B".to_string()],
                    values: vec![5.0, 0.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            None,
            None,
            &ctx,
            &mut of_pie_html,
        );
        assert!(of_pie_html.contains("chart-of-pie-primary"));
    }
}
