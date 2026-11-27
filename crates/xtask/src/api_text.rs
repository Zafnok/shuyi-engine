//! `cargo xtask api-text`: the engine's public API as plain text, a file per
//! crate, and the other engines' words held to it (ticket 3413).
//!
//! `--out <dir>` writes `<dir>/<crate>.txt` for each crate of
//! `site/data/api.toml`: a line per item of its `cargo-public-api` listing
//! ([`api::listing`], with the features [`api::API_CRATES`] checks it with),
//! and after an item's line its first doc sentence, its doc aliases and its
//! deprecation, read from rustdoc's JSON ([`rustdoc_index`]). A line with
//! no item in the index (an `impl`, a trait's method) is written alone.
//! pages.yml writes them into the site's `api/`, beside rustdoc's HTML, and
//! `llms.txt` links each.
//!
//! `--check` writes nothing, and fails on what [`check`] finds between
//! `site/data/engine-terms.toml` (appendix A's table and the names agents
//! guess) and the crates: a path that names no public item, an alias the
//! data gives an item that it lacks, a `doc(alias)` the data lacks, and a
//! "none yet" row whose ticket is done ([`shape`], which the tests hold the
//! data to as well). CI's `api` job runs it, as it has the nightly.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::api::{self, Runner};
use crate::rustdoc_index::{self, Index};
use crate::site_data::ticket_ids;

#[cfg(test)]
mod tests;

/// The table of other engines' words, from the repository's root.
pub const TERMS: &str = "site/data/engine-terms.toml";
/// The crates the site documents, from the repository's root.
const API_TOML: &str = "site/data/api.toml";
/// What a row Shuyi has no counterpart for starts with.
const NONE_YET: &str = "none yet";

/// What `cargo xtask api-text` was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Options {
    /// `--out <dir>`: write the files there.
    Out(PathBuf),
    /// `--check`: check the terms against the crates, and write nothing.
    Check,
}

impl Options {
    /// `--out <dir>` or `--check`; `None` for anything else.
    pub fn parse(args: &[String]) -> Option<Self> {
        match args {
            [flag] if flag == "--check" => Some(Self::Check),
            [flag, dir] if flag == "--out" => Some(Self::Out(PathBuf::from(dir))),
            _ => None,
        }
    }
}

/// `engine-terms.toml`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Terms {
    #[serde(default)]
    pub term: Vec<Term>,
    #[serde(default)]
    pub guess: Vec<Guess>,
}

/// A row of appendix A's table.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Term {
    /// Shuyi's words, with the chapters' links.
    pub shuyi: String,
    /// The items they name, each at a path of its own crate.
    #[serde(default)]
    pub paths: Vec<String>,
    /// The doc aliases the first path's item carries.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// The open ticket that plans a "none yet" row.
    pub ticket: Option<String>,
    /// `false` for a "none yet" row no ticket plans.
    pub planned: Option<bool>,
    pub godot: String,
    pub unity: String,
    pub bevy: String,
    pub gamemaker: String,
    pub pico8: String,
}

impl Term {
    /// The engines' words, in the table's order, with their columns' names.
    pub fn engines(&self) -> [(&'static str, &str); 5] {
        [
            ("Godot", &self.godot),
            ("Unity", &self.unity),
            ("Bevy", &self.bevy),
            ("GameMaker", &self.gamemaker),
            ("PICO-8", &self.pico8),
        ]
    }

    /// Whether Shuyi has no counterpart yet.
    pub fn none_yet(&self) -> bool {
        self.shuyi.starts_with(NONE_YET)
    }
}

/// A name an agent guesses, and the item it is an alias of.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Guess {
    pub name: String,
    pub path: String,
}

/// The terms in `text`, `engine-terms.toml`'s.
///
/// # Errors
///
/// It doesn't parse, or has a field it doesn't know.
pub fn read_terms(text: &str) -> Result<Terms, String> {
    toml::from_str(text).map_err(|e| format!("{TERMS}: {e}"))
}

