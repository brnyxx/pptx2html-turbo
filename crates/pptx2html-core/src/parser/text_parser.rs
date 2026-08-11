use quick_xml::events::BytesStart;

use super::master_parser::{is_lvl_ppr, parse_def_rpr_attrs, parse_lvl_index, parse_lvl_ppr_attrs};
use super::slide_parser::ShapeBuilder;
use super::xml_utils;
use crate::model::*;

#[derive(Default)]
pub(crate) struct TextSaxState {
    pub(crate) paragraph: Option<ParagraphBuilder>,
    pub(crate) run: Option<RunBuilder>,
    pub(crate) in_text: bool,
    pub(crate) in_run_properties: bool,
    pub(crate) in_line_spacing: bool,
    pub(crate) in_space_before: bool,
    pub(crate) in_space_after: bool,
    pub(crate) in_default_run_properties: bool,
    pub(crate) in_list_style: bool,
    pub(crate) in_list_default_run_properties: bool,
    pub(crate) in_list_line_spacing: bool,
    pub(crate) in_list_space_before: bool,
    pub(crate) in_list_space_after: bool,
    pub(crate) level: Option<usize>,
    pub(crate) paragraph_defaults: Option<ParagraphDefaults>,
    pub(crate) run_defaults: Option<RunDefaults>,
    pub(crate) in_bullet_color: bool,
}

impl TextSaxState {
    pub(crate) fn handle_text(&mut self, text: &str) -> bool {
        if !self.in_text {
            return false;
        }

        append_text(&mut self.run, text);
        true
    }

    pub(crate) fn handle_start(
        &mut self,
        local: &str,
        element: &BytesStart<'_>,
        shape: &mut Option<ShapeBuilder>,
        in_table_cell: bool,
    ) -> bool {
        if in_table_cell {
            return false;
        }
        match local {
            "txBody" => {
                if let Some(shape) = shape.as_mut() {
                    shape.has_text_body = true;
                }
            }
            "bodyPr" if shape.is_some() => parse_body_properties(element, shape),
            "lstStyle" if shape.is_some() => self.in_list_style = true,
            tag if self.in_list_style && is_lvl_ppr(tag) => {
                self.level = Some(parse_lvl_index(tag));
                let mut defaults = ParagraphDefaults::default();
                parse_lvl_ppr_attrs(element, &mut defaults);
                self.paragraph_defaults = Some(defaults);
            }
            "defRPr" if self.in_list_style && self.level.is_some() => {
                self.in_list_default_run_properties = true;
                let mut defaults = RunDefaults::default();
                parse_def_rpr_attrs(element, &mut defaults);
                self.run_defaults = Some(defaults);
            }
            "lnSpc"
                if self.in_list_style
                    && self.level.is_some()
                    && !self.in_list_default_run_properties =>
            {
                self.in_list_line_spacing = true
            }
            "spcBef"
                if self.in_list_style
                    && self.level.is_some()
                    && !self.in_list_default_run_properties =>
            {
                self.in_list_space_before = true
            }
            "spcAft"
                if self.in_list_style
                    && self.level.is_some()
                    && !self.in_list_default_run_properties =>
            {
                self.in_list_space_after = true
            }
            "normAutofit" | "noAutofit" | "spAutoFit" if shape.is_some() => {
                shape
                    .as_mut()
                    .expect("shape builder for autofit")
                    .text_auto_fit = parse_auto_fit(local, element)
            }
            "p" if shape.is_some() => start_paragraph(&mut self.paragraph),
            "pPr" if self.paragraph.is_some() => {
                parse_paragraph_properties(element, &mut self.paragraph)
            }
            "defRPr" if self.paragraph.is_some() && self.run.is_none() => {
                self.in_default_run_properties = true;
                apply_paragraph_default_run_properties(
                    self.paragraph
                        .as_mut()
                        .expect("paragraph builder for defRPr"),
                    element,
                );
            }
            "lnSpc" if self.paragraph.is_some() => self.in_line_spacing = true,
            "spcBef" if self.paragraph.is_some() => self.in_space_before = true,
            "spcAft" if self.paragraph.is_some() => self.in_space_after = true,
            "buClr" if self.paragraph.is_some() => self.in_bullet_color = true,
            "r" if self.paragraph.is_some() => start_run(&mut self.run),
            "rPr" if self.run.is_some() => {
                self.in_run_properties = true;
                parse_run_properties(element, &mut self.run);
            }
            "t" if self.run.is_some() => self.in_text = true,
            "br" if self.paragraph.is_some() => self.push_break(),
            _ => return false,
        }
        true
    }

