use crate::xlsx_xml::{
    calc_pr_name, element_namespace, is_workbook_name, parse_end_tag, parse_start_tag, tag_end,
    validate_xml_text, workbook_child_name,
};
use crate::{NativeError, NativeResult};

const MAX_XML_DEPTH: usize = 128;
const CALC_PR_FOLLOWING_CHILDREN: [&str; 9] = [
    "oleSize",
    "customWorkbookViews",
    "pivotCaches",
    "smartTagPr",
    "smartTagTypes",
    "webPublishing",
    "fileRecoveryPr",
    "webPublishObjects",
    "extLst",
];

pub(crate) fn freeze_workbook_calculation(workbook: &[u8]) -> NativeResult<Vec<u8>> {
    let text = std::str::from_utf8(workbook)
        .map_err(|_| malformed_error("xl/workbook.xml is not UTF-8 XML"))?;
    let mut stack = Vec::new();
    let mut position = 0_usize;
    let mut root_name = None;
    let mut root_namespace = None;
    let mut root_namespaces = Vec::new();
    let mut root_end = None;
    let mut calc_range = None;
    let mut calc_insertion = None;
    let mut calc_depth = None;

    while position < text.len() {
        if text.as_bytes()[position] != b'<' {
            let next = text[position..]
                .find('<')
                .map(|offset| position + offset)
                .unwrap_or(text.len());
            validate_xml_text(&text[position..next])?;
            if (calc_depth.is_some() || root_end.is_some())
                && !text[position..next]
                    .bytes()
                    .all(|byte| byte.is_ascii_whitespace())
            {
                return malformed("xl/workbook.xml calcPr contains text");
            }
            position = next;
            continue;
        }

        if text[position..].starts_with("<!--") {
            if calc_depth.is_some() {
                return malformed("xl/workbook.xml calcPr contains markup");
            }
            let Some(end) = text[position + 4..].find("-->") else {
                return malformed("xl/workbook.xml has an unterminated comment");
            };
            position += end + 7;
            continue;
        }
        if text[position..].starts_with("<![CDATA[") {
            if calc_depth.is_some() {
                return malformed("xl/workbook.xml calcPr contains markup");
            }
            let Some(end) = text[position + 9..].find("]]>") else {
                return malformed("xl/workbook.xml has an unterminated CDATA section");
            };
            position += end + 12;
            continue;
        }
        if text[position..].starts_with("<?") {
            if calc_depth.is_some() {
                return malformed("xl/workbook.xml calcPr contains markup");
            }
            let Some(end) = text[position + 2..].find("?>") else {
                return malformed("xl/workbook.xml has an unterminated processing instruction");
            };
            position += end + 4;
            continue;
        }
        if text[position..].starts_with("<!") {
            return malformed("xl/workbook.xml has an unsupported declaration");
        }

        let end = tag_end(text, position)?;
        if text[position..].starts_with("</") {
            let name = parse_end_tag(&text[position + 2..end])?;
            let Some(open) = stack.pop() else {
                return malformed("xl/workbook.xml has an unmatched closing tag");
            };
            if open != name {
                return malformed("xl/workbook.xml has mismatched element tags");
            }
            if calc_depth == Some(stack.len() + 1) {
                calc_depth = None;
                let Some((start, _)) = calc_range else {
                    return malformed("xl/workbook.xml calcPr state is invalid");
                };
                calc_range = Some((start, end + 1));
            }
            if stack.is_empty() {
                root_end = Some(position);
            }
        } else {
            let start_tag = parse_start_tag(&text[position + 1..end])?;
            let name = start_tag.name;
            let self_closing = start_tag.self_closing;
            if calc_depth.is_some() {
                return malformed("xl/workbook.xml calcPr contains markup");
            }
            if stack.is_empty() {
                if root_name.is_some() || !is_workbook_name(&name) || self_closing {
                    return malformed("xl/workbook.xml must have one non-empty workbook root");
                }
                root_name = Some(name.clone());
                root_namespace = element_namespace(&name, "workbook", &start_tag.namespaces, &[])
                    .map(str::to_owned);
                root_namespaces = start_tag.namespaces.clone();
            } else if root_end.is_some() {
                return malformed("xl/workbook.xml has content after its root element");
            }

            if let Some(root) = root_name.as_deref() {
                let matches_workbook_child = |local_name: &str| {
                    root_namespace.as_deref().map_or_else(
                        || name == workbook_child_name(root, local_name),
                        |namespace| {
                            element_namespace(
                                &name,
                                local_name,
                                &start_tag.namespaces,
                                &root_namespaces,
                            ) == Some(namespace)
                        },
                    )
                };
                if stack.len() == 1
                    && calc_range.is_none()
                    && calc_insertion.is_none()
                    && CALC_PR_FOLLOWING_CHILDREN
                        .iter()
                        .any(|local_name| matches_workbook_child(local_name))
                {
                    calc_insertion = Some(position);
                }
                if matches_workbook_child("calcPr") {
                    if calc_insertion.is_some() {
                        return malformed("xl/workbook.xml calcPr is out of schema order");
                    }
                    if calc_range.is_some() {
                        return malformed("xl/workbook.xml has duplicate calcPr elements");
                    }
                    if stack.len() != 1 {
                        return malformed("xl/workbook.xml calcPr is not a workbook child");
                    }
                    calc_range = Some((position, end + 1));
                    if !self_closing {
                        calc_depth = Some(stack.len() + 1);
                    }
                }
            }
            if !self_closing {
                if stack.len() == MAX_XML_DEPTH {
                    return malformed("xl/workbook.xml nesting exceeds the supported limit");
                }
                stack.push(name);
            }
        }
        position = end + 1;
    }

    if !stack.is_empty() || root_name.is_none() || root_end.is_none() {
        return malformed("xl/workbook.xml has malformed element structure");
    }
    let Some(root) = root_name else {
        return malformed("xl/workbook.xml has no workbook root");
    };
    let Some(root_end) = root_end else {
        return malformed("xl/workbook.xml has no closed workbook root");
    };
    let calc = format!(
        "<{} calcMode=\"manual\" calcOnSave=\"0\" forceFullCalc=\"0\" fullCalcOnLoad=\"0\"/>",
        calc_pr_name(&root)
    );
    let insertion = calc_insertion.unwrap_or(root_end);
    let (start, end) = calc_range.unwrap_or((insertion, insertion));
    let mut frozen = Vec::with_capacity(workbook.len().saturating_add(calc.len()));
    frozen.extend_from_slice(&workbook[..start]);
    frozen.extend_from_slice(calc.as_bytes());
    frozen.extend_from_slice(&workbook[end..]);
    Ok(frozen)
}

fn malformed<T>(reason: &str) -> NativeResult<T> {
    Err(malformed_error(reason))
}

fn malformed_error(reason: &str) -> NativeError {
    NativeError::MalformedBackendOutput {
        backend: "libreoffice",
        reason: reason.to_owned(),
    }
}
