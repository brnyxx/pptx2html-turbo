//! Classification of ECMA-376 number format codes.
//!
//! Only a bounded subset is reproduced: decimal precision, thousands
//! grouping, a leading or trailing literal, percentages, scientific notation,
//! and dates or times. Other codes remain unsupported so attribution fails
//! closed.

const MAX_FORMAT_CODE_CHARS: usize = 254;
const MAX_RENDERED_DIGITS: usize = 64;

/// How a cell's stored value must be turned into visible text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NumberFormat {
    /// Render the stored value as-is.
    General,
    /// Fixed-point decimal, optionally grouped and currency-prefixed.
    Decimal(DecimalFormat),
    /// Multiply by 100 and append `%`, with the given decimal precision.
    Percent { decimals: usize },
    /// Exponential notation with bounded mantissa and exponent precision.
    Scientific(ScientificFormat),
    /// Serial date rendered as an ISO calendar date.
    IsoDate,
    /// Serial date rendered as an ISO date and time.
    IsoDateTime,
    /// Fractional-day value rendered as a clock time.
    Time { padded_hour: bool },
    /// The format alters the visible value but is not emulated here.
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DecimalFormat {
    pub(super) minimum_decimals: usize,
    pub(super) maximum_decimals: usize,
    pub(super) grouped: bool,
    pub(super) prefix: String,
    pub(super) suffix: String,
    /// Negative values render in parentheses instead of with a minus sign.
    pub(super) parenthesized_negative: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScientificFormat {
    pub(super) decimals: usize,
    pub(super) exponent_digits: usize,
    pub(super) uppercase: bool,
    pub(super) show_positive_sign: bool,
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
        minimum_decimals: decimals,
        maximum_decimals: decimals,
        grouped,
        prefix: String::new(),
        suffix: String::new(),
        parenthesized_negative: false,
    })
}

fn accounting(decimals: usize) -> NumberFormat {
    NumberFormat::Decimal(DecimalFormat {
        minimum_decimals: decimals,
        maximum_decimals: decimals,
        grouped: true,
        prefix: String::new(),
        suffix: String::new(),
        parenthesized_negative: true,
    })
}

/// Classifies a custom format code.
pub(super) fn classify_format_code(code: &str) -> NumberFormat {
    if code.chars().count() > MAX_FORMAT_CODE_CHARS {
        return NumberFormat::Unsupported;
    }
    if code.trim().eq_ignore_ascii_case("general") {
        return NumberFormat::General;
    }
    let mut sections = code.split(';');
    let positive = sections.next().unwrap_or(code);
    let negative = sections.next();
    if sections.next().is_some() {
        return NumberFormat::Unsupported;
    }
    let literals = split_literals(positive);
    if literals.conditional {
        return NumberFormat::Unsupported;
    }
    let body = literals.body.trim();
    if body.is_empty() {
        return NumberFormat::Unsupported;
    }
    if let Some(format) = temporal_format(body, &literals) {
        return match negative.map(str::trim) {
            None | Some("@") => format,
            Some(_) => NumberFormat::Unsupported,
        };
    }
    if has_scaling_comma(body) {
        return NumberFormat::Unsupported;
    }
    // A negative section in parentheses is the accounting convention; any
    // other rewrite of the negative form is not reproduced.
    let parenthesized_negative = match negative {
        None => false,
        Some(section) => {
            let trimmed = section.trim();
            if (trimmed.starts_with('(') && trimmed.ends_with(')'))
                || (trimmed.starts_with("\\(") && trimmed.ends_with("\\)"))
            {
                true
            } else {
                return NumberFormat::Unsupported;
            }
        }
    };
    if body.contains('%') {
        return percent_format(body, parenthesized_negative);
    }
    if body.contains(['E', 'e']) {
        return scientific_format(body, &literals, parenthesized_negative);
    }
    numeric_format(body, &literals, parenthesized_negative)
}