/// Whether `word` can be a doc alias here: an identifier.
fn is_identifier(word: &str) -> bool {
    let mut chars = word.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// What is wrong with the terms on their own, and against the tickets
/// `open` and `done`: each problem a line.
pub fn shape(terms: &Terms, open: &HashSet<String>, done: &HashSet<String>) -> Vec<String> {
    let mut errors = Vec::new();
    for term in &terms.term {
        let row = format!("{TERMS}: the row \"{}\"", term.shuyi);
        if term.shuyi.trim().is_empty() {
            errors.push(format!("{TERMS}: a row has no `shuyi` words"));
        }
        for (engine, words) in term.engines() {
            if words.trim().is_empty() {
                errors.push(format!("{row} has no {engine} words: write \"—\" for none"));
            }
        }
        for alias in &term.aliases {
            if !is_identifier(alias) {
                errors.push(format!(
                    "{row}: the alias `{alias}` isn't an identifier (rustdoc refuses a space)"
                ));
            }
        }
        if !term.aliases.is_empty() && term.paths.is_empty() {
            errors.push(format!("{row} has aliases and no path to carry them"));
        }
        if term.none_yet() {
            if !term.paths.is_empty() {
                errors.push(format!("{row} says none yet, and names a path"));
            }
            match (&term.ticket, term.planned) {
                (Some(id), None) if done.contains(id) => errors.push(format!(
                    "{row} says none yet, and its ticket {id} is done: the row is out of date"
                )),
                (Some(id), None) if !open.contains(id) => errors.push(format!(
                    "{row} names the ticket {id}, which isn't in tickets/open/"
                )),
                (Some(_), None) | (None, Some(false)) => {}
                _ => errors.push(format!(
                    "{row} says none yet: name the open ticket that plans it (`ticket`), or say \
                     `planned = false`"
                )),
            }
        } else if term.ticket.is_some() || term.planned.is_some() {
            errors.push(format!(
                "{row} has a counterpart: `ticket` and `planned` are for a \"none yet\" row"
            ));
        }
    }
    let mut seen = HashSet::new();
    for guess in &terms.guess {
        if !is_identifier(&guess.name) {
            errors.push(format!(
                "{TERMS}: the guess `{}` isn't an identifier",
                guess.name
            ));
        }
        if !seen.insert((&guess.name, &guess.path)) {
            errors.push(format!(
                "{TERMS}: the guess `{}` of `{}` is there twice",
                guess.name, guess.path
            ));
        }
    }
    errors
}

/// The aliases the terms give each path: a row's to its first path, each
/// guess's to its own.
pub fn wanted_aliases(terms: &Terms) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut out: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for term in &terms.term {
        if let Some(first) = term.paths.first() {
            out.entry(first)
                .or_default()
                .extend(term.aliases.iter().map(String::as_str));
        }
    }
    for guess in &terms.guess {
        out.entry(&guess.path).or_default().insert(&guess.name);
    }
    out
}

/// Every path the terms name.
fn named_paths(terms: &Terms) -> BTreeSet<&str> {
    terms
        .term
        .iter()
        .flat_map(|term| term.paths.iter())
        .chain(terms.guess.iter().map(|guess| &guess.path))
        .map(String::as_str)
        .collect()
}

/// What `--check` finds between the terms and the crates' `indexes`, and
/// the tickets `open` and `done` ([`shape`] too).
pub fn check(
    terms: &Terms,
    indexes: &[Index],
    open: &HashSet<String>,
    done: &HashSet<String>,
) -> Vec<String> {
    let mut errors = shape(terms, open, done);
    let index_of = |path: &str| {
        let krate = path.split("::").next().unwrap_or_default();
        indexes.iter().find(|index| index.krate == krate)
    };
    for path in named_paths(terms) {
        match index_of(path) {
            None => errors.push(format!("{TERMS}: `{path}` names no crate of {API_TOML}")),
            Some(index) if index.get(path).is_none() => errors.push(format!(
                "{TERMS}: `{path}` names no public item of {}",
                index.krate
            )),
            Some(_) => {}
        }
    }
    let wanted = wanted_aliases(terms);
    for (path, aliases) in &wanted {
        let Some(item) = index_of(path).and_then(|index| index.get(path)) else {
            continue;
        };
        for alias in aliases {
            if !item.aliases.iter().any(|have| have == alias) {
                errors.push(format!(
                    "`{path}` lacks `#[doc(alias = \"{alias}\")]`, which {TERMS} gives it"
                ));
            }
        }
    }
    for index in indexes {
        for item in index.items() {
            let given: BTreeSet<&str> = item
                .paths
                .iter()
                .filter_map(|path| wanted.get(path.as_str()))
                .flatten()
                .copied()
                .collect();
            for alias in &item.aliases {
                if !given.contains(alias.as_str()) {
                    errors.push(format!(
                        "`{}` has `#[doc(alias = \"{alias}\")]`, which {TERMS} doesn't name: add it \
                         to a row's `aliases` or as a `[[guess]]`",
                        item.paths.first().map_or("?", String::as_str)
                    ));
                }
            }
        }
    }
    errors
}