    pub(crate) fn handle_empty(
        &mut self,
        local: &str,
        element: &BytesStart<'_>,
        shape: &mut Option<ShapeBuilder>,
        in_table_cell: bool,
    ) -> bool {
        if in_table_cell {
            return false;
        }
        match local {
            "bodyPr" if shape.is_some() => parse_body_properties(element, shape),
            "normAutofit" | "noAutofit" | "spAutoFit" if shape.is_some() => {
                shape
                    .as_mut()
                    .expect("shape builder for autofit")
                    .text_auto_fit = parse_auto_fit(local, element)
            }
            "pPr" if self.paragraph.is_some() => {
                parse_paragraph_properties(element, &mut self.paragraph)
            }
            "rPr" if self.run.is_some() => parse_run_properties(element, &mut self.run),
            "defRPr" if self.paragraph.is_some() && self.run.is_none() => {
                apply_paragraph_default_run_properties(
                    self.paragraph
                        .as_mut()
                        .expect("paragraph builder for defRPr"),
                    element,
                )
            }
            "br" if self.paragraph.is_some() => self.push_break(),
            "latin" | "ea" | "cs" => {
                let Some(typeface) = xml_utils::attr_str(element, "typeface") else {
                    return true;
                };
                if self.in_list_default_run_properties {
                    if let Some(defaults) = self.run_defaults.as_mut() {
                        assign_typeface_to_defaults(defaults, local, typeface);
                    }
                } else if self.in_default_run_properties {
                    if let Some(paragraph) = self.paragraph.as_mut() {
                        assign_typeface_to_paragraph(paragraph, local, typeface);
                    }
                } else if let Some(run) = self.run.as_mut() {
                    assign_typeface_to_run(run, local, typeface);
                }
            }
            "spcPct" | "spcPts" => {
                if let Some(spacing) = parse_spacing(local, element) {
                    if self.in_list_style {
                        assign_spacing_defaults(
                            self.paragraph_defaults.as_mut(),
                            spacing,
                            self.in_list_line_spacing,
                            self.in_list_space_before,
                            self.in_list_space_after,
                        );
                    } else {
                        assign_spacing_paragraph(
                            self.paragraph.as_mut(),
                            spacing,
                            self.in_line_spacing,
                            self.in_space_before,
                            self.in_space_after,
                        );
                    }
                }
            }
            "buFont" => parse_bullet_font(element, &mut self.paragraph),
            "buSzPct" | "buSzPts" => parse_bullet_size(local, element, &mut self.paragraph),
            "buNone" | "buChar" | "buAutoNum" => parse_bullet(local, element, &mut self.paragraph),
            _ => return false,
        }
        true
    }

