use quick_xml::Reader;
use quick_xml::events::Event;

use super::pattern_fill_parser::PatternBuilder;
use super::xml_utils;
use crate::error::PptxResult;
use crate::model::presentation::{ColorScheme, Theme};
use crate::model::{
    Border, BorderStyle, Color, CompoundLine, DashStyle, EffectStyle, Emu, Fill, FmtScheme,
    GlowEffect, LineAlignment, LineCap, LineJoin, OuterShadow, SolidFill,
};

pub fn parse_theme(xml: &str) -> PptxResult<Theme> {
    let mut reader = Reader::from_str(xml);
    let mut theme = Theme::default();
    let mut in_color_scheme = false;
    let mut current_color_role: Option<String> = None;
    let mut in_major_font = false;
    let mut in_minor_font = false;

    // FmtScheme state
    let mut in_fmt_scheme = false;
    let mut fmt_list_kind: Option<FmtKind> = None;
    let mut current_fill_color: Option<Color> = None;
    let mut current_pattern: Option<PatternBuilder> = None;
    let mut in_ln = false;
    let mut current_ln_width: f64 = 0.0;
    let mut current_ln_color: Option<Color> = None;
    let mut current_ln_cap = LineCap::Square;
    let mut current_ln_compound = CompoundLine::Single;
    let mut current_ln_alignment = LineAlignment::Center;
    let mut current_ln_join = LineJoin::Miter;
    let mut current_ln_miter_limit: Option<f64> = None;
    let mut current_ln_dash = DashStyle::Solid;

    // effectStyleLst state
    let mut in_effect_style = false;
    let mut in_effect_lst = false;
    let mut in_outer_shdw = false;
    let mut in_effect_glow = false;
    let mut effect_shdw_blur: f64 = 0.0;
    let mut effect_shdw_dist: f64 = 0.0;
    let mut effect_shdw_dir: f64 = 0.0;
    let mut effect_shdw_color: Option<Color> = None;
    let mut effect_shdw_alpha: f64 = 1.0;
    let mut effect_glow_rad: f64 = 0.0;
    let mut effect_glow_color: Option<Color> = None;
    let mut effect_glow_alpha: f64 = 1.0;
    let mut current_effect_style = EffectStyle::default();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                let local = xml_utils::local_name(name.as_ref());
                if local == "pattFill"
                    && matches!(fmt_list_kind, Some(FmtKind::Fill | FmtKind::BgFill))
                {
                    let mut pattern = PatternBuilder::default();
                    pattern.begin(e);
                    current_pattern = Some(pattern);
                    continue;
                }
                if matches!(local, "fgClr" | "bgClr")
                    && let Some(pattern) = current_pattern.as_mut()
                {
                    pattern.start_color_role(local);
                    continue;
                }
                if let Some(pattern) = current_pattern.as_mut()
                    && (pattern.parse_color(local, e) || pattern.append_modifier(local, e))
                {
                    continue;
                }
                match local {
                    "clrScheme" => {
                        in_color_scheme = true;
                        if let Some(name) = xml_utils::attr_str(e, "name") {
                            theme.name = name;
                        }
                    }
                    "dk1" | "lt1" | "dk2" | "lt2" | "accent1" | "accent2" | "accent3"
                    | "accent4" | "accent5" | "accent6" | "hlink" | "folHlink"
                        if in_color_scheme =>
                    {
                        current_color_role = Some(local.to_string());
                    }
                    "majorFont" => in_major_font = true,
                    "minorFont" => in_minor_font = true,
                    // FmtScheme
                    "fmtScheme" => in_fmt_scheme = true,
                    "fillStyleLst" if in_fmt_scheme => {
                        fmt_list_kind = Some(FmtKind::Fill);
                    }
                    "lnStyleLst" if in_fmt_scheme => {
                        fmt_list_kind = Some(FmtKind::Ln);
                    }
                    "effectStyleLst" if in_fmt_scheme => {
                        fmt_list_kind = Some(FmtKind::Effect);
                    }
                    "bgFillStyleLst" if in_fmt_scheme => {
                        fmt_list_kind = Some(FmtKind::BgFill);
                    }
                    // effectStyle entry inside effectStyleLst
                    "effectStyle" if matches!(fmt_list_kind, Some(FmtKind::Effect)) => {
                        in_effect_style = true;
                        current_effect_style = EffectStyle::default();
                    }
                    // effectLst inside effectStyle
                    "effectLst" if in_effect_style => {
                        in_effect_lst = true;
                    }
                    // outerShdw inside effectLst
                    "outerShdw" if in_effect_lst => {
                        in_outer_shdw = true;
                        effect_shdw_blur = xml_utils::attr_str(e, "blurRad")
                            .map(|v| Emu::parse_emu(&v).to_pt())
                            .unwrap_or(0.0);
                        effect_shdw_dist = xml_utils::attr_str(e, "dist")
                            .map(|v| Emu::parse_emu(&v).to_pt())
                            .unwrap_or(0.0);
                        effect_shdw_dir = xml_utils::attr_str(e, "dir")
                            .and_then(|v| v.parse::<f64>().ok())
                            .map(|v| v / 60000.0)
                            .unwrap_or(0.0);
                        effect_shdw_color = None;
                        effect_shdw_alpha = 1.0;
                    }
                    // glow inside effectLst
                    "glow" if in_effect_lst => {
                        in_effect_glow = true;
                        effect_glow_rad = xml_utils::attr_str(e, "rad")
                            .map(|v| Emu::parse_emu(&v).to_pt())
                            .unwrap_or(0.0);
                        effect_glow_color = None;
                        effect_glow_alpha = 1.0;
                    }
                    // Color elements inside outerShdw or glow in effectLst
                    "srgbClr" if in_outer_shdw || in_effect_glow => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            assign_effect_color(
                                &mut effect_shdw_color,
                                &mut effect_glow_color,
                                in_outer_shdw,
                                false,
                                val,
                            );
                        }
                    }
                    "schemeClr" if in_outer_shdw || in_effect_glow => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            assign_effect_color(
                                &mut effect_shdw_color,
                                &mut effect_glow_color,
                                in_outer_shdw,
                                true,
                                val,
                            );
                        }
                    }
                    // Line element inside lnStyleLst
                    "ln" if matches!(fmt_list_kind, Some(FmtKind::Ln)) => {
                        in_ln = true;
                        current_ln_width = xml_utils::attr_str(e, "w")
                            .map(|w| Emu::parse_emu(&w).to_pt())
                            .unwrap_or(0.0);
                        current_ln_color = None;
                        current_ln_cap = match xml_utils::attr_str(e, "cap").as_deref() {
                            Some("rnd") => LineCap::Round,
                            Some("flat") => LineCap::Flat,
                            _ => LineCap::Square,
                        };
                        current_ln_compound = match xml_utils::attr_str(e, "cmpd").as_deref() {
                            Some("dbl") => CompoundLine::Double,
                            Some("thickThin") => CompoundLine::ThickThin,
                            Some("thinThick") => CompoundLine::ThinThick,
                            Some("tri") => CompoundLine::Triple,
                            _ => CompoundLine::Single,
                        };
                        current_ln_alignment = match xml_utils::attr_str(e, "algn").as_deref() {
                            Some("in") => LineAlignment::Inset,
                            _ => LineAlignment::Center,
                        };
                        current_ln_join = LineJoin::Miter;
                        current_ln_miter_limit = None;
                        current_ln_dash = DashStyle::Solid;
                    }
                    // solidFill inside fill/bg lists
                    "solidFill" if fmt_list_kind.is_some() => {
                        current_fill_color = None;
                    }
                    // Color elements (Start variant, may have child modifiers)
                    "srgbClr" if fmt_list_kind.is_some() && !in_outer_shdw && !in_effect_glow => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            assign_fmt_color(
                                &mut current_ln_color,
                                &mut current_fill_color,
                                in_ln,
                                false,
                                val,
                            );
                        }
                    }
                    "schemeClr" if fmt_list_kind.is_some() && !in_outer_shdw && !in_effect_glow => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            assign_fmt_color(
                                &mut current_ln_color,
                                &mut current_fill_color,
                                in_ln,
                                true,
                                val,
                            );
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name = e.name();
                let local = xml_utils::local_name(name.as_ref());
                if local == "pattFill"
                    && matches!(fmt_list_kind, Some(FmtKind::Fill | FmtKind::BgFill))
                {
                    let mut pattern = PatternBuilder::default();
                    pattern.begin(e);
                    push_fill(
                        &fmt_list_kind,
                        &mut theme.fmt_scheme,
                        Fill::Pattern(pattern.finish()),
                    );
                    continue;
                }
                if let Some(pattern) = current_pattern.as_mut()
                    && (pattern.parse_color(local, e) || pattern.append_modifier(local, e))
                {
                    continue;
                }
                match local {
                    "srgbClr" if current_color_role.is_some() => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            set_color_scheme(
                                &mut theme.color_scheme,
                                current_color_role.as_deref().unwrap_or(""),
                                &val,
                            );
                        }
                        current_color_role = None;
                    }
                    "sysClr" if current_color_role.is_some() => {
                        if let Some(val) = xml_utils::attr_str(e, "lastClr") {
                            set_color_scheme(
                                &mut theme.color_scheme,
                                current_color_role.as_deref().unwrap_or(""),
                                &val,
                            );
                        }
                        current_color_role = None;
                    }
                    "latin" if in_major_font => {
                        if let Some(typeface) = xml_utils::attr_str(e, "typeface") {
                            theme.font_scheme.major_latin = typeface;
                        }
                    }
                    "latin" if in_minor_font => {
                        if let Some(typeface) = xml_utils::attr_str(e, "typeface") {
                            theme.font_scheme.minor_latin = typeface;
                        }
                    }
                    "ea" if in_major_font => {
                        set_optional_string(
                            &mut theme.font_scheme.major_east_asian,
                            xml_utils::attr_str(e, "typeface"),
                        );
                    }
                    "ea" if in_minor_font => {
                        set_optional_string(
                            &mut theme.font_scheme.minor_east_asian,
                            xml_utils::attr_str(e, "typeface"),
                        );
                    }
                    "cs" if in_major_font => {
                        set_optional_string(
                            &mut theme.font_scheme.major_complex_script,
                            xml_utils::attr_str(e, "typeface"),
                        );
                    }
                    "cs" if in_minor_font => {
                        set_optional_string(
                            &mut theme.font_scheme.minor_complex_script,
                            xml_utils::attr_str(e, "typeface"),
                        );
                    }
                    // Empty color elements inside effect outerShdw/glow
                    "srgbClr" if in_outer_shdw || in_effect_glow => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            assign_effect_color(
                                &mut effect_shdw_color,
                                &mut effect_glow_color,
                                in_outer_shdw,
                                false,
                                val,
                            );
                        }
                    }
                    "schemeClr" if in_outer_shdw || in_effect_glow => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            assign_effect_color(
                                &mut effect_shdw_color,
                                &mut effect_glow_color,
                                in_outer_shdw,
                                true,
                                val,
                            );
                        }
                    }
                    // Alpha modifier inside effect outerShdw/glow
                    "alpha" if in_outer_shdw => {
                        if let Some(val) = xml_utils::attr_str(e, "val")
                            && let Ok(v) = val.parse::<f64>()
                        {
                            effect_shdw_alpha = v / 100_000.0;
                        }
                    }
                    "alpha" if in_effect_glow => {
                        if let Some(val) = xml_utils::attr_str(e, "val")
                            && let Ok(v) = val.parse::<f64>()
                        {
                            effect_glow_alpha = v / 100_000.0;
                        }
                    }
                    // Empty effectLst (no effects) inside effectStyle
                    "effectLst" if in_effect_style => {
                        // No effects in this style entry -- default EffectStyle is fine
                    }
                    // Empty color elements inside fmtScheme lists
                    "srgbClr" if fmt_list_kind.is_some() => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            assign_fmt_color(
                                &mut current_ln_color,
                                &mut current_fill_color,
                                in_ln,
                                false,
                                val,
                            );
                        }
                    }
                    "schemeClr" if fmt_list_kind.is_some() => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            assign_fmt_color(
                                &mut current_ln_color,
                                &mut current_fill_color,
                                in_ln,
                                true,
                                val,
                            );
                        }
                    }
                    // noFill in fill/bg lists
                    "noFill" if fmt_list_kind.is_some() && !in_ln => {
                        push_fill(&fmt_list_kind, &mut theme.fmt_scheme, Fill::None);
                    }
                    // Dash style inside lnStyleLst line
                    "prstDash" if in_ln => {
                        if let Some(val) = xml_utils::attr_str(e, "val") {
                            current_ln_dash = match val.as_str() {
                                "solid" => DashStyle::Solid,
                                "dash" => DashStyle::Dash,
                                "dot" => DashStyle::Dot,
                                "dashDot" => DashStyle::DashDot,
                                "lgDash" => DashStyle::LongDash,
                                "lgDashDot" => DashStyle::LongDashDot,
                                "lgDashDotDot" => DashStyle::LongDashDotDot,
                                "sysDash" => DashStyle::SystemDash,
                                "sysDot" => DashStyle::SystemDot,
                                "sysDashDot" => DashStyle::SystemDashDot,
                                "sysDashDotDot" => DashStyle::SystemDashDotDot,
                                _ => DashStyle::Solid,
                            };
                        }
                    }
                    // Line join styles inside lnStyleLst line
                    "round" if in_ln => {
                        current_ln_join = LineJoin::Round;
                    }
                    "bevel" if in_ln => {
                        current_ln_join = LineJoin::Bevel;
                    }
                    "miter" if in_ln => {
                        current_ln_join = LineJoin::Miter;
                        current_ln_miter_limit = xml_utils::attr_str(e, "lim")
                            .and_then(|v| v.parse::<f64>().ok())
                            .map(|v| v / 100_000.0);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                let local = xml_utils::local_name(name.as_ref());
                match local {
                    "fgClr" | "bgClr" if current_pattern.is_some() => {
                        if let Some(pattern) = current_pattern.as_mut() {
                            pattern.finish_color_role();
                        }
                    }
                    "pattFill" if current_pattern.is_some() => {
                        if let Some(mut pattern) = current_pattern.take() {
                            push_fill(
                                &fmt_list_kind,
                                &mut theme.fmt_scheme,
                                Fill::Pattern(pattern.finish()),
                            );
                        }
                    }
                    "clrScheme" => in_color_scheme = false,
                    "majorFont" => in_major_font = false,
                    "minorFont" => in_minor_font = false,
                    "dk1" | "lt1" | "dk2" | "lt2" | "accent1" | "accent2" | "accent3"
                    | "accent4" | "accent5" | "accent6" | "hlink" | "folHlink" => {
                        current_color_role = None;
                    }
                    // End of FmtScheme sections
                    "fmtScheme" => {
                        in_fmt_scheme = false;
                        fmt_list_kind = None;
                    }
                    "fillStyleLst" | "lnStyleLst" | "effectStyleLst" | "bgFillStyleLst" => {
                        fmt_list_kind = None;
                    }
                    // End of outerShdw inside effectLst
                    "outerShdw" if in_outer_shdw => {
                        in_outer_shdw = false;
                        if let Some(color) = effect_shdw_color.take() {
                            current_effect_style.outer_shadow = Some(OuterShadow {
                                blur_radius: effect_shdw_blur,
                                distance: effect_shdw_dist,
                                direction: effect_shdw_dir,
                                color,
                                alpha: effect_shdw_alpha,
                            });
                        }
                    }
                    // End of glow inside effectLst
                    "glow" if in_effect_glow => {
                        in_effect_glow = false;
                        if let Some(color) = effect_glow_color.take() {
                            current_effect_style.glow = Some(GlowEffect {
                                radius: effect_glow_rad,
                                color,
                                alpha: effect_glow_alpha,
                            });
                        }
                    }
                    // End of effectLst
                    "effectLst" if in_effect_lst => {
                        in_effect_lst = false;
                    }
                    // End of effectStyle -- push accumulated style
                    "effectStyle" if in_effect_style => {
                        in_effect_style = false;
                        theme
                            .fmt_scheme
                            .effect_style_lst
                            .push(std::mem::take(&mut current_effect_style));
                    }
                    // End of solidFill in fill/bg lists
                    "solidFill" if fmt_list_kind.is_some() && !in_ln => {
                        if let Some(color) = current_fill_color.take() {
                            push_fill(
                                &fmt_list_kind,
                                &mut theme.fmt_scheme,
                                Fill::Solid(SolidFill { color }),
                            );
                        }
                    }
                    // End of solidFill inside line
                    "solidFill" if in_ln => {
                        // color captured in current_ln_color
                    }
                    // End of line element
                    "ln" if in_ln => {
                        in_ln = false;
                        let border = Border {
                            width: current_ln_width,
                            color: current_ln_color.take().unwrap_or_default(),
                            style: if current_ln_width > 0.0 {
                                BorderStyle::Solid
                            } else {
                                BorderStyle::None
                            },
                            dash_style: std::mem::take(&mut current_ln_dash),
                            cap: std::mem::take(&mut current_ln_cap),
                            compound: std::mem::take(&mut current_ln_compound),
                            alignment: std::mem::take(&mut current_ln_alignment),
                            join: std::mem::take(&mut current_ln_join),
                            miter_limit: current_ln_miter_limit.take(),
                            ..Default::default()
                        };
                        theme.fmt_scheme.ln_style_lst.push(border);
                    }
                    // End of color elements (with modifiers)
                    "srgbClr" | "schemeClr" if fmt_list_kind.is_some() => {
                        // Colors already captured
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(crate::error::PptxError::Xml(e)),
            _ => {}
        }
    }

    Ok(theme)
}

