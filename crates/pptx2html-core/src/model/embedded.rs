use std::collections::BTreeMap;
use std::fmt::Write;

pub(crate) const RAW_REFERENCE_LIMIT: usize = 16 * 1024;
pub(crate) const PREVIEW_BYTE_LIMIT: usize = 2 * 1024 * 1024;

/// Backward-compatible marker for embedded-content inventory support.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmbeddedInventory;

/// Deterministic parser sideband owned by a presentation.
#[doc(hidden)]
#[derive(Debug, Clone, Default)]
pub struct EmbeddedInventoryStore {
    entries: BTreeMap<String, Vec<EmbeddedInventoryEntry>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EmbeddedInventoryEntry {
    pub source_identity: String,
    pub relationships: Vec<EmbeddedRelationship>,
    pub preview: Option<EmbeddedPreview>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmbeddedRelationship {
    pub id: String,
    pub relationship_type: String,
    pub part_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmbeddedPreview {
    pub relationship_id: String,
    pub mime_type: String,
    pub base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AlternateContentInventory {
    pub source_identity: String,
    pub selected_branch: usize,
    pub branches: Vec<AlternateContentBranch>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AlternateContentBranch {
    pub kind: &'static str,
    pub requires: Vec<String>,
    pub supported: bool,
    pub raw_xml: String,
}

impl EmbeddedInventoryStore {
    pub(crate) fn register(&mut self, key: String, inventory: EmbeddedInventoryEntry) {
        self.entries.entry(key).or_default().push(inventory);
    }

    pub(crate) fn get(&self, key: &str, occurrence: usize) -> Option<&EmbeddedInventoryEntry> {
        self.entries.get(key)?.get(occurrence)
    }
}

pub(crate) fn inventory_key(
    owner_part: &str,
    shape_id: u32,
    domain: &str,
    raw_xml: &str,
) -> String {
    format!("{owner_part}\u{1f}{shape_id}\u{1f}{domain}\u{1f}{raw_xml}")
}

impl EmbeddedInventoryEntry {
    pub fn to_json_with_raw_xml(&self, raw_xml: &str) -> String {
        bounded_inventory_json(self, (!raw_xml.is_empty()).then_some(raw_xml))
    }
}

fn bounded_inventory_json(inventory: &EmbeddedInventoryEntry, raw_xml: Option<&str>) -> String {
    let mut relationship_count = inventory.relationships.len();
    loop {
        let mut output = String::from("{\"source_identity\":\"");
        push_json_bounded(&mut output, &inventory.source_identity, 512);
        output.push_str("\",\"relationships\":[");
        for (index, relationship) in inventory
            .relationships
            .iter()
            .take(relationship_count)
            .enumerate()
        {
            if index != 0 {
                output.push(',');
            }
            output.push_str("{\"id\":\"");
            push_json_bounded(&mut output, &relationship.id, 256);
            output.push_str("\",\"type\":\"");
            push_json_bounded(&mut output, &relationship.relationship_type, 256);
            output.push_str("\",\"part_name\":\"");
            push_json_bounded(&mut output, &relationship.part_name, 512);
            output.push_str("\"}");
        }
        output.push(']');
        if relationship_count < inventory.relationships.len() {
            output.push_str(",\"relationships_truncated\":true");
        }
        if let Some(preview) = &inventory.preview {
            output.push_str(",\"preview\":{\"relationship_id\":\"");
            push_json_bounded(&mut output, &preview.relationship_id, 256);
            output.push_str("\",\"mime_type\":\"");
            push_json_bounded(&mut output, &preview.mime_type, 64);
            output.push_str("\"}");
        }
        let required_suffix = if raw_xml.is_some() {
            ",\"raw_xml\":\"\"}".len()
        } else {
            1
        };
        if output.len().saturating_add(required_suffix) <= RAW_REFERENCE_LIMIT {
            if let Some(raw_xml) = raw_xml {
                output.push_str(",\"raw_xml\":\"");
                let suffix_len = "\"}".len();
                let available = RAW_REFERENCE_LIMIT.saturating_sub(output.len() + suffix_len);
                push_json_bounded(&mut output, raw_xml, available);
                output.push('"');
            }
            output.push('}');
            return output;
        }
        if relationship_count == 0 {
            return "{\"metadata_truncated\":true}".to_owned();
        }
        relationship_count -= 1;
    }
}

impl AlternateContentInventory {
    pub fn to_json(&self) -> String {
        let mut branch_count = self.branches.len();
        loop {
            let mut output = String::from("{\"source_identity\":\"");
            push_json_bounded(&mut output, &self.source_identity, 512);
            let _ = write!(
                output,
                "\",\"selected_branch\":{},\"branches\":[",
                self.selected_branch
            );
            for (index, branch) in self.branches.iter().take(branch_count).enumerate() {
                if index != 0 {
                    output.push(',');
                }
                output.push_str("{\"kind\":\"");
                output.push_str(branch.kind);
                output.push_str("\",\"requires\":[");
                for (token_index, token) in branch.requires.iter().enumerate() {
                    if token_index != 0 {
                        output.push(',');
                    }
                    output.push('"');
                    push_json_bounded(&mut output, token, 256);
                    output.push('"');
                }
                let _ = write!(
                    output,
                    "],\"supported\":{},\"raw_xml\":\"",
                    branch.supported
                );
                push_json_bounded(&mut output, &branch.raw_xml, 2_048);
                output.push_str("\"}");
            }
            output.push(']');
            if branch_count < self.branches.len() {
                output.push_str(",\"branches_truncated\":true");
            }
            output.push('}');
            if output.len() <= RAW_REFERENCE_LIMIT {
                return output;
            }
            if branch_count == 0 {
                return "{\"metadata_truncated\":true}".to_owned();
            }
            branch_count -= 1;
        }
    }
}

fn push_json_bounded(output: &mut String, value: &str, limit: usize) {
    let start = output.len();
    let marker = "...[truncated]";
    let full_len = value.chars().map(json_character_len).sum::<usize>();
    if full_len <= limit {
        for character in value.chars() {
            push_json_character(output, character);
        }
        return;
    }
    let content_limit = limit.saturating_sub(marker.len());
    for character in value.chars() {
        let escaped_len = json_character_len(character);
        if output.len().saturating_sub(start) + escaped_len > content_limit {
            break;
        }
        push_json_character(output, character);
    }
    if marker.len() <= limit {
        output.push_str(marker);
    }
}

fn json_character_len(character: char) -> usize {
    match character {
        '"' | '\\' | '\n' | '\r' | '\t' => 2,
        c if c <= '\u{1f}' || matches!(c, '<' | '>' | '&') => 6,
        c => c.len_utf8(),
    }
}

fn push_json_character(output: &mut String, character: char) {
    match character {
        '"' => output.push_str("\\\""),
        '\\' => output.push_str("\\\\"),
        '\n' => output.push_str("\\n"),
        '\r' => output.push_str("\\r"),
        '\t' => output.push_str("\\t"),
        c if c <= '\u{1f}' => {
            let _ = write!(output, "\\u{:04x}", c as u32);
        }
        '<' => output.push_str("\\u003c"),
        '>' => output.push_str("\\u003e"),
        '&' => output.push_str("\\u0026"),
        c => output.push(c),
    }
}