    pub(crate) fn handle_end(
        &mut self,
        local: &str,
        shape: &mut Option<ShapeBuilder>,
        current_color: &mut Option<Color>,
    ) -> bool {
        match local {
            "t" if self.in_text => self.in_text = false,
            "rPr" if self.in_run_properties => self.in_run_properties = false,
            "defRPr" if self.in_default_run_properties => {
                if let Some(color) = current_color.take()
                    && let Some(paragraph) = self.paragraph.as_mut()
                {
                    paragraph.def_rpr_color = Some(color);
                }
                self.in_default_run_properties = false;
            }
            "defRPr" if self.in_list_default_run_properties => {
                if let Some(defaults) = self.paragraph_defaults.as_mut() {
                    defaults.def_run_props = self.run_defaults.take();
                }
                self.in_list_default_run_properties = false;
            }
            "lnSpc" if self.in_list_line_spacing => self.in_list_line_spacing = false,
            "lnSpc" if self.in_line_spacing => self.in_line_spacing = false,
            "spcBef" if self.in_list_space_before => self.in_list_space_before = false,
            "spcBef" if self.in_space_before => self.in_space_before = false,
            "spcAft" if self.in_list_space_after => self.in_list_space_after = false,
            "spcAft" if self.in_space_after => self.in_space_after = false,
            "lstStyle" if self.in_list_style => self.in_list_style = false,
            tag if self.in_list_style && is_lvl_ppr(tag) => {
                if let (Some(defaults), Some(level)) = (self.paragraph_defaults.take(), self.level)
                {
                    store_level_defaults(shape, level, defaults);
                }
                self.level = None;
            }
            "buClr" if self.in_bullet_color => self.in_bullet_color = false,
            "r" if self.paragraph.is_some() => finish_run(&mut self.run, &mut self.paragraph),
            "p" if shape.is_some() => finish_paragraph(&mut self.paragraph, shape),
            _ => return false,
        }
        true
    }

    pub(crate) fn push_break(&mut self) {
        let run = RunBuilder {
            is_break: true,
            text: "\n".to_owned(),
            ..Default::default()
        };
        if let Some(paragraph) = self.paragraph.as_mut() {
            paragraph.runs.push(run.build());
        }
    }
}

pub(crate) fn store_level_defaults(
    shape: &mut Option<ShapeBuilder>,
    level: usize,
    defaults: ParagraphDefaults,
) {
    if level >= 9 {
        return;
    }
    if let Some(shape) = shape.as_mut() {
        let list_style = shape.text_list_style.get_or_insert_with(ListStyle::default);
        list_style.levels[level] = Some(defaults);
    }
}

#[derive(Default)]
pub(crate) struct ParagraphBuilder {
    pub(crate) runs: Vec<TextRun>,
    pub(crate) alignment: Alignment,
    pub(crate) rtl: bool,
    pub(crate) level: u32,
    pub(crate) indent: Option<f64>,
    pub(crate) margin_left: Option<f64>,
    pub(crate) bullet: Option<Bullet>,
    pub(crate) line_spacing: Option<SpacingValue>,
    pub(crate) space_before: Option<SpacingValue>,
    pub(crate) space_after: Option<SpacingValue>,
    pub(crate) bu_font: Option<String>,
    pub(crate) bu_size_pct: Option<f64>,
    pub(crate) bu_color: Option<Color>,
    pub(crate) def_rpr_font_size: Option<f64>,
    pub(crate) def_rpr_letter_spacing: Option<f64>,
    pub(crate) def_rpr_baseline: Option<i32>,
    pub(crate) def_rpr_capitalization: Option<TextCapitalization>,
    pub(crate) def_rpr_underline: Option<UnderlineType>,
    pub(crate) def_rpr_strikethrough: Option<StrikethroughType>,
    pub(crate) def_rpr_bold: Option<bool>,
    pub(crate) def_rpr_italic: Option<bool>,
    pub(crate) def_rpr_color: Option<Color>,
    pub(crate) def_rpr_font_latin: Option<String>,
    pub(crate) def_rpr_font_ea: Option<String>,
    pub(crate) def_rpr_font_cs: Option<String>,
}

