//! A pack's dialogue text (`dialogue/<file>.ron`): one entry per line id or
//! caption key. Rewording an English line gives it a new id, so the old
//! entry becomes an **orphan**: it fails nothing, and [`Lang::status`]
//! pairs it with the new line it most likely belongs to.
use crate::prelude::*;

use alloc::collections::BTreeMap;
use core::fmt;

use serde::de::DeserializeSeed;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

use super::source::{LineKind, SourceText};
use super::{Lang, LangCode, LangPack, PackRules};
use crate::dialogue::{
    ChoiceOption, DialogueLimits, LineId, Scene, Step, caption_key, longest_width, scene_of,
    token_problems,
};
use crate::error::ContentError;
use crate::names::Names;

/// What every caption key starts with.
const CAPTION_PREFIX: &str = "caption.";

/// A dialogue entry as written: `key`, `source`, and `text` or a
/// `text_<variant>` for each variant.
struct RawLine {
    key: String,
    source: String,
    text: Option<String>,
    /// The `text_<variant>` fields, by variant.
    variants: BTreeMap<String, String>,
}

/// Reads a [`RawLine`] whose variants may be any of `variants`.
struct RawLineSeed<'a> {
    variants: &'a [&'a str],
}

impl<'de> DeserializeSeed<'de> for RawLineSeed<'_> {
    type Value = RawLine;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<RawLine, D::Error> {
        deserializer.deserialize_struct("RawLine", &["key", "source", "text"], self)
    }
}

impl<'de> Visitor<'de> for RawLineSeed<'_> {
    type Value = RawLine;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a dialogue entry: key, source and text")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<RawLine, A::Error> {
        let mut fields: BTreeMap<String, String> = BTreeMap::new();
        while let Some(name) = map.next_key::<String>()? {
            let known = matches!(name.as_str(), "key" | "source" | "text")
                || name
                    .strip_prefix("text_")
                    .is_some_and(|v| self.variants.contains(&v));
            if !known {
                let mut expected = vec!["`key`".to_owned(), "`source`".to_owned()];
                expected.push("`text`".to_owned());
                expected.extend(self.variants.iter().map(|v| format!("`text_{v}`")));
                return Err(de::Error::custom(format!(
                    "unknown field `{name}`, expected one of {}",
                    expected.join(", ")
                )));
            }
            if fields.contains_key(&name) {
                return Err(de::Error::custom(format!("duplicate field `{name}`")));
            }
            let value = map.next_value::<String>()?;
            fields.insert(name, value);
        }
        let mut take = |name: &'static str| fields.remove(name);
        let key = take("key").ok_or_else(|| de::Error::missing_field("key"))?;
        let source = take("source").ok_or_else(|| de::Error::missing_field("source"))?;
        let text = take("text");
        let variants = fields
            .into_iter()
            .map(|(name, text)| (name["text_".len()..].to_owned(), text))
            .collect();
        Ok(RawLine {
            key,
            source,
            text,
            variants,
        })
    }
}

/// Reads a file of [`RawLine`]s.
struct RawLines<'a> {
    variants: &'a [&'a str],
}

impl<'de> DeserializeSeed<'de> for RawLines<'_> {
    type Value = Vec<RawLine>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Vec<RawLine>, D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for RawLines<'_> {
    type Value = Vec<RawLine>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a list of dialogue entries")
    }

    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<RawLine>, A::Error> {
        let mut lines = Vec::new();
        let seed = || RawLineSeed {
            variants: self.variants,
        };
        while let Some(line) = seq.next_element_seed(seed())? {
            lines.push(line);
        }
        Ok(lines)
    }
}

/// A line in a pack's language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineText {
    /// The same whichever variant is played (`text`).
    One(String),
    /// One per variant the game names (`text_<variant>`), for a language
    /// where the line changes with more than a token (a hero's gender,
    /// say), by variant name.
    Variants(BTreeMap<String, String>),
}

