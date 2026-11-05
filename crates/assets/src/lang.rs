//! Languages: the source language's screen text by key (`en/ui.ron`), and
//! the packs that overlay it (`<tag>/`): their screen text (`ui.ron`), the
//! game's data text (`data.ron`: names, and whatever else the game hands
//! over by key) and the dialogue's lines and captions (`dialogue/*.ron`).
//!
//! **The source** is the language the game is written in ([`SourceLang`]):
//! English unless the game declares another
//! ([`Lang::with_source_lang`]), its folder holding its screen text only. A
//! pack entry carries the source text it was made from; when that is no
//! longer today's the entry is **stale**, and a key with no entry is
//! **missing**. Neither fails anything.
//!
//! **Tags.** A language is named by its BCP 47 tag ([`LangCode`]: `ja`,
//! `pt-BR`, `zh-Hans`), and its folder by that tag as BCP 47 writes it, so
//! `pt-br` never quietly becomes a second language on a file system that
//! ignores case. The engine reads the subset of tags it takes itself rather
//! than with `icu_locale_core`: checking a tag is a short function, and the
//! engine wants none of CLDR's data.
//!
//! **The fallback chain.** A key a pack lacks, or has stale, falls back
//! along [`Lang::chain`]: the pack of each shorter tag the game has
//! (`pt-BR`, then `pt`), then the source. So a small `pt-BR` pack can sit
//! over a full `pt` one. Which language a game starts in is
//! [`best_match`] of the system's languages among the source and the
//! packs: a tag, its shorter tags, Chinese's script from its region, then
//! the one pack of the same language.
//!
//! The game names its folder and says what else its text must keep to
//! ([`PackRules`]); the source text of its data is the [`SourceText`] it
//! builds. As a format (ADR-0007), one pack is read from its `lang.ron`
//! ([`LangPackRon`]). [`Lang::texts`] lists every text of every language,
//! each with its part, key and file ([`TextAt`]). The source's screen text
//! can say where each key's text is drawn ([`Place`]: its width in cells
//! and a note for translators), and [`Lang::overflow`] lists a pack's
//! entries too wide for it.
use crate::prelude::*;

mod formats;
mod lines;
mod matching;
mod places;
mod source;
mod tag;
mod texts;

use alloc::collections::{BTreeMap, BTreeSet};
use core::fmt;

use serde::de::{Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

use crate::dialogue::{DialogueLimits, Tokens};
use crate::error::{ContentError, codes};
use crate::font::CellWidths;
use crate::format::AssetKind;
use crate::import_settings::is_settings_file;
use crate::names::Names;
use crate::ron::{parse_ron, parse_ron_implicit_some};
use crate::tip::placeholders;
use places::UiValue;

pub use formats::LangPackRon;
pub use lines::{LineEntry, LineText, Orphan};
pub use matching::best_match;
pub use places::{Overflow, Place, Places};
pub use source::{LineKind, SourceLine, SourceText, names_key};
pub use tag::{LangCode, TagError};
pub use texts::{TextAt, TextPart};

/// English's tag, the source language when a game declares none.
#[deprecated(note = "a game's source language is `Lang::source_lang()`")]
pub const ENGLISH: &str = tag::DEFAULT_SOURCE;

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

/// How many cells of the screen `text` takes: one per character, two for a
/// character `widths` has wide (a Japanese or Chinese one, from the font
/// atlas's wide glyphs). Every length limit on text and every layout that
/// measures it counts with this.
pub fn text_width(text: &str, widths: &CellWidths) -> usize {
    text.chars().map(|c| widths.of_char(c)).sum()
}

/// The language the game is written in: its screen text is in its folder
/// (`<code>/ui.ron`), and its data and dialogue in the game's own files.
/// Every pack is checked against it, and every lookup ends with it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SourceLang {
    /// Its tag, the name of its folder.
    pub code: LangCode,
    /// Its own name for itself, which a language menu shows.
    pub name: String,
}

impl SourceLang {
    /// The source language `code`, named `name` in itself.
    pub fn new(code: LangCode, name: impl Into<String>) -> Self {
        Self {
            code,
            name: name.into(),
        }
    }
}