impl ParagraphBuilder {
    pub(crate) fn build(self) -> TextParagraph {
        let def_rpr = if self.def_rpr_font_size.is_some()
            || self.def_rpr_letter_spacing.is_some()
            || self.def_rpr_baseline.is_some()
            || self.def_rpr_capitalization.is_some()
            || self.def_rpr_underline.is_some()
            || self.def_rpr_strikethrough.is_some()
            || self.def_rpr_bold.is_some()
            || self.def_rpr_italic.is_some()
            || self.def_rpr_color.is_some()
            || self.def_rpr_font_latin.is_some()
            || self.def_rpr_font_ea.is_some()
            || self.def_rpr_font_cs.is_some()
        {
            Some(ParagraphDefRPr {
                font_size: self.def_rpr_font_size,
                letter_spacing: self.def_rpr_letter_spacing,
                baseline: self.def_rpr_baseline,
                capitalization: self.def_rpr_capitalization,
                underline: self.def_rpr_underline,
                strikethrough: self.def_rpr_strikethrough,
                bold: self.def_rpr_bold,
                italic: self.def_rpr_italic,
                color: self.def_rpr_color,
                font_latin: self.def_rpr_font_latin,
                font_ea: self.def_rpr_font_ea,
                font_cs: self.def_rpr_font_cs,
            })
        } else {
            None
        };
        TextParagraph {
            runs: self.runs,
            alignment: self.alignment,
            rtl: self.rtl,
            line_spacing: self.line_spacing,
            space_before: self.space_before,
            space_after: self.space_after,
            indent: self.indent,
            margin_left: self.margin_left,
            bullet: self.bullet,
            level: self.level,
            def_rpr,
        }
    }
}

#[derive(Default)]
pub(crate) struct RunBuilder {
    pub(crate) text: String,
    pub(crate) font_size: Option<f64>,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) underline: UnderlineType,
    pub(crate) strikethrough: StrikethroughType,
    pub(crate) capitalization: TextCapitalization,
    pub(crate) color: Color,
    pub(crate) font_latin: Option<String>,
    pub(crate) font_ea: Option<String>,
    pub(crate) font_cs: Option<String>,
    pub(crate) baseline: Option<i32>,
    pub(crate) letter_spacing: Option<f64>,
    pub(crate) highlight: Option<Color>,
    pub(crate) shadow: Option<TextShadow>,
    pub(crate) hyperlink: Option<String>,
    pub(crate) is_break: bool,
}

impl RunBuilder {
    pub(crate) fn build(self) -> TextRun {
        TextRun {
            text: self.text,
            style: TextStyle {
                font_size: self.font_size,
                bold: self.bold,
                italic: self.italic,
                underline: self.underline,
                strikethrough: self.strikethrough,
                capitalization: self.capitalization,
                color: self.color,
                baseline: self.baseline,
                letter_spacing: self.letter_spacing,
                highlight: self.highlight,
                shadow: self.shadow,
                ..Default::default()
            },
            font: FontStyle {
                latin: self.font_latin,
                east_asian: self.font_ea,
                complex_script: self.font_cs,
            },
            hyperlink: self.hyperlink,
            is_break: self.is_break,
        }
    }
}

pub(crate) fn start_paragraph(paragraph: &mut Option<ParagraphBuilder>) {
    *paragraph = Some(ParagraphBuilder::default());
}

pub(crate) fn start_run(run: &mut Option<RunBuilder>) {
    *run = Some(RunBuilder::default());
}

pub(crate) fn append_text(run: &mut Option<RunBuilder>, text: &str) {
    if let Some(run) = run.as_mut() {
        run.text.push_str(text);
    }
}

pub(crate) fn finish_run(run: &mut Option<RunBuilder>, paragraph: &mut Option<ParagraphBuilder>) {
    if let Some(run) = run.take()
        && let Some(paragraph) = paragraph.as_mut()
    {
        paragraph.runs.push(run.build());
    }
}

pub(crate) fn finish_paragraph(
    paragraph: &mut Option<ParagraphBuilder>,
    shape: &mut Option<ShapeBuilder>,
) {
    if let Some(paragraph) = paragraph.take()
        && let Some(shape) = shape.as_mut()
    {
        shape.paragraphs.push(paragraph.build());
    }
}

pub(crate) fn parse_bullet_font(
    element: &BytesStart<'_>,
    paragraph: &mut Option<ParagraphBuilder>,
) {
    if let Some(paragraph) = paragraph.as_mut()
        && let Some(typeface) = xml_utils::attr_str(element, "typeface")
    {
        paragraph.bu_font = Some(typeface);
    }
}