impl LineText {
    /// The text of `variant`: the one text, else the variant's. `None` if
    /// the line has variants and `variant` isn't one of them.
    pub fn get(&self, variant: &str) -> Option<&str> {
        match self {
            LineText::One(text) => Some(text),
            LineText::Variants(texts) => texts.get(variant).map(String::as_str),
        }
    }

    /// Every text of the entry.
    pub fn all(&self) -> Vec<&str> {
        match self {
            LineText::One(text) => vec![text],
            LineText::Variants(texts) => texts.values().map(String::as_str).collect(),
        }
    }
}

/// One dialogue line or caption of a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineEntry {
    /// The line id, or the caption key.
    pub key: String,
    /// The English the text was made from.
    pub source: String,
    /// The text in the pack's language.
    pub text: LineText,
}

/// A pack entry for a dialogue line that English no longer has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orphan {
    /// The entry's key: the id the line had.
    pub key: String,
    /// The English it was made from.
    pub source: String,
    /// Its translation.
    pub text: LineText,
    /// The key of the line of the same scene, of the same kind and without
    /// an entry, whose English is nearest to `source`: most likely the
    /// reworded line. `None` if the scene has no such line.
    pub nearest: Option<String>,
}

/// The id of the scene the line id or caption key `key` belongs to, or
/// `None` if `key` is neither.
fn scene_of_key(key: &str) -> Option<&str> {
    scene_of(key.strip_prefix(CAPTION_PREFIX).unwrap_or(key))
}

/// Whether `key` is a caption key.
fn is_caption(key: &str) -> bool {
    key.starts_with(CAPTION_PREFIX)
}

/// The most cells a line of `kind` may take under `limits`, if it has a
/// limit.
fn limit(kind: LineKind, limits: &DialogueLimits) -> Option<usize> {
    match kind {
        LineKind::Speech => Some(limits.max_text),
        LineKind::Reply => Some(limits.max_option),
        LineKind::Caption => None,
    }
}

/// What an entry with neither one `text` nor every variant is told (an
/// entry has `text`, or both `text_a` and `text_b`).
fn variants_message(variants: &[&str]) -> String {
    let fields: Vec<String> = variants.iter().map(|v| format!("`text_{v}`")).collect();
    let all = match fields.as_slice() {
        [] => return "an entry has `text`".to_owned(),
        [one] => one.clone(),
        [a, b] => format!("both {a} and {b}"),
        [rest @ .., last] => format!("all of {}, and {last}", rest.join(", ")),
    };
    format!("an entry has `text`, or {all}")
}

/// Parses and validates the dialogue entries of the pack file `file`
/// (`source`), adding them to `lines` and every problem to `errors`.
///
/// An entry has `text`, or a `text_<variant>` for every one of `rules`'
/// variants. Its key is a line id or a caption key, used once in the pack.
/// Its text follows the token rule of English dialogue (only the game's
/// tokens and name tokens with an id in `data`'s names table). When English
/// has the line, the text also keeps to the line's length limit, counted in
/// cells, with every name token as long as the longest of `names` (the
/// pack's).
pub(super) fn add_lines(
    (file, source): (&str, &str),
    data: &SourceText,
    names: &Names,
    rules: &PackRules<'_>,
    lines: &mut BTreeMap<String, LineEntry>,
    errors: &mut Vec<ContentError>,
) {
    let seed = RawLines {
        variants: rules.variants,
    };
    let raw = crate::ron::parse_ron_seed(file, source, seed)
        .map_err(|e| errors.push(e))
        .unwrap_or_default();
    for entry in raw {
        let mut err = |message: String| {
            errors.push(ContentError::new(
                file,
                format!("\"{}\": {message}", entry.key),
            ));
        };
        if scene_of_key(&entry.key).is_none() {
            err("not a line id or a caption key".to_owned());
        }
        if lines.contains_key(&entry.key) {
            err("the key is used twice".to_owned());
        }
        let every = entry.variants.len() == rules.variants.len();
        let text = match (entry.text, entry.variants.is_empty()) {
            (Some(text), true) => LineText::One(text),
            (None, false) if every => LineText::Variants(entry.variants),
            _ => {
                err(variants_message(rules.variants));
                continue;
            }
        };
        let english = data.lines.get(&entry.key);
        for text in text.all() {
            if text.trim().is_empty() {
                err("the text must not be empty".to_owned());
            }
            for problem in token_problems(text, rules.tokens, Some(&data.names)) {
                err(problem);
            }
            let width = longest_width(text, rules.tokens, Some(names.longest()));
            match english.and_then(|line| limit(line.kind, &rules.limits)) {
                Some(limit) if width > limit => {
                    err(format!("text is {width} cells; the limit is {limit}"));
                }
                _ => {}
            }
        }
        let entry = LineEntry {
            key: entry.key,
            source: entry.source,
            text,
        };
        lines.insert(entry.key.clone(), entry);
    }
}

