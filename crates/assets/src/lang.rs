//! Languages: the source language's screen text by key (`en/ui.ron`), and
//! the packs that overlay it (`<code>/`): their screen text (`ui.ron`), the
//! game's data text (`data.ron`: names, and whatever else the game hands
//! over by key) and the dialogue's lines and captions (`dialogue/*.ron`).
//! A pack entry carries the English it was made from; when that is no
//! longer today's English the entry is **stale**, and a key with no entry
//! is **missing**. Both show in English and fail nothing.
//!
//! The game names its folder and says what else its text must keep to
//! ([`PackRules`]); the English of its data is the [`SourceText`] it builds.
//! As a format (ADR-0007), one pack is read from its `lang.ron`
//! ([`LangPackRon`]).
use crate::prelude::*;

mod formats;
mod lines;
mod source;

use alloc::collections::{BTreeMap, BTreeSet};
use core::fmt;

use serde::de::{Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

use crate::dialogue::{DialogueLimits, Tokens};
use crate::error::{ContentError, codes};
use crate::names::Names;
use crate::ron::parse_ron;
use crate::tip::placeholders;

pub use formats::LangPackRon;
pub use lines::{LineEntry, LineText, Orphan};
pub use source::{LineKind, SourceLine, SourceText, names_key};

/// The source language's code.
pub const ENGLISH: &str = "en";

/// File name of a language's screen text.
const UI_FILE: &str = "ui.ron";

/// File name of a pack's description.
const INFO_FILE: &str = "lang.ron";

/// File name of a pack's data text: names, tips, titles and notes.
const DATA_FILE: &str = "data.ron";

/// Directory of a pack's dialogue text, one file per `.dlg` file.
const DIALOGUE_DIR: &str = "dialogue/";

/// Extension of a pack's dialogue files.
const DIALOGUE_EXTENSION: &str = ".ron";

/// How many cells of the screen `text` takes: one per character today.
/// Every length limit on text is counted with this, so that it can learn
/// the glyphs that are two cells wide in one place.
pub fn text_width(text: &str) -> usize {
    text.chars().count()
}

/// Longest [`LangCode`], in characters.
const MAX_CODE_CHARS: usize = 8;

/// A language's id: 2 to 8 lowercase letters (`en`, `ja`), the name of its
/// directory among the languages.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LangCode(String);

impl LangCode {
    /// `code` as a language id, or `None` if it isn't 2 to 8 lowercase
    /// letters.
    pub fn new(code: &str) -> Option<Self> {
        let ok = (2..=MAX_CODE_CHARS).contains(&code.len())
            && code.bytes().all(|b| b.is_ascii_lowercase());
        ok.then(|| Self(code.to_owned()))
    }

    /// English, the source language.
    pub fn english() -> Self {
        Self(ENGLISH.to_owned())
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for LangCode {
    fn default() -> Self {
        Self::english()
    }
}

impl fmt::Display for LangCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Who made a pack's text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum MadeBy {
    /// Machine-translated.
    Machine,
    /// Translated by a person or team: who to credit.
    Human(String),
}

/// A pack's `lang.ron`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LangInfo {
    /// The language's own name for itself.
    pub name: String,
    /// Who made the text.
    pub made_by: MadeBy,
}

/// One piece of a pack's text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Which text this is.
    pub key: String,
    /// The English `text` was made from.
    pub source: String,
    /// The text in the pack's language.
    pub text: String,
}

/// One language other than English.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LangPack {
    /// Its name and who made it.
    pub info: LangInfo,
    /// Its screen text, by key.
    ui: BTreeMap<String, Entry>,
    /// Its data text (names, and the game's other data text), by key.
    data: BTreeMap<String, Entry>,
    /// Its dialogue lines and captions, by line id or caption key.
    lines: BTreeMap<String, LineEntry>,
}

impl LangPack {
    /// The pack's screen-text or data-text entry for `key`, if it has one.
    pub fn entry(&self, key: &str) -> Option<&Entry> {
        self.ui.get(key).or_else(|| self.data.get(key))
    }
}

/// What a pack lacks ([`Lang::status`]): each list has the screen text's
/// keys, then the data's, then the dialogue's, each in key order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LangStatus {
    /// Keys the pack has no entry for.
    pub missing: Vec<String>,
    /// Keys whose entry was made from English that has since changed.
    pub stale: Vec<String>,
    /// Dialogue entries whose line English no longer has (it was reworded
    /// or cut), each with the line it most likely became.
    pub orphans: Vec<Orphan>,
}

