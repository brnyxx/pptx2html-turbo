use std::fmt::Write;

use crate::resolver::style_ref;

use super::text_metrics::{ScriptCategory, classify_script_category, segment_by_script};
use super::{
    AutoFit, Bullet, DEFAULT_FONT_SIZE_PT, FontResolutionEntry, FontResolutionSource, FontScheme,
    HtmlRenderer, ListStyle, ParagraphDefaults, ProvenanceSource, RenderCtx, ResolvedColor,
    RunRenderDefaults, Shape, SpacingValue, StrikethroughType, TextBody, TextCapitalization,
    TextMargins, TextParagraph, TextRun, UnderlineType, VerticalAlign, actions, escape_html,
    placeholder, push_sep,
};

pub(super) struct ParagraphRenderContext<'context, 'model> {
    pub(super) text_style: &'context TextStyleCtx<'model>,
    pub(super) font_ref_font: Option<&'context str>,
    pub(super) font_ref_color: Option<&'context ResolvedColor>,
    pub(super) font_scale: Option<f64>,
    pub(super) line_spacing_reduction: Option<f64>,
}

impl HtmlRenderer {
    pub(super) fn build_text_style_ctx<'a>(
        shape: &'a Shape,
        layout_match: Option<&'a Shape>,
        master_match: Option<&'a Shape>,
        ctx: &RenderCtx<'a>,
    ) -> TextStyleCtx<'a> {
        // Determine which txStyles list to use based on placeholder type
        let ph_type = shape
            .placeholder
            .as_ref()
            .and_then(|ph| ph.ph_type.as_ref());
        let source = placeholder::text_style_source(ph_type);

        let slide_list_style = shape
            .text_body
            .as_ref()
            .and_then(|tb| tb.list_style.as_ref());

        let layout_list_style = layout_match
            .and_then(|matched| matched.text_body.as_ref())
            .and_then(|tb| tb.list_style.as_ref());

        let master_placeholder_list_style = master_match
            .and_then(|matched| matched.text_body.as_ref())
            .and_then(|tb| tb.list_style.as_ref());

        // txStyles from first master
        let master_list_style = ctx.pres.masters.first().and_then(|m| match source {
            placeholder::TextStyleSource::TitleStyle => m.tx_styles.title_style.as_ref(),
            placeholder::TextStyleSource::BodyStyle => m.tx_styles.body_style.as_ref(),
            placeholder::TextStyleSource::OtherStyle => m.tx_styles.other_style.as_ref(),
        });

        // defaultTextStyle from presentation
        let default_list_style = ctx.pres.default_text_style.as_ref();

        TextStyleCtx {
            slide_list_style,
            layout_list_style,
            master_placeholder_list_style,
            master_list_style,
            default_list_style,
        }
    }

    pub(super) fn resolve_text_auto_fit(
        text_body: &TextBody,
        layout_match: Option<&Shape>,
        master_match: Option<&Shape>,
    ) -> AutoFit {
        fn merge_same_mode_norm_autofit(child: &AutoFit, parent: &AutoFit) -> AutoFit {
            match (child, parent) {
                (
                    AutoFit::Normal {
                        font_scale: child_font_scale,
                        line_spacing_reduction: child_line_spacing_reduction,
                    },
                    AutoFit::Normal {
                        font_scale: parent_font_scale,
                        line_spacing_reduction: parent_line_spacing_reduction,
                    },
                ) => AutoFit::Normal {
                    font_scale: child_font_scale.or(*parent_font_scale),
                    line_spacing_reduction: child_line_spacing_reduction
                        .or(*parent_line_spacing_reduction),
                },
                _ => child.clone(),
            }
        }

        let master_auto_fit = master_match
            .and_then(|shape| shape.text_body.as_ref())
            .map(|tb| tb.auto_fit.clone())
            .unwrap_or_default();

        let inherited_auto_fit = if let Some(layout_auto_fit) = layout_match
            .and_then(|shape| shape.text_body.as_ref())
            .map(|tb| tb.auto_fit.clone())
            && !matches!(layout_auto_fit, AutoFit::None)
        {
            merge_same_mode_norm_autofit(&layout_auto_fit, &master_auto_fit)
        } else {
            master_auto_fit
        };

        if !matches!(text_body.auto_fit, AutoFit::None) {
            merge_same_mode_norm_autofit(&text_body.auto_fit, &inherited_auto_fit)
        } else {
            inherited_auto_fit
        }
    }

    pub(super) fn resolve_text_vertical_align(
        text_body: &TextBody,
        layout_match: Option<&Shape>,
        master_match: Option<&Shape>,
    ) -> VerticalAlign {
        if text_body.vertical_align_explicit {
            return text_body.vertical_align.clone();
        }
        if let Some(vertical_align) = layout_match
            .and_then(|shape| shape.text_body.as_ref())
            .filter(|tb| tb.vertical_align_explicit)
            .map(|tb| tb.vertical_align.clone())
        {
            return vertical_align;
        }
        if let Some(vertical_align) = master_match
            .and_then(|shape| shape.text_body.as_ref())
            .filter(|tb| tb.vertical_align_explicit)
            .map(|tb| tb.vertical_align.clone())
        {
            return vertical_align;
        }
        text_body.vertical_align.clone()
    }

    pub(super) fn resolve_text_word_wrap(
        text_body: &TextBody,
        layout_match: Option<&Shape>,
        master_match: Option<&Shape>,
    ) -> bool {
        if text_body.word_wrap_explicit {
            return text_body.word_wrap;
        }
        if let Some(word_wrap) = layout_match
            .and_then(|shape| shape.text_body.as_ref())
            .filter(|tb| tb.word_wrap_explicit)
            .map(|tb| tb.word_wrap)
        {
            return word_wrap;
        }
        if let Some(word_wrap) = master_match
            .and_then(|shape| shape.text_body.as_ref())
            .filter(|tb| tb.word_wrap_explicit)
            .map(|tb| tb.word_wrap)
        {
            return word_wrap;
        }
        text_body.word_wrap
    }

    pub(super) fn resolve_text_anchor_center(
        text_body: &TextBody,
        layout_match: Option<&Shape>,
        master_match: Option<&Shape>,
    ) -> bool {
        if text_body.anchor_center {
            return true;
        }
        if let Some(anchor_center) = layout_match
            .and_then(|shape| shape.text_body.as_ref())
            .map(|tb| tb.anchor_center)
            && anchor_center
        {
            return true;
        }
        if let Some(anchor_center) = master_match
            .and_then(|shape| shape.text_body.as_ref())
            .map(|tb| tb.anchor_center)
            && anchor_center
        {
            return true;
        }
        false
    }

    pub(super) fn resolve_text_rotation(
        text_body: &TextBody,
        layout_match: Option<&Shape>,
        master_match: Option<&Shape>,
    ) -> f64 {
        if text_body.text_rotation_deg != 0.0 {
            return text_body.text_rotation_deg;
        }
        if let Some(rot) = layout_match
            .and_then(|shape| shape.text_body.as_ref())
            .map(|tb| tb.text_rotation_deg)
            && rot != 0.0
        {
            return rot;
        }
        if let Some(rot) = master_match
            .and_then(|shape| shape.text_body.as_ref())
            .map(|tb| tb.text_rotation_deg)
            && rot != 0.0
        {
            return rot;
        }
        0.0
    }

    pub(super) fn resolve_vertical_text<'a>(
        shape: &'a Shape,
        layout_match: Option<&'a Shape>,
        master_match: Option<&'a Shape>,
    ) -> Option<&'a String> {
        if shape.vertical_text_explicit {
            return shape.vertical_text.as_ref();
        }
        if let Some(layout_match) = layout_match
            && layout_match.vertical_text_explicit
        {
            return layout_match.vertical_text.as_ref();
        }
        if let Some(master_match) = master_match
            && master_match.vertical_text_explicit
        {
            return master_match.vertical_text.as_ref();
        }
        shape
            .vertical_text
            .as_ref()
            .or_else(|| layout_match.and_then(|matched| matched.vertical_text.as_ref()))
            .or_else(|| master_match.and_then(|matched| matched.vertical_text.as_ref()))
    }

    pub(super) fn resolve_text_margins(
        text_body: &TextBody,
        layout_match: Option<&Shape>,
        master_match: Option<&Shape>,
    ) -> TextMargins {
        fn side(
            own_explicit: bool,
            own: f64,
            layout_match: Option<&Shape>,
            master_match: Option<&Shape>,
            layout_explicit: fn(&TextBody) -> bool,
            layout_value: fn(&TextBody) -> f64,
        ) -> f64 {
            if own_explicit {
                return own;
            }
            if let Some(value) = layout_match
                .and_then(|shape| shape.text_body.as_ref())
                .filter(|tb| layout_explicit(tb))
                .map(layout_value)
            {
                return value;
            }
            if let Some(value) = master_match
                .and_then(|shape| shape.text_body.as_ref())
                .filter(|tb| layout_explicit(tb))
                .map(layout_value)
            {
                return value;
            }
            own
        }

        TextMargins {
            top: side(
                text_body.margin_top_explicit,
                text_body.margins.top,
                layout_match,
                master_match,
                |tb| tb.margin_top_explicit,
                |tb| tb.margins.top,
            ),
            bottom: side(
                text_body.margin_bottom_explicit,
                text_body.margins.bottom,
                layout_match,
                master_match,
                |tb| tb.margin_bottom_explicit,
                |tb| tb.margins.bottom,
            ),
            left: side(
                text_body.margin_left_explicit,
                text_body.margins.left,
                layout_match,
                master_match,
                |tb| tb.margin_left_explicit,
                |tb| tb.margins.left,
            ),
            right: side(
                text_body.margin_right_explicit,
                text_body.margins.right,
                layout_match,
                master_match,
                |tb| tb.margin_right_explicit,
                |tb| tb.margins.right,
            ),
        }
    }

    /// Resolve fontRef from shape's <p:style> to a font-family name and optional color
    pub(super) fn resolve_font_ref_font(
        shape: &Shape,
        ctx: &RenderCtx<'_>,
    ) -> Option<(String, Option<ResolvedColor>)> {
        let sr = shape.style_ref.as_ref()?;
        let font_ref = sr.font_ref.as_ref()?;
        let theme = ctx.pres.primary_theme()?;
        let font_scheme = &theme.font_scheme;
        let scheme = ctx.scheme?;
        let clr_map = ctx.clr_map?;
        style_ref::resolve_font_ref(font_ref, font_scheme, scheme, clr_map)
    }

    pub(super) fn render_paragraph(
        para: &TextParagraph,
        ctx: &RenderCtx<'_>,
        auto_num_counters: &mut [i32; 9],
        html: &mut String,
    ) {
        let text_style = TextStyleCtx::default();
        Self::render_paragraph_with_defaults(
            para,
            ctx,
            auto_num_counters,
            ParagraphRenderContext {
                text_style: &text_style,
                font_ref_font: None,
                font_ref_color: None,
                font_scale: None,
                line_spacing_reduction: None,
            },
            html,
        );
    }

    /// Render paragraph with inherited text style defaults from txStyles / defaultTextStyle
    pub(super) fn render_paragraph_with_defaults(
        para: &TextParagraph,
        ctx: &RenderCtx<'_>,
        auto_num_counters: &mut [i32; 9],
        render: ParagraphRenderContext<'_, '_>,
        html: &mut String,
    ) {
        let level = (para.level as usize).min(8);

        // Look up inherited paragraph defaults for this level
        let inherited = render.text_style.get_level_defaults(level);

        let align = para.alignment.to_css();
        let mut para_style = String::with_capacity(128);
        let _ = write!(para_style, "text-align: {align}");
        if para.rtl {
            para_style.push_str("; direction: rtl; unicode-bidi: bidi-override");
        }

        // Line spacing (explicit > inherited), with optional reduction from normAutofit
        let line_spacing = para
            .line_spacing
            .as_ref()
            .or_else(|| inherited.and_then(|d| d.line_spacing.as_ref()));
        let reduction_factor = render
            .line_spacing_reduction
            .map(|r| 1.0 - r)
            .unwrap_or(1.0);
        if let Some(ls) = line_spacing {
            match ls {
                SpacingValue::Percent(p) => {
                    let effective = p * reduction_factor;
                    let _ = write!(para_style, "; line-height: {effective:.2}");
                }
                SpacingValue::Points(pt) => {
                    let effective = pt * reduction_factor;
                    let _ = write!(para_style, "; line-height: {effective:.1}pt");
                }
            }
        } else if render.line_spacing_reduction.is_some() {
            // Apply reduction to default line-height (1.0 = 100%)
            let effective = reduction_factor;
            let _ = write!(para_style, "; line-height: {effective:.2}");
        }
        // Space before (explicit > inherited)
        let space_before = para
            .space_before
            .as_ref()
            .or_else(|| inherited.and_then(|d| d.space_before.as_ref()));
        if let Some(sb) = space_before {
            match sb {
                SpacingValue::Percent(p) => {
                    let _ = write!(para_style, "; margin-top: {p:.1}em");
                }
                SpacingValue::Points(pt) => {
                    let _ = write!(para_style, "; margin-top: {pt:.1}pt");
                }
            }
        }
        // Space after (explicit > inherited)
        let space_after = para
            .space_after
            .as_ref()
            .or_else(|| inherited.and_then(|d| d.space_after.as_ref()));
        if let Some(sa) = space_after {
            match sa {
                SpacingValue::Percent(p) => {
                    let _ = write!(para_style, "; margin-bottom: {p:.1}em");
                }
                SpacingValue::Points(pt) => {
                    let _ = write!(para_style, "; margin-bottom: {pt:.1}pt");
                }
            }
        }

        // Level-based indentation via margin_left and indent (explicit > inherited)
        let margin_left = para
            .margin_left
            .or_else(|| inherited.and_then(|d| d.margin_left));
        let indent = para.indent.or_else(|| inherited.and_then(|d| d.indent));

        if let Some(ml) = margin_left {
            let _ = write!(para_style, "; padding-left: {ml:.1}pt");
        } else if para.level > 0 {
            // Fallback: ~36pt (0.5in) per level when no explicit margin
            let margin = para.level as f64 * 36.0;
            let _ = write!(para_style, "; padding-left: {margin:.1}pt");
        }
        if let Some(ind) = indent {
            let _ = write!(para_style, "; text-indent: {ind:.1}pt");
        }

        let _ = write!(html, "<p class=\"paragraph\" style=\"{para_style}\">");

        // Skip bullet for empty paragraphs (no visible text content)
        let has_visible_text = para
            .runs
            .iter()
            .any(|r| !r.is_break && !r.text.trim().is_empty());

        // Bullet rendering (explicit > inherited)
        let bullet = if has_visible_text {
            para.bullet
                .as_ref()
                .or_else(|| inherited.and_then(|d| d.bullet.as_ref()))
        } else {
            None
        };
        if let Some(bullet) = bullet {
            match bullet {
                Bullet::Char(bc) => {
                    // Reset counters at deeper levels when a char bullet is encountered
                    for counter in auto_num_counters.iter_mut().skip(level) {
                        *counter = 0;
                    }
                    let mut bullet_style = String::new();
                    if let Some(ref font) = bc.font {
                        let _ = write!(bullet_style, "font-family: '{}'; ", escape_html(font));
                    }
                    if let Some(ref color) = bc.color
                        && let Some(css) = ctx.color_to_css(color)
                    {
                        let _ = write!(bullet_style, "color: {}; ", css);
                    }
                    if let Some(size_pct) = bc.size_pct {
                        if size_pct < 0.0 {
                            // Absolute points (stored as negative)
                            let pts = -size_pct;
                            let _ = write!(bullet_style, "font-size: {pts:.1}pt; ");
                        } else if (size_pct - 1.0).abs() > 0.01 {
                            // Percentage of text size (only if not 100%)
                            let pct = size_pct * 100.0;
                            let _ = write!(bullet_style, "font-size: {pct:.0}%; ");
                        }
                    }
                    let _ = write!(
                        html,
                        "<span class=\"bullet\" style=\"{bullet_style}\">{} </span>",
                        escape_html(&bc.char)
                    );
                }
                Bullet::AutoNum(an) => {
                    // Increment counter for this level
                    let start = an.start_at.unwrap_or(1);
                    auto_num_counters[level] += 1;
                    // Reset deeper level counters
                    for counter in auto_num_counters.iter_mut().skip(level + 1) {
                        *counter = 0;
                    }
                    let counter_val = start + auto_num_counters[level] - 1;

                    let label = format_auto_num(&an.num_type, counter_val);
                    let mut bullet_style = String::new();
                    if let Some(ref font) = an.font {
                        let _ = write!(bullet_style, "font-family: '{}'; ", escape_html(font));
                    }
                    if let Some(ref color) = an.color
                        && let Some(css) = ctx.color_to_css(color)
                    {
                        let _ = write!(bullet_style, "color: {}; ", css);
                    }
                    if let Some(size_pct) = an.size_pct {
                        if size_pct < 0.0 {
                            let pts = -size_pct;
                            let _ = write!(bullet_style, "font-size: {pts:.1}pt; ");
                        } else if (size_pct - 1.0).abs() > 0.01 {
                            let pct = size_pct * 100.0;
                            let _ = write!(bullet_style, "font-size: {pct:.0}%; ");
                        }
                    }
                    let _ = write!(
                        html,
                        "<span class=\"bullet\" style=\"{bullet_style}\">{} </span>",
                        escape_html(&label)
                    );
                }
                Bullet::None => {
                    // Reset counters when bullet is explicitly suppressed
                    for counter in auto_num_counters.iter_mut().skip(level) {
                        *counter = 0;
                    }
                }
            }
        } else {
            // No bullet specified — reset counters at this level
            for counter in auto_num_counters.iter_mut().skip(level) {
                *counter = 0;
            }
        }

        // Get inherited run defaults for this level
        let run_defaults = inherited.and_then(|d| d.def_run_props.as_ref());

        for run in &para.runs {
            Self::render_run_with_defaults(
                run,
                ctx,
                RunRenderDefaults {
                    para_def_rpr: para.def_rpr.as_ref(),
                    run_defaults,
                    font_ref_font: render.font_ref_font,
                    font_ref_color: render.font_ref_color,
                    font_scale: render.font_scale,
                },
                html,
            );
        }

        if para.runs.is_empty() {
            html.push_str("&nbsp;");
        }

        html.push_str("</p>\n");
    }

    /// Render run with inherited defaults from txStyles/defaultTextStyle
    pub(super) fn render_run_with_defaults(
        run: &TextRun,
        ctx: &RenderCtx<'_>,
        defaults: RunRenderDefaults<'_>,
        html: &mut String,
    ) {
        // Line break (early return)
        if run.is_break {
            html.push_str("<br/>");
            return;
        }

        let mut run_style = String::with_capacity(128);

        // Font family: explicit > para defRPr > inherited defRPr > fontRef > theme
        fn choose_script_font<'a>(
            text: &str,
            latin: Option<&'a str>,
            east_asian: Option<&'a str>,
            complex_script: Option<&'a str>,
        ) -> Option<&'a str> {
            match classify_script_category(text) {
                ScriptCategory::Complex => complex_script.or(latin).or(east_asian),
                ScriptCategory::Emoji => complex_script.or(latin).or(east_asian),
                ScriptCategory::EastAsian => east_asian.or(latin).or(complex_script),
                ScriptCategory::LatinLike => latin.or(east_asian).or(complex_script),
            }
        }

        fn choose_script_font_for_category<'a>(
            category: ScriptCategory,
            latin: Option<&'a str>,
            east_asian: Option<&'a str>,
            complex_script: Option<&'a str>,
        ) -> Option<&'a str> {
            match category {
                ScriptCategory::Complex => complex_script.or(latin).or(east_asian),
                ScriptCategory::Emoji => complex_script.or(latin).or(east_asian),
                ScriptCategory::EastAsian => east_asian.or(latin).or(complex_script),
                ScriptCategory::LatinLike => latin.or(east_asian).or(complex_script),
            }
        }

        let font = choose_script_font(
            &run.text,
            run.font.latin.as_deref(),
            run.font.east_asian.as_deref(),
            run.font.complex_script.as_deref(),
        );

        let font_scheme = ctx.pres.primary_theme().map(|t| &t.font_scheme);

        // Resolve font through typeface -> theme -> inherited -> fontRef chain,
        // skipping empty strings and unresolved theme references ("+mj-*"/"+mn-*").
        fn resolve_font_name<'a>(
            name: &'a str,
            font_scheme: Option<&'a FontScheme>,
        ) -> Option<&'a str> {
            if name.starts_with('+') {
                font_scheme.and_then(|fs| fs.resolve_typeface(name))
            } else if name.is_empty() {
                None
            } else {
                Some(name)
            }
        }

        let font_resolution = font
            .map(|f| (Some(f), FontResolutionSource::ExplicitRun))
            .or_else(|| {
                defaults.para_def_rpr.and_then(|pd| {
                    choose_script_font(
                        &run.text,
                        pd.font_latin.as_deref(),
                        pd.font_ea.as_deref(),
                        pd.font_cs.as_deref(),
                    )
                    .map(|f| (Some(f), FontResolutionSource::ParagraphDefaults))
                })
            })
            .or_else(|| {
                defaults.run_defaults.and_then(|rd| {
                    choose_script_font(
                        &run.text,
                        rd.font_latin.as_deref(),
                        rd.font_ea.as_deref(),
                        rd.font_cs.as_deref(),
                    )
                    .map(|f| (Some(f), FontResolutionSource::InheritedDefaults))
                })
            })
            .or_else(|| {
                defaults
                    .font_ref_font
                    .map(|f| (Some(f), FontResolutionSource::FontRef))
            });

        let (requested_font, font_source, resolved_font) =
            if let Some((requested, source)) = font_resolution {
                (
                    requested.map(|s| s.to_string()),
                    Some(source),
                    requested
                        .and_then(|f| resolve_font_name(f, font_scheme))
                        .map(|s| s.to_string()),
                )
            } else {
                (None, None, None)
            };

        let font_slide_index = ctx.collector.borrow().current_slide_index + 1;
        ctx.push_font_resolution(FontResolutionEntry {
            slide_index: font_slide_index,
            shape_name: None,
            run_text: run.text.clone(),
            requested_typeface: requested_font.clone(),
            resolved_typeface: resolved_font.clone(),
            source: font_source,
            fallback_used: match (&requested_font, &resolved_font) {
                (Some(requested), Some(resolved)) => requested != resolved,
                _ => false,
            },
        });

        // Font size: explicit > para defRPr > inherited, scaled by fontScale from normAutofit
        let font_size = run
            .style
            .font_size
            .or_else(|| defaults.para_def_rpr.and_then(|pd| pd.font_size))
            .or_else(|| defaults.run_defaults.and_then(|rd| rd.font_size))
            .or(Some(DEFAULT_FONT_SIZE_PT));
        if let Some(sz) = font_size {
            let effective_sz = sz * defaults.font_scale.unwrap_or(1.0);
            push_sep(&mut run_style);
            let _ = write!(run_style, "font-size: {effective_sz:.1}pt");
        }

        // Bold: explicit > para defRPr > inherited
        let bold = if run.style.bold {
            true
        } else if let Some(b) = defaults.para_def_rpr.and_then(|pd| pd.bold) {
            b
        } else {
            defaults
                .run_defaults
                .and_then(|rd| rd.bold)
                .unwrap_or(false)
        };
        if bold {
            push_sep(&mut run_style);
            run_style.push_str("font-weight: bold");
        }

        // Italic: explicit > para defRPr > inherited
        let italic = if run.style.italic {
            true
        } else if let Some(i) = defaults.para_def_rpr.and_then(|pd| pd.italic) {
            i
        } else {
            defaults
                .run_defaults
                .and_then(|rd| rd.italic)
                .unwrap_or(false)
        };
        if italic {
            push_sep(&mut run_style);
            run_style.push_str("font-style: italic");
        }

        let underline: UnderlineType = if !matches!(run.style.underline, UnderlineType::None) {
            run.style.underline.clone()
        } else if let Some(u) = defaults.para_def_rpr.and_then(|pd| pd.underline.clone()) {
            u.clone()
        } else {
            defaults
                .run_defaults
                .and_then(|rd| rd.underline.clone())
                .unwrap_or_default()
        };
        if let Some(ul_css) = underline.to_css() {
            push_sep(&mut run_style);
            run_style.push_str(&ul_css);
        }
        let strikethrough: StrikethroughType =
            if !matches!(run.style.strikethrough, StrikethroughType::None) {
                run.style.strikethrough.clone()
            } else if let Some(s) = defaults
                .para_def_rpr
                .and_then(|pd| pd.strikethrough.clone())
            {
                s.clone()
            } else {
                defaults
                    .run_defaults
                    .and_then(|rd| rd.strikethrough.clone())
                    .unwrap_or_default()
            };
        if let Some(st_css) = strikethrough.to_css() {
            push_sep(&mut run_style);
            run_style.push_str(st_css);
        }
        let capitalization = if !matches!(run.style.capitalization, TextCapitalization::None) {
            run.style.capitalization.clone()
        } else if let Some(cap) = defaults
            .para_def_rpr
            .and_then(|pd| pd.capitalization.clone())
        {
            cap
        } else {
            defaults
                .run_defaults
                .and_then(|rd| rd.capitalization.clone())
                .unwrap_or_default()
        };
        if let Some(cap_css) = capitalization.to_css() {
            push_sep(&mut run_style);
            run_style.push_str(cap_css);
        }

        // Color -- explicit > para defRPr > inherited > fontRef > none
        // Use or_else chaining so that a None at any level falls through to the next
        let color_css = if !run.style.color.is_none() {
            ctx.color_to_css(&run.style.color)
        } else {
            defaults
                .para_def_rpr
                .and_then(|pd| pd.color.as_ref())
                .and_then(|c| ctx.color_to_css(c))
                .or_else(|| {
                    defaults
                        .run_defaults
                        .and_then(|rd| rd.color.as_ref())
                        .and_then(|c| ctx.color_to_css(c))
                })
                .or_else(|| defaults.font_ref_color.as_ref().map(|c| c.to_css()))
        };
        if let Some(css_color) = color_css {
            push_sep(&mut run_style);
            let _ = write!(run_style, "color: {css_color}");
        }

        // Superscript/subscript -- use actual OOXML baseline percentage
        // baseline is in thousandths of percent (e.g., 30000 = 30%)
        let baseline = run
            .style
            .baseline
            .or_else(|| defaults.para_def_rpr.and_then(|pd| pd.baseline))
            .or_else(|| defaults.run_defaults.and_then(|rd| rd.baseline));
        if let Some(baseline) = baseline
            && baseline != 0
        {
            let pct = baseline as f64 / 1000.0;
            let abs_pct = pct.abs();
            // Scale font size proportionally: larger offset = smaller font
            let scale = (1.0 - abs_pct / 100.0).max(0.5);
            push_sep(&mut run_style);
            let _ = write!(
                run_style,
                "vertical-align: {pct:.1}%; font-size: {scale:.2}em"
            );
        }

        // Letter spacing
        let letter_spacing = run
            .style
            .letter_spacing
            .or_else(|| defaults.para_def_rpr.and_then(|pd| pd.letter_spacing))
            .or_else(|| defaults.run_defaults.and_then(|rd| rd.letter_spacing));
        if let Some(spacing) = letter_spacing {
            push_sep(&mut run_style);
            let _ = write!(run_style, "letter-spacing: {spacing:.2}pt");
        }

        // Text highlight
        if let Some(ref highlight) = run.style.highlight
            && let Some(hl_css) = ctx.color_to_css(highlight)
        {
            push_sep(&mut run_style);
            let _ = write!(run_style, "background-color: {hl_css}");
        }

        // Text shadow
        if let Some(ref shadow) = run.style.shadow {
            let angle_rad = shadow.dir.to_radians();
            let dx = shadow.dist * angle_rad.cos();
            let dy = shadow.dist * angle_rad.sin();
            let shadow_color = ctx
                .color_to_css(&shadow.color)
                .unwrap_or_else(|| "rgba(0,0,0,0.5)".to_string());
            push_sep(&mut run_style);
            let _ = write!(
                run_style,
                "text-shadow: {dx:.1}pt {dy:.1}pt {blur:.1}pt {shadow_color}",
                blur = shadow.blur_rad,
            );
        }

        let segment_html = {
            let segments = segment_by_script(&run.text);
            let mut inner_html = String::new();
            for segment in segments {
                let requested_segment_font = choose_script_font_for_category(
                    segment.category,
                    run.font.latin.as_deref(),
                    run.font.east_asian.as_deref(),
                    run.font.complex_script.as_deref(),
                )
                .or_else(|| {
                    defaults.para_def_rpr.and_then(|pd| {
                        choose_script_font_for_category(
                            segment.category,
                            pd.font_latin.as_deref(),
                            pd.font_ea.as_deref(),
                            pd.font_cs.as_deref(),
                        )
                    })
                })
                .or_else(|| {
                    defaults.run_defaults.and_then(|rd| {
                        choose_script_font_for_category(
                            segment.category,
                            rd.font_latin.as_deref(),
                            rd.font_ea.as_deref(),
                            rd.font_cs.as_deref(),
                        )
                    })
                })
                .or(defaults.font_ref_font);

                let resolved_segment_font = requested_segment_font
                    .and_then(|name| resolve_font_name(name, font_scheme))
                    .map(str::to_string);

                if let Some(font_name) = resolved_segment_font {
                    let _ = write!(
                        inner_html,
                        "<span class=\"run-segment\" style=\"font-family: '{}'\">{}</span>",
                        escape_html(&font_name),
                        escape_html(&segment.text)
                    );
                } else {
                    let _ = write!(
                        inner_html,
                        "<span class=\"run-segment\">{}</span>",
                        escape_html(&segment.text)
                    );
                }
            }
            inner_html
        };

        actions::render_run_wrapper(run.hyperlink.as_deref(), &run_style, &segment_html, html);
    }
}