impl Default for SourceLang {
    /// English, named "English".
    fn default() -> Self {
        Self::new(LangCode::default(), "English")
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
    /// The source text `text` was made from.
    pub source: String,
    /// The text in the pack's language.
    pub text: String,
}

/// One language other than the source.
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
    /// Keys whose entry was made from source text that has since changed.
    pub stale: Vec<String>,
    /// Dialogue entries whose line the source no longer has (it was
    /// reworded or cut), each with the line it most likely became.
    pub orphans: Vec<Orphan>,
}

/// A game's English screen text by key, as one file (`en.ron`) holds it:
/// a kind ([`AssetKind`]) read with `Ron::<English>::new()`
/// ([`Ron`](crate::ron::Ron)). [`Lang::new`] takes its map.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct English(pub BTreeMap<String, String>);

impl AssetKind for English {
    const NAME: &'static str = "english";
}

/// The source language's text and every pack.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lang {
    /// Which language the game is written in ([`Lang::with_source_lang`]).
    written_in: SourceLang,
    /// The source's screen text, by key.
    ui: BTreeMap<String, String>,
    /// The source's data and dialogue text, by key.
    source: SourceText,
    /// The other languages, by tag.
    packs: BTreeMap<LangCode, LangPack>,
    /// Where the source's screen text is drawn, by key
    /// ([`Lang::with_places`]).
    places: Places,
}

impl Lang {
    /// The source's screen text `ui` with `packs` over it, and no data or
    /// dialogue. The source is English until
    /// [`with_source_lang`](Self::with_source_lang) says otherwise.
    pub fn new(ui: BTreeMap<String, String>, packs: BTreeMap<LangCode, LangPack>) -> Self {
        Self::with_source(ui, SourceText::default(), packs)
    }

    /// The source's screen text `ui` and `source` (the data's and the
    /// dialogue's source text), with `packs` over them. The source is
    /// English until [`with_source_lang`](Self::with_source_lang) says
    /// otherwise.
    pub fn with_source(
        ui: BTreeMap<String, String>,
        source: SourceText,
        packs: BTreeMap<LangCode, LangPack>,
    ) -> Self {
        Self {
            written_in: SourceLang::default(),
            ui,
            source,
            packs,
            places: BTreeMap::new(),
        }
    }

    /// These languages, the game written in `source`.
    #[must_use]
    pub fn with_source_lang(self, source: SourceLang) -> Self {
        Self {
            written_in: source,
            ..self
        }
    }

    /// The language the game is written in.
    pub fn source_lang(&self) -> &SourceLang {
        &self.written_in
    }

    /// The source text of the data and the dialogue, by key.
    pub fn source(&self) -> &SourceText {
        &self.source
    }

    /// Whether the source's screen text has `key`.
    pub fn has(&self, key: &str) -> bool {
        self.ui.contains_key(key)
    }

    /// Today's source text for `key`, if there is such a key: a screen
    /// text's, a data text's, or a dialogue line's or caption's.
    pub fn source_text(&self, key: &str) -> Option<&str> {
        let line = || self.source.lines.get(key).map(|line| &line.text);
        self.ui
            .get(key)
            .or_else(|| self.source.data.get(key))
            .or_else(line)
            .map(String::as_str)
    }

    /// Today's source text for `key`: [`source_text`](Self::source_text).
    #[deprecated(note = "renamed `source_text`: the source language needn't be English")]
    pub fn english(&self, key: &str) -> Option<&str> {
        self.source_text(key)
    }

    /// The pack for `code`; `None` for the source or an unknown tag.
    pub fn pack(&self, code: &LangCode) -> Option<&LangPack> {
        self.packs.get(code)
    }

    /// The tags of the packs, in order (the source isn't one).
    pub fn codes(&self) -> impl Iterator<Item = &LangCode> {
        self.packs.keys()
    }

