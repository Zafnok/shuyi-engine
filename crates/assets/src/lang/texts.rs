//! Every text a [`Lang`] holds, each with where it is from
//! ([`Lang::texts`]): the one scan of the game's text that the font
//! atlas's tool and its check ([`crate::font::undrawn`]) read.
use crate::prelude::*;

use super::{Lang, LangCode};

/// Which part of a language a text is from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum TextPart {
    /// Screen text, by key (English's screen text, a pack's `ui.ron`).
    Screen,
    /// Data text, by key: names (`names.<id>`) and the game's other data
    /// text (a pack's `data.ron`).
    Data,
    /// A dialogue line or caption (a `.dlg` file, a pack's
    /// `dialogue/<file>.ron`).
    Dialogue,
    /// A pack's name for its language (`lang.ron`'s `name`), which the
    /// language menu shows whatever language is on.
    LangName,
}

/// Where a text of [`Lang::texts`] is from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextAt<'a> {
    /// The language: `None` for the source, else the pack's tag.
    pub lang: Option<&'a LangCode>,
    /// The part of the language.
    pub part: TextPart,
    /// Its key: a screen or data text's key, a line id or caption key, and
    /// `name` for a [`TextPart::LangName`].
    pub key: &'a str,
    /// For a dialogue line, the stem of its source line's `.dlg` file
    /// ([`SourceLine::file`](super::SourceLine::file)); empty otherwise,
    /// and for a pack's line the source no longer has.
    pub file: &'a str,
    /// The source language's tag, which names its folder.
    source: &'a LangCode,
}

/// The key a [`TextPart::LangName`] text is listed under.
const NAME_KEY: &str = "name";

impl TextAt<'_> {
    /// The file the text is from, as messages name it. A pack's is its file
    /// under the languages' directory, as [`from_files`](super::from_files)
    /// reads it: `ru/lang.ron` for its name, `ru/ui.ron`, `ru/data.ron`,
    /// and for a line `ru/dialogue/` and its English line's `.dlg` stem
    /// (`ru/dialogue/ch01.ron`; just `ru/dialogue/` for a line the source
    /// no longer has). The source's is a line's `.dlg` file (`ch01.dlg`),
    /// and its folder for the rest (`en`, or the tag of the source the game
    /// declares): a [`Lang`] doesn't keep which file the game read them
    /// from.
    pub fn path(&self) -> String {
        let Some(code) = self.lang else {
            return if self.part == TextPart::Dialogue && !self.file.is_empty() {
                format!("{}.dlg", self.file)
            } else {
                self.source.to_string()
            };
        };
        match self.part {
            TextPart::Screen => format!("{code}/{}", super::UI_FILE),
            TextPart::Data => format!("{code}/{}", super::DATA_FILE),
            TextPart::LangName => format!("{code}/{}", super::INFO_FILE),
            TextPart::Dialogue if self.file.is_empty() => format!("{code}/{}", super::DIALOGUE_DIR),
            TextPart::Dialogue => format!(
                "{code}/{}{}{}",
                super::DIALOGUE_DIR,
                self.file,
                super::DIALOGUE_EXTENSION
            ),
        }
    }
}

impl Lang {
    /// Every text the languages hold, with where each is from. First the
    /// source: its screen text, its data text (names included, as
    /// `names.<id>`), then its dialogue lines and captions, each in key
    /// order. Then each pack in tag order: its name, its screen text, its
    /// data text and its dialogue lines, every variant of a line with
    /// variants ([`LineText::all`](super::LineText::all)).
    pub fn texts(&self) -> Vec<(TextAt<'_>, &str)> {
        let source = &self.written_in.code;
        let at = |lang, part, key, file| TextAt {
            lang,
            part,
            key,
            file,
            source,
        };
        let mut texts = Vec::new();
        for (key, text) in &self.ui {
            texts.push((at(None, TextPart::Screen, key, ""), text.as_str()));
        }
        for (key, text) in &self.source.data {
            texts.push((at(None, TextPart::Data, key, ""), text.as_str()));
        }
        for (key, line) in &self.source.lines {
            let file = line.file.as_str();
            texts.push((at(None, TextPart::Dialogue, key, file), line.text.as_str()));
        }
        for (code, pack) in &self.packs {
            let lang = Some(code);
            texts.push((
                at(lang, TextPart::LangName, NAME_KEY, ""),
                pack.info.name.as_str(),
            ));
            for (key, entry) in &pack.ui {
                texts.push((at(lang, TextPart::Screen, key, ""), entry.text.as_str()));
            }
            for (key, entry) in &pack.data {
                texts.push((at(lang, TextPart::Data, key, ""), entry.text.as_str()));
            }
            for (key, entry) in &pack.lines {
                let file = self.source.lines.get(key).map_or("", |l| l.file.as_str());
                for text in entry.text.all() {
                    texts.push((at(lang, TextPart::Dialogue, key, file), text));
                }
            }
        }
        texts
    }
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;