enum FmtKind {
    Fill,
    Ln,
    Effect,
    BgFill,
}

fn push_fill(kind: &Option<FmtKind>, fmt: &mut FmtScheme, fill: Fill) {
    match kind {
        Some(FmtKind::Fill) => fmt.fill_style_lst.push(fill),
        Some(FmtKind::BgFill) => fmt.bg_fill_style_lst.push(fill),
        _ => {}
    }
}

fn set_color_scheme(scheme: &mut ColorScheme, role: &str, hex: &str) {
    let hex = hex.to_string();
    match role {
        "dk1" => scheme.dk1 = hex,
        "lt1" => scheme.lt1 = hex,
        "dk2" => scheme.dk2 = hex,
        "lt2" => scheme.lt2 = hex,
        "accent1" => scheme.accent1 = hex,
        "accent2" => scheme.accent2 = hex,
        "accent3" => scheme.accent3 = hex,
        "accent4" => scheme.accent4 = hex,
        "accent5" => scheme.accent5 = hex,
        "accent6" => scheme.accent6 = hex,
        "hlink" => scheme.hlink = hex,
        "folHlink" => scheme.fol_hlink = hex,
        _ => {}
    }
}

fn assign_effect_color(
    shadow_color: &mut Option<Color>,
    glow_color: &mut Option<Color>,
    in_outer_shdw: bool,
    themed: bool,
    value: String,
) {
    let color = if themed {
        Color::theme(value)
    } else {
        Color::rgb(value)
    };
    if in_outer_shdw {
        *shadow_color = Some(color);
    } else {
        *glow_color = Some(color);
    }
}