pub(crate) fn parse_bullet_size(
    local: &str,
    element: &BytesStart<'_>,
    paragraph: &mut Option<ParagraphBuilder>,
) {
    let Some(paragraph) = paragraph.as_mut() else {
        return;
    };
    let Some(value) =
        xml_utils::attr_str(element, "val").and_then(|value| value.parse::<f64>().ok())
    else {
        return;
    };
    paragraph.bu_size_pct = match local {
        "buSzPct" => Some(value / 100_000.0),
        "buSzPts" => Some(-(value / 100.0)),
        _ => paragraph.bu_size_pct,
    };
}

pub(crate) fn parse_bullet(
    local: &str,
    element: &BytesStart<'_>,
    paragraph: &mut Option<ParagraphBuilder>,
) {
    let Some(paragraph) = paragraph.as_mut() else {
        return;
    };
    paragraph.bullet = match local {
        "buNone" => Some(Bullet::None),
        "buChar" => xml_utils::attr_str(element, "char").map(|char| {
            Bullet::Char(BulletChar {
                char,
                font: paragraph.bu_font.take(),
                size_pct: paragraph.bu_size_pct.take(),
                color: paragraph.bu_color.take(),
            })
        }),
        "buAutoNum" => Some(Bullet::AutoNum(BulletAutoNum {
            num_type: xml_utils::attr_str(element, "type")
                .unwrap_or_else(|| "arabicPeriod".to_owned()),
            start_at: xml_utils::attr_str(element, "startAt")
                .and_then(|value| value.parse::<i32>().ok()),
            font: paragraph.bu_font.take(),
            size_pct: paragraph.bu_size_pct.take(),
            color: paragraph.bu_color.take(),
        })),
        _ => paragraph.bullet.take(),
    };
}

pub(crate) fn apply_paragraph_default_run_properties(
    paragraph: &mut ParagraphBuilder,
    element: &BytesStart<'_>,
) {
    if let Some(size) = xml_utils::attr_str(element, "sz") {
        paragraph.def_rpr_font_size = size.parse::<f64>().ok().map(|value| value / 100.0);
    }
    if let Some(spacing) = xml_utils::attr_str(element, "spc") {
        paragraph.def_rpr_letter_spacing = spacing.parse::<f64>().ok().map(|value| value / 100.0);
    }
    if let Some(baseline) = xml_utils::attr_str(element, "baseline") {
        paragraph.def_rpr_baseline = baseline.parse::<i32>().ok();
    }
    if let Some(capitalization) = xml_utils::attr_str(element, "cap") {
        paragraph.def_rpr_capitalization = Some(TextCapitalization::from_ooxml(&capitalization));
    }
    if let Some(underline) = xml_utils::attr_str(element, "u") {
        paragraph.def_rpr_underline = Some(UnderlineType::from_ooxml(&underline));
    }
    if let Some(strikethrough) = xml_utils::attr_str(element, "strike") {
        paragraph.def_rpr_strikethrough = Some(StrikethroughType::from_ooxml(&strikethrough));
    }
    if let Some(bold) = xml_utils::attr_str(element, "b") {
        paragraph.def_rpr_bold = Some(bold == "1" || bold == "true");
    }
    if let Some(italic) = xml_utils::attr_str(element, "i") {
        paragraph.def_rpr_italic = Some(italic == "1" || italic == "true");
    }
}

pub(crate) fn parse_paragraph_properties(
    element: &BytesStart<'_>,
    paragraph: &mut Option<ParagraphBuilder>,
) {
    let Some(paragraph) = paragraph.as_mut() else {
        return;
    };
    if let Some(alignment) = xml_utils::attr_str(element, "algn") {
        paragraph.alignment = Alignment::from_ooxml(&alignment);
    }
    if let Some(rtl) = xml_utils::attr_str(element, "rtl") {
        paragraph.rtl = rtl == "1" || rtl == "true";
    }
    if let Some(level) = xml_utils::attr_str(element, "lvl") {
        paragraph.level = level.parse::<u32>().unwrap_or(0);
    }
    if let Some(indent) = xml_utils::attr_str(element, "indent") {
        paragraph.indent = Some(Emu::parse_emu(&indent).to_pt());
    }
    if let Some(margin_left) = xml_utils::attr_str(element, "marL") {
        paragraph.margin_left = Some(Emu::parse_emu(&margin_left).to_pt());
    }
}