/// English's text and every pack.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lang {
    /// English screen text, by key.
    english: BTreeMap<String, String>,
    /// English data and dialogue text, by key.
    source: SourceText,
    /// The other languages, by code.
    packs: BTreeMap<LangCode, LangPack>,
}

impl Lang {
    /// English screen text with `packs` over it, and no data or dialogue.
    pub fn new(english: BTreeMap<String, String>, packs: BTreeMap<LangCode, LangPack>) -> Self {
        Self::with_source(english, SourceText::default(), packs)
    }

    /// English screen text and `source` (the data's and the dialogue's
    /// English), with `packs` over them.
    pub fn with_source(
        english: BTreeMap<String, String>,
        source: SourceText,
        packs: BTreeMap<LangCode, LangPack>,
    ) -> Self {
        Self {
            english,
            source,
            packs,
        }
    }

    /// The English of the data and the dialogue, by key.
    pub fn source(&self) -> &SourceText {
        &self.source
    }

    /// Whether English's screen text has `key`.
    pub fn has(&self, key: &str) -> bool {
        self.english.contains_key(key)
    }

    /// Today's English for `key`, if there is such a key: a screen text's,
    /// a data text's, or a dialogue line's or caption's.
    pub fn english(&self, key: &str) -> Option<&str> {
        let line = || self.source.lines.get(key).map(|line| &line.text);
        self.english
            .get(key)
            .or_else(|| self.source.data.get(key))
            .or_else(line)
            .map(String::as_str)
    }

    /// The pack for `code`; `None` for English or an unknown code.
    pub fn pack(&self, code: &LangCode) -> Option<&LangPack> {
        self.packs.get(code)
    }

    /// The codes of the packs, in order (English isn't one).
    pub fn codes(&self) -> impl Iterator<Item = &LangCode> {
        self.packs.keys()
    }

    /// The text for `key` in language `code`: the pack's, when it has an
    /// entry made from today's English; otherwise English. A key English
    /// lacks comes back as it is.
    pub fn text<'a>(&'a self, code: &LangCode, key: &'a str) -> &'a str {
        let Some(english) = self.english.get(key) else {
            return key;
        };
        let entry = self.packs.get(code).and_then(|pack| pack.ui.get(key));
        match entry {
            Some(entry) if entry.source == *english => &entry.text,
            _ => english,
        }
    }

    /// What `code`'s pack lacks. English lacks nothing; `None` for a code
    /// with no pack.
    pub fn status(&self, code: &LangCode) -> Option<LangStatus> {
        if code.as_str() == ENGLISH {
            return Some(LangStatus::default());
        }
        let pack = self.packs.get(code)?;
        let mut status = LangStatus::default();
        let mut check = |key: &String, english: &String, source: Option<&String>| match source {
            None => status.missing.push(key.clone()),
            Some(source) if source != english => status.stale.push(key.clone()),
            Some(_) => {}
        };
        for (key, english) in &self.english {
            check(key, english, pack.ui.get(key).map(|e| &e.source));
        }
        for (key, english) in &self.source.data {
            check(key, english, pack.data.get(key).map(|e| &e.source));
        }
        for (key, line) in &self.source.lines {
            check(key, &line.text, pack.lines.get(key).map(|e| &e.source));
        }
        status.orphans = pack.orphans(&self.source);
        Some(status)
    }
}

/// A RON map read as its pairs in file order, so a key written twice is
/// seen twice.
struct Pairs(Vec<(String, String)>);

impl<'de> Deserialize<'de> for Pairs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PairsVisitor;

        impl<'de> Visitor<'de> for PairsVisitor {
            type Value = Pairs;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a map of key to text")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Pairs, A::Error> {
                let mut pairs = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    pairs.push(pair);
                }
                Ok(Pairs(pairs))
            }
        }

        deserializer.deserialize_map(PairsVisitor)
    }
}

/// Whether `key` is made of lowercase letters, digits, `_` and `.` only
/// (and isn't empty).
fn key_ok(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.')
}