    use super::*;
    use crate::lang::{Entry, LangInfo, LangPack, LineEntry, LineText, MadeBy};
    use crate::lang::{LineKind, SourceLang, SourceLine, SourceText};
    use crate::names::Names;

    fn owned(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|&(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    }

    fn entry(key: &str, text: &str) -> (String, Entry) {
        let entry = Entry {
            key: key.to_owned(),
            source: String::new(),
            text: text.to_owned(),
        };
        (key.to_owned(), entry)
    }

    fn line(key: &str, text: LineText) -> (String, LineEntry) {
        let source = String::new();
        let key = key.to_owned();
        (key.clone(), LineEntry { key, source, text })
    }

    /// English with a screen text, a name and a line of `gate.dlg`, and the
    /// pack `ru` with one of each, the line with two variants, and a line
    /// English no longer has.
    fn lang() -> Lang {
        let names = Names {
            names: owned(&[("king", "Osgar")]),
        };
        let mut source = SourceText::new(owned(&[("items.rope", "Rope")]), names);
        let english_line = SourceLine {
            scene: "gate".to_owned(),
            text: "Halt.".to_owned(),
            kind: LineKind::Speech,
            file: "gate".to_owned(),
        };
        source.lines.insert("gate.l1".to_owned(), english_line);
        let pack = LangPack {
            info: LangInfo {
                name: "Русский".to_owned(),
                made_by: MadeBy::Machine,
            },
            ui: [entry("menu.title", "Жa")].into(),
            data: [entry("names.king", "Осгар")].into(),
            lines: [
                line(
                    "gate.l1",
                    LineText::Variants(owned(&[("a", "Стой."), ("b", "Стой!")])),
                ),
                line("gate.old", LineText::One("Был.".to_owned())),
            ]
            .into(),
        };
        let code = LangCode::new("ru").unwrap();
        Lang::with_source(
            owned(&[("menu.title", "Start")]),
            source,
            [(code, pack)].into(),
        )
    }

    #[test]
    fn texts_list_english_then_each_pack_with_part_key_and_file() {
        use TextPart::{Data, Dialogue, LangName, Screen};
        let lang = lang();
        let ru = LangCode::new("ru").unwrap();
        let found: Vec<(Option<&str>, TextPart, &str, &str, &str)> = lang
            .texts()
            .into_iter()
            .map(|(at, text)| {
                (
                    at.lang.map(LangCode::as_str),
                    at.part,
                    at.key,
                    at.file,
                    text,
                )
            })
            .collect();
        let (en, ru) = (None, Some(ru.as_str()));
        assert_eq!(
            found,
            [
                (en, Screen, "menu.title", "", "Start"),
                (en, Data, "items.rope", "", "Rope"),
                (en, Data, "names.king", "", "Osgar"),
                (en, Dialogue, "gate.l1", "gate", "Halt."),
                (ru, LangName, "name", "", "Русский"),
                (ru, Screen, "menu.title", "", "Жa"),
                (ru, Data, "names.king", "", "Осгар"),
                (ru, Dialogue, "gate.l1", "gate", "Стой."),
                (ru, Dialogue, "gate.l1", "gate", "Стой!"),
                (ru, Dialogue, "gate.old", "", "Был."),
            ]
        );
        assert_eq!(Lang::default().texts(), []);
    }

    #[test]
    fn each_text_names_the_file_it_is_from() {
        let lang = lang();
        let paths: Vec<String> = lang.texts().iter().map(|(at, _)| at.path()).collect();
        assert_eq!(
            paths,
            [
                "en",
                "en",
                "en",
                "gate.dlg",
                "ru/lang.ron",
                "ru/ui.ron",
                "ru/data.ron",
                "ru/dialogue/gate.ron",
                "ru/dialogue/gate.ron",
                "ru/dialogue/",
            ]
        );
        // A source line not read from a file is the source's too.
        let en = LangCode::default();
        let unfiled = TextAt {
            lang: None,
            part: TextPart::Dialogue,
            key: "a.l1",
            file: "",
            source: &en,
        };
        assert_eq!(unfiled.path(), "en");
    }

    #[test]
    fn a_declared_sources_texts_name_its_folder() {
        let ja = SourceLang::new(LangCode::new("ja").unwrap(), "日本語");
        let lang = lang().with_source_lang(ja);
        let paths: Vec<String> = lang.texts().iter().map(|(at, _)| at.path()).collect();
        assert_eq!(paths[..4], ["ja", "ja", "ja", "gate.dlg"]);
    }
}