    /// The packs a lookup in `code` reads, nearest first: `code`'s, then
    /// each of its parents' that the game has; never the source's. Walks
    /// `code`'s text without allocating.
    fn packs_along<'a>(
        &'a self,
        code: &LangCode,
    ) -> impl Iterator<Item = (&'a LangCode, &'a LangPack)> {
        let source = self.written_in.code.as_str();
        tag::ancestry(code.as_str())
            .filter(move |tag| *tag != source)
            .filter_map(|tag| self.packs.get_key_value(tag))
    }

    /// Where a lookup in `code` looks, in order: `code`, then each of its
    /// [`parent`](LangCode::parent)s, keeping the tags that have a pack,
    /// then the source, which always ends it. With packs `pt` and `pt-BR`,
    /// `pt-BR`'s chain is `pt-BR`, `pt`, `en`. Each lookup takes the first
    /// entry along it made from today's source text.
    pub fn chain(&self, code: &LangCode) -> Vec<LangCode> {
        let mut chain: Vec<LangCode> = self.packs_along(code).map(|(c, _)| c.clone()).collect();
        chain.push(self.written_in.code.clone());
        chain
    }

    /// The text for `key` in language `code`: the first entry along
    /// [`chain`](Self::chain) made from today's source text, else the
    /// source's. A key the source lacks comes back as it is.
    pub fn text<'a>(&'a self, code: &LangCode, key: &'a str) -> &'a str {
        let Some(source) = self.ui.get(key) else {
            return key;
        };
        self.packs_along(code)
            .find_map(|(_, pack)| pack.ui.get(key).filter(|entry| entry.source == *source))
            .map_or(source, |entry| &entry.text)
    }

    /// What `code`'s pack lacks, itself, without its chain. The source
    /// lacks nothing; `None` for a tag with no pack.
    pub fn status(&self, code: &LangCode) -> Option<LangStatus> {
        if *code == self.written_in.code {
            return Some(LangStatus::default());
        }
        let pack = self.packs.get(code)?;
        let mut status = LangStatus::default();
        let mut check = |key: &String, english: &String, source: Option<&String>| match source {
            None => status.missing.push(key.clone()),
            Some(source) if source != english => status.stale.push(key.clone()),
            Some(_) => {}
        };
        for (key, english) in &self.ui {
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
struct Pairs<V>(Vec<(String, V)>);

impl<'de, V: Deserialize<'de>> Deserialize<'de> for Pairs<V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PairsVisitor<V>(core::marker::PhantomData<V>);

        impl<'de, V: Deserialize<'de>> Visitor<'de> for PairsVisitor<V> {
            type Value = Pairs<V>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a map of key to text")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Pairs<V>, A::Error> {
                let mut pairs = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    pairs.push(pair);
                }
                Ok(Pairs(pairs))
            }
        }

        deserializer.deserialize_map(PairsVisitor(core::marker::PhantomData))
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

/// Parses and validates the source language's screen text `source` (a map
/// of key to text), whatever the language, attributing errors to `file`.
/// Reports every problem found. A key's places
/// ([`english_and_places_from_source`]) are read and left out.
pub fn english_from_source(
    file: &str,
    source: &str,
) -> Result<BTreeMap<String, String>, Vec<ContentError>> {
    english_and_places_from_source(file, source).map(|(english, _)| english)
}