pub(super) fn format_auto_num(num_type: &str, val: i32) -> String {
    match num_type {
        "arabicPeriod" => format!("{val}."),
        "arabicParenR" => format!("{val})"),
        "arabicParenBoth" => format!("({val})"),
        "arabicPlain" => format!("{val}"),
        "alphaLcPeriod" => format!("{}.", to_alpha_lc(val)),
        "alphaLcParenR" => format!("{})", to_alpha_lc(val)),
        "alphaLcParenBoth" => format!("({})", to_alpha_lc(val)),
        "alphaUcPeriod" => format!("{}.", to_alpha_uc(val)),
        "alphaUcParenR" => format!("{})", to_alpha_uc(val)),
        "alphaUcParenBoth" => format!("({})", to_alpha_uc(val)),
        "romanLcPeriod" => format!("{}.", to_roman_lc(val)),
        "romanLcParenR" => format!("{})", to_roman_lc(val)),
        "romanLcParenBoth" => format!("({})", to_roman_lc(val)),
        "romanUcPeriod" => format!("{}.", to_roman_uc(val)),
        "romanUcParenR" => format!("{})", to_roman_uc(val)),
        "romanUcParenBoth" => format!("({})", to_roman_uc(val)),
        _ => format!("{val}."),
    }
}

/// Convert number to lowercase alphabetic (1=a, 2=b, ..., 26=z, 27=aa, ...)
pub(super) fn to_alpha_lc(mut val: i32) -> String {
    if val <= 0 {
        return "a".to_string();
    }
    let mut result = String::new();
    while val > 0 {
        val -= 1;
        result.insert(0, (b'a' + (val % 26) as u8) as char);
        val /= 26;
    }
    result
}

