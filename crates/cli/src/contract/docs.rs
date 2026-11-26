//! `docs <query>`: the engine's docs searched (ADR-0153): the Learn pages,
//! the book's snippets, the skill pack, the problem codes' pages and the
//! examples, as this `shuyi` embeds them, at the engine's revision it was
//! built from. It needs no project, like `explain` and `doctor`, and is
//! how an agent with no training on Shuyi finds a page, a skill or a name
//! before guessing one (`shuyi docs play a sound`).
//!
//! The search is lexical, BM25 over chunks ([`index`]), with no network:
//! a chunk is a Learn page's or a skill's section, an example's file, or a
//! problem code's page ([`corpus`]). It also answers the rows of the "how
//! do I" table, `site/data/how-do-i.toml`, whose task phrase the query
//! holds.

pub mod corpus;
pub mod index;

use std::fmt::Write as _;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::codes;
use super::{Answer, Op};
use index::{Index, tokens};

/// The hits `docs` answers when the input names no limit.
pub const DEFAULT_LIMIT: u32 = 8;
/// The most hits `docs` answers.
pub const MAX_LIMIT: u32 = 50;
/// The longest an excerpt is, in bytes, before its `…`.
pub const EXCERPT: usize = 400;

/// What a hit is from.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
// `#[non_exhaustive]` (ADR-0039, kind 6): new sources join the index (the
// API's listing, ticket 3413), so a `match` outside this crate keeps a `_`
// arm.
#[non_exhaustive]
pub enum HitKind {
    /// A Learn page's section (`site/content/learn`).
    Learn,
    /// A skill's section or file (`skills/`).
    Skill,
    /// An example's file (`examples/`).
    Example,
    /// A problem code's page (`crates/cli/errors`).
    Code,
}

impl HitKind {
    /// Every kind, in order.
    pub const ALL: &'static [HitKind] = &[
        HitKind::Learn,
        HitKind::Skill,
        HitKind::Example,
        HitKind::Code,
    ];

    /// Its name, as the input and the answer write it.
    pub fn name(self) -> &'static str {
        match self {
            HitKind::Learn => "learn",
            HitKind::Skill => "skill",
            HitKind::Example => "example",
            HitKind::Code => "code",
        }
    }
}

/// What `docs` searches for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An input: built with `default()` and its fields assigned, it may grow
// (ADR-0039).
#[non_exhaustive]
pub struct DocsInput {
    /// What to find, in words or names (`play a sound`, `add_sprite`,
    /// `SH2003`).
    pub query: String,
    /// The most hits to answer: 8 if none, at least 1 and at most 50.
    #[serde(default)]
    pub limit: Option<u32>,
    /// Only hits of these kinds; every kind if none.
    #[serde(default)]
    pub kinds: Option<Vec<HitKind>>,
}

impl DocsInput {
    /// A search for `query`, with the default limit, of every kind.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Self::default()
        }
    }
}

/// A chunk of the docs that the query matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Hit {
    /// What it is from.
    pub kind: HitKind,
    /// Its page's or skill's title, and its section's heading.
    pub title: String,
    /// Its source in the engine's repository
    /// (`site/content/learn/book/actions.md`).
    pub path: String,
    /// Its section's anchor in the page (`naming-a-key-in-text`); none at a
    /// page's start.
    pub anchor: Option<String>,
    /// The Markdown twin of its Learn page on the engine's site; none for
    /// other kinds.
    pub url: Option<String>,
    /// How well it matched: BM25 in thousandths. Higher is better; only
    /// comparable within one answer.
    pub score: u64,
    /// Its text from where it first holds the query's rarest term, at most
    /// 400 bytes, with `…` where it was cut.
    pub excerpt: String,
}

/// A row of the "how do I" table whose task the query asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HowRow {
    /// The row's task phrase the query holds (`play a sound`).
    pub ask: String,
    /// Where it is answered, as the table writes it
    /// (`learn/book/games-and-screens.md#the-context`, `skill/shuyi-input`,
    /// `example/hello-grid/src/lib.rs`).
    pub go: String,
    /// What that is.
    pub kind: HitKind,
    /// Its title.
    pub title: String,
    /// Its source in the engine's repository.
    pub path: String,
    /// Its section's anchor, if it names one.
    pub anchor: Option<String>,
    /// The Markdown twin of its Learn page; none for other kinds.
    pub url: Option<String>,
}

/// What `docs` answers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocsOutput {
    /// The "how do I" rows the query asked, in the table's order.
    pub how: Vec<HowRow>,
    /// The chunks it matched, best first, then by path and position.
    pub hits: Vec<Hit>,
}

