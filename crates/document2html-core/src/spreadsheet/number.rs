//! Classification of ECMA-376 number format codes.
//!
//! Only a bounded subset is reproduced: decimal precision, thousands
//! grouping, a leading or trailing currency literal, percentages and ISO
//! dates. A code that changes the visible value in any other way is reported
//! as unsupported so the caller fails closed on attribution.

/// How a cell's stored value must be turned into visible text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NumberFormat {
    /// Render the stored value as-is.
    General,
    /// Fixed-point decimal, optionally grouped and currency-prefixed.
    Decimal(DecimalFormat),
    /// Multiply by 100 and append `%`, with the given decimal precision.
    Percent { decimals: usize },
    /// Serial date rendered as an ISO calendar date.
    IsoDate,
    /// Serial date rendered as an ISO date and time.
    IsoDateTime,
    /// The format alters the visible value but is not emulated here.
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DecimalFormat {
    pub(super) decimals: usize,
    pub(super) grouped: bool,
    pub(super) prefix: String,
    pub(super) suffix: String,
    /// Negative values render in parentheses instead of with a minus sign.
    pub(super) parenthesized_negative: bool,
}

/// Built-in format ids defined by ECMA-376 that this module reproduces.
pub(super) fn builtin_format(id: &str) -> NumberFormat {
    match id {
        // 0 General and 49 Text keep the stored text.
        "0" | "49" => NumberFormat::General,
        // 1 `0`, 2 `0.00`, 3 `#,##0`, 4 `#,##0.00`.
        "1" => plain(0, false),
        "2" => plain(2, false),
        "3" => plain(0, true),
        "4" => plain(2, true),
        // 9 `0%`, 10 `0.00%`.
        "9" => NumberFormat::Percent { decimals: 0 },
        "10" => NumberFormat::Percent { decimals: 2 },
        // 14-17 are date forms; 22 is date with time.
        "14" | "15" | "16" | "17" => NumberFormat::IsoDate,
        "22" => NumberFormat::IsoDateTime,
        // 37-40 are accounting forms that render negatives in parentheses.
        "37" | "38" => accounting(0),
        "39" | "40" => accounting(2),
        _ => NumberFormat::Unsupported,
    }
}

fn plain(decimals: usize, grouped: bool) -> NumberFormat {
    NumberFormat::Decimal(DecimalFormat {
        decimals,
        grouped,
        prefix: String::new(),
        suffix: String::new(),
        parenthesized_negative: false,
    })
}

fn accounting(decimals: usize) -> NumberFormat {
    NumberFormat::Decimal(DecimalFormat {
        decimals,
        grouped: true,
        prefix: String::new(),
        suffix: String::new(),
        parenthesized_negative: true,
    })
}

/// Classifies a custom format code.
pub(super) fn classify_format_code(code: &str) -> NumberFormat {
    let mut sections = code.split(';');
    let positive = sections.next().unwrap_or(code);
    // A negative section in parentheses is the accounting convention; any
    // other rewrite of the negative form is not reproduced.
    let negative = sections.next();
    let parenthesized_negative = match negative {
        None => false,
        Some(section) => {
            let trimmed = section.trim();
            if trimmed.starts_with('(') && trimmed.ends_with(')') {
                true
            } else {
                return NumberFormat::Unsupported;
            }
        }
    };
    if sections.next().is_some() {
        return NumberFormat::Unsupported;
    }

    let literals = split_literals(positive);
    let body = literals.body.trim();
    if body.is_empty() {
        return NumberFormat::Unsupported;
    }
    if body.contains('%') {
        return percent_format(body, parenthesized_negative);
    }
    let lowered = body.to_ascii_lowercase();
    if lowered.contains('y') || lowered.contains('d') || lowered.contains('m') {
        if !literals.prefix.is_empty() || !literals.suffix.is_empty() {
            return NumberFormat::Unsupported;
        }
        return if lowered.contains('h') || lowered.contains('s') {
            NumberFormat::IsoDateTime
        } else {
            NumberFormat::IsoDate
        };
    }
    numeric_format(body, &literals, parenthesized_negative)
}

