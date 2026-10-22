//! A language pack as a format (ADR-0007): read from its `lang.ron`, with
//! the other files of its directory and English's screen text beside it.
use crate::prelude::*;

use super::{
    DATA_FILE, ENGLISH, INFO_FILE, LangCode, LangPack, PackRules, PackSources, SourceText, UI_FILE,
    english_from_source, is_dialogue_file, pack_from_files,
};
use crate::dialogue::{DialogueLimits, Tokens};
use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, join, text};

impl AssetKind for LangPack {
    const NAME: &'static str = "language pack";
}

/// What a file a language doesn't have is told.
const NOT_A_LANGUAGE_FILE: &str = "not a file a language has";

/// A language pack (`lang-pack`, `.ron`), read from its `lang.ron` in its
/// directory (`lang/fr/lang.ron`), which is named by its code: its
/// `ui.ron` beside it, checked against English's (`lang/en/ui.ron`), and
/// its `data.ron` and `dialogue/*.ron`, checked against the game's data
/// text when it gives it ([`LangPackRon::with_data`]; left out
/// otherwise). Any other file in the directory is an error. Checked as
/// [`pack_from_files`] says, with the game's rules.
pub struct LangPackRon {
    data_problem: fn(&str, &str) -> Option<String>,
    tokens: &'static (dyn Tokens + Sync),
    limits: DialogueLimits,
    variants: &'static [&'static str],
    data: Option<SourceText>,
}

impl LangPackRon {
    /// Packs whose dialogue fits `limits` and may use the game's `tokens`,
    /// with no rules of the game's for its data text, no variants, and no
    /// data text to check theirs against.
    pub fn new(limits: DialogueLimits, tokens: &'static (dyn Tokens + Sync)) -> Self {
        Self {
            data_problem: |_, _| None,
            tokens,
            limits,
            variants: &[],
            data: None,
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
    /// the English of the game's data, `data`.
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
            .field("variants", &self.variants)
            .field("data", &self.data.is_some())
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
        if file.name() != INFO_FILE || code == ENGLISH {
            let message = format!("{NOT_A_LANGUAGE_FILE}: a pack is read from its {INFO_FILE}");
            return Err(vec![ContentError::new(file.shown(), message)]);
        }
        if LangCode::new(code).is_none() {
            let message = "a language's directory must be named with 2 to 8 lowercase letters";
            return Err(vec![ContentError::new(cx.shown(dir), message)]);
        }
        let english_path = join(&join(languages, ENGLISH), UI_FILE);
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
            } else if ![UI_FILE, INFO_FILE, DATA_FILE].contains(&name) {
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
        let rules = PackRules {
            data_problem: self.data_problem,
            tokens: self.tokens,
            limits: self.limits,
            variants: self.variants,
        };
        match pack_from_files(&sources, &english, self.data.as_ref(), &rules) {
            Ok(pack) if errors.is_empty() => Ok(pack),
            Ok(_) => Err(errors),
            Err(e) => {
                errors.extend(e);
                Err(errors)
            }
        }
    }
}