/// The "how do I" table, `site/data/how-do-i.toml`.
const HOW_DO_I: &str = include_str!("../../../../site/data/how-do-i.toml");

/// A row of the "how do I" table, as written.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    /// Its task phrases.
    pub ask: Vec<String>,
    /// Its target: `learn/<path>[#anchor]`, `skill/<name>` or
    /// `example/<path>`.
    pub go: String,
}

/// The "how do I" table's file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Table {
    row: Vec<Row>,
}

/// The rows of the "how do I" table `text`, or why it can't be read.
pub fn rows_of(text: &str) -> Result<Vec<Row>, String> {
    toml_edit::de::from_str::<Table>(text)
        .map(|table| table.row)
        .map_err(|e| format!("site/data/how-do-i.toml: {e}"))
}

/// The embedded "how do I" table's rows; none if it can't be read (its
/// test fails then).
pub fn rows() -> Vec<Row> {
    rows_of(HOW_DO_I).unwrap_or_default()
}

/// The chunk `go`, a row's target, names in `chunks`: a Learn page's at
/// its anchor (its first without one), a skill's `SKILL.md`'s first, an
/// example's file.
pub fn target<'a>(go: &str, chunks: &'a [corpus::Chunk]) -> Option<&'a corpus::Chunk> {
    let (place, anchor) = match go.split_once('#') {
        Some((place, anchor)) => (place, Some(anchor)),
        None => (go, None),
    };
    let path = if let Some(page) = place.strip_prefix("learn/") {
        format!("{}/{page}", corpus::LEARN_ROOT)
    } else if let Some(skill) = place.strip_prefix("skill/") {
        format!("{}/{skill}/{}", corpus::SKILLS_ROOT, crate::skills::FILE)
    } else {
        let file = place.strip_prefix("example/")?;
        format!("{}/{file}", corpus::EXAMPLES_ROOT)
    };
    chunks
        .iter()
        .find(|c| c.path == path && (anchor.is_none() || c.anchor.as_deref() == anchor))
}

/// The rows of `rows` whose task the query's `terms` hold, with where
/// they go in `chunks`: a row matches when every term of one of its
/// phrases is in the query.
fn how(terms: &[String], rows: &[Row], chunks: &[corpus::Chunk]) -> Vec<HowRow> {
    let mut out = Vec::new();
    for row in rows {
        let asked = row.ask.iter().find(|phrase| {
            let wanted = tokens(phrase);
            !wanted.is_empty() && wanted.iter().all(|t| terms.contains(t))
        });
        let (Some(ask), Some(chunk)) = (asked, target(&row.go, chunks)) else {
            continue;
        };
        out.push(HowRow {
            ask: ask.clone(),
            go: row.go.clone(),
            kind: chunk.kind,
            title: chunk.title.clone(),
            path: chunk.path.clone(),
            anchor: chunk.anchor.clone(),
            url: chunk.url.clone(),
        });
    }
    out
}

/// `text` from byte `from`, at most [`EXCERPT`] bytes cut at a character's
/// boundary, with `…` before it if it doesn't start at 0 and after it if
/// it was cut.
pub fn excerpt(text: &str, from: usize) -> String {
    let mut start = from.min(text.len());
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    let rest = &text[start..];
    let mut end = rest.len().min(EXCERPT);
    while !rest.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.push_str(rest[..end].trim_end());
    if end < rest.len() {
        out.push('…');
    }
    out
}

/// Where the excerpt of `text` starts for `terms`, the query's: at the
/// start, unless the line where it first holds its rarest term (by
/// `idf`) is past the first [`EXCERPT`] bytes.
fn excerpt_start(text: &str, terms: &[String], idf: impl Fn(&str) -> f64) -> usize {
    let mut at = 0;
    let lines: Vec<(usize, Vec<String>)> = text
        .split_inclusive('\n')
        .map(|line| {
            let start = at;
            at += line.len();
            (start, tokens(line))
        })
        .collect();
    let mut held: Vec<&String> = terms
        .iter()
        .filter(|t| lines.iter().any(|(_, line)| line.contains(t)))
        .collect();
    held.sort_by(|a, b| idf(b).total_cmp(&idf(a)).then_with(|| a.cmp(b)));
    let Some(rarest) = held.first() else {
        return 0;
    };
    let start = lines
        .iter()
        .find(|(_, line)| line.contains(rarest))
        .map_or(0, |(start, _)| *start);
    if start + rarest.len() <= EXCERPT {
        0
    } else {
        start
    }
}

/// What `docs` says when nothing matched.
pub const NOTHING: &str = "nothing matched; try fewer words, or `shuyi docs <a type's name>`";

