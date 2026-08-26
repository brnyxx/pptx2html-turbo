//! Indexed occurrence matching.
//!
//! Cell values are compiled into a token-keyed trie, then rendered text is
//! scanned once. From each token position the walk advances only while the
//! consumed tokens remain a live prefix of some cell value, so the operation
//! count is bounded by `tokens * max_live_prefix_depth` and is independent of
//! how many distinct phrase widths the workbook uses.

use std::collections::HashMap;
use std::ops::Range;

use super::text::{normalized_chars, phrase_text, text_nodes, word_spans};

/// Upper bound on the tokens scanned per document. Matching is linear in the
/// token count, and this keeps a hostile or machine-generated sheet from
/// turning annotation into an unbounded amount of work.
const MAX_MATCH_TOKENS: usize = 1 << 20;

/// Trie over whitespace-delimited tokens of the wanted cell values.
#[derive(Default)]
struct PhraseTrie<'a> {
    nodes: Vec<TrieNode<'a>>,
}

#[derive(Default)]
struct TrieNode<'a> {
    children: HashMap<&'a str, usize>,
    /// Set when this node terminates a complete cell value.
    value: Option<&'a str>,
}

impl<'a> PhraseTrie<'a> {
    fn build(values: impl Iterator<Item = &'a str>) -> Self {
        let mut trie = Self {
            nodes: vec![TrieNode::default()],
        };
        for value in values {
            let mut current = 0usize;
            for token in value.split(' ').filter(|token| !token.is_empty()) {
                current = match trie.nodes[current].children.get(token) {
                    Some(next) => *next,
                    None => {
                        let next = trie.nodes.len();
                        trie.nodes.push(TrieNode::default());
                        let _ = trie.nodes[current].children.insert(token, next);
                        next
                    }
                };
            }
            if current != 0 {
                trie.nodes[current].value = Some(value);
            }
        }
        trie
    }

    fn child(&self, node: usize, token: &str) -> Option<usize> {
        self.nodes[node].children.get(token).copied()
    }

    fn value(&self, node: usize) -> Option<&'a str> {
        self.nodes[node].value
    }

    fn is_empty(&self) -> bool {
        self.nodes.len() == 1
    }
}

pub(super) struct OccurrenceScan<'a> {
    pub(super) occurrences: HashMap<&'a str, Vec<(usize, usize)>>,
    pub(super) truncated: bool,
    /// Trie transitions attempted. Exposed so tests can pin the cost model.
    #[cfg(test)]
    pub(super) probes: usize,
}

/// Walks the body once, skipping tags and raw-text element bodies, and records
/// every token span that matches a wanted cell value.
pub(super) fn scan_occurrences<'a>(
    html: &str,
    content: &Range<usize>,
    wanted: &HashMap<&'a str, usize>,
) -> OccurrenceScan<'a> {
    let mut occurrences: HashMap<&'a str, Vec<(usize, usize)>> = HashMap::new();
    let mut tokens = 0usize;
    #[cfg(test)]
    let mut probes = 0usize;
    let mut truncated = false;
    let trie = PhraseTrie::build(wanted.keys().copied());
    if trie.is_empty() {
        return OccurrenceScan {
            occurrences,
            truncated,
            #[cfg(test)]
            probes,
        };
    }

    for (start, end) in text_nodes(html, content) {
        if tokens >= MAX_MATCH_TOKENS {
            truncated = true;
            break;
        }
        let decoded = normalized_chars(&html[start..end]);
        let words = word_spans(&decoded);
        tokens = tokens.saturating_add(words.len());
        for (index, first) in words.iter().enumerate() {
            let mut node = 0usize;
            // Advance only while the consumed tokens stay a live prefix, so a
            // token that matches nothing costs a single failed transition.
            for last in &words[index..] {
                let token = phrase_text(&decoded, last.0, last.1);
                #[cfg(test)]
                {
                    probes = probes.saturating_add(1);
                }
                let Some(next) = trie.child(node, token.as_str()) else {
                    break;
                };
                node = next;
                if let Some(value) = trie.value(node) {
                    occurrences.entry(value).or_default().push((
                        start + decoded[first.0].start,
                        start + decoded[last.1 - 1].end,
                    ));
                }
            }
        }
    }
    OccurrenceScan {
        occurrences,
        truncated,
        #[cfg(test)]
        probes,
    }
}

pub(super) const fn max_match_tokens() -> usize {
    MAX_MATCH_TOKENS
}
