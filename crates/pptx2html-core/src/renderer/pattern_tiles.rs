use std::fmt::Write;

use crate::model::PatternPreset;

pub(super) struct TileSpec {
    pub(super) width: u8,
    pub(super) height: u8,
    pub(super) motif: String,
}

fn percentage_tile(percent: u8, foreground: &str) -> TileSpec {
    let count = usize::from(percent).div_ceil(5).min(20);
    let mut motif = String::new();
    for index in 0..count {
        let x = 1 + (index % 5) * 2;
        let y = 1 + (index / 5) * 2;
        let _ = write!(
            motif,
            "<circle cx='{x}' cy='{y}' r='.7' fill='{foreground}'/>"
        );
    }
    tile(10, 8, motif)
}

fn tile(width: u8, height: u8, motif: String) -> TileSpec {
    TileSpec {
        width,
        height,
        motif,
    }
}

pub(super) fn tile_spec(preset: &PatternPreset, foreground: &str) -> Option<TileSpec> {
    let stroke = format!("stroke='{foreground}' stroke-width='1' fill='none'");
    let thick = format!("stroke='{foreground}' stroke-width='2' fill='none'");
    let result = match preset {
        PatternPreset::Pct5 => percentage_tile(5, foreground),
        PatternPreset::Pct10 => percentage_tile(10, foreground),
        PatternPreset::Pct20 => percentage_tile(20, foreground),
        PatternPreset::Pct25 => percentage_tile(25, foreground),
        PatternPreset::Pct30 => percentage_tile(30, foreground),
        PatternPreset::Pct40 => percentage_tile(40, foreground),
        PatternPreset::Pct50 => percentage_tile(50, foreground),
        PatternPreset::Pct60 => percentage_tile(60, foreground),
        PatternPreset::Pct70 => percentage_tile(70, foreground),
        PatternPreset::Pct75 => percentage_tile(75, foreground),
        PatternPreset::Pct80 => percentage_tile(80, foreground),
        PatternPreset::Pct90 => percentage_tile(90, foreground),
        PatternPreset::Horz => tile(8, 8, format!("<path d='M0 4H8' {stroke}/>")),
        PatternPreset::LtHorz => tile(
            8,
            10,
            format!("<path d='M0 5H8' stroke='{foreground}' stroke-width='.6'/>"),
        ),
        PatternPreset::NarHorz => tile(6, 6, format!("<path d='M0 3H6' {stroke}/>")),
        PatternPreset::DkHorz => tile(8, 8, format!("<path d='M0 3H8M0 5H8' {thick}/>")),
        PatternPreset::DashHorz => tile(10, 8, format!("<path d='M0 4H4' {stroke}/>")),
        PatternPreset::Vert => tile(8, 8, format!("<path d='M4 0V8' {stroke}/>")),
        PatternPreset::LtVert => tile(
            10,
            8,
            format!("<path d='M5 0V8' stroke='{foreground}' stroke-width='.6'/>"),
        ),
        PatternPreset::NarVert => tile(6, 6, format!("<path d='M3 0V6' {stroke}/>")),
        PatternPreset::DkVert => tile(8, 8, format!("<path d='M3 0V8M5 0V8' {thick}/>")),
        PatternPreset::DashVert => tile(8, 10, format!("<path d='M4 0V4' {stroke}/>")),
        PatternPreset::Cross => tile(8, 8, format!("<path d='M0 4H8M4 0V8' {stroke}/>")),
        PatternPreset::DnDiag => tile(8, 8, format!("<path d='M-2 0L8 10M6-2L10 2' {stroke}/>")),
        PatternPreset::LtDnDiag => tile(
            10,
            10,
            format!("<path d='M-2 0L10 12M8-2L12 2' stroke='{foreground}' stroke-width='.6'/>"),
        ),
        PatternPreset::WdDnDiag => {
            tile(12, 12, format!("<path d='M-3 0L12 15M9-3L15 3' {stroke}/>"))
        }
        PatternPreset::DkDnDiag => tile(8, 8, format!("<path d='M-2 0L8 10M6-2L10 2' {thick}/>")),
        PatternPreset::DashDnDiag => {
            tile(10, 10, format!("<path d='M0 0L4 4M6 6L10 10' {stroke}/>"))
        }
        PatternPreset::UpDiag => tile(8, 8, format!("<path d='M-2 8L8-2M6 10L10 6' {stroke}/>")),
        PatternPreset::LtUpDiag => tile(
            10,
            10,
            format!("<path d='M-2 10L10-2M8 12L12 8' stroke='{foreground}' stroke-width='.6'/>"),
        ),
        PatternPreset::WdUpDiag => tile(
            12,
            12,
            format!("<path d='M-3 12L12-3M9 15L15 9' {stroke}/>"),
        ),
        PatternPreset::DkUpDiag => tile(8, 8, format!("<path d='M-2 8L8-2M6 10L10 6' {thick}/>")),
        PatternPreset::DashUpDiag => {
            tile(10, 10, format!("<path d='M0 10L4 6M6 4L10 0' {stroke}/>"))
        }
        PatternPreset::DiagCross => tile(
            8,
            8,
            format!("<path d='M-2 0L8 10M0 10L10 0M6-2L10 2' {stroke}/>"),
        ),
        PatternPreset::SmCheck => tile(
            6,
            6,
            format!("<path d='M0 0H3V3H0ZM3 3H6V6H3Z' fill='{foreground}'/>"),
        ),
        PatternPreset::LgCheck => tile(
            12,
            12,
            format!("<path d='M0 0H6V6H0ZM6 6H12V12H6Z' fill='{foreground}'/>"),
        ),
        PatternPreset::SmGrid => tile(6, 6, format!("<path d='M0 0H6V6' {stroke}/>")),
        PatternPreset::LgGrid => tile(12, 12, format!("<path d='M0 0H12V12' {stroke}/>")),
        PatternPreset::DotGrid => tile(
            8,
            8,
            format!("<circle cx='1' cy='1' r='1' fill='{foreground}'/>"),
        ),
        PatternPreset::SmConfetti => tile(
            8,
            8,
            format!("<path d='M1 2l2 1M5 5l1 2M2 7l2-2' {stroke}/>"),
        ),
        PatternPreset::LgConfetti => tile(
            14,
            14,
            format!("<path d='M1 3l5 2M9 8l3 5M3 12l4-4' {thick}/>"),
        ),
        PatternPreset::HorzBrick => tile(
            12,
            8,
            format!("<path d='M0 0H12M0 4H12M6 0V4M3 4V8M9 4V8' {stroke}/>"),
        ),
        PatternPreset::DiagBrick => tile(
            12,
            12,
            format!("<path d='M0 12L12 0M-3 3L3-3M9 15L15 9M3 3l3 3M6 6l3 3' {stroke}/>"),
        ),
        PatternPreset::SolidDmnd => tile(
            10,
            10,
            format!("<path d='M5 0L10 5 5 10 0 5Z' fill='{foreground}'/>"),
        ),
        PatternPreset::OpenDmnd => {
            tile(10, 10, format!("<path d='M5 0L10 5 5 10 0 5Z' {stroke}/>"))
        }
        PatternPreset::DotDmnd => tile(
            10,
            10,
            format!(
                "<path d='M5 1L9 5 5 9 1 5Z' {stroke}/><circle cx='5' cy='5' r='1' fill='{foreground}'/>"
            ),
        ),
        PatternPreset::Plaid => tile(
            12,
            12,
            format!("<path d='M2 0V12M5 0V12M0 7H12M0 10H12' {stroke}/>"),
        ),
        PatternPreset::Sphere => tile(
            10,
            10,
            format!("<circle cx='5' cy='5' r='3' {stroke}/><path d='M3 4c1-2 3-2 4 0' {stroke}/>"),
        ),
        PatternPreset::Weave => tile(
            12,
            12,
            format!("<path d='M0 3H12M0 9H12M3 0V12M9 0V12' {thick}/>"),
        ),
        PatternPreset::Divot => tile(10, 10, format!("<path d='M1 5q2-3 4 0t4 0' {stroke}/>")),
        PatternPreset::Shingle => tile(
            12,
            8,
            format!("<path d='M0 4Q3 0 6 4T12 4M0 8Q3 4 6 8T12 8' {stroke}/>"),
        ),
        PatternPreset::Wave => tile(12, 8, format!("<path d='M0 4Q3 0 6 4T12 4' {stroke}/>")),
        PatternPreset::Trellis => tile(
            12,
            12,
            format!("<path d='M-3 0L12 15M0 15L15 0M9-3L15 3' {stroke}/>"),
        ),
        PatternPreset::ZigZag => tile(12, 8, format!("<path d='M0 6L3 2 6 6 9 2 12 6' {stroke}/>")),
        PatternPreset::Unknown(_) => return None,
    };
    Some(result)
}