/// Parses and validates English screen text `source` (a map of key to
/// text), attributing errors to `file`. Reports every problem found.
pub fn english_from_source(
    file: &str,
    source: &str,
) -> Result<BTreeMap<String, String>, Vec<ContentError>> {
    let Pairs(pairs) = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let mut english = BTreeMap::new();
    for (key, text) in pairs {
        if !key_ok(&key) {
            errors.push(ContentError::new(
                file,
                format!("\"{key}\": a key must be lowercase letters, digits, `_` and `.`"),
            ));
        }
        if english.contains_key(&key) {
            errors.push(
                ContentError::new(file, format!("\"{key}\": the key is used twice"))
                    .with_code(codes::DUPLICATE),
            );
        }
        english.insert(key, text);
    }
    if errors.is_empty() {
        Ok(english)
    } else {
        Err(errors)
    }
}

/// What a game's packs keep to besides matching English: the rules of its
/// data text, its tokens, its dialogue's limits and the variants a line
/// may have.
#[derive(Clone, Copy)]
pub struct PackRules<'a> {
    /// What is wrong with a pack's text `text` for the data key `key`, if
    /// anything (a tip over its box, say): the game's rules for the data
    /// text it hands over. Names (`names.<id>`) keep the engine's rules
    /// first.
    pub data_problem: fn(key: &str, text: &str) -> Option<String>,
    /// The game's own `{tokens}`: a pack's dialogue text may use them.
    pub tokens: &'a dyn Tokens,
    /// How much text the dialogue box and the reply menu hold.
    pub limits: DialogueLimits,
    /// The names of the variants a dialogue entry may have instead of one
    /// `text` (`text_<variant>` fields, every one of them): one per hero
    /// a line can be said of, say. Empty for none.
    pub variants: &'a [&'a str],
}

impl fmt::Debug for PackRules<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackRules")
            .field("limits", &self.limits)
            .field("variants", &self.variants)
            .finish_non_exhaustive()
    }
}

/// What is wrong with a pack's text `text` for the data key `key`, if
/// anything: a name is not empty and holds no brace (names can't hold
/// tokens); any other text keeps to the game's `data_problem`.
fn data_text_problem(
    key: &str,
    text: &str,
    data_problem: fn(&str, &str) -> Option<String>,
) -> Option<String> {
    if !key.starts_with(source::NAMES_PREFIX) {
        return data_problem(key, text);
    }
    if text.trim().is_empty() {
        return Some("a name must not be empty".to_owned());
    }
    text.contains(['{', '}'])
        .then(|| "a name can't hold a brace".to_owned())
}

/// A file of a pack: the name it is reported under, and its text.
pub type Source<'a> = (&'a str, &'a str);

/// The files of one pack.
#[derive(Debug, Clone, Default)]
pub struct PackSources<'a> {
    /// Its `lang.ron`.
    pub info: Source<'a>,
    /// Its `ui.ron`.
    pub ui: Source<'a>,
    /// Its `data.ron`, if it has one.
    pub data: Option<Source<'a>>,
    /// Its `dialogue/*.ron` files.
    pub dialogue: Vec<Source<'a>>,
}

/// Parses the entries of the pack file `file` (`source`) and checks each
/// against `english` (key to today's English), adding every problem to
/// `errors`. `no_key` is what an entry English has no key for is told.
///
/// An entry's key must be one of English's and be there once. Its text
/// must have the same placeholders as its `source`: the English it was
/// made from, not today's, so that changing English can make an entry
/// stale but never an error. And it keeps to the rules of data text.
fn entries(
    (file, source): Source<'_>,
    english: &BTreeMap<String, String>,
    no_key: &str,
    data_problem: fn(&str, &str) -> Option<String>,
    errors: &mut Vec<ContentError>,
) -> BTreeMap<String, Entry> {
    let entries = parse_ron::<Vec<Entry>>(file, source)
        .map_err(|e| errors.push(e))
        .unwrap_or_default();
    let mut by_key = BTreeMap::new();
    for entry in entries {
        let at = |message: &str| ContentError::new(file, format!("\"{}\": {message}", entry.key));
        if !english.contains_key(&entry.key) {
            errors.push(at(no_key));
        }
        if by_key.contains_key(&entry.key) {
            errors.push(at("the key is used twice").with_code(codes::DUPLICATE));
        }
        let names = |text| placeholders(text).into_iter().collect::<BTreeSet<_>>();
        if names(&entry.text) != names(&entry.source) {
            errors.push(at(
                "the text and its source must have the same placeholders",
            ));
        }
        if let Some(problem) = data_text_problem(&entry.key, &entry.text, data_problem) {
            errors.push(at(&problem));
        }
        by_key.insert(entry.key.clone(), entry);
    }
    by_key
}

