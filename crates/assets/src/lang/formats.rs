//! A language pack as a format (ADR-0007): read from its `lang.ron`, with
//! the other files of its folder and the source's screen text beside it.
use crate::prelude::*;

use alloc::collections::BTreeSet;

use super::{
    DATA_FILE, INFO_FILE, LangPack, PackRules, PackSources, SourceLang, SourceText, UI_FILE,
    english_from_source, is_dialogue_file, pack_code, pack_from_files_in,
};
use crate::dialogue::{DialogueLimits, Tokens};
use crate::error::ContentError;
use crate::font::{CellWidths, FontAtlasDef};
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, join, text};
use crate::import_settings::is_settings_file;

impl AssetKind for LangPack {
    const NAME: &'static str = "language pack";
}

/// What a file a language doesn't have is told.
const NOT_A_LANGUAGE_FILE: &str = "not a file a language has";

/// A language pack (`lang-pack`, `.ron`), read from its `lang.ron` in its
/// folder (`lang/pt-BR/lang.ron`), which is named by its tag as BCP 47
/// writes it: its `ui.ron` beside it, checked against the source's
/// (`lang/en/ui.ron`, or the folder of the source the game declares,
/// [`LangPackRon::source_lang`]), and its `data.ron` and
/// `dialogue/*.ron`, checked against the game's data text when it gives it
/// ([`LangPackRon::with_data`]; left out otherwise). Any other file in the
/// folder is an error, and the source's folder is no pack. Checked as
/// [`pack_from_files`](super::pack_from_files) says, with the game's
/// rules, its limits counting a character of the font atlas's wide glyphs
/// as two cells when the game gives the atlas
/// ([`LangPackRon::widths_of`]), as the pack has them
/// ([`LangInfo::widths`](super::LangInfo::widths)); with the atlas, the
/// wide set its `wide_font` names must be one the atlas has.
pub struct LangPackRon {
    data_problem: fn(&str, &str) -> Option<String>,
    tokens: &'static (dyn Tokens + Sync),
    limits: DialogueLimits,
    /// The characters that take two cells ([`CellWidths`], which can't be
    /// shared between threads, is made of them as a pack is read).
    wide: BTreeSet<char>,
    /// The atlas's named wide sets, when the game gives the atlas.
    sets: Option<BTreeSet<String>>,
    variants: &'static [&'static str],
    data: Option<SourceText>,
    source: SourceLang,
}

impl LangPackRon {
    /// Packs whose dialogue fits `limits` and may use the game's `tokens`,
    /// with no rules of the game's for its data text, no variants, no data
    /// text to check theirs against, and English as the source.
    pub fn new(limits: DialogueLimits, tokens: &'static (dyn Tokens + Sync)) -> Self {
        Self {
            data_problem: |_, _| None,
            tokens,
            limits,
            wide: BTreeSet::new(),
            sets: None,
            variants: &[],
            data: None,
            source: SourceLang::default(),
        }
    }

    /// These packs, of a game written in `source`: checked against its
    /// screen text (`<languages>/<code>/ui.ron`), its folder no pack.
    #[must_use]
    pub fn source_lang(self, source: SourceLang) -> Self {
        Self { source, ..self }
    }

    /// These packs, their limits counting a character with a wide glyph in
    /// `atlas` as two cells ([`CellWidths::of`]), as the screen prints it,
    /// and their `wide_font` naming one of its wide sets.
    #[must_use]
    pub fn widths_of(self, atlas: &FontAtlasDef) -> Self {
        Self {
            wide: atlas.wide_chars(),
            sets: Some(atlas.wide_sets.keys().cloned().collect()),
            ..self
        }
    }

    /// These packs, their data text kept to `data_problem`
    /// ([`PackRules::data_problem`]).
    #[must_use]
    pub fn data_problem(self, data_problem: fn(&str, &str) -> Option<String>) -> Self {
        Self {
            data_problem,
            ..self
        }
    }

    /// These packs, whose dialogue entries may have `variants`
    /// ([`PackRules::variants`]).
    #[must_use]
    pub fn variants(self, variants: &'static [&'static str]) -> Self {
        Self { variants, ..self }
    }

    /// These packs, their data and dialogue text read and checked against
    /// the source text of the game's data, `data`.
    #[must_use]
    pub fn with_data(self, data: SourceText) -> Self {
        Self {
            data: Some(data),
            ..self
        }
    }
}