fn assign_fmt_color(
    line_color: &mut Option<Color>,
    fill_color: &mut Option<Color>,
    in_ln: bool,
    themed: bool,
    value: String,
) {
    let color = if themed {
        Color::theme(value)
    } else {
        Color::rgb(value)
    };
    if in_ln {
        *line_color = Some(color);
    } else {
        *fill_color = Some(color);
    }
}

fn set_optional_string(target: &mut Option<String>, value: Option<String>) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        *target = Some(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_theme_covers_color_font_fill_line_and_effect_schemes() {
        let theme = parse_theme(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="CoverageTheme">
  <a:themeElements>
    <a:clrScheme name="CoverageColors">
      <a:dk1><a:sysClr val="windowText" lastClr="111111"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="222222"/></a:dk2>
      <a:lt2><a:srgbClr val="F7F7F7"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="CoverageFonts">
      <a:majorFont>
        <a:latin typeface="Aptos"/>
        <a:ea typeface="Yu Gothic"/>
        <a:cs typeface="Noto Sans Devanagari"/>
      </a:majorFont>
      <a:minorFont>
        <a:latin typeface="Aptos Narrow"/>
        <a:ea typeface="Meiryo"/>
        <a:cs typeface="Noto Sans Arabic"/>
      </a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="CoverageFmt">
      <a:fillStyleLst>
        <a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
        <a:solidFill><a:schemeClr val="accent2"/></a:solidFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="12700" cap="rnd" cmpd="dbl" algn="in">
          <a:solidFill><a:schemeClr val="accent3"/></a:solidFill>
          <a:prstDash val="lgDashDot"/>
          <a:miter lim="200000"/>
        </a:ln>
        <a:ln w="0" cap="flat">
          <a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>
          <a:round/>
        </a:ln>
        <a:ln w="25400">
          <a:solidFill><a:srgbClr val="0000FF"/></a:solidFill>
          <a:bevel/>
        </a:ln>
      </a:lnStyleLst>
      <a:effectStyleLst>
        <a:effectStyle>
          <a:effectLst>
            <a:outerShdw blurRad="12700" dist="25400" dir="5400000">
              <a:schemeClr val="accent4"/>
              <a:alpha val="50000"/>
            </a:outerShdw>
            <a:glow rad="6350">
              <a:srgbClr val="ABCDEF"/>
              <a:alpha val="60000"/>
            </a:glow>
          </a:effectLst>
        </a:effectStyle>
        <a:effectStyle><a:effectLst/></a:effectStyle>
      </a:effectStyleLst>
      <a:bgFillStyleLst>
        <a:noFill/>
        <a:solidFill><a:schemeClr val="accent5"/></a:solidFill>
      </a:bgFillStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#,
        )
        .expect("theme should parse");

        assert_eq!(theme.name, "CoverageColors");
        assert_eq!(theme.color_scheme.dk1, "111111");
        assert_eq!(theme.color_scheme.lt1, "FFFFFF");
        assert_eq!(theme.color_scheme.hlink, "0563C1");
        assert_eq!(theme.color_scheme.fol_hlink, "954F72");

        assert_eq!(theme.font_scheme.major_latin, "Aptos");
        assert_eq!(
            theme.font_scheme.major_east_asian.as_deref(),
            Some("Yu Gothic")
        );
        assert_eq!(
            theme.font_scheme.major_complex_script.as_deref(),
            Some("Noto Sans Devanagari")
        );
        assert_eq!(theme.font_scheme.minor_latin, "Aptos Narrow");
        assert_eq!(
            theme.font_scheme.minor_east_asian.as_deref(),
            Some("Meiryo")
        );
        assert_eq!(
            theme.font_scheme.minor_complex_script.as_deref(),
            Some("Noto Sans Arabic")
        );

        assert_eq!(theme.fmt_scheme.fill_style_lst.len(), 2);
        assert_eq!(
            std::mem::discriminant(&theme.fmt_scheme.bg_fill_style_lst[0]),
            std::mem::discriminant(&Fill::None)
        );
        assert!(matches!(
            &theme.fmt_scheme.bg_fill_style_lst[1],
            Fill::Solid(fill) if fill.color.to_css().as_deref() == Some("#5B9BD5")
        ));

        assert_eq!(theme.fmt_scheme.ln_style_lst.len(), 3);
        let first_ln = &theme.fmt_scheme.ln_style_lst[0];
        assert_eq!(first_ln.width, 1.0);
        assert_eq!(
            std::mem::discriminant(&first_ln.cap),
            std::mem::discriminant(&LineCap::Round)
        );
        assert_eq!(
            std::mem::discriminant(&first_ln.compound),
            std::mem::discriminant(&CompoundLine::Double)
        );
        assert_eq!(
            std::mem::discriminant(&first_ln.alignment),
            std::mem::discriminant(&LineAlignment::Inset)
        );
        assert_eq!(
            std::mem::discriminant(&first_ln.join),
            std::mem::discriminant(&LineJoin::Miter)
        );
        assert_eq!(first_ln.miter_limit, Some(2.0));
        assert_eq!(
            std::mem::discriminant(&first_ln.dash_style),
            std::mem::discriminant(&DashStyle::LongDashDot)
        );
        assert_eq!(first_ln.color.to_css().as_deref(), Some("#A5A5A5"));

        assert_eq!(
            std::mem::discriminant(&theme.fmt_scheme.ln_style_lst[1].join),
            std::mem::discriminant(&LineJoin::Round)
        );
        assert_eq!(
            std::mem::discriminant(&theme.fmt_scheme.ln_style_lst[2].join),
            std::mem::discriminant(&LineJoin::Bevel)
        );

        assert_eq!(theme.fmt_scheme.effect_style_lst.len(), 2);
        let effect = &theme.fmt_scheme.effect_style_lst[0];
        let shadow = effect.outer_shadow.as_ref().expect("outer shadow");
        assert!((shadow.blur_radius - 1.0).abs() < 1e-6);
        assert!((shadow.distance - 2.0).abs() < 1e-6);
        assert!((shadow.direction - 90.0).abs() < 1e-6);
        assert_eq!(shadow.color.to_css().as_deref(), Some("#FFC000"));
        assert!((shadow.alpha - 0.5).abs() < 1e-6);
        let glow = effect.glow.as_ref().expect("glow");
        assert!((glow.radius - 0.5).abs() < 1e-6);
        assert_eq!(glow.color.to_css().as_deref(), Some("#ABCDEF"));
        assert!((glow.alpha - 0.6).abs() < 1e-6);
        assert!(theme.fmt_scheme.effect_style_lst[1].outer_shadow.is_none());
        assert!(theme.fmt_scheme.effect_style_lst[1].glow.is_none());
    }

    #[test]
    fn parse_theme_covers_start_color_variants_and_remaining_dash_values() {
        let theme = parse_theme(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:themeElements>
    <a:clrScheme name="StartVariants">
      <a:accent1><a:srgbClr val="112233"/></a:accent1>
      <a:accent6><a:srgbClr val="667788"/></a:accent6>
    </a:clrScheme>
    <a:fontScheme name="StartFonts">
      <a:majorFont>
        <a:latin typeface="Major Latin"/>
      </a:majorFont>
      <a:minorFont>
        <a:latin typeface="Minor Latin"/>
      </a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="StartFmt">
      <a:fillStyleLst>
        <a:solidFill><a:srgbClr val="010203"><a:alpha val="50000"/></a:srgbClr></a:solidFill>
        <a:solidFill><a:schemeClr val="accent1"><a:alpha val="50000"/></a:schemeClr></a:solidFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="12700"><a:solidFill><a:srgbClr val="AA0000"><a:alpha val="50000"/></a:srgbClr></a:solidFill><a:prstDash val="solid"/></a:ln>
        <a:ln w="12700"><a:solidFill><a:schemeClr val="accent1"><a:alpha val="50000"/></a:schemeClr></a:solidFill><a:prstDash val="dash"/></a:ln>
        <a:ln w="12700"><a:solidFill><a:srgbClr val="BB0000"/></a:solidFill><a:prstDash val="dot"/></a:ln>
        <a:ln w="12700"><a:solidFill><a:srgbClr val="CC0000"/></a:solidFill><a:prstDash val="dashDot"/></a:ln>
        <a:ln w="12700"><a:solidFill><a:srgbClr val="DD0000"/></a:solidFill><a:prstDash val="lgDash"/></a:ln>
        <a:ln w="12700"><a:solidFill><a:srgbClr val="EE0000"/></a:solidFill><a:prstDash val="lgDashDotDot"/></a:ln>
        <a:ln w="12700"><a:solidFill><a:srgbClr val="0A0A0A"/></a:solidFill><a:prstDash val="sysDash"/></a:ln>
        <a:ln w="12700"><a:solidFill><a:srgbClr val="0B0B0B"/></a:solidFill><a:prstDash val="sysDot"/></a:ln>
        <a:ln w="12700"><a:solidFill><a:srgbClr val="0C0C0C"/></a:solidFill><a:prstDash val="sysDashDot"/></a:ln>
        <a:ln w="12700"><a:solidFill><a:srgbClr val="0D0D0D"/></a:solidFill><a:prstDash val="sysDashDotDot"/></a:ln>
        <a:ln w="12700"><a:solidFill><a:srgbClr val="0E0E0E"/></a:solidFill><a:prstDash val="mysteryDash"/></a:ln>
      </a:lnStyleLst>
      <a:effectStyleLst>
        <a:effectStyle>
          <a:effectLst>
            <a:outerShdw blurRad="12700" dist="25400" dir="5400000">
              <a:srgbClr val="ABCDEF"><a:alpha val="25000"/></a:srgbClr>
            </a:outerShdw>
            <a:glow rad="6350">
              <a:schemeClr val="accent6"><a:alpha val="75000"/></a:schemeClr>
            </a:glow>
          </a:effectLst>
        </a:effectStyle>
      </a:effectStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#,
        )
        .expect("theme should parse");

        assert_eq!(theme.fmt_scheme.fill_style_lst.len(), 2);
        assert_eq!(
            theme.fmt_scheme.fill_style_lst[0]
                .color_ref()
                .to_css()
                .as_deref(),
            Some("#010203")
        );
        assert_eq!(
            theme.fmt_scheme.fill_style_lst[1].color_ref().kind,
            crate::model::color::ColorKind::Theme("accent1".to_string())
        );

        let expected_dashes = [
            DashStyle::Solid,
            DashStyle::Dash,
            DashStyle::Dot,
            DashStyle::DashDot,
            DashStyle::LongDash,
            DashStyle::LongDashDotDot,
            DashStyle::SystemDash,
            DashStyle::SystemDot,
            DashStyle::SystemDashDot,
            DashStyle::SystemDashDotDot,
            DashStyle::Solid,
        ];
        assert_eq!(theme.fmt_scheme.ln_style_lst.len(), expected_dashes.len());
        for (border, expected) in theme
            .fmt_scheme
            .ln_style_lst
            .iter()
            .zip(expected_dashes.iter())
        {
            assert_eq!(
                std::mem::discriminant(&border.dash_style),
                std::mem::discriminant(expected)
            );
        }
        assert_eq!(
            theme.fmt_scheme.ln_style_lst[0].color.to_css().as_deref(),
            Some("#AA0000")
        );
        assert_eq!(
            theme.fmt_scheme.ln_style_lst[1].color.kind,
            crate::model::color::ColorKind::Theme("accent1".to_string())
        );

        let effect = &theme.fmt_scheme.effect_style_lst[0];
        let shadow = effect.outer_shadow.as_ref().expect("outer shadow");
        assert_eq!(shadow.color.to_css().as_deref(), Some("#ABCDEF"));
        assert!((shadow.alpha - 0.25).abs() < 1e-6);
        let glow = effect.glow.as_ref().expect("glow");
        assert_eq!(
            glow.color.kind,
            crate::model::color::ColorKind::Theme("accent6".to_string())
        );
        assert!((glow.alpha - 0.75).abs() < 1e-6);
    }

    #[test]
    fn push_fill_and_set_color_scheme_ignore_non_matching_roles() {
        let mut fmt = FmtScheme::default();
        push_fill(
            &Some(FmtKind::Fill),
            &mut fmt,
            Fill::Solid(SolidFill {
                color: Color::rgb("010203"),
            }),
        );
        push_fill(
            &Some(FmtKind::BgFill),
            &mut fmt,
            Fill::Solid(SolidFill {
                color: Color::rgb("040506"),
            }),
        );
        push_fill(&Some(FmtKind::Ln), &mut fmt, Fill::None);
        assert_eq!(fmt.fill_style_lst.len(), 1);
        assert_eq!(fmt.bg_fill_style_lst.len(), 1);

        let mut scheme = ColorScheme::default();
        set_color_scheme(&mut scheme, "accent6", "AABBCC");
        set_color_scheme(&mut scheme, "unknown", "DEADBE");
        assert_eq!(scheme.accent6, "AABBCC");
        assert_ne!(scheme.dk1, "DEADBE");
    }
}