/// Parses and validates a pack with screen text only: its `lang.ron`
/// (`info`) and its `ui.ron` (`ui`), checked against `english`. See
/// [`pack_from_files`].
pub fn pack_from_sources(
    info: Source<'_>,
    ui: Source<'_>,
    english: &BTreeMap<String, String>,
    rules: &PackRules<'_>,
) -> Result<LangPack, Vec<ContentError>> {
    let sources = PackSources {
        info,
        ui,
        ..PackSources::default()
    };
    pack_from_files(&sources, english, None, rules)
}

/// Parses and validates the pack made of `sources`, its screen text
/// checked against `english` and its data and dialogue text against
/// `data` (left out without it: the data files failed to load). Reports
/// every problem found.
///
/// A `ui.ron` or `data.ron` entry's key must be one of English's and be
/// there once, and its text must have the same placeholders as its
/// `source`; a name, and the game's data text, also keep to the rules of
/// the English ones (`rules`). A dialogue entry is checked as
/// [`LineEntry`] says: one whose line English no longer has is an orphan,
/// not an error.
pub fn pack_from_files(
    sources: &PackSources<'_>,
    english: &BTreeMap<String, String>,
    data: Option<&SourceText>,
    rules: &PackRules<'_>,
) -> Result<LangPack, Vec<ContentError>> {
    let mut errors = Vec::new();
    let (info_file, info_source) = sources.info;
    let info = parse_ron::<LangInfo>(info_file, info_source)
        .map_err(|e| errors.push(e))
        .ok();
    if info.as_ref().is_some_and(|i| i.name.trim().is_empty()) {
        errors.push(ContentError::new(info_file, "the name must not be empty"));
    }
    let problem = rules.data_problem;
    let ui = entries(
        sources.ui,
        english,
        "no such key in English",
        problem,
        &mut errors,
    );
    let mut pack_data = BTreeMap::new();
    let mut pack_lines = BTreeMap::new();
    if let Some(data) = data {
        if let Some(file) = sources.data {
            let no_key = "no such key in the data";
            pack_data = entries(file, &data.data, no_key, problem, &mut errors);
        }
        // The pack's names: what a name token in its lines is filled from.
        let name = |(id, english): (&String, &String)| {
            let entry = pack_data.get(&names_key(id));
            let name = entry.filter(|e| e.source == *english);
            (id.clone(), name.map_or(english, |e| &e.text).clone())
        };
        let names = Names {
            names: data.names.names.iter().map(name).collect(),
        };
        for &file in &sources.dialogue {
            lines::add_lines(file, data, &names, rules, &mut pack_lines, &mut errors);
        }
    }
    match info {
        Some(info) if errors.is_empty() => Ok(LangPack {
            info,
            ui,
            data: pack_data,
            lines: pack_lines,
        }),
        _ => Err(errors),
    }
}

/// One file of the languages' directory.
#[derive(Debug, Clone, Copy)]
pub struct LangFile<'a> {
    /// Its path under the languages' directory (`en/ui.ron`).
    pub path: &'a str,
    /// Its text; `None` if it isn't UTF-8.
    pub source: Option<&'a str>,
}

/// Whether `name` (a path inside a pack's directory) is one of a pack's
/// dialogue files: `dialogue/<file>.ron`.
fn is_dialogue_file(name: &str) -> bool {
    name.strip_prefix(DIALOGUE_DIR)
        .and_then(|file| file.strip_suffix(DIALOGUE_EXTENSION))
        .is_some_and(|stem| !stem.is_empty() && !stem.contains('/'))
}

