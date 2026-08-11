use quick_xml::events::BytesStart;

use super::xml_utils;
use crate::model::*;

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