fn percent_format(body: &str, parenthesized_negative: bool) -> NumberFormat {
    if parenthesized_negative {
        return NumberFormat::Unsupported;
    }
    let numeric = body.replace('%', "");
    if !numeric
        .chars()
        .all(|value| matches!(value, '0' | '#' | '.' | ',' | ' ' | '?'))
    {
        return NumberFormat::Unsupported;
    }
    match decimals_of(&numeric) {
        Some(decimals) => NumberFormat::Percent { decimals },
        None => NumberFormat::Unsupported,
    }
}

fn numeric_format(body: &str, literals: &Literals, parenthesized_negative: bool) -> NumberFormat {
    // Only digit placeholders, grouping and a sign may remain.
    if !body
        .chars()
        .all(|value| matches!(value, '0' | '#' | '.' | ',' | '-' | '+' | ' ' | '?'))
    {
        return NumberFormat::Unsupported;
    }
    if !body.contains('0') && !body.contains('#') {
        return NumberFormat::Unsupported;
    }
    let integer = body.split('.').next().unwrap_or(body);
    let Some(decimals) = decimals_of(body) else {
        return NumberFormat::Unsupported;
    };
    NumberFormat::Decimal(DecimalFormat {
        decimals,
        grouped: integer.contains(','),
        prefix: literals.prefix.clone(),
        suffix: literals.suffix.clone(),
        parenthesized_negative,
    })
}

/// Number of fixed fraction digits, or `None` when the fraction uses
/// optional placeholders. `#` and `?` suppress or pad trailing zeros, which
/// this module does not reproduce.
fn decimals_of(code: &str) -> Option<usize> {
    match code.split_once('.') {
        None => Some(0),
        Some((_, fraction)) => {
            let digits = fraction.chars().filter(|value| *value == '0').count();
            if fraction.chars().any(|value| matches!(value, '#' | '?')) {
                return None;
            }
            Some(digits)
        }
    }
}

struct Literals {
    prefix: String,
    suffix: String,
    body: String,
}

/// Splits a section into leading literal text, placeholder body and trailing
/// literal text. Currency markers such as `[$$-409]` and quoted runs become
/// literal text; alignment and fill directives are dropped.
fn split_literals(code: &str) -> Literals {
    let mut prefix = String::new();
    let mut suffix = String::new();
    let mut body = String::new();
    let mut chars = code.chars().peekable();
    while let Some(value) = chars.next() {
        match value {
            '"' => {
                let literal: String = chars.by_ref().take_while(|item| *item != '"').collect();
                push_literal(&mut prefix, &mut suffix, &body, &literal);
            }
            '[' => {
                let token: String = chars.by_ref().take_while(|item| *item != ']').collect();
                // `[$SYMBOL-LOCALE]` carries a currency symbol; colour and
                // condition tokens carry none.
                if let Some(rest) = token.strip_prefix('$') {
                    let symbol = rest.split('-').next().unwrap_or(rest);
                    push_literal(&mut prefix, &mut suffix, &body, symbol);
                }
            }
            '\\' => {
                if let Some(escaped) = chars.next() {
                    push_literal(&mut prefix, &mut suffix, &body, &escaped.to_string());
                }
            }
            '_' => {
                // Reserves the width of the next character; contributes no
                // visible glyph.
                let _ = chars.next();
            }
            '*' => {
                let _ = chars.next();
            }
            '$' | '\u{a4}' | '\u{20ac}' | '\u{a3}' | '\u{a5}' | '\u{20a9}' => {
                push_literal(&mut prefix, &mut suffix, &body, &value.to_string());
            }
            other => body.push(other),
        }
    }
    Literals {
        prefix,
        suffix,
        body,
    }
}

fn push_literal(prefix: &mut String, suffix: &mut String, body: &str, value: &str) {
    if body.chars().any(|item| matches!(item, '0' | '#' | '?')) {
        suffix.push_str(value);
    } else {
        prefix.push_str(value);
    }
}