/// Validates the languages made of `files`: English's `en/ui.ron`, and a
/// pack for every other directory, which needs its `lang.ron` and
/// `ui.ron` and may have a `data.ron` and `dialogue/<file>.ron` files
/// (checked against `data`; left out without it). Any other file is an
/// error. `display` gives the name a path is reported under. Reports
/// every problem found; with English broken the packs are not checked
/// (each entry would be an unknown key).
pub fn from_files(
    files: &[LangFile<'_>],
    data: Option<SourceText>,
    display: impl Fn(&str) -> String,
    rules: &PackRules<'_>,
) -> Result<Lang, Vec<ContentError>> {
    let mut errors = Vec::new();
    let find = |path: &str| files.iter().find(|f| f.path == path);
    let read = |path: &str, errors: &mut Vec<ContentError>| {
        let source = find(path).and_then(|f| f.source);
        if source.is_none() {
            errors.push(ContentError::not_found(display(path)));
        }
        source
    };
    let mut dirs = BTreeSet::new();
    for file in files {
        let known = match file.path.split_once('/') {
            Some((ENGLISH, name)) => name == UI_FILE,
            Some((dir, name)) => {
                dirs.insert(dir);
                [UI_FILE, INFO_FILE, DATA_FILE].contains(&name) || is_dialogue_file(name)
            }
            None => false,
        };
        if !known {
            errors.push(ContentError::new(
                display(file.path),
                "not a file a language has",
            ));
        }
    }
    let english_path = format!("{ENGLISH}/{UI_FILE}");
    let english = read(&english_path, &mut errors)
        .map(|source| english_from_source(&display(&english_path), source));
    let english = match english {
        Some(Ok(english)) => Some(english),
        Some(Err(e)) => {
            errors.extend(e);
            None
        }
        None => None,
    };
    let mut packs = BTreeMap::new();
    for dir in dirs {
        let Some(code) = LangCode::new(dir) else {
            errors.push(ContentError::new(
                display(dir),
                "a language's directory must be named with 2 to 8 lowercase letters",
            ));
            continue;
        };
        let info_path = format!("{dir}/{INFO_FILE}");
        let ui_path = format!("{dir}/{UI_FILE}");
        let data_path = format!("{dir}/{DATA_FILE}");
        let info = read(&info_path, &mut errors);
        let ui = read(&ui_path, &mut errors);
        // The optional files: only read (and reported, if not text) when
        // the pack has them.
        let data_source = find(&data_path).and_then(|_| read(&data_path, &mut errors));
        let dialogue_dir = format!("{dir}/{DIALOGUE_DIR}");
        let dialogue: Vec<(String, &str)> = files
            .iter()
            .filter(|f| f.path.starts_with(&dialogue_dir))
            .filter_map(|f| Some((display(f.path), read(f.path, &mut errors)?)))
            .collect();
        let (Some(info), Some(ui), Some(english)) = (info, ui, &english) else {
            continue;
        };
        let (info_name, ui_name, data_name) =
            (display(&info_path), display(&ui_path), display(&data_path));
        let sources = PackSources {
            info: (&info_name, info),
            ui: (&ui_name, ui),
            data: data_source.map(|source| (data_name.as_str(), source)),
            dialogue: dialogue.iter().map(|(n, s)| (n.as_str(), *s)).collect(),
        };
        match pack_from_files(&sources, english, data.as_ref(), rules) {
            Ok(pack) => {
                packs.insert(code, pack);
            }
            Err(e) => errors.extend(e),
        }
    }
    match english {
        Some(english) if errors.is_empty() => {
            Ok(Lang::with_source(english, data.unwrap_or_default(), packs))
        }
        _ => Err(errors),
    }
}

#[cfg(test)]
mod pack_tests;

#[cfg(test)]
mod tests {
    use super::*;

    const ENGLISH_SOURCE: &str = r#"{
        "a.one": "One",
        "a.two": "Two {count}",
        "a.three": "Three",
    }"#;

    const INFO: &str = r#"(name: "Test", made_by: Machine)"#;

    /// No data text rules of a game's, no tokens, no variants.
    const RULES: PackRules<'static> = PackRules {
        data_problem: |_, _| None,
        tokens: &crate::dialogue::NoTokens,
        limits: crate::dialogue::tests::LIMITS,
        variants: &[],
    };

    fn english() -> BTreeMap<String, String> {
        english_from_source("en/ui.ron", ENGLISH_SOURCE).unwrap()
    }

    fn pack(ui: &str) -> Result<LangPack, Vec<String>> {
        pack_from_sources(("xx/lang.ron", INFO), ("xx/ui.ron", ui), &english(), &RULES)
            .map_err(|errors| errors.iter().map(ToString::to_string).collect())
    }

    fn code(code: &str) -> LangCode {
        LangCode::new(code).unwrap()
    }

    /// English with one pack `xx`: `a.one` translated, `a.two` stale,
    /// `a.three` missing.
    fn lang() -> Lang {
        let ui = r#"[
            (key: "a.one", source: "One", text: "Un"),
            (key: "a.two", source: "Two of {count}", text: "{count} deux"),
        ]"#;
        let packs = BTreeMap::from([(code("xx"), pack(ui).unwrap())]);
        Lang::new(english(), packs)
    }

    #[test]
    fn codes_are_two_to_eight_lowercase_letters() {
        for good in ["en", "ja", "test", "abcdefgh"] {
            assert_eq!(LangCode::new(good).unwrap().as_str(), good);
        }
        for bad in ["", "e", "EN", "e1", "pt-br", "abcdefghi", "日本"] {
            assert_eq!(LangCode::new(bad), None, "{bad}");
        }
        assert_eq!(LangCode::default(), LangCode::english());
        assert_eq!(LangCode::english().to_string(), "en");
    }

    #[test]
    fn english_loads_as_a_map() {
        let english = english();
        assert_eq!(english.len(), 3);
        assert_eq!(english["a.two"], "Two {count}");
    }

    #[test]
    fn bad_english_keys_are_reported_with_file_and_key() {
        let source = r#"{"A.b": "x", "": "y", "a b": "z", "ok_1.x": "fine", "ok_1.x": "again"}"#;
        let errors = english_from_source("en/ui.ron", source).unwrap_err();
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        let must = "a key must be lowercase letters, digits, `_` and `.`";
        assert_eq!(
            errors,
            [
                format!("en/ui.ron: \"A.b\": {must}"),
                format!("en/ui.ron: \"\": {must}"),
                format!("en/ui.ron: \"a b\": {must}"),
                "en/ui.ron: \"ok_1.x\": the key is used twice".to_owned(),
            ]
        );
        let codes: Vec<_> = english_from_source("en/ui.ron", source)
            .unwrap_err()
            .iter()
            .map(|e| e.code)
            .collect();
        assert_eq!(codes, [None, None, None, Some(codes::DUPLICATE)]);
    }

    #[test]
    fn english_that_is_not_a_map_is_a_syntax_error() {
        let errors = english_from_source("en/ui.ron", "[1, 2]").unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, Some(1));
        let errors = english_from_source("en/ui.ron", "{\"a\": \"b\", \"c\": 3}").unwrap_err();
        assert_eq!(errors.len(), 1);
    }

    /// What a wrong value is told the file should be.
    #[test]
    fn english_that_is_not_a_map_says_what_was_expected() {
        use serde::de::IntoDeserializer;
        use serde::de::value::{BoolDeserializer, Error};
        let wrong: BoolDeserializer<Error> = true.into_deserializer();
        let error = Pairs::deserialize(wrong).err().unwrap();
        assert_eq!(
            error.to_string(),
            "invalid type: boolean `true`, expected a map of key to text"
        );
    }

    #[test]
    fn a_pack_loads_its_info_and_entries() {
        let p = pack(r#"[(key: "a.one", source: "One", text: "Un")]"#).unwrap();
        assert_eq!(p.info.name, "Test");
        assert_eq!(p.info.made_by, MadeBy::Machine);
        assert_eq!(p.entry("a.one").unwrap().text, "Un");
        assert_eq!(p.entry("a.two"), None);
        let human = r#"(name: "Test", made_by: Human("A. Translator"))"#;
        let p = pack_from_sources(("l", human), ("u", "[]"), &english(), &RULES).unwrap();
        assert_eq!(p.info.made_by, MadeBy::Human("A. Translator".to_owned()));
    }

    #[test]
    fn an_unknown_key_fails_with_file_and_key() {
        let errors = pack(r#"[(key: "a.nope", source: "x", text: "y")]"#).unwrap_err();
        assert_eq!(errors, ["xx/ui.ron: \"a.nope\": no such key in English"]);
    }

    #[test]
    fn different_placeholders_fail_with_file_and_key() {
        let message = "the text and its source must have the same placeholders";
        for text in ["deux", "{n} deux", "{count} {n}"] {
            let ui = format!(r#"[(key: "a.two", source: "Two {{count}}", text: "{text}")]"#);
            assert_eq!(
                pack(&ui).unwrap_err(),
                [format!("xx/ui.ron: \"a.two\": {message}")],
                "{text}"
            );
        }
        // The same set, in any order and any number of times, is fine.
        let ui = r#"[(key: "a.two", source: "{a} {b}", text: "{b} {a} {b}")]"#;
        assert!(pack(ui).is_ok());
    }

    #[test]
    fn placeholders_are_checked_against_the_source_not_todays_english() {
        // English gained `{count}` since: the entry is stale, not an error.
        let ui = r#"[(key: "a.two", source: "Two", text: "Deux")]"#;
        assert!(pack(ui).is_ok());
    }

    #[test]
    fn a_key_used_twice_in_a_pack_fails() {
        let ui = r#"[
            (key: "a.one", source: "One", text: "Un"),
            (key: "a.one", source: "One", text: "Une"),
        ]"#;
        assert_eq!(
            pack(ui).unwrap_err(),
            ["xx/ui.ron: \"a.one\": the key is used twice"]
        );
        let errors =
            pack_from_sources(("xx/lang.ron", INFO), ("xx/ui.ron", ui), &english(), &RULES)
                .unwrap_err();
        assert_eq!(errors[0].code, Some(codes::DUPLICATE));
    }

    #[test]
    fn every_problem_in_a_pack_is_reported() {
        let errors = pack_from_sources(
            ("xx/lang.ron", r#"(name: " ", made_by: Machine)"#),
            (
                "xx/ui.ron",
                r#"[(key: "a.nope", source: "{a}", text: "b")]"#,
            ),
            &english(),
            &RULES,
        )
        .unwrap_err();
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        assert_eq!(errors.len(), 3, "{errors:?}");
        assert_eq!(errors[0], "xx/lang.ron: the name must not be empty");
        // A file that doesn't parse is one error, attributed to that file.
        let errors =
            pack_from_sources(("l.ron", "("), ("u.ron", "["), &english(), &RULES).unwrap_err();
        let files: Vec<&str> = errors.iter().map(|e| e.file.as_str()).collect();
        assert_eq!(files, ["l.ron", "u.ron"]);
        // A good `ui.ron` doesn't make up for a bad `lang.ron`.
        assert!(pack_from_sources(("l.ron", "("), ("u.ron", "[]"), &english(), &RULES).is_err());
    }

    #[test]
    fn lookup_uses_the_pack_and_falls_back_to_english() {
        let lang = lang();
        let xx = code("xx");
        assert_eq!(lang.text(&xx, "a.one"), "Un");
        // Stale and missing entries show English.
        assert_eq!(lang.text(&xx, "a.two"), "Two {count}");
        assert_eq!(lang.text(&xx, "a.three"), "Three");
        // English, and a code with no pack, are English.
        assert_eq!(lang.text(&LangCode::english(), "a.one"), "One");
        assert_eq!(lang.text(&code("zz"), "a.one"), "One");
        // An unknown key comes back as it is.
        assert_eq!(lang.text(&xx, "a.nope"), "a.nope");
        assert!(lang.has("a.one"));
        assert!(!lang.has("a.nope"));
        assert_eq!(lang.english("a.one"), Some("One"));
        assert_eq!(lang.english("a.nope"), None);
    }

    #[test]
    fn status_lists_missing_and_stale_keys() {
        let lang = lang();
        let status = lang.status(&code("xx")).unwrap();
        assert_eq!(status.missing, ["a.three"]);
        assert_eq!(status.stale, ["a.two"]);
        assert_eq!(
            lang.status(&LangCode::english()),
            Some(LangStatus::default())
        );
        assert_eq!(lang.status(&code("zz")), None);
        assert_eq!(lang.codes().collect::<Vec<_>>(), [&code("xx")]);
        assert!(lang.pack(&code("xx")).is_some());
        assert!(lang.pack(&LangCode::english()).is_none());
    }

    /// The languages made of `files` (path, text), or the errors as text.
    fn files(files: &[(&str, &str)]) -> Result<Lang, Vec<String>> {
        let files: Vec<LangFile<'_>> = files
            .iter()
            .map(|&(path, source)| LangFile {
                path,
                source: Some(source),
            })
            .collect();
        from_files(&files, None, |path| format!("lang/{path}"), &RULES)
            .map_err(|errors| errors.iter().map(ToString::to_string).collect())
    }

    const UI: &str = r#"[(key: "a.one", source: "One", text: "Un")]"#;

    #[test]
    fn files_make_english_and_a_pack_per_directory() {
        let lang = files(&[
            ("en/ui.ron", ENGLISH_SOURCE),
            ("xx/lang.ron", INFO),
            ("xx/ui.ron", UI),
            ("yy/ui.ron", "[]"),
            ("yy/lang.ron", INFO),
        ])
        .unwrap();
        assert_eq!(lang.codes().collect::<Vec<_>>(), [&code("xx"), &code("yy")]);
        assert_eq!(lang.text(&code("xx"), "a.one"), "Un");
        assert_eq!(lang.text(&code("yy"), "a.one"), "One");
        // English alone is enough.
        let lang = files(&[("en/ui.ron", ENGLISH_SOURCE)]).unwrap();
        assert_eq!(lang.codes().count(), 0);
        assert!(lang.has("a.one"));
    }

    #[test]
    fn a_missing_file_is_reported() {
        assert_eq!(
            files(&[]).unwrap_err(),
            ["lang/en/ui.ron: file not found in asset bundle"]
        );
        assert_eq!(
            files(&[("en/ui.ron", ENGLISH_SOURCE), ("xx/ui.ron", UI)]).unwrap_err(),
            ["lang/xx/lang.ron: file not found in asset bundle"]
        );
        assert_eq!(
            files(&[("en/ui.ron", ENGLISH_SOURCE), ("xx/lang.ron", INFO)]).unwrap_err(),
            ["lang/xx/ui.ron: file not found in asset bundle"]
        );
        // A file that isn't text counts as missing.
        let binary = [LangFile {
            path: "en/ui.ron",
            source: None,
        }];
        let errors = from_files(&binary, None, str::to_owned, &RULES).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].file, "en/ui.ron");
    }

    #[test]
    fn a_file_no_language_has_is_reported() {
        let stray = "not a file a language has";
        assert_eq!(
            files(&[
                ("en/ui.ron", ENGLISH_SOURCE),
                ("en/lang.ron", INFO),
                ("notes.txt", ""),
                ("xx/lang.ron", INFO),
                ("xx/ui.ron", UI),
                // A pack may have these two kinds of file...
                ("xx/data.ron", "[]"),
                ("xx/dialogue/ch01.ron", "[]"),
                // ...but English can't, and a pack has no others.
                ("en/data.ron", "[]"),
                ("xx/dialogue/ch01.dlg", ""),
                ("xx/dialogue/.ron", "[]"),
                ("xx/dialogue/a/b.ron", "[]"),
                ("xx/notes/ch01.ron", "[]"),
            ])
            .unwrap_err(),
            [
                format!("lang/en/lang.ron: {stray}"),
                format!("lang/notes.txt: {stray}"),
                format!("lang/en/data.ron: {stray}"),
                format!("lang/xx/dialogue/ch01.dlg: {stray}"),
                format!("lang/xx/dialogue/.ron: {stray}"),
                format!("lang/xx/dialogue/a/b.ron: {stray}"),
                format!("lang/xx/notes/ch01.ron: {stray}"),
            ]
        );
    }

    #[test]
    fn a_badly_named_directory_is_reported() {
        assert_eq!(
            files(&[
                ("en/ui.ron", ENGLISH_SOURCE),
                ("JA/lang.ron", INFO),
                ("JA/ui.ron", UI),
            ])
            .unwrap_err(),
            ["lang/JA: a language's directory must be named with 2 to 8 lowercase letters"]
        );
    }

    #[test]
    fn pack_errors_are_reported_for_every_pack() {
        let bad = r#"[(key: "a.nope", source: "x", text: "y")]"#;
        assert_eq!(
            files(&[
                ("en/ui.ron", ENGLISH_SOURCE),
                ("xx/lang.ron", INFO),
                ("xx/ui.ron", bad),
                ("yy/lang.ron", INFO),
                ("yy/ui.ron", bad),
            ])
            .unwrap_err(),
            [
                "lang/xx/ui.ron: \"a.nope\": no such key in English",
                "lang/yy/ui.ron: \"a.nope\": no such key in English",
            ]
        );
    }

    #[test]
    fn with_english_broken_the_packs_are_not_checked() {
        let errors = files(&[
            ("en/ui.ron", r#"{"Bad Key": "x"}"#),
            ("xx/lang.ron", INFO),
            ("xx/ui.ron", UI),
        ])
        .unwrap_err();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("lang/en/ui.ron: \"Bad Key\""));
    }

    #[test]
    fn width_counts_characters() {
        assert_eq!(text_width(""), 0);
        assert_eq!(text_width("abc"), 3);
        assert_eq!(text_width("né!"), 3);
    }
}