pub(crate) fn parse_run_properties(element: &BytesStart<'_>, run: &mut Option<RunBuilder>) {
    let Some(run) = run.as_mut() else {
        return;
    };
    if let Some(size) = xml_utils::attr_str(element, "sz") {
        run.font_size = size.parse::<f64>().ok().map(|value| value / 100.0);
    }
    if let Some(bold) = xml_utils::attr_str(element, "b") {
        run.bold = bold == "1" || bold == "true";
    }
    if let Some(italic) = xml_utils::attr_str(element, "i") {
        run.italic = italic == "1" || italic == "true";
    }
    if let Some(underline) = xml_utils::attr_str(element, "u") {
        run.underline = UnderlineType::from_ooxml(&underline);
    }
    if let Some(strikethrough) = xml_utils::attr_str(element, "strike") {
        run.strikethrough = StrikethroughType::from_ooxml(&strikethrough);
    }
    if let Some(capitalization) = xml_utils::attr_str(element, "cap") {
        run.capitalization = TextCapitalization::from_ooxml(&capitalization);
    }
    if let Some(baseline) = xml_utils::attr_str(element, "baseline") {
        run.baseline = baseline.parse::<i32>().ok();
    }
    if let Some(spacing) = xml_utils::attr_str(element, "spc") {
        run.letter_spacing = spacing.parse::<f64>().ok().map(|value| value / 100.0);
    }
}

pub(crate) fn parse_body_properties(element: &BytesStart<'_>, shape: &mut Option<ShapeBuilder>) {
    let Some(shape) = shape.as_mut() else {
        return;
    };
    if let Some(anchor) = xml_utils::attr_str(element, "anchor") {
        shape.text_vertical_align = VerticalAlign::from_ooxml(&anchor);
        shape.text_vertical_align_explicit = true;
    }
    if let Some(anchor_center) = xml_utils::attr_str(element, "anchorCtr") {
        shape.text_anchor_center = anchor_center == "1" || anchor_center == "true";
    }
    if let Some(rotation) = xml_utils::attr_str(element, "rot") {
        shape.text_rotation_deg = rotation.parse::<f64>().unwrap_or(0.0) / 60_000.0;
    }
    for (name, target, explicit) in [
        (
            "lIns",
            &mut shape.text_margins.left,
            &mut shape.text_margin_left_explicit,
        ),
        (
            "tIns",
            &mut shape.text_margins.top,
            &mut shape.text_margin_top_explicit,
        ),
        (
            "rIns",
            &mut shape.text_margins.right,
            &mut shape.text_margin_right_explicit,
        ),
        (
            "bIns",
            &mut shape.text_margins.bottom,
            &mut shape.text_margin_bottom_explicit,
        ),
    ] {
        if let Some(value) = xml_utils::attr_str(element, name) {
            *target = Emu::parse_emu(&value).to_pt();
            *explicit = true;
        }
    }
    if let Some(wrap) = xml_utils::attr_str(element, "wrap") {
        shape.text_word_wrap = wrap != "none";
        shape.text_word_wrap_explicit = true;
    }
    if let Some(vertical) = xml_utils::attr_str(element, "vert") {
        shape.vertical_text_explicit = true;
        shape.vertical_text = (vertical != "horz").then_some(vertical);
    }
}

