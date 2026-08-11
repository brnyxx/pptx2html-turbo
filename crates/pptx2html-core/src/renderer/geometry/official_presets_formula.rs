use std::collections::HashMap;

pub(super) struct GuideEnvironment {
    values: HashMap<String, f64>,
}

impl GuideEnvironment {
    pub(super) fn new(width: f64, height: f64) -> Self {
        let width = extent(width);
        let height = extent(height);
        let short_side = width.min(height);
        let long_side = width.max(height);
        let mut values = HashMap::from([
            ("w".into(), width),
            ("h".into(), height),
            ("l".into(), 0.0),
            ("t".into(), 0.0),
            ("r".into(), width),
            ("b".into(), height),
            ("hc".into(), width / 2.0),
            ("vc".into(), height / 2.0),
            ("ss".into(), short_side),
            ("ls".into(), long_side),
            ("cd2".into(), 10_800_000.0),
            ("cd4".into(), 5_400_000.0),
            ("3cd4".into(), 16_200_000.0),
            ("cd8".into(), 2_700_000.0),
            ("3cd8".into(), 8_100_000.0),
            ("5cd8".into(), 13_500_000.0),
            ("7cd8".into(), 18_900_000.0),
        ]);
        for divisor in [2, 3, 4, 5, 6, 8, 10, 12, 16, 20, 32] {
            values.insert(format!("wd{divisor}"), width / divisor as f64);
            values.insert(format!("hd{divisor}"), height / divisor as f64);
            values.insert(format!("ssd{divisor}"), short_side / divisor as f64);
        }
        Self { values }
    }

    pub(super) fn insert(&mut self, name: &str, value: f64) {
        self.values.insert(name.to_owned(), finite(value));
    }

    pub(super) fn resolve(&self, token: &str) -> Result<f64, String> {
        if let Ok(value) = token.parse::<f64>() {
            return if value.is_finite() {
                Ok(value)
            } else {
                Err(format!("non-finite guide token: {token}"))
            };
        }
        self.values
            .get(token)
            .copied()
            .ok_or_else(|| format!("unknown guide token: {token}"))
    }

    pub(super) fn evaluate(&self, formula: &str) -> Result<f64, String> {
        let parts = formula.split_whitespace().collect::<Vec<_>>();
        let value = match parts.as_slice() {
            ["val", value] => self.resolve(value)?,
            ["+-", x, y, z] | ["+-", x, y, z, "0"] => {
                self.resolve(x)? + self.resolve(y)? - self.resolve(z)?
            }
            ["*/", x, y, z] => divide(self.resolve(x)? * self.resolve(y)?, self.resolve(z)?),
            ["+/", x, y, z] => divide(self.resolve(x)? + self.resolve(y)?, self.resolve(z)?),
            ["?:", condition, positive, negative] => {
                if self.resolve(condition)? > 0.0 {
                    self.resolve(positive)?
                } else {
                    self.resolve(negative)?
                }
            }
            ["abs", value] => self.resolve(value)?.abs(),
            ["at2", x, y] => self.resolve(y)?.atan2(self.resolve(x)?).to_degrees() * 60_000.0,
            ["cat2", scale, x, y] => {
                self.resolve(scale)? * self.resolve(y)?.atan2(self.resolve(x)?).cos()
            }
            ["cos", scale, angle] => self.resolve(scale)? * radians(self.resolve(angle)?).cos(),
            ["max", x, y] => self.resolve(x)?.max(self.resolve(y)?),
            ["min", x, y] => self.resolve(x)?.min(self.resolve(y)?),
            ["mod", x, y, z] => self
                .resolve(x)?
                .hypot(self.resolve(y)?)
                .hypot(self.resolve(z)?),
            ["pin", minimum, value, maximum] => self
                .resolve(value)?
                .max(self.resolve(minimum)?)
                .min(self.resolve(maximum)?),
            ["sat2", scale, x, y] => {
                self.resolve(scale)? * self.resolve(y)?.atan2(self.resolve(x)?).sin()
            }
            ["sin", scale, angle] => self.resolve(scale)? * radians(self.resolve(angle)?).sin(),
            ["sqrt", value] => self.resolve(value)?.max(0.0).sqrt(),
            ["tan", scale, angle] => self.resolve(scale)? * radians(self.resolve(angle)?).tan(),
            _ => return Err(format!("unsupported guide formula: {formula}")),
        };
        Ok(finite(value))
    }
}

fn extent(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

fn divide(numerator: f64, denominator: f64) -> f64 {
    if denominator.abs() < f64::EPSILON {
        0.0
    } else {
        numerator / denominator
    }
}

fn radians(angle: f64) -> f64 {
    (angle / 60_000.0).to_radians()
}

#[cfg(test)]
mod tests {
    use super::GuideEnvironment;

    #[test]
    fn official_operator_matrix_matches_drawingml_semantics() {
        let mut environment = GuideEnvironment::new(160.0, 100.0);
        environment.insert("x", 3.0);
        environment.insert("y", 4.0);
        environment.insert("z", 12.0);
        let cases = [
            ("val x", 3.0),
            ("+- 5 4 3", 6.0),
            ("*/ 6 4 3", 8.0),
            ("+/ 6 4 2", 5.0),
            ("?: 1 8 9", 8.0),
            ("?: 0 8 9", 9.0),
            ("?: -1 8 9", 9.0),
            ("abs -7", 7.0),
            ("max 7 3", 7.0),
            ("min 7 3", 3.0),
            ("mod x y z", 13.0),
            ("pin 1 5 3", 3.0),
            ("sqrt 16", 4.0),
            ("sin 10 5400000", 10.0),
            ("cos 10 0", 10.0),
            ("tan 10 2700000", 10.0),
            ("at2 x y", 3.187_806_141_249_358_7e6),
            ("cat2 10 y z", 3.162_277_660_168_38),
            ("sat2 10 y z", 9.486_832_980_505_138),
            ("+- 8 3 2 0", 9.0),
        ];
        for (formula, expected) in cases {
            let actual = environment.evaluate(formula).expect("official formula");
            assert!((actual - expected).abs() < 1e-6, "{formula}: {actual}");
        }
        assert!(environment.evaluate("unsupported x").is_err());
        assert!(environment.evaluate("+- 8 3 2 999").is_err());
        assert!(environment.evaluate("val 1 2").is_err());
    }
}
