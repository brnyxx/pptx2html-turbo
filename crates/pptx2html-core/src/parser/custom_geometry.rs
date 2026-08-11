use std::collections::{HashMap, HashSet};

use crate::model::{CustomGeometryIssue, CustomGuide, GuideFormulaError, Size};

use super::custom_guide;

pub(super) struct CustomGeometryState {
    values: HashMap<String, f64>,
    guides: Vec<CustomGuide>,
    issues: Vec<CustomGeometryIssue>,
    declared_guides: HashSet<String>,
    width: f64,
    height: f64,
}

impl CustomGeometryState {
    pub(super) fn new() -> Self {
        Self {
            values: HashMap::new(),
            guides: Vec::new(),
            issues: Vec::new(),
            declared_guides: HashSet::new(),
            width: 0.0,
            height: 0.0,
        }
    }

    pub(super) fn reset(&mut self, size: Size) {
        self.values.clear();
        self.guides.clear();
        self.issues.clear();
        self.declared_guides.clear();
        self.width = size.width.0.max(0) as f64;
        self.height = size.height.0.max(0) as f64;
        self.insert_predefined();
    }

    pub(super) fn add_guide(&mut self, name: String, raw_formula: String) {
        let evaluation = custom_guide::evaluate(&raw_formula, &self.values);
        if let Ok(value) = evaluation {
            self.values.insert(name.clone(), value);
        }
        self.declared_guides.insert(name.clone());
        self.guides.push(CustomGuide {
            name,
            raw_formula,
            evaluation,
        });
    }

    pub(super) fn resolve(&mut self, element: &str, attribute: &str, token: &str) -> f64 {
        match custom_guide::resolve(token, &self.values) {
            Ok(value) => value,
            Err(GuideFormulaError::UnresolvedToken(_)) if self.declared_guides.contains(token) => {
                0.0
            }
            Err(error) => {
                self.issues.push(CustomGeometryIssue {
                    element: element.to_owned(),
                    attribute: attribute.to_owned(),
                    token: token.to_owned(),
                    error,
                });
                0.0
            }
        }
    }

    pub(super) fn path_extent(&mut self, attribute: &str, token: Option<&str>) -> f64 {
        match token {
            Some(token) => self.resolve("a:path", attribute, token),
            None if attribute == "w" => self.width,
            None => self.height,
        }
    }

    pub(super) fn has_failures(&self) -> bool {
        self.guides.iter().any(|guide| guide.evaluation.is_err()) || !self.issues.is_empty()
    }

    pub(super) fn single_guide_error_formula(&self) -> Option<String> {
        if !self.issues.is_empty() {
            return None;
        }
        let mut failures = self.guides.iter().filter(|guide| guide.evaluation.is_err());
        let formula = failures.next()?.raw_formula.clone();
        failures.next().is_none().then_some(formula)
    }

    pub(super) fn take_guides(&mut self) -> Vec<CustomGuide> {
        std::mem::take(&mut self.guides)
    }

    pub(super) fn take_issues(&mut self) -> Vec<CustomGeometryIssue> {
        std::mem::take(&mut self.issues)
    }

    fn insert_predefined(&mut self) {
        let short_side = self.width.min(self.height);
        let long_side = self.width.max(self.height);
        self.values.extend([
            ("w".to_owned(), self.width),
            ("h".to_owned(), self.height),
            ("l".to_owned(), 0.0),
            ("t".to_owned(), 0.0),
            ("r".to_owned(), self.width),
            ("b".to_owned(), self.height),
            ("hc".to_owned(), self.width / 2.0),
            ("vc".to_owned(), self.height / 2.0),
            ("ss".to_owned(), short_side),
            ("ls".to_owned(), long_side),
            ("cd2".to_owned(), 10_800_000.0),
            ("cd4".to_owned(), 5_400_000.0),
            ("3cd4".to_owned(), 16_200_000.0),
            ("cd8".to_owned(), 2_700_000.0),
            ("3cd8".to_owned(), 8_100_000.0),
            ("5cd8".to_owned(), 13_500_000.0),
            ("7cd8".to_owned(), 18_900_000.0),
        ]);
        for divisor in [2, 3, 4, 5, 6, 8] {
            self.values
                .insert(format!("wd{divisor}"), self.width / f64::from(divisor));
            self.values
                .insert(format!("hd{divisor}"), self.height / f64::from(divisor));
        }
        self.values.insert("wd10".to_owned(), self.width / 10.0);
        for divisor in [2, 4, 6, 8, 16, 32] {
            self.values
                .insert(format!("ssd{divisor}"), short_side / f64::from(divisor));
        }
    }
}

impl Default for CustomGeometryState {
    fn default() -> Self {
        Self::new()
    }
}