/// How many single-character edits turn `a` into `b` (Levenshtein).
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let substituted = diagonal + usize::from(ca != cb);
            diagonal = row[j + 1];
            row[j + 1] = substituted.min(diagonal + 1).min(row[j] + 1);
        }
    }
    row[b.len()]
}

impl LangPack {
    /// The pack's entry for the dialogue line or caption `key`, if it has
    /// one.
    pub fn line(&self, key: &str) -> Option<&LineEntry> {
        self.lines.get(key)
    }

    /// The pack's dialogue entries English has no line for, each paired
    /// with the nearest untranslated line of its scene, in key order.
    pub(super) fn orphans(&self, data: &SourceText) -> Vec<Orphan> {
        let orphan = |entry: &LineEntry| {
            let scene = scene_of_key(&entry.key).unwrap_or_default();
            let caption = is_caption(&entry.key);
            let nearest = data
                .lines
                .iter()
                .filter(|(key, line)| {
                    line.scene == scene
                        && (line.kind == LineKind::Caption) == caption
                        && !self.lines.contains_key(*key)
                })
                .min_by_key(|(_, line)| distance(&entry.source, &line.text))
                .map(|(key, _)| key.clone());
            Orphan {
                key: entry.key.clone(),
                source: entry.source.clone(),
                text: entry.text.clone(),
                nearest,
            }
        };
        self.lines
            .values()
            .filter(|entry| !data.lines.contains_key(&entry.key))
            .map(orphan)
            .collect()
    }
}

