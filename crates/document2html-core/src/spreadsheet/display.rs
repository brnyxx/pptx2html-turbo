//! Turns a stored cell value plus its number format into visible text.
//!
//! Formatting is deliberately narrow: percentages and ISO dates are the only
//! transformations reproduced. An unreproducible format never fails the
//! conversion; it yields [`Display::Unattributable`] so the cell is excluded
//! from coordinate attribution while the document still converts.

use super::styles::NumberFormat;

/// Outcome of rendering a stored value for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Display {
    /// Text that matches what a spreadsheet application would show.
    Trusted(String),
    /// The visible text cannot be reproduced, so the cell must not be used to
    /// claim a coordinate. Conversion continues regardless.
    Unattributable,
}

/// Days from the 1900 serial epoch to 1970-01-01, accounting for the
/// spreadsheet's non-existent 1900-02-29 leap day.
const SERIAL_EPOCH_OFFSET: i64 = 25_569;
const SECONDS_PER_DAY: i64 = 86_400;

/// Renders `raw` for display under `format`.
pub(super) fn formatted_value(raw: &str, format: NumberFormat) -> Display {
    match format {
        NumberFormat::General => Display::Trusted(raw.to_owned()),
        NumberFormat::Percent { decimals } => percent(raw, decimals),
        NumberFormat::IsoDate => serial_to_iso(raw, false),
        NumberFormat::IsoDateTime => serial_to_iso(raw, true),
        NumberFormat::Unsupported => Display::Unattributable,
    }
}

fn percent(raw: &str, decimals: usize) -> Display {
    let Ok(value) = raw.parse::<f64>() else {
        return Display::Unattributable;
    };
    Display::Trusted(format!("{:.*}%", decimals, value * 100.0))
}

/// Converts a 1900-system serial number into an ISO 8601 date or date-time.
fn serial_to_iso(raw: &str, with_time: bool) -> Display {
    let Ok(serial) = raw.parse::<f64>() else {
        return Display::Unattributable;
    };
    if !serial.is_finite() || serial < 1.0 {
        return Display::Unattributable;
    }
    let days = serial.trunc() as i64;
    // Round the fractional day to whole seconds so 0.5 lands exactly on noon.
    let seconds =
        ((serial.fract() * SECONDS_PER_DAY as f64).round() as i64).clamp(0, SECONDS_PER_DAY);
    let unix_days = days - SERIAL_EPOCH_OFFSET;
    let (year, month, day) = civil_from_days(unix_days);
    if !with_time {
        return Display::Trusted(format!("{year:04}-{month:02}-{day:02}"));
    }
    let (hour, minute, second) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    Display::Trusted(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}"
    ))
}

/// Days since 1970-01-01 to a proleptic Gregorian calendar date.
/// Uses Howard Hinnant's civil-from-days algorithm.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (year + i64::from(month <= 2), month, day)
}

/// Normalizes an ISO date already stored as text by a `t="d"` cell.
///
/// ECMA-376 stores these as ISO 8601, so the value is validated and a
/// midnight time component is dropped for a stable display form. A malformed
/// value is unattributable rather than fatal.
pub(super) fn iso_date_text(raw: &str) -> Display {
    let trimmed = raw.trim();
    let (date, time) = match trimmed.split_once('T') {
        Some((date, time)) => (date, Some(time)),
        None => (trimmed, None),
    };
    if !valid_iso_date(date) {
        return Display::Unattributable;
    }
    match time {
        None => Display::Trusted(date.to_owned()),
        Some(value) if is_midnight(value) => Display::Trusted(date.to_owned()),
        Some(value) if valid_iso_time(value) => Display::Trusted(format!("{date}T{value}")),
        Some(_) => Display::Unattributable,
    }
}

fn valid_iso_date(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return false;
    }
    let Ok(year) = parts[0].parse::<i64>() else {
        return false;
    };
    let Ok(month) = parts[1].parse::<u32>() else {
        return false;
    };
    let Ok(day) = parts[2].parse::<u32>() else {
        return false;
    };
    (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

fn valid_iso_time(value: &str) -> bool {
    let core = value.trim_end_matches('Z');
    let parts: Vec<&str> = core.split(':').collect();
    if parts.len() != 3 {
        return false;
    }
    let Ok(hour) = parts[0].parse::<u32>() else {
        return false;
    };
    let Ok(minute) = parts[1].parse::<u32>() else {
        return false;
    };
    let Ok(second) = parts[2].split('.').next().unwrap_or("").parse::<u32>() else {
        return false;
    };
    hour < 24 && minute < 60 && second < 60
}

fn is_midnight(value: &str) -> bool {
    matches!(
        value.trim_end_matches('Z'),
        "00:00" | "00:00:00" | "00:00:00.000"
    )
}
