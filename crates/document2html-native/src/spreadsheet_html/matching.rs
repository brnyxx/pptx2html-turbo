//! Indexed occurrence matching.
//!
//! Rendered text is scanned once and looked up against a hash index of wanted
//! cell values. Cost is `O(tokens * distinct cell word-widths)` with hash
//! lookups rather than the `O(cells * nodes)` scan it replaces, so cell count
//! and document size no longer multiply.

use std::collections::{BTreeSet, HashMap};
use std::ops::Range;

use super::text::{normalized_chars, phrase_text, text_nodes, word_spans};

/// Upper bound on the tokens scanned per document. Matching is linear in the
/// token count, and this keeps a hostile or machine-generated sheet from
/// turning annotation into an unbounded amount of work.
const MAX_MATCH_TOKENS: usize = 1 << 20;

pub(super) struct OccurrenceScan<'a> {
    pub(super) occurrences: HashMap<&'a str, Vec<(usize, usize)>>,
    pub(super) truncated: bool,
    /// Phrase lookups performed. Exposed so tests can assert the matching cost
    /// grows with input size instead of with cells x nodes.
    #[cfg(test)]
    pub(super) probes: usize,
}

/// Walks the body once, skipping tags and raw-text element bodies, and records
/// every whitespace-delimited token span that matches a wanted cell value.
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
    let keys: HashMap<&str, &'a str> = wanted.keys().map(|value| (*value, *value)).collect();
    // Only the word counts some cell value actually has can ever match, so
    // phrase widths outside this set are never even formed.
    let widths: Vec<usize> = keys
        .keys()
        .map(|value| value.split(' ').count())
        .filter(|width| *width > 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    for (start, end) in text_nodes(html, content) {
        if tokens >= MAX_MATCH_TOKENS {
            truncated = true;
            break;
        }
        let decoded = normalized_chars(&html[start..end]);
        let words = word_spans(&decoded);
        tokens = tokens.saturating_add(words.len());
        for width in &widths {
            if *width > words.len() {
                continue;
            }
            for window in words.windows(*width) {
                let first = &window[0];
                let last = &window[*width - 1];
                #[cfg(test)]
                {
                    probes = probes.saturating_add(1);
                }
                let phrase = phrase_text(&decoded, first.0, last.1);
                if let Some(key) = keys.get(phrase.as_str()) {
                    occurrences.entry(*key).or_default().push((
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
