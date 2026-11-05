//! What a key of English's screen text says about the place its text is
//! drawn in ([`Place`]): how many cells wide it is (`max_cells`), and what
//! a translator should know (`note`: where the text shows, what a
//! placeholder holds). `en/ui.ron` writes them in place of the key's text:
//!
//! ```ron
//! {
//!     "title.start": (text: "Start", max_cells: 12, note: "The title menu's first item."),
//!     "title.quit": "Quit",
//! }
//! ```
//!
//! A pack sets neither: they describe the place and the English, not the
//! translation. [`Lang::overflow`] lists a pack's entries too wide for
//! their place; the game never shows a note.
use crate::prelude::*;

use alloc::collections::BTreeMap;
use core::fmt;

use serde::Deserialize;
use serde::de::value::MapAccessDeserializer;
use serde::de::{Deserializer, MapAccess, Visitor};

use super::{Lang, LangCode, text_width};
use crate::font::CellWidths;

/// What English's screen text says about where a key's text is drawn.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Place {
    /// How many cells wide the place is; `None` for no limit.
    pub max_cells: Option<usize>,
    /// What a translator should know about the text, never shown in the
    /// game.
    pub note: Option<String>,
}

/// Where English's screen text is drawn, by key.
pub type Places = BTreeMap<String, Place>;

/// A value of `en/ui.ron`: the text, alone or with its place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UiValue {
    pub(super) text: String,
    pub(super) place: Place,
}

/// The written form of a text with its place.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Placed {
    text: String,
    #[serde(default)]
    max_cells: Option<usize>,
    #[serde(default)]
    note: Option<String>,
}

