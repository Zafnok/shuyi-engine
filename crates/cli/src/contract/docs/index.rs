//! The index `docs` searches (ADR-0153): BM25 over the corpus's chunks,
//! built once per process. A chunk's title terms count twice. Scores are
//! floats, as a tool's may be (no simulation reads them), and are
//! answered as integers, thousandths rounded, with ties broken by path,
//! then position, so the same query answers the same on every machine.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use super::corpus::{self, Chunk};

/// BM25's term-frequency saturation.
const K1: f64 = 1.2;
/// BM25's length normalization.
const B: f64 = 0.75;

/// The words a query or a chunk drops: too common to rank by.
pub const STOP: [&str; 8] = ["the", "a", "of", "to", "is", "and", "in", "it"];

/// The terms of `text`: each word (letters, digits and `_`, any script)
/// lowercased, and the parts of a `snake_case` or `CamelCase` word after
/// it; the stop words left out.
pub fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in text.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        let word = word.trim_matches('_');
        if word.is_empty() {
            continue;
        }
        let parts = parts(word);
        let whole = word.to_ascii_lowercase();
        let split = parts.len() > 1;
        push(&mut out, whole);
        if split {
            for part in parts {
                push(&mut out, part.to_ascii_lowercase());
            }
        }
    }
    out
}

/// `term` added to `out` unless it is a stop word.
fn push(out: &mut Vec<String>, term: String) {
    if !STOP.contains(&term.as_str()) {
        out.push(term);
    }
}

/// The parts of `word`: split at `_`, and at a case change (`addSprite`,
/// `HTTPServer` is `HTTP` and `Server`).
fn parts(word: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for piece in word.split('_').filter(|p| !p.is_empty()) {
        let chars: Vec<(usize, char)> = piece.char_indices().collect();
        let mut start = 0;
        for i in 1..chars.len() {
            let (at, c) = chars[i];
            let prev = chars[i - 1].1;
            let next = chars.get(i + 1).map(|(_, c)| *c);
            let lower_to_upper = (prev.is_lowercase() || prev.is_ascii_digit()) && c.is_uppercase();
            let acronym_end =
                prev.is_uppercase() && c.is_uppercase() && next.is_some_and(char::is_lowercase);
            if lower_to_upper || acronym_end {
                out.push(&piece[start..at]);
                start = at;
            }
        }
        out.push(&piece[start..]);
    }
    out
}

/// The index: the chunks, each one's length in terms, and each term's
/// chunks with its count there.
pub struct Index {
    /// The chunks, in the corpus's order.
    pub chunks: Vec<Chunk>,
    lengths: Vec<u32>,
    average: f64,
    postings: BTreeMap<String, Vec<(u32, u32)>>,
}

/// A chunk found: its place in [`Index::chunks`] and its score in
/// thousandths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Found {
    /// Its place in the index's chunks.
    pub chunk: usize,
    /// Its score, in thousandths, rounded.
    pub score: u64,
}

impl Index {
    /// The index of `chunks`.
    pub fn new(chunks: Vec<Chunk>) -> Self {
        let mut lengths = Vec::with_capacity(chunks.len());
        let mut postings: BTreeMap<String, Vec<(u32, u32)>> = BTreeMap::new();
        for (at, chunk) in chunks.iter().enumerate() {
            let at = u32::try_from(at).unwrap_or(u32::MAX);
            let mut counts: BTreeMap<String, u32> = BTreeMap::new();
            let title = tokens(&chunk.title);
            let body = tokens(&chunk.text);
            for term in title.iter().chain(&title).chain(&body) {
                *counts.entry(term.clone()).or_default() += 1;
            }
            lengths.push(counts.values().sum());
            for (term, count) in counts {
                postings.entry(term).or_default().push((at, count));
            }
        }
        let total: f64 = lengths.iter().copied().map(f64::from).sum();
        let average = total / f64::from(count_of(lengths.len()));
        Self {
            chunks,
            lengths,
            average: average.max(1.0),
            postings,
        }
    }

    /// The index of the embedded corpus, built on first use.
    pub fn embedded() -> &'static Index {
        static INDEX: OnceLock<Index> = OnceLock::new();
        INDEX.get_or_init(|| Index::new(corpus::chunks()))
    }

    /// How rare `term` is: BM25's inverse document frequency, 0 for a term
    /// no chunk has.
    pub fn idf(&self, term: &str) -> f64 {
        let Some(list) = self.postings.get(term) else {
            return 0.0;
        };
        let n = f64::from(count_of(self.chunks.len()));
        let has = f64::from(count_of(list.len()));
        (1.0 + (n - has + 0.5) / (has + 0.5)).ln()
    }

    /// The chunks that hold a term of `terms`, by score, highest first,
    /// then by path and position.
    pub fn search(&self, terms: &[String]) -> Vec<Found> {
        let mut unique: Vec<&String> = terms.iter().collect();
        unique.sort();
        unique.dedup();
        let mut scores: BTreeMap<u32, f64> = BTreeMap::new();
        for term in unique {
            let Some(list) = self.postings.get(term.as_str()) else {
                continue;
            };
            let idf = self.idf(term);
            for &(chunk, count) in list {
                let length = f64::from(self.lengths[chunk as usize]);
                let tf = f64::from(count);
                let part =
                    idf * tf * (K1 + 1.0) / (tf + K1 * (1.0 - B + B * length / self.average));
                *scores.entry(chunk).or_default() += part;
            }
        }
        let mut found: Vec<Found> = scores
            .into_iter()
            .map(|(chunk, score)| Found {
                chunk: chunk as usize,
                score: thousandths(score),
            })
            .collect();
        found.sort_by(|a, b| {
            let (x, y) = (&self.chunks[a.chunk], &self.chunks[b.chunk]);
            b.score
                .cmp(&a.score)
                .then_with(|| x.path.cmp(&y.path))
                .then_with(|| x.position.cmp(&y.position))
        });
        found
    }
}

/// `n`, a count of chunks, as a `u32`; at least 1, so it divides.
fn count_of(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX).max(1)
}

/// `score` in thousandths, rounded: a non-negative float, far below 2^53.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn thousandths(score: f64) -> u64 {
    (score * 1000.0).round().max(0.0) as u64
}

#[cfg(test)]
mod tests;
