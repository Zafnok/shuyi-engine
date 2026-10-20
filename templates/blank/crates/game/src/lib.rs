//! The game: its data and state ([`Data`], the engine's `GameData`),
//! loaded from its files in `files/`, embedded: the keys, the palette,
//! the English text and the font atlas. Its one screen is the title
//! ([`title`]); `src/main.rs` hands it to the engine's platform shell,
//! and `examples/tools.rs` to the engine's tools ([`tools`]).

pub mod action;
pub mod settings;
pub mod title;
#[cfg(feature = "tools")]
pub mod tools;

use std::collections::BTreeMap;

use include_dir::{Dir, include_dir};
use shuyi_assets::ContentError;
use shuyi_assets::audio::AudioManifest;
use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::image::ImageTable;
use shuyi_assets::keymap::{KeymapDef, Layout};
use shuyi_assets::lang::Lang;
use shuyi_assets::palette::PaletteDef;
use shuyi_assets::parse_ron;
use shuyi_assets::voice::ClipVariant;
use shuyi_ui::frame::{ConsoleSpec, EngineColor};
use shuyi_ui::{Ctx, Game, GameData, GameHooks, Screen};

pub use action::Action;
pub use settings::Prefs;

/// The game's files, embedded.
static FILES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/files");

/// The embedded file at `path` (relative to `files/`), if there is one.
pub fn file(path: &str) -> Option<&'static [u8]> {
    FILES.get_file(path).map(include_dir::File::contents)
}

/// The embedded text file at `path`.
fn text(path: &str) -> Result<&'static str, Vec<ContentError>> {
    let bytes = file(path).ok_or_else(|| vec![ContentError::new(path, "file not found")])?;
    std::str::from_utf8(bytes).map_err(|e| vec![ContentError::new(path, e.to_string())])
}

/// What `load` gives, or `None` with its problems added to `errors`.
fn gather<T>(
    errors: &mut Vec<ContentError>,
    load: impl FnOnce() -> Result<T, Vec<ContentError>>,
) -> Option<T> {
    load().map_err(|e| errors.extend(e)).ok()
}

/// The font atlas's description and picture, in `files/`.
pub const FONT_RON: &str = "font/atlas.ron";
/// See [`FONT_RON`].
pub const FONT_PNG: &str = "font/atlas.png";

/// The glyphs the game draws: printable ASCII, and the box and shape
/// glyphs of the engine's widgets.
pub const REQUIRED_GLYPHS: &str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~·…─│┌┐└┘├┤┬┴┼═║╔╗╚╝▓▲▼◄►";

/// The layout the game plays in: its keymap file's only one.
pub const LAYOUT: Layout = Layout::named("Arrows");

/// The game's console: 48 × 24 cells of 8 × 16 pixels.
pub const CONSOLE: ConsoleSpec = ConsoleSpec::text(48, 24);

/// The game's data and state, which its screens share.
#[derive(Debug)]
pub struct Data {
    lang: Lang,
    keymap: KeymapDef<Action>,
    audio: AudioManifest,
    palette: PaletteDef,
    /// The font atlas.
    pub font: FontAtlasDef,
    /// The pictures: none yet.
    pub images: ImageTable,
}

impl Data {
    /// The game's data from its embedded files, or every problem found.
    pub fn load() -> Result<Self, Vec<ContentError>> {
        let mut errors = Vec::new();
        let english = gather(&mut errors, || {
            parse_ron::<BTreeMap<String, String>>("en.ron", text("en.ron")?).map_err(|e| vec![e])
        });
        let keymap = gather(&mut errors, || {
            KeymapDef::from_source("keymap.ron", text("keymap.ron")?)
        });
        let names: Vec<&str> = EngineColor::ALL.iter().map(|c| c.name()).collect();
        let palette = gather(&mut errors, || {
            PaletteDef::from_source("palette.ron", text("palette.ron")?, &names)
        });
        let font = gather(&mut errors, || {
            let png = file(FONT_PNG);
            FontAtlasDef::from_source(FONT_RON, text(FONT_RON)?, FONT_PNG, png, REQUIRED_GLYPHS)
        });
        match (english, keymap, palette, font) {
            (Some(english), Some(keymap), Some(palette), Some(font)) => Ok(Self {
                lang: Lang::new(english, BTreeMap::new()),
                keymap,
                audio: AudioManifest::default(),
                palette,
                font,
                images: ImageTable::default(),
            }),
            _ => Err(errors),
        }
    }
}

impl GameData for Data {
    type Action = Action;
    type Colors = EngineColor;
    type Voice = OnlyClip;
    type Settings = Prefs;

    fn lang(&self) -> &Lang {
        &self.lang
    }

    fn keymap_def(&self) -> &KeymapDef<Action> {
        &self.keymap
    }

    fn audio_manifest(&self) -> &AudioManifest {
        &self.audio
    }

    fn palette_def(&self) -> &PaletteDef {
        &self.palette
    }

    fn console(&self) -> ConsoleSpec {
        CONSOLE
    }

    fn theme_def(&self, (): ()) -> Option<&PaletteDef> {
        None
    }

    fn voice_variant(&self) -> OnlyClip {
        OnlyClip
    }
}

/// The game's context: its data, in its one layout.
///
/// # Panics
///
/// If the game's files don't load or its palette lacks a color, which its
/// tests rule out.
pub fn ctx() -> Ctx<Data> {
    let game = Data::load().unwrap_or_else(|errors| {
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        panic!("the game's files: {}", errors.join("\n"))
    });
    let mut ctx = Ctx::new(game).unwrap_or_else(|missing| panic!("colors: {missing:?}"));
    ctx.use_layout(LAYOUT);
    ctx
}

/// The game on `ctx`, at its title.
pub fn game(ctx: Ctx<Data>) -> Game<Data> {
    Game::new(ctx, Box::new(title::Title), GameHooks::default())
}

/// Every screen the game can show, overlays on their own, for the check
/// that its controls fit each target (`cargo shuyi check --target`): its
/// own and any of the engine's it opens. A new screen goes here too.
pub fn screens(_: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    vec![Box::new(title::Title)]
}

/// Which clip of a voiced line plays: the game has no voices, so only
/// ever a line's only clip.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
pub struct OnlyClip;

impl ClipVariant for OnlyClip {
    fn suffix(self) -> &'static str {
        ""
    }

    fn split() -> &'static [Self] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_files_load() {
        let data = Data::load().unwrap_or_else(|e| panic!("{e:?}"));
        assert!(data.font.glyphs.len() >= REQUIRED_GLYPHS.chars().count());
        assert!(data.images.images.is_empty());
    }

    #[test]
    fn the_game_plays_in_its_console_and_layout() {
        let ctx = ctx();
        assert_eq!(ctx.console, CONSOLE);
        assert_eq!(ctx.layout(), Some(LAYOUT));
        assert!(ctx.game.theme_def(()).is_none());
        assert_eq!(ctx.game.voice_variant(), OnlyClip);
        assert_eq!(OnlyClip.suffix(), "");
        assert!(OnlyClip::split().is_empty());
    }
}