/// The source's screen text and where it is drawn, by key: what
/// [`english_from_source`] reads, with each key's [`Place`] where its value
/// is written as `(text: …, max_cells: …, note: …)` rather than as text
/// (both optional). A `max_cells` is at least 1, and a note isn't empty.
pub fn english_and_places_from_source(
    file: &str,
    source: &str,
) -> Result<(BTreeMap<String, String>, Places), Vec<ContentError>> {
    let Pairs(pairs) =
        parse_ron_implicit_some::<Pairs<UiValue>>(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let mut english = BTreeMap::new();
    let mut places = BTreeMap::new();
    for (key, UiValue { text, place }) in pairs {
        let at = |message: &str| ContentError::new(file, format!("\"{key}\": {message}"));
        if !key_ok(&key) {
            errors.push(at("a key must be lowercase letters, digits, `_` and `.`"));
        }
        if english.contains_key(&key) {
            errors.push(at("the key is used twice").with_code(codes::DUPLICATE));
        }
        if place.max_cells == Some(0) {
            errors.push(at("max_cells must be at least 1"));
        }
        if place.note.as_ref().is_some_and(|n| n.trim().is_empty()) {
            errors.push(at("a note must not be empty"));
        }
        if place != Place::default() {
            places.insert(key.clone(), place);
        }
        english.insert(key, text);
    }
    if errors.is_empty() {
        Ok((english, places))
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
    /// How many cells each character takes, which the limits count: the
    /// font atlas's ([`CellWidths::of`]), or [`CellWidths::NARROW`].
    pub widths: &'a CellWidths,
    /// The names of the variants a dialogue entry may have instead of one
    /// `text` (`text_<variant>` fields, every one of them): one per hero
    /// a line can be said of, say. Empty for none.
    pub variants: &'a [&'a str],
}

impl fmt::Debug for PackRules<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackRules")
            .field("limits", &self.limits)
            .field("widths", &self.widths)
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
/// checked against `english` (the source's screen text, whatever the
/// language) and its data and dialogue text against `data` (left out
/// without it: the data files failed to load). Reports every problem
/// found; the messages call the source `en`.
///
/// A `ui.ron` or `data.ron` entry's key must be one of the source's and be
/// there once, and its text must have the same placeholders as its
/// `source`; a name, and the game's data text, also keep to the rules of
/// the source's (`rules`). A dialogue entry is checked as [`LineEntry`]
/// says: one whose line the source no longer has is an orphan, not an
/// error.
pub fn pack_from_files(
    sources: &PackSources<'_>,
    english: &BTreeMap<String, String>,
    data: Option<&SourceText>,
    rules: &PackRules<'_>,
) -> Result<LangPack, Vec<ContentError>> {
    pack_from_files_in(&LangCode::default(), sources, english, data, rules)
}

/// [`pack_from_files`], its messages naming the source by its tag `source`.
fn pack_from_files_in(
    source: &LangCode,
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
    let no_key = format!("no such key in the source language ({source})");
    let ui = entries(sources.ui, english, &no_key, problem, &mut errors);
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

/// The tag a pack's folder `dir` names, or what its error says: a folder is
/// named by a tag as BCP 47 writes it, so `pt-br` is refused rather than
/// becoming a second language where file names ignore case.
fn pack_code(dir: &str) -> Result<LangCode, String> {
    match LangCode::parse(dir) {
        Ok(code) if code.as_str() == dir => Ok(code),
        Ok(code) => Err(format!(
            "name a language's folder by its tag as BCP 47 writes it: {code}"
        )),
        Err(e) => Err(format!(
            "name a language's folder by a BCP 47 tag (en, pt-BR, zh-Hans): {e}"
        )),
    }
}

/// [`from_files_in`] for a game written in English.
pub fn from_files(
    files: &[LangFile<'_>],
    data: Option<SourceText>,
    display: impl Fn(&str) -> String,
    rules: &PackRules<'_>,
) -> Result<Lang, Vec<ContentError>> {
    from_files_in(&SourceLang::default(), files, data, display, rules)
}

/// Validates the languages made of `files`, for a game written in
/// `source`: the source's screen text, `<code>/ui.ron` (with its keys'
/// places, [`english_and_places_from_source`]), and a pack for every other
/// folder, named by its tag, which needs its `lang.ron` and `ui.ron` and
/// may have a `data.ron` and `dialogue/<file>.ron` files (checked against
/// `data`; left out without it). Import settings files are skipped
/// (ADR-0057); any other file is an error, a `lang.ron` in the source's
/// folder too. `display` gives the name a path is reported under. Reports
/// every problem found; with the source's screen text broken the packs are
/// not checked (each entry would be an unknown key).
pub fn from_files_in(
    source: &SourceLang,
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
        if is_settings_file(file.path) {
            continue;
        }
        let known = match file.path.split_once('/') {
            Some((dir, name)) if dir == source.code.as_str() => name == UI_FILE,
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
    let english_path = format!("{}/{UI_FILE}", source.code);
    let english = read(&english_path, &mut errors)
        .map(|source| english_and_places_from_source(&display(&english_path), source));
    let (english, places) = match english {
        Some(Ok((english, places))) => (Some(english), places),
        Some(Err(e)) => {
            errors.extend(e);
            (None, BTreeMap::new())
        }
        None => (None, BTreeMap::new()),
    };
    let mut packs = BTreeMap::new();
    for dir in dirs {
        let code = match pack_code(dir) {
            Ok(code) => code,
            Err(message) => {
                errors.push(ContentError::new(display(dir), message));
                continue;
            }
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
        match pack_from_files_in(&source.code, &sources, english, data.as_ref(), rules) {
            Ok(pack) => {
                packs.insert(code, pack);
            }
            Err(e) => errors.extend(e),
        }
    }
    match english {
        Some(english) if errors.is_empty() => {
            Ok(Lang::with_source(english, data.unwrap_or_default(), packs)
                .with_places(places)
                .with_source_lang(source.clone()))
        }
        _ => Err(errors),
    }
}

#[cfg(test)]
mod chain_tests;
#[cfg(test)]
mod pack_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_read_through_the_registry() {
        use crate::bundle::{Bundle, Layer};
        use crate::format::Formats;
        use crate::ron::Ron;

        static FILES: &[(&str, &[u8])] = &[
            ("bad.ron", b"{ \"a\": 1 }"),
            ("en.ron", b"{ \"title.name\": \"Demo\" }"),
        ];
        let bundle = Bundle::new(vec![Layer::table("files", FILES)]);
        let mut formats = Formats::standard();
        formats.register(Ron::<English>::new()).unwrap();
        let english = formats.load::<English>(&bundle, "en.ron").unwrap();
        assert_eq!(
            english.0.get("title.name").map(String::as_str),
            Some("Demo")
        );
        let errors = formats.load::<English>(&bundle, "bad.ron").unwrap_err();
        assert!(
            errors[0].to_string().starts_with("files/bad.ron:1:"),
            "{errors:?}"
        );
    }

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
        widths: &CellWidths::NARROW,
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
    #[expect(deprecated)]
    fn english_is_the_default_sources_tag() {
        assert_eq!(ENGLISH, LangCode::default().as_str());
        assert_eq!(ENGLISH, SourceLang::default().code.as_str());
    }

    #[test]
    #[expect(deprecated)]
    fn english_of_a_key_is_its_source_text() {
        let lang = lang();
        for key in ["a.one", "a.nope"] {
            assert_eq!(lang.english(key), lang.source_text(key), "{key}");
        }
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
        let error = Pairs::<UiValue>::deserialize(wrong).err().unwrap();
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
        assert_eq!(
            errors,
            ["xx/ui.ron: \"a.nope\": no such key in the source language (en)"]
        );
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
        assert_eq!(lang.text(&LangCode::default(), "a.one"), "One");
        assert_eq!(lang.text(&code("zz"), "a.one"), "One");
        // An unknown key comes back as it is.
        assert_eq!(lang.text(&xx, "a.nope"), "a.nope");
        assert!(lang.has("a.one"));
        assert!(!lang.has("a.nope"));
        assert_eq!(lang.source_text("a.one"), Some("One"));
        assert_eq!(lang.source_text("a.nope"), None);
    }

    #[test]
    fn status_lists_missing_and_stale_keys() {
        let lang = lang();
        let status = lang.status(&code("xx")).unwrap();
        assert_eq!(status.missing, ["a.three"]);
        assert_eq!(status.stale, ["a.two"]);
        assert_eq!(
            lang.status(&LangCode::default()),
            Some(LangStatus::default())
        );
        assert_eq!(lang.status(&code("zz")), None);
        assert_eq!(lang.codes().collect::<Vec<_>>(), [&code("xx")]);
        assert!(lang.pack(&code("xx")).is_some());
        assert!(lang.pack(&LangCode::default()).is_none());
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
    fn a_settings_file_among_the_languages_is_no_error() {
        let read = files(&[
            ("_folder.meta", "()"),
            ("en/ui.ron", ENGLISH_SOURCE),
            ("en/ui.ron.meta", "()"),
            ("xx/_folder.meta", "()"),
            ("xx/lang.ron", INFO),
            ("xx/ui.ron", UI),
        ]);
        assert!(read.is_ok(), "{read:?}");
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
            ["lang/JA: name a language's folder by its tag as BCP 47 writes it: ja"]
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
                "lang/xx/ui.ron: \"a.nope\": no such key in the source language (en)",
                "lang/yy/ui.ron: \"a.nope\": no such key in the source language (en)",
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
    fn width_counts_characters_and_a_wide_one_twice() {
        let narrow = &CellWidths::NARROW;
        assert_eq!(text_width("", narrow), 0);
        assert_eq!(text_width("abc", narrow), 3);
        assert_eq!(text_width("né!", narrow), 3);
        assert_eq!(text_width("あA", narrow), 2);
        let wide = &CellWidths::wide(['あ', 'い']);
        assert_eq!(text_width("あA", wide), 3);
        assert_eq!(text_width("あいう", wide), 5);
    }
}
