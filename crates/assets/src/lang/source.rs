//! The English a pack's `data.ron` and `dialogue/` files translate: the
//! game's data text by key, every dialogue line and caption by key, and the
//! names table ([`SourceText`]); and the lookups that give each in a
//! language. English stays in the game's files; a lookup is handed today's
//! English and returns the pack's text only when its entry was made from
//! exactly that.
use crate::prelude::*;

use alloc::collections::BTreeMap;

use super::{Lang, LangCode};
use crate::dialogue::{DialogueTable, REPLY_SPEAKER, Step, caption_key};
use crate::names::Names;

/// What every key of a name of the names table starts with.
pub(super) const NAMES_PREFIX: &str = "names.";

/// The key of the name `id` of the names table: `names.<id>`.
pub fn names_key(id: &str) -> String {
    format!("{NAMES_PREFIX}{id}")
}

/// What kind of dialogue text a key is for: its length limit depends on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// A speech or narration line (one text box).
    Speech,
    /// One of the replies the player picks from (a menu row).
    Reply,
    /// A `@caption`.
    Caption,
}

/// One dialogue line or caption in English.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLine {
    /// The id of its scene.
    pub scene: String,
    /// Its text as written (tokens unexpanded).
    pub text: String,
    /// What it is.
    pub kind: LineKind,
    /// The stem of the `.dlg` file its scene is in (`ch01` for
    /// `ch01.dlg`); empty if the dialogue wasn't read from files.
    pub file: String,
}

/// Today's English for everything `data.ron` and `dialogue/*.ron` can
/// translate, by key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceText {
    /// Data text by key: `names.<id>` for the names table, and whatever
    /// else the game hands over (`items.<id>.name`, say).
    pub data: BTreeMap<String, String>,
    /// Dialogue lines by line id, and captions by [`caption_key`].
    pub lines: BTreeMap<String, SourceLine>,
    /// The names table: what a name token in a pack's text may refer to.
    pub names: Names,
}

/// `ch01` for `story/ch01.dlg`.
fn file_stem(path: &str) -> &str {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.strip_suffix(".dlg").unwrap_or(name)
}

/// Adds the captions of `steps` (of scene `scene`) to `lines`.
fn add_captions<D>(
    scene: &str,
    file: &str,
    steps: &[Step<D>],
    lines: &mut BTreeMap<String, SourceLine>,
) {
    for step in steps {
        match step {
            Step::Caption { text } => {
                let line = SourceLine {
                    scene: scene.to_owned(),
                    text: text.clone(),
                    kind: LineKind::Caption,
                    file: file.to_owned(),
                };
                lines.insert(caption_key(scene, text), line);
            }
            Step::Choice { options } => {
                for option in options {
                    add_captions(scene, file, &option.steps, lines);
                }
            }
            Step::If {
                then, otherwise, ..
            } => {
                add_captions(scene, file, then, lines);
                add_captions(scene, file, otherwise, lines);
            }
            Step::Place { .. }
            | Step::Clear { .. }
            | Step::Say { .. }
            | Step::Narrate { .. }
            | Step::Music(_)
            | Step::Directive(_) => {}
        }
    }
}

impl SourceText {
    /// The data text `data` and the names table `names` (whose names are
    /// added to `data` by [`names_key`]), with no dialogue yet.
    pub fn new(mut data: BTreeMap<String, String>, names: Names) -> Self {
        for (id, value) in &names.names {
            data.insert(names_key(id), value.clone());
        }
        Self {
            data,
            lines: BTreeMap::new(),
            names,
        }
    }

    /// Adds every line, reply and caption of `dialogue`.
    pub fn add_dialogue<D>(&mut self, dialogue: &DialogueTable<D>) {
        for scene in dialogue.scenes.values() {
            let file = dialogue
                .files
                .get(&scene.id)
                .map_or("", |path| file_stem(path));
            for line in scene.lines() {
                let kind = if line.speaker == REPLY_SPEAKER {
                    LineKind::Reply
                } else {
                    LineKind::Speech
                };
                let source = SourceLine {
                    scene: scene.id.clone(),
                    text: line.text.to_owned(),
                    kind,
                    file: file.to_owned(),
                };
                self.lines.insert(line.id.as_str().to_owned(), source);
            }
            add_captions(&scene.id, file, &scene.steps, &mut self.lines);
        }
    }
}

impl Lang {
    /// `english` in language `code`: the pack's `data.ron` text for the key
    /// `key` gives, when its entry was made from exactly `english`;
    /// otherwise `english`. The key is only built when `code` has a pack.
    pub fn data_text<'a>(
        &'a self,
        code: &LangCode,
        key: impl FnOnce() -> String,
        english: &'a str,
    ) -> &'a str {
        let Some(pack) = self.packs.get(code) else {
            return english;
        };
        match pack.data.get(&key()) {
            Some(entry) if entry.source == english => &entry.text,
            _ => english,
        }
    }

    /// The display name `id` of the names table in language `code`;
    /// `english` is its value in the table.
    pub fn name<'a>(&'a self, code: &LangCode, id: &str, english: &'a str) -> &'a str {
        self.data_text(code, || names_key(id), english)
    }

    /// The names table in language `code`: each name the pack translates
    /// in its language, the others in English. What a name token in a
    /// line shown in `code` is filled from.
    pub fn names(&self, code: &LangCode, names: &Names) -> Names {
        if !self.packs.contains_key(code) {
            return names.clone();
        }
        let translated = |(id, english): (&String, &String)| {
            (id.clone(), self.name(code, id, english).to_owned())
        };
        Names {
            names: names.names.iter().map(translated).collect(),
        }
    }

    /// `english` in language `code`, for a text whose key isn't known
    /// where it is shown: the pack's text for a key starting with `prefix`
    /// whose English is `english` today and was when the entry was made;
    /// otherwise `english`.
    pub fn found<'a>(&'a self, code: &LangCode, prefix: &str, english: &'a str) -> &'a str {
        let Some(pack) = self.packs.get(code) else {
            return english;
        };
        let end = format!("{prefix}\u{10ffff}");
        self.source
            .data
            .range(prefix.to_owned()..end)
            .filter(|&(_, today)| today == english)
            .find_map(|(key, today)| pack.data.get(key).filter(|e| e.source == *today))
            .map_or(english, |entry| entry.text.as_str())
    }
}