/// The item's path a `cargo-public-api` listing line names, its generics
/// left out (`shuyi_ui::input::frame::FrameInput::held`); none for a line
/// that isn't an item's (`impl …`).
pub fn listed_path(line: &str) -> Option<String> {
    let mut rest = line.trim_start();
    while rest.starts_with("#[") {
        rest = rest[rest.find(']')? + 1..].trim_start();
    }
    rest = rest.strip_prefix("pub ")?;
    const WORDS: [&str; 15] = [
        "const ",
        "unsafe ",
        "async ",
        "extern \"C\" ",
        "fn ",
        "struct ",
        "enum ",
        "union ",
        "trait ",
        "type ",
        "mod ",
        "static ",
        "macro ",
        "use ",
        "mut ",
    ];
    while let Some(word) = WORDS.iter().find(|word| rest.starts_with(**word)) {
        rest = &rest[word.len()..];
    }
    let mut path = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '<' => {
                let mut depth = 1;
                for c in chars.by_ref() {
                    match c {
                        '<' => depth += 1,
                        '>' => depth -= 1,
                        _ => {}
                    }
                    if depth == 0 {
                        break;
                    }
                }
            }
            ':' if chars.peek() == Some(&':') => {
                chars.next();
                path.push_str("::");
            }
            c if c.is_alphanumeric() || c == '_' => path.push(c),
            _ => break,
        }
    }
    (path.contains("::")).then_some(path)
}

/// `text` with its Markdown links as their words: `[words](target)`,
/// `[words][name]` and `[words]` become `words`.
fn unlinked(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else {
            out.push_str(&rest[open..]);
            return out;
        };
        out.push_str(&after[..close]);
        rest = &after[close + 1..];
        let end = match rest.chars().next() {
            Some('(') => rest.find(')'),
            Some('[') => rest.find(']'),
            _ => None,
        };
        if let Some(end) = end {
            rest = &rest[end + 1..];
        }
    }
    out.push_str(rest);
    out
}

/// The first sentence of `docs`: its first paragraph up to a `.`, `?` or
/// `!` that ends a sentence outside code, its links as their words.
pub fn first_sentence(docs: &str) -> String {
    let mut paragraph = String::new();
    for line in docs.lines().map(str::trim) {
        if line.is_empty() || line.starts_with("```") || line.starts_with('#') {
            if paragraph.is_empty() && line.is_empty() {
                continue;
            }
            break;
        }
        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(line);
    }
    let plain = unlinked(&paragraph);
    let mut code = false;
    let mut chars = plain.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            '`' => code = !code,
            '.' | '?' | '!' if !code => {
                if chars.peek().is_none_or(|(_, next)| next.is_whitespace()) {
                    return plain[..=at].to_owned();
                }
            }
            _ => {}
        }
    }
    plain
}

/// The auto traits whose `impl` lines the text leaves out: every type has
/// them or not by what it holds, and they would be half of each file.
const AUTO_TRAITS: [&str; 7] = [
    "core::marker::Freeze",
    "core::marker::Send",
    "core::marker::Sync",
    "core::marker::Unpin",
    "core::marker::UnsafeUnpin",
    "core::panic::unwind_safe::RefUnwindSafe",
    "core::panic::unwind_safe::UnwindSafe",
];

/// Whether `line` is the `impl` of an auto trait (`impl core::marker::Send
/// for …`, or `impl !core::marker::Send for …`).
fn is_auto_impl(line: &str) -> bool {
    line.strip_prefix("impl")
        .and_then(|rest| rest.split_once(" for "))
        .is_some_and(|(head, _)| {
            let name = head.rsplit(' ').next().unwrap_or_default();
            AUTO_TRAITS.contains(&name.trim_start_matches('!'))
        })
}