impl core::fmt::Debug for LangPackRon {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LangPackRon")
            .field("limits", &self.limits)
            .field("wide", &self.wide)
            .field("sets", &self.sets)
            .field("variants", &self.variants)
            .field("data", &self.data.is_some())
            .field("source", &self.source.code.as_str())
            .finish_non_exhaustive()
    }
}

impl Format for LangPackRon {
    type Asset = LangPack;

    fn id(&self) -> &'static str {
        "lang-pack"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ron"]
    }

    fn max_bytes(&self) -> usize {
        TEXT_MAX_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<LangPack, Vec<ContentError>> {
        let info = text(file, bytes)?;
        let dir = file.dir();
        let (languages, code) = dir.rsplit_once('/').unwrap_or(("", dir));
        let source = &self.source.code;
        if file.name() != INFO_FILE || code == source.as_str() {
            let message = format!("{NOT_A_LANGUAGE_FILE}: a pack is read from its {INFO_FILE}");
            return Err(vec![ContentError::new(file.shown(), message)]);
        }
        if let Err(message) = pack_code(code) {
            return Err(vec![ContentError::new(cx.shown(dir), message)]);
        }
        let english_path = join(&join(languages, source.as_str()), UI_FILE);
        let english_shown = cx.shown(&english_path);
        let english = english_from_source(
            &english_shown,
            cx.text(&english_path)
                .ok_or_else(|| vec![ContentError::not_found(&english_shown)])?,
        )?;
        let mut errors = Vec::new();
        let mut read = |path: &str| {
            let source = cx.text(path);
            if source.is_none() {
                errors.push(ContentError::not_found(cx.shown(path)));
            }
            source.map(|source| (cx.shown(path), source))
        };
        let ui = read(&join(dir, UI_FILE));
        let data_path = join(dir, DATA_FILE);
        let data = cx.has(&data_path).then(|| read(&data_path)).flatten();
        let mut dialogue = Vec::new();
        let mut unknown = Vec::new();
        for path in cx.files_under(dir) {
            let name = path
                .strip_prefix(dir)
                .and_then(|name| name.strip_prefix('/'))
                .unwrap_or(path);
            if is_dialogue_file(name) {
                dialogue.extend(read(path));
            } else if !is_settings_file(name) && ![UI_FILE, INFO_FILE, DATA_FILE].contains(&name) {
                unknown.push(ContentError::new(cx.shown(path), NOT_A_LANGUAGE_FILE));
            }
        }
        errors.extend(unknown);
        let Some((ui_shown, ui)) = ui else {
            return Err(errors);
        };
        let sources = PackSources {
            info: (file.shown(), info),
            ui: (&ui_shown, ui),
            data: data
                .as_ref()
                .map(|(shown, source)| (shown.as_str(), *source)),
            dialogue: dialogue.iter().map(|(n, s)| (n.as_str(), *s)).collect(),
        };
        let widths = CellWidths::wide(self.wide.iter().copied());
        let rules = PackRules {
            data_problem: self.data_problem,
            tokens: self.tokens,
            limits: self.limits,
            widths: &widths,
            variants: self.variants,
        };
        let pack = pack_from_files_in(source, &sources, &english, self.data.as_ref(), &rules);
        if let (Ok(pack), Some(sets)) = (&pack, &self.sets)
            && let Some(set) = pack.info.wide_font.as_deref()
            && !sets.contains(set)
        {
            let message = format!(
                "wide_font names {set:?}, a wide set the font atlas doesn't have: it has {}",
                if sets.is_empty() {
                    "none".to_owned()
                } else {
                    let names: Vec<String> = sets.iter().map(|s| format!("{s:?}")).collect();
                    names.join(", ")
                }
            );
            errors.push(ContentError::new(file.shown(), message));
        }
        match pack {
            Ok(pack) if errors.is_empty() => Ok(pack),
            Ok(_) => Err(errors),
            Err(e) => {
                errors.extend(e);
                Err(errors)
            }
        }
    }
}
