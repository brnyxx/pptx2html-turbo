use std::collections::HashMap;

use crate::model::GuideFormulaError;

pub(super) fn evaluate(
    formula: &str,
    guides: &HashMap<String, f64>,
) -> Result<f64, GuideFormulaError> {
    let tokens = formula.split_whitespace().collect::<Vec<_>>();
    let Some(operator) = tokens.first().copied() else {
        return Err(GuideFormulaError::Empty);
    };
    let operands = &tokens[1..];
    let value = match operator {
        "val" => unary(operator, operands, guides, |value| value)?,
        "+-" => ternary(operator, operands, guides, |x, y, z| Ok(x + y - z))?,
        "*/" => ternary(operator, operands, guides, |x, y, z| divide(x * y, z))?,
        "+/" => ternary(operator, operands, guides, |x, y, z| divide(x + y, z))?,
        "pin" => ternary(operator, operands, guides, |low, value, high| {
            Ok(value.max(low).min(high))
        })?,
        "min" => binary(operator, operands, guides, |x, y| x.min(y))?,
        "max" => binary(operator, operands, guides, |x, y| x.max(y))?,
        "?:" => ternary(
            operator,
            operands,
            guides,
            |condition, positive, negative| Ok(if condition > 0.0 { positive } else { negative }),
        )?,
        "abs" => unary(operator, operands, guides, f64::abs)?,
        "sqrt" => {
            exact_arity(operator, operands, 1)?;
            let value = resolve(operands[0], guides)?;
            if value < 0.0 {
                return Err(GuideFormulaError::DomainError {
                    operator: operator.to_owned(),
                    operand: operands[0].to_owned(),
                });
            }
            value.sqrt()
        }
        "mod" => ternary(operator, operands, guides, |x, y, z| {
            Ok(x.hypot(y).hypot(z))
        })?,
        "sin" => binary(operator, operands, guides, |scale, angle| {
            scale * radians(angle).sin()
        })?,
        "cos" => binary(operator, operands, guides, |scale, angle| {
            scale * radians(angle).cos()
        })?,
        "cat2" => ternary(operator, operands, guides, |scale, x, y| {
            Ok(scale * y.atan2(x).cos())
        })?,
        "sat2" => ternary(operator, operands, guides, |scale, x, y| {
            Ok(scale * y.atan2(x).sin())
        })?,
        "at2" => binary(operator, operands, guides, |x, y| {
            y.atan2(x).to_degrees() * 60_000.0
        })?,
        "tan" => binary(operator, operands, guides, |scale, angle| {
            scale * radians(angle).tan()
        })?,
        _ => return Err(GuideFormulaError::UnknownOperator(operator.to_owned())),
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(GuideFormulaError::NonFiniteResult)
    }
}

pub(super) fn resolve(
    token: &str,
    guides: &HashMap<String, f64>,
) -> Result<f64, GuideFormulaError> {
    if let Ok(value) = token.parse::<f64>() {
        return if value.is_finite() {
            Ok(value)
        } else {
            Err(GuideFormulaError::NonFiniteToken(token.to_owned()))
        };
    }
    guides
        .get(token)
        .copied()
        .ok_or_else(|| GuideFormulaError::UnresolvedToken(token.to_owned()))
}

fn unary(
    operator: &str,
    operands: &[&str],
    guides: &HashMap<String, f64>,
    operation: impl FnOnce(f64) -> f64,
) -> Result<f64, GuideFormulaError> {
    exact_arity(operator, operands, 1)?;
    Ok(operation(resolve(operands[0], guides)?))
}

fn binary(
    operator: &str,
    operands: &[&str],
    guides: &HashMap<String, f64>,
    operation: impl FnOnce(f64, f64) -> f64,
) -> Result<f64, GuideFormulaError> {
    exact_arity(operator, operands, 2)?;
    Ok(operation(
        resolve(operands[0], guides)?,
        resolve(operands[1], guides)?,
    ))
}

fn ternary(
    operator: &str,
    operands: &[&str],
    guides: &HashMap<String, f64>,
    operation: impl FnOnce(f64, f64, f64) -> Result<f64, GuideFormulaError>,
) -> Result<f64, GuideFormulaError> {
    exact_arity(operator, operands, 3)?;
    operation(
        resolve(operands[0], guides)?,
        resolve(operands[1], guides)?,
        resolve(operands[2], guides)?,
    )
}

fn exact_arity(
    operator: &str,
    operands: &[&str],
    expected: usize,
) -> Result<(), GuideFormulaError> {
    if operands.len() == expected {
        Ok(())
    } else {
        Err(GuideFormulaError::InvalidArity {
            operator: operator.to_owned(),
            expected,
            actual: operands.len(),
        })
    }
}

fn divide(numerator: f64, denominator: f64) -> Result<f64, GuideFormulaError> {
    if denominator.abs() < f64::EPSILON {
        Err(GuideFormulaError::DivisionByZero)
    } else {
        Ok(numerator / denominator)
    }
}

fn radians(angle: f64) -> f64 {
    (angle / 60_000.0).to_radians()
}