impl Lang {
    /// The dialogue line or caption `key` in language `code`, as played
    /// for `variant`: the pack's text when its entry was made from exactly
    /// `english`; otherwise `english`.
    pub fn line<'a>(
        &'a self,
        code: &LangCode,
        key: &str,
        english: &'a str,
        variant: &str,
    ) -> &'a str {
        let entry = self.packs.get(code).and_then(|pack| pack.lines.get(key));
        match entry {
            Some(entry) if entry.source == english => entry.text.get(variant).unwrap_or(english),
            _ => english,
        }
    }

    /// `scene` in language `code`, as played for `variant`: every line,
    /// reply and caption the pack translates in its language, the others
    /// in English. The lines keep their ids (a voice clip is found by
    /// them) and their tokens.
    #[must_use]
    pub fn scene<D: Clone>(&self, code: &LangCode, scene: &Scene<D>, variant: &str) -> Scene<D> {
        let mut scene = scene.clone();
        if self.packs.contains_key(code) {
            let id = scene.id.clone();
            self.translate(code, &id, &mut scene.steps, variant);
        }
        scene
    }

    /// Puts the steps `steps` of scene `scene` in language `code`.
    fn translate<D>(&self, code: &LangCode, scene: &str, steps: &mut [Step<D>], variant: &str) {
        let line = |id: &LineId, text: &mut String| {
            let shown = self.line(code, id.as_str(), text, variant).to_owned();
            *text = shown;
        };
        for step in steps {
            match step {
                Step::Caption { text } => {
                    let shown = self.line(code, &caption_key(scene, text), text, variant);
                    *text = shown.to_owned();
                }
                Step::Say { text, line: id, .. } | Step::Narrate { text, line: id } => {
                    line(id, text);
                }
                Step::Choice { options } => {
                    for ChoiceOption {
                        text,
                        line: id,
                        steps,
                        ..
                    } in options
                    {
                        line(id, text);
                        self.translate(code, scene, steps, variant);
                    }
                }
                Step::If {
                    then, otherwise, ..
                } => {
                    self.translate(code, scene, then, variant);
                    self.translate(code, scene, otherwise, variant);
                }
                Step::Place { .. } | Step::Clear { .. } | Step::Music(_) | Step::Directive(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::de::IntoDeserializer;
    use serde::de::value::{BoolDeserializer, Error};

    use super::*;

    /// What a file and an entry are told they should be.
    #[test]
    fn a_wrong_value_is_told_what_was_expected() {
        let wrong = || -> BoolDeserializer<Error> { true.into_deserializer() };
        let file = RawLines { variants: &[] }.deserialize(wrong());
        assert_eq!(
            file.err().map(|e| e.to_string()).as_deref(),
            Some("invalid type: boolean `true`, expected a list of dialogue entries")
        );
        let entry = RawLineSeed { variants: &[] }.deserialize(wrong());
        assert_eq!(
            entry.err().map(|e| e.to_string()).as_deref(),
            Some("invalid type: boolean `true`, expected a dialogue entry: key, source and text")
        );
    }

    #[test]
    fn distance_counts_single_character_edits() {
        assert_eq!(distance("", ""), 0);
        assert_eq!(distance("abc", "abc"), 0);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("abc", ""), 3);
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(distance("flaw", "lawn"), 2);
        assert_eq!(distance("abc", "cab"), 2);
        assert_eq!(distance("abcdef", "abdef"), 1);
        assert_eq!(distance("abdef", "abcdef"), 1);
        // Characters, not bytes.
        assert_eq!(distance("né", "ne"), 1);
    }

    #[test]
    fn a_key_names_its_scene() {
        assert_eq!(scene_of_key("ch1_gate_0123abcd"), Some("ch1_gate"));
        assert_eq!(scene_of_key("ch1_gate_0123abcd_12"), Some("ch1_gate"));
        assert_eq!(scene_of_key("caption.ch1_gate_0123abcd"), Some("ch1_gate"));
        assert_eq!(scene_of_key("ch1_gate"), None);
        assert_eq!(scene_of_key("caption.ch1_gate"), None);
        assert!(is_caption("caption.a_0123abcd"));
        assert!(!is_caption("a_0123abcd"));
    }

    #[test]
    fn only_text_boxes_and_replies_have_a_limit() {
        let limits = crate::dialogue::tests::LIMITS;
        assert_eq!(limit(LineKind::Speech, &limits), Some(limits.max_text));
        assert_eq!(limit(LineKind::Reply, &limits), Some(limits.max_option));
        assert_eq!(limit(LineKind::Caption, &limits), None);
    }

    #[test]
    fn the_message_names_every_variant() {
        assert_eq!(variants_message(&[]), "an entry has `text`");
        assert_eq!(variants_message(&["a"]), "an entry has `text`, or `text_a`");
        assert_eq!(
            variants_message(&["m", "f"]),
            "an entry has `text`, or both `text_m` and `text_f`"
        );
        assert_eq!(
            variants_message(&["a", "b", "c"]),
            "an entry has `text`, or all of `text_a`, `text_b`, and `text_c`"
        );
    }

    #[test]
    fn a_line_has_its_one_text_or_its_variants() {
        let one = LineText::One("x".to_owned());
        assert_eq!(one.get("a"), Some("x"));
        assert_eq!(one.all(), ["x"]);
        let texts = [("a", "1"), ("b", "2")].map(|(v, t)| (v.to_owned(), t.to_owned()));
        let both = LineText::Variants(texts.into());
        assert_eq!(both.get("a"), Some("1"));
        assert_eq!(both.get("b"), Some("2"));
        assert_eq!(both.get("c"), None);
        assert_eq!(both.all(), ["1", "2"]);
    }
}
