use std::io::{Cursor, Read};

use zip::ZipArchive;

use super::relationships::{self, Relationship, TargetMode};
use crate::error::PptxResult;
use crate::model::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    SupportTier,
};

pub(crate) const RELATIONSHIP_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableStyles";
const OWNER_PART: &str = "ppt/presentation.xml";
const RELS_PART: &str = "ppt/_rels/presentation.xml.rels";

#[derive(Debug)]
pub(crate) struct Selection {
    pub(crate) target: Option<String>,
    issues: Vec<SelectionIssue>,
}

#[derive(Debug)]
enum SelectionIssue {
    Rejected { id: String, reason: &'static str },
    Duplicate { id: String, count: usize },
}

pub(crate) fn select(relationships: &[Relationship]) -> Selection {
    let mut valid = Vec::new();
    let mut issues = Vec::new();
    for relationship in relationships
        .iter()
        .filter(|item| item.relationship_type == RELATIONSHIP_TYPE)
    {
        let target = match &relationship.target_mode {
            TargetMode::Internal => {
                relationships::resolve_internal_target(OWNER_PART, &relationship.target)
            }
            TargetMode::External => {
                issues.push(SelectionIssue::Rejected {
                    id: safe_id(&relationship.id),
                    reason: "external",
                });
                continue;
            }
            TargetMode::Other(_) => {
                issues.push(SelectionIssue::Rejected {
                    id: safe_id(&relationship.id),
                    reason: "invalid_target_mode",
                });
                continue;
            }
        };
        match target {
            Ok(path) => valid.push((safe_id(&relationship.id), path)),
            Err(error) => issues.push(SelectionIssue::Rejected {
                id: safe_id(&relationship.id),
                reason: error.as_str(),
            }),
        }
    }
    valid.sort();
    if valid.len() > 1 {
        issues.push(SelectionIssue::Duplicate {
            id: valid[0].0.clone(),
            count: valid.len(),
        });
    }
    Selection {
        target: valid.first().map(|(_, path)| path.clone()),
        issues,
    }
}

pub(crate) fn collect_diagnostics(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    diagnostics: &mut Vec<ConversionDiagnostic>,
) -> PptxResult<Option<String>> {
    let relationships = match read_entry(archive, RELS_PART) {
        Ok(xml) => relationships::parse_relationship_records(&xml)?,
        Err(_) => return Ok(None),
    };
    let selection = select(&relationships);
    for issue in selection.issues {
        let (code, raw, reason) = match issue {
            SelectionIssue::Rejected { id, reason } => (
                "TABLE_STYLE_RELATIONSHIP_REJECTED",
                format!("relationship_id={id};reason={reason}"),
                "Table styles relationship was rejected before package access",
            ),
            SelectionIssue::Duplicate { id, count } => (
                "TABLE_STYLE_RELATIONSHIP_DUPLICATE",
                format!("selected_relationship_id={id};count={count}"),
                "Multiple internal table styles relationships were present; the deterministic first relationship was used",
            ),
        };
        diagnostics.push(diagnostic(code, RELS_PART, raw, reason));
    }
    let Some(target) = selection.target else {
        return Ok(None);
    };
    let xml = match read_entry(archive, &target) {
        Ok(xml) => xml,
        Err(_) => {
            diagnostics.push(diagnostic(
                "TABLE_STYLE_PART_MISSING",
                RELS_PART,
                "reason=missing_internal_part".to_owned(),
                "Selected internal table styles part is missing",
            ));
            return Ok(Some(target));
        }
    };
    if let Err(error) = super::table_style_parser::parse_table_styles(&xml) {
        let (code, reason) = match error {
            crate::error::PptxError::Xml(_) => (
                "TABLE_STYLE_PART_MALFORMED",
                "Table styles part is not well-formed XML",
            ),
            _ => (
                "TABLE_STYLE_XML_INVALID",
                "Table styles part violates the supported DrawingML table style grammar",
            ),
        };
        diagnostics.push(diagnostic(
            code,
            &target,
            "reason=invalid_table_styles_xml".to_owned(),
            reason,
        ));
    }
    Ok(Some(target))
}

fn diagnostic(code: &str, part: &str, raw: String, reason: &str) -> ConversionDiagnostic {
    ConversionDiagnostic {
        code: code.to_owned(),
        family: FeatureFamily::Tables,
        support_tier: SupportTier::Approximate,
        stage: Some(CapabilityStage::Parsed),
        location: DiagnosticLocation {
            part_name: Some(part.to_owned()),
            ..Default::default()
        },
        raw_reference: Some(raw),
        fallback_kind: FallbackKind::TableStyleDefinitionUnavailable,
        reason: reason.to_owned(),
    }
}

fn read_entry(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> PptxResult<String> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| crate::error::PptxError::MissingFile(name.to_owned()))?;
    let mut xml = String::new();
    file.read_to_string(&mut xml)?;
    Ok(xml)
}

fn safe_id(id: &str) -> String {
    if !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        id.to_owned()
    } else {
        "redacted".to_owned()
    }
}