impl<'de> Deserialize<'de> for UiValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = UiValue;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a text, or (text: …, max_cells: …, note: …)")
            }

            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<UiValue, E> {
                Ok(UiValue {
                    text: text.to_owned(),
                    place: Place::default(),
                })
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<UiValue, A::Error> {
                let placed = Placed::deserialize(MapAccessDeserializer::new(map))?;
                Ok(UiValue {
                    text: placed.text,
                    place: Place {
                        max_cells: placed.max_cells,
                        note: placed.note,
                    },
                })
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

/// A pack's entry wider than its key's place ([`Lang::overflow`]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Overflow {
    /// The entry's key.
    pub key: String,
    /// How many cells its place has ([`Place::max_cells`]).
    pub max_cells: usize,
    /// How many cells its text takes ([`text_width`]).
    pub width: usize,
    /// Its text.
    pub text: String,
}

impl Lang {
    /// These languages, English's screen text drawn in `places` (by key).
    #[must_use]
    pub fn with_places(mut self, places: Places) -> Self {
        self.places = places;
        self
    }

    /// What English says about where `key`'s text is drawn, if anything.
    pub fn place(&self, key: &str) -> Option<&Place> {
        self.places.get(key)
    }

    /// How many cells wide `key`'s place is, if English says.
    pub fn max_cells(&self, key: &str) -> Option<usize> {
        self.place(key).and_then(|p| p.max_cells)
    }

    /// What a translator should know about `key`'s text, if English says.
    pub fn note(&self, key: &str) -> Option<&str> {
        self.place(key).and_then(|p| p.note.as_deref())
    }

    /// The screen-text entries of `code`'s pack wider than their key's
    /// [`max_cells`](Self::max_cells), measured with `widths`, in key
    /// order. Every entry is measured, stale ones too, as its text is
    /// written (placeholders unfilled). `None` for a code with no pack.
    pub fn overflow(&self, code: &LangCode, widths: &CellWidths) -> Option<Vec<Overflow>> {
        let pack = self.packs.get(code)?;
        let too_wide = |(key, entry): (&String, &super::Entry)| {
            let max_cells = self.max_cells(key)?;
            let width = text_width(&entry.text, widths);
            (width > max_cells).then(|| Overflow {
                key: key.clone(),
                max_cells,
                width,
                text: entry.text.clone(),
            })
        };
        Some(pack.ui.iter().filter_map(too_wide).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        LangFile, PackRules, english_and_places_from_source, english_from_source, from_files,
        pack_from_sources,
    };
    use super::*;

    const SOURCE_UI: &str = r#"{
        "title.start": (text: "Start", max_cells: 8, note: "The title menu's first item."),
        "title.quit": (text: "Quit", max_cells: 6),
        "title.help": (text: "Press {Confirm}", note: "{Confirm} is a key's name."),
        "title.name": "Demo",
    }"#;

    const RULES: PackRules<'static> = PackRules {
        data_problem: |_, _| None,
        tokens: &crate::dialogue::NoTokens,
        limits: crate::dialogue::tests::LIMITS,
        widths: &CellWidths::NARROW,
        variants: &[],
    };

    fn errors(source: &str) -> Vec<String> {
        english_and_places_from_source("en/ui.ron", source)
            .unwrap_err()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn a_key_with_max_cells_and_a_note_loads() {
        let (english, places) = english_and_places_from_source("en/ui.ron", SOURCE_UI).unwrap();
        assert_eq!(english["title.start"], "Start");
        assert_eq!(english["title.name"], "Demo");
        assert_eq!(places.len(), 3, "{places:?}");
        let lang = Lang::new(english, BTreeMap::new()).with_places(places);
        assert_eq!(
            lang.note("title.start"),
            Some("The title menu's first item.")
        );
        assert_eq!(lang.max_cells("title.start"), Some(8));
        assert_eq!(lang.note("title.quit"), None);
        assert_eq!(lang.max_cells("title.quit"), Some(6));
        assert_eq!(lang.max_cells("title.help"), None);
        assert_eq!(lang.note("title.name"), None);
        assert_eq!(lang.place("title.name"), None);
        assert_eq!(lang.place("no.such"), None);
        // Text alone reads the same as before.
        assert_eq!(
            english_from_source("en/ui.ron", SOURCE_UI).unwrap().len(),
            4
        );
    }

    #[test]
    fn a_place_must_say_something_sensible() {
        assert_eq!(
            errors(r#"{"a": (text: "A", max_cells: 0), "b": (text: "B", note: " ")}"#),
            [
                "en/ui.ron: \"a\": max_cells must be at least 1",
                "en/ui.ron: \"b\": a note must not be empty",
            ]
        );
        let unknown = errors(r#"{"a": (text: "A", width: 3)}"#);
        assert!(unknown[0].contains("width"), "{unknown:?}");
        let no_text = errors(r#"{"a": (max_cells: 3)}"#);
        assert!(no_text[0].contains("text"), "{no_text:?}");
        let wrong = errors(r#"{"a": 3}"#);
        assert!(wrong[0].contains("a text, or (text:"), "{wrong:?}");
    }

    #[test]
    fn a_pack_sets_neither() {
        let (english, _) = english_and_places_from_source("en/ui.ron", SOURCE_UI).unwrap();
        let info = r#"(name: "X", made_by: Machine)"#;
        let ui = r#"[(key: "title.quit", source: "Quit", text: "Q", max_cells: 9)]"#;
        let errors = pack_from_sources(("xx/lang.ron", info), ("xx/ui.ron", ui), &english, &RULES)
            .unwrap_err();
        assert!(errors[0].to_string().contains("max_cells"), "{errors:?}");
    }

    #[test]
    fn overflow_lists_the_entries_wider_than_their_place() {
        let ui = r#"[
            (key: "title.start", source: "Start", text: "はじめから"),
            (key: "title.quit", source: "Quit", text: "Beenden!"),
            (key: "title.help", source: "Press {Confirm}", text: "Drücke {Confirm} lange"),
            (key: "title.name", source: "Demo", text: "Démonstration"),
        ]"#;
        let files = [
            ("en/ui.ron", SOURCE_UI),
            ("xx/lang.ron", r#"(name: "X", made_by: Machine)"#),
            ("xx/ui.ron", ui),
        ]
        .map(|(path, source)| LangFile {
            path,
            source: Some(source),
        });
        let lang = from_files(&files, None, str::to_owned, &RULES).unwrap();
        let xx = LangCode::new("xx").unwrap();
        let overflow = |widths: &CellWidths| lang.overflow(&xx, widths).unwrap();
        // Narrow, `はじめから` is 5 cells and fits 8; `Beenden!` is 8 > 6.
        let narrow = overflow(&CellWidths::NARROW);
        assert_eq!(
            narrow,
            [Overflow {
                key: "title.quit".to_owned(),
                max_cells: 6,
                width: 8,
                text: "Beenden!".to_owned(),
            }]
        );
        // Wide, it is 10 cells: too wide too.
        let wide = overflow(&CellWidths::wide("はじめから".chars()));
        let keys: Vec<(&str, usize)> = wide.iter().map(|o| (o.key.as_str(), o.width)).collect();
        assert_eq!(keys, [("title.quit", 8), ("title.start", 10)]);
        // Exactly as wide as its place, it fits: 8 cells of 8.
        let exact = overflow(&CellWidths::wide("はじめ".chars()));
        let keys: Vec<&str> = exact.iter().map(|o| o.key.as_str()).collect();
        assert_eq!(keys, ["title.quit"]);
        assert_eq!(
            lang.overflow(&LangCode::default(), &CellWidths::NARROW),
            None
        );
    }
}