pub(crate) fn parse_auto_fit(local: &str, element: &BytesStart<'_>) -> AutoFit {
    match local {
        "normAutofit" => AutoFit::Normal {
            font_scale: parse_autofit_ratio(element, "fontScale"),
            line_spacing_reduction: parse_autofit_ratio(element, "lnSpcReduction"),
        },
        "noAutofit" => AutoFit::NoAutoFit,
        "spAutoFit" => AutoFit::Shrink,
        _ => AutoFit::None,
    }
}

pub(crate) fn parse_autofit_ratio(element: &BytesStart<'_>, attribute: &str) -> Option<f64> {
    xml_utils::attr_str(element, attribute)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| (value / 100_000.0).clamp(0.0, 1.0))
}

pub(crate) fn parse_spacing(local: &str, element: &BytesStart<'_>) -> Option<SpacingValue> {
    let value = xml_utils::attr_str(element, "val")?.parse::<f64>().ok()?;
    match local {
        "spcPct" => Some(SpacingValue::Percent(value / 100_000.0)),
        "spcPts" => Some(SpacingValue::Points(value / 100.0)),
        _ => None,
    }
}

pub(crate) fn assign_spacing_defaults(
    target: Option<&mut ParagraphDefaults>,
    spacing: SpacingValue,
    in_line_spacing: bool,
    in_space_before: bool,
    in_space_after: bool,
) {
    if let Some(target) = target {
        if in_line_spacing {
            target.line_spacing = Some(spacing);
        } else if in_space_before {
            target.space_before = Some(spacing);
        } else if in_space_after {
            target.space_after = Some(spacing);
        }
    }
}

pub(crate) fn assign_spacing_paragraph(
    target: Option<&mut ParagraphBuilder>,
    spacing: SpacingValue,
    in_line_spacing: bool,
    in_space_before: bool,
    in_space_after: bool,
) {
    if let Some(target) = target {
        if in_line_spacing {
            target.line_spacing = Some(spacing);
        } else if in_space_before {
            target.space_before = Some(spacing);
        } else if in_space_after {
            target.space_after = Some(spacing);
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn assign_typeface(
    local: &str,
    element: &BytesStart<'_>,
    cell_run: &mut Option<RunBuilder>,
    in_shape_defaults: bool,
    shape_defaults: &mut Option<RunDefaults>,
    in_paragraph_defaults: bool,
    paragraph: Option<&mut ParagraphBuilder>,
    shape_run: &mut Option<RunBuilder>,
) {
    let Some(typeface) = xml_utils::attr_str(element, "typeface") else {
        return;
    };
    if let Some(run) = cell_run.as_mut() {
        assign_typeface_to_run(run, local, typeface);
    } else if in_shape_defaults {
        if let Some(defaults) = shape_defaults.as_mut() {
            assign_typeface_to_defaults(defaults, local, typeface);
        }
    } else if in_paragraph_defaults {
        if let Some(paragraph) = paragraph {
            assign_typeface_to_paragraph(paragraph, local, typeface);
        }
    } else if let Some(run) = shape_run.as_mut() {
        assign_typeface_to_run(run, local, typeface);
    }
}

pub(crate) fn assign_typeface_to_run(run: &mut RunBuilder, local: &str, typeface: String) {
    match local {
        "latin" => run.font_latin = Some(typeface),
        "ea" => run.font_ea = Some(typeface),
        "cs" => run.font_cs = Some(typeface),
        _ => {}
    }
}

pub(crate) fn assign_typeface_to_defaults(
    defaults: &mut RunDefaults,
    local: &str,
    typeface: String,
) {
    match local {
        "latin" => defaults.font_latin = Some(typeface),
        "ea" => defaults.font_ea = Some(typeface),
        "cs" => defaults.font_cs = Some(typeface),
        _ => {}
    }
}

pub(crate) fn assign_typeface_to_paragraph(
    paragraph: &mut ParagraphBuilder,
    local: &str,
    typeface: String,
) {
    match local {
        "latin" => paragraph.def_rpr_font_latin = Some(typeface),
        "ea" => paragraph.def_rpr_font_ea = Some(typeface),
        "cs" => paragraph.def_rpr_font_cs = Some(typeface),
        _ => {}
    }
}
