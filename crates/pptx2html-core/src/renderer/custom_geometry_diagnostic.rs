use crate::model::{CustomGeometry, GuideFormulaError, UnsupportedData};

use super::fallback::write_json_string;

pub(super) struct CustomGeometryMetadata {
    pub(super) raw_reference: Option<String>,
    pub(super) data_model: Option<String>,
}

pub(super) fn metadata(data: &UnsupportedData) -> CustomGeometryMetadata {
    let data_model = data.custom_geometry.as_ref().map(serialize);
    let raw_reference = data.raw_xml.clone().or_else(|| data_model.clone());
    CustomGeometryMetadata {
        raw_reference,
        data_model,
    }
}

fn serialize(geometry: &CustomGeometry) -> String {
    let mut json = String::from("{\"guides\":[");
    for (index, guide) in geometry.guides.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str("{\"name\":");
        write_json_string(&mut json, &guide.name);
        json.push_str(",\"raw_formula\":");
        write_json_string(&mut json, &guide.raw_formula);
        json.push_str(",\"error\":");
        match &guide.evaluation {
            Ok(_) => json.push_str("null"),
            Err(error) => write_error(&mut json, error),
        }
        json.push('}');
    }
    json.push_str("],\"issues\":[");
    for (index, issue) in geometry.issues.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str("{\"element\":");
        write_json_string(&mut json, &issue.element);
        json.push_str(",\"attribute\":");
        write_json_string(&mut json, &issue.attribute);
        json.push_str(",\"token\":");
        write_json_string(&mut json, &issue.token);
        json.push_str(",\"error\":");
        write_error(&mut json, &issue.error);
        json.push('}');
    }
    json.push_str("]}");
    json
}

fn write_error(json: &mut String, error: &GuideFormulaError) {
    json.push_str("{\"kind\":");
    write_json_string(json, error_kind(error));
    match error {
        GuideFormulaError::UnknownOperator(operator) => string_detail(json, "operator", operator),
        GuideFormulaError::InvalidArity {
            operator,
            expected,
            actual,
        } => {
            string_detail(json, "operator", operator);
            json.push_str(&format!(",\"expected\":{expected},\"actual\":{actual}"));
        }
        GuideFormulaError::UnresolvedToken(token) | GuideFormulaError::NonFiniteToken(token) => {
            string_detail(json, "token", token)
        }
        GuideFormulaError::DomainError { operator, operand } => {
            string_detail(json, "operator", operator);
            string_detail(json, "operand", operand);
        }
        GuideFormulaError::Empty
        | GuideFormulaError::NonFiniteResult
        | GuideFormulaError::DivisionByZero => {}
    }
    json.push('}');
}

fn string_detail(json: &mut String, name: &str, value: &str) {
    json.push(',');
    write_json_string(json, name);
    json.push(':');
    write_json_string(json, value);
}

fn error_kind(error: &GuideFormulaError) -> &'static str {
    match error {
        GuideFormulaError::Empty => "Empty",
        GuideFormulaError::UnknownOperator(_) => "UnknownOperator",
        GuideFormulaError::InvalidArity { .. } => "InvalidArity",
        GuideFormulaError::UnresolvedToken(_) => "UnresolvedToken",
        GuideFormulaError::NonFiniteToken(_) => "NonFiniteToken",
        GuideFormulaError::NonFiniteResult => "NonFiniteResult",
        GuideFormulaError::DivisionByZero => "DivisionByZero",
        GuideFormulaError::DomainError { .. } => "DomainError",
    }
}