/// `docs`: the chunks of the embedded corpus that `input`'s query matches,
/// and the "how do I" rows it asked.
pub fn docs(input: &DocsInput) -> Answer<DocsOutput> {
    search(input, Index::embedded(), &rows())
}

/// `docs` on `index`, with the "how do I" table's `rows`.
pub fn search(input: &DocsInput, index: &Index, rows: &[Row]) -> Answer<DocsOutput> {
    let terms = tokens(&input.query);
    if terms.is_empty() {
        let error = if input.query.trim().is_empty() {
            "no query: say what to find".to_owned()
        } else {
            format!("`{}` holds only common words", input.query.trim())
        };
        return Answer::failed(Op::Docs, codes::WRONG_ARGUMENTS, error)
            .with_help("`shuyi docs <words or a name>`, as `shuyi docs play a sound`");
    }
    let limit = input.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as usize;
    let wanted = |kind: HitKind| input.kinds.as_ref().is_none_or(|k| k.contains(&kind));
    let hits: Vec<Hit> = index
        .search(&terms)
        .into_iter()
        .filter(|found| wanted(index.chunks[found.chunk].kind))
        .take(limit)
        .map(|found| {
            let chunk = &index.chunks[found.chunk];
            let from = excerpt_start(&chunk.text, &terms, |t| index.idf(t));
            Hit {
                kind: chunk.kind,
                title: chunk.title.clone(),
                path: chunk.path.clone(),
                anchor: chunk.anchor.clone(),
                url: chunk.url.clone(),
                score: found.score,
                excerpt: excerpt(&chunk.text, from),
            }
        })
        .collect();
    let how: Vec<HowRow> = how(&terms, rows, &index.chunks)
        .into_iter()
        .filter(|row| wanted(row.kind))
        .collect();
    let mut limits = Vec::new();
    if hits.is_empty() && how.is_empty() {
        limits.push(NOTHING.to_owned());
    }
    Answer::new(Op::Docs, true, DocsOutput { how, hits }, limits)
}

/// Reads `docs`'s arguments (not `--json`): the query's words, then
/// `--limit N` and `--kind K` (again, or `K,K`) anywhere.
pub fn parse_args(args: &[String]) -> Result<DocsInput, String> {
    let mut input = DocsInput::default();
    let mut words = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--limit" => {
                let n = rest.next().ok_or("--limit takes a number")?;
                let n = n
                    .parse::<u32>()
                    .map_err(|_| format!("--limit {n}: a number from 0"))?;
                input.limit = Some(n);
            }
            "--kind" => {
                let names = rest.next().ok_or("--kind takes a kind")?;
                for name in names.split(',') {
                    let kind = HitKind::ALL
                        .iter()
                        .copied()
                        .find(|k| k.name() == name)
                        .ok_or_else(|| {
                            format!("--kind {name}: one of learn, skill, example, code")
                        })?;
                    input.kinds.get_or_insert_with(Vec::new).push(kind);
                }
            }
            flag if flag.starts_with("--") => return Err(format!("no flag {flag}")),
            word => words.push(word),
        }
    }
    if args.is_empty() {
        return Err("a query: what to find".to_owned());
    }
    input.query = words.join(" ");
    Ok(input)
}

/// `docs`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!(
        "usage: {command} docs <words…> [--limit N] [--kind learn|skill|example|code] [--json]\n\n\
         Searches the engine's docs as this {command} embeds them: the Learn pages, the skills,\n\
         the examples and the problem codes' pages. Best first; at most N (8, up to 50)."
    )
}

/// What `docs` found, for people: each "how do I" row, then each hit with
/// its excerpt.
pub fn text(output: &DocsOutput) -> String {
    let mut out = String::new();
    for row in &output.how {
        let _ = writeln!(out, "how do I {}: {}", row.ask, row.go);
    }
    if !output.how.is_empty() && !output.hits.is_empty() {
        out.push('\n');
    }
    for hit in &output.hits {
        let place = hit
            .anchor
            .as_ref()
            .map_or_else(|| hit.path.clone(), |a| format!("{}#{a}", hit.path));
        // Writing to a `String` can't fail.
        let _ = writeln!(out, "{}  {}  [{}]", hit.score, hit.title, hit.kind.name());
        let _ = writeln!(out, "  {place}");
        if let Some(url) = &hit.url {
            let _ = writeln!(out, "  {url}");
        }
        for line in hit.excerpt.lines().filter(|l| !l.trim().is_empty()).take(4) {
            let _ = writeln!(out, "    {line}");
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests;