/// Convert number to uppercase alphabetic
pub(super) fn to_alpha_uc(val: i32) -> String {
    to_alpha_lc(val).to_uppercase()
}

/// Convert number to lowercase Roman numerals
pub(super) fn to_roman_lc(val: i32) -> String {
    to_roman_uc(val).to_lowercase()
}

/// Convert number to uppercase Roman numerals
pub(super) fn to_roman_uc(mut val: i32) -> String {
    if val <= 0 || val > 3999 {
        return val.to_string();
    }
    const NUMERALS: &[(i32, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut result = String::new();
    for &(value, symbol) in NUMERALS {
        while val >= value {
            result.push_str(symbol);
            val -= value;
        }
    }
    result
}

/// Context for resolving inherited text styles from txStyles/defaultTextStyle
#[derive(Default)]
pub(super) struct TextStyleCtx<'a> {
    slide_list_style: Option<&'a ListStyle>,
    layout_list_style: Option<&'a ListStyle>,
    master_placeholder_list_style: Option<&'a ListStyle>,
    master_list_style: Option<&'a ListStyle>,
    default_list_style: Option<&'a ListStyle>,
}

impl<'a> TextStyleCtx<'a> {
    pub(super) fn primary_source(&self) -> Option<ProvenanceSource> {
        if self.slide_list_style.is_some() {
            return Some(ProvenanceSource::SlideListStyle);
        }
        if self.layout_list_style.is_some() {
            return Some(ProvenanceSource::LayoutListStyle);
        }
        if self.master_placeholder_list_style.is_some() {
            return Some(ProvenanceSource::MasterListStyle);
        }
        if self.master_list_style.is_some() {
            return Some(ProvenanceSource::MasterTextStyles);
        }
        if self.default_list_style.is_some() {
            return Some(ProvenanceSource::DefaultTextStyle);
        }
        None
    }

    /// Get paragraph defaults for a given level (0-based).
    /// Priority: slide lstStyle > layout/master/template styles > defaultTextStyle
    pub(super) fn get_level_defaults(&self, level: usize) -> Option<&'a ParagraphDefaults> {
        if level >= 9 {
            return None;
        }
        if let Some(ls) = self.slide_list_style
            && let Some(ref pd) = ls.levels[level]
        {
            return Some(pd);
        }
        if let Some(ls) = self.layout_list_style
            && let Some(ref pd) = ls.levels[level]
        {
            return Some(pd);
        }
        if let Some(ls) = self.master_placeholder_list_style
            && let Some(ref pd) = ls.levels[level]
        {
            return Some(pd);
        }
        if let Some(ls) = self.master_list_style
            && let Some(ref pd) = ls.levels[level]
        {
            return Some(pd);
        }
        // Fallback to defaultTextStyle
        if let Some(ls) = self.default_list_style
            && let Some(ref pd) = ls.levels[level]
        {
            return Some(pd);
        }
        None
    }
}