fn temporal_format(body: &str, literals: &Literals) -> Option<NumberFormat> {
    if !literals
        .prefix
        .chars()
        .chain(literals.suffix.chars())
        .all(|value| value.is_ascii_whitespace() || matches!(value, '-' | '/' | ':' | '.' | ','))
    {
        return None;
    }
    let lowered = body.to_ascii_lowercase();
    let has_date = lowered.contains('y') || lowered.contains('d');
    let has_time = lowered.contains('h') || lowered.contains('s');
    if has_date {
        return Some(if has_time {
            NumberFormat::IsoDateTime
        } else {
            NumberFormat::IsoDate
        });
    }
    if has_time {
        let compact: String = lowered
            .chars()
            .filter(|value| !value.is_whitespace())
            .collect();
        return match compact.as_str() {
            "h:mm:ss" => Some(NumberFormat::Time { padded_hour: false }),
            "hh:mm:ss" => Some(NumberFormat::Time { padded_hour: true }),
            _ => None,
        };
    }
    lowered.contains('m').then_some(NumberFormat::IsoDate)
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
    match decimal_places(&numeric) {
        Some((minimum, maximum)) if minimum == maximum => {
            NumberFormat::Percent { decimals: minimum }
        }
        None => NumberFormat::Unsupported,
        Some(_) => NumberFormat::Unsupported,
    }
}

fn scientific_format(
    body: &str,
    literals: &Literals,
    parenthesized_negative: bool,
) -> NumberFormat {
    if parenthesized_negative || !literals.prefix.is_empty() || !literals.suffix.is_empty() {
        return NumberFormat::Unsupported;
    }
    let Some((index, marker)) = body
        .match_indices(['E', 'e'])
        .next()
        .map(|(index, marker)| (index, marker.as_bytes()[0] as char))
    else {
        return NumberFormat::Unsupported;
    };
    let mantissa = &body[..index];
    let exponent = &body[index + 1..];
    let Some(sign) = exponent.chars().next() else {
        return NumberFormat::Unsupported;
    };
    let exponent_digits = &exponent[sign.len_utf8()..];
    let Some((minimum, maximum)) = decimal_places(mantissa) else {
        return NumberFormat::Unsupported;
    };
    if !matches!(sign, '+' | '-')
        || exponent_digits.is_empty()
        || exponent_digits.len() > MAX_RENDERED_DIGITS
        || !exponent_digits.chars().all(|value| value == '0')
        || minimum != maximum
        || !mantissa
            .chars()
            .all(|value| matches!(value, '0' | '#' | '.'))
        || mantissa.split('.').next().unwrap_or("") != "0"
    {
        return NumberFormat::Unsupported;
    }
    NumberFormat::Scientific(ScientificFormat {
        decimals: minimum,
        exponent_digits: exponent_digits.len(),
        uppercase: marker == 'E',
        show_positive_sign: sign == '+',
    })
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
    let Some((minimum_decimals, maximum_decimals)) = decimal_places(body) else {
        return NumberFormat::Unsupported;
    };
    NumberFormat::Decimal(DecimalFormat {
        minimum_decimals,
        maximum_decimals,
        grouped: integer.contains(','),
        prefix: literals.prefix.clone(),
        suffix: literals.suffix.clone(),
        parenthesized_negative,
    })
}

fn decimal_places(code: &str) -> Option<(usize, usize)> {
    match code.split_once('.') {
        None => Some((0, 0)),
        Some((_, fraction)) => {
            if fraction.contains('?') {
                return None;
            }
            let minimum = fraction.chars().filter(|value| *value == '0').count();
            let maximum = fraction
                .chars()
                .filter(|value| matches!(value, '0' | '#'))
                .count();
            (maximum <= MAX_RENDERED_DIGITS).then_some((minimum, maximum))
        }
    }
}

fn has_scaling_comma(code: &str) -> bool {
    let Some(last_placeholder) = code.rfind(['0', '#', '?']) else {
        return false;
    };
    code[last_placeholder + 1..].contains(',')
}

struct Literals {
    prefix: String,
    suffix: String,
    body: String,
    conditional: bool,
}

/// Splits a section into leading literal text, placeholder body and trailing
/// literal text. Currency markers such as `[$$-409]` and quoted runs become
/// literal text; alignment and fill directives are dropped.
fn split_literals(code: &str) -> Literals {
    let mut prefix = String::new();
    let mut suffix = String::new();
    let mut body = String::new();
    let mut conditional = false;
    let mut chars = code.chars().peekable();
    while let Some(value) = chars.next() {
        match value {
            '"' => {
                let literal: String = chars.by_ref().take_while(|item| *item != '"').collect();
                push_literal(&mut prefix, &mut suffix, &body, &literal);
            }
            '[' => {
                let token: String = chars.by_ref().take_while(|item| *item != ']').collect();
                conditional |= matches!(token.trim_start().chars().next(), Some('<' | '>' | '='));
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
        conditional,
    }
}

fn push_literal(prefix: &mut String, suffix: &mut String, body: &str, value: &str) {
    if body.chars().any(|item| matches!(item, '0' | '#' | '?')) {
        suffix.push_str(value);
    } else {
        prefix.push_str(value);
    }
}