/// The text file of the crate `lib`: a header, then each line of its
/// `listing` once but an auto trait's `impl`, an item's line with what
/// `index` knows of it.
pub fn text(lib: &str, description: &str, listing: &[String], index: &Index) -> String {
    let mut out = format!(
        "# {lib}: {description}\n#\n\
         # Its public API, an item a line, as `cargo public-api` lists it. After ` // `,\n\
         # an item's first doc sentence, its doc aliases (the names other engines use)\n\
         # and its deprecation. Made by `cargo xtask api-text` (ticket 3413).\n\n"
    );
    let mut seen = HashSet::new();
    for line in listing {
        if is_auto_impl(line) || !seen.insert(line) {
            continue;
        }
        out.push_str(line);
        let item = listed_path(line).and_then(|path| index.get(&path));
        if let Some(item) = item {
            let mut notes = Vec::new();
            let sentence = first_sentence(&item.docs);
            if !sentence.is_empty() {
                notes.push(sentence);
            }
            if !item.aliases.is_empty() {
                notes.push(format!("aliases: {}", item.aliases.join(", ")));
            }
            match item.deprecated.as_deref() {
                Some("") => notes.push("deprecated".to_owned()),
                Some(note) => notes.push(format!("deprecated: {note}")),
                None => {}
            }
            for note in notes {
                let _ = write!(out, " // {note}");
            }
        }
        out.push('\n');
    }
    out
}

/// A crate of `api.toml`: rustdoc's folder name and its words.
#[derive(Debug, Deserialize)]
struct SiteCrate {
    name: String,
    text: String,
}

/// `api.toml`'s crates.
#[derive(Debug, Deserialize)]
struct SiteCrates {
    #[serde(rename = "crate")]
    crates: Vec<SiteCrate>,
}

/// The crates of `api.toml`, each with its entry of [`api::API_CRATES`].
fn site_crates(root: &Path) -> Result<Vec<(SiteCrate, &'static api::ApiCrate)>, String> {
    let path = root.join(API_TOML);
    let text = fs::read_to_string(&path).map_err(|e| format!("{API_TOML}: {e}"))?;
    let crates: SiteCrates = toml::from_str(&text).map_err(|e| format!("{API_TOML}: {e}"))?;
    crates
        .crates
        .into_iter()
        .map(|krate| {
            let entry = api::API_CRATES
                .iter()
                .find(|entry| entry.name.replace('-', "_") == krate.name)
                .ok_or_else(|| {
                    format!(
                        "{API_TOML}: `{}` is no crate of api::API_CRATES",
                        krate.name
                    )
                })?;
            Ok((krate, entry))
        })
        .collect()
}

/// Runs `options` on the repository at `root`.
///
/// # Errors
///
/// Each problem found, or why it couldn't run.
pub fn run(root: &Path, options: &Options, runner: &dyn Runner) -> Result<(), Vec<String>> {
    let one = |e: String| vec![e];
    let crates = site_crates(root).map_err(one)?;
    let declared = api::declared_features(root, runner).map_err(one)?;
    let target_dir = root.join("target");
    let mut indexes = Vec::new();
    let mut files = Vec::new();
    for (krate, entry) in &crates {
        let features = declared
            .get(entry.name)
            .ok_or_else(|| vec![format!("no package `{}` in the workspace", entry.name)])?;
        let listing = match options {
            Options::Out(_) => {
                api::listing(root, entry, features, &target_dir, runner).map_err(one)?
            }
            Options::Check => Vec::new(),
        };
        let args = entry.features.listing_args(features);
        let index =
            rustdoc_index::build(root, entry.name, &args, &target_dir, runner).map_err(one)?;
        if let Options::Out(_) = options {
            files.push((
                krate.name.clone(),
                text(&krate.name, &krate.text, &listing, &index),
            ));
        }
        indexes.push(index);
    }
    match options {
        Options::Check => {
            let terms_text =
                fs::read_to_string(root.join(TERMS)).map_err(|e| one(format!("{TERMS}: {e}")))?;
            let terms = read_terms(&terms_text).map_err(one)?;
            let open = ticket_ids(&root.join("tickets/open")).map_err(one)?;
            let done = ticket_ids(&root.join("tickets/done")).map_err(one)?;
            let errors = check(&terms, &indexes, &open, &done);
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
        Options::Out(dir) => {
            fs::create_dir_all(dir).map_err(|e| one(format!("{}: {e}", dir.display())))?;
            for (lib, text) in files {
                let path = dir.join(format!("{lib}.txt"));
                fs::write(&path, text).map_err(|e| one(format!("{}: {e}", path.display())))?;
            }
            Ok(())
        }
    }
}
