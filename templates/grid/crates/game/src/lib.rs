//! The game: its data and state ([`Data`], the engine's `GameData`),
//! loaded from its files in `files/`, embedded: the keys, the palette,
//! the English text, the stage's map and the font atlas. Its one screen
//! is the stage ([`stage`]); its rules are the `rules` crate;
//! `src/main.rs` hands it to the engine's platform shell, and
//! `examples/tools.rs` to the engine's tools ([`tools`]).

pub mod action;
pub mod settings;
pub mod stage;
#[cfg(feature = "tools")]
pub mod tools;

use std::collections::BTreeMap;

use include_dir::{Dir, include_dir};
use rules::Feature;
use shuyi::assets::ContentError;
use shuyi::assets::audio::AudioManifest;
use shuyi::assets::font::FontAtlasDef;
use shuyi::assets::grid_map::GridMap;
use shuyi::assets::image::ImageTable;
use shuyi::assets::keymap::{KeymapDef, Layout};
use shuyi::assets::lang::Lang;
use shuyi::assets::lighting::Lighting;
use shuyi::assets::palette::PaletteDef;
use shuyi::assets::parse_ron;
use shuyi::assets::voice::ClipVariant;
use shuyi::core::TerrainId;
use shuyi::ui::frame::{ConsoleSpec, EngineColor};
use shuyi::ui::module::ModuleError;
use shuyi::ui::{Ctx, Game, GameData, GameHooks, Screen};

pub use action::Action;
pub use settings::Prefs;

/// The game's files, embedded.
// check-text: not player text, a folder
static FILES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/files");

/// The embedded file at `path` (relative to `files/`), if there is one.
pub fn file(path: &str) -> Option<&'static [u8]> {
    FILES.get_file(path).map(include_dir::File::contents)
}

/// The embedded text file at `path`.
// check-text: not player text, a content error `cargo shuyi check` shows
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
/// glyphs of its map and the engine's widgets.
// check-text: not player text, the font's glyphs
pub const REQUIRED_GLYPHS: &str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~·…─│┌┐└┘├┤┬┴┼═║╔╗╚╝▓▲▼◄►";

/// The layout the game plays in: its keymap file's only one.
// check-text: not player text, the keymap file's name for it
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
    /// The stage's map.
    pub stage_map: GridMap<TerrainId, Feature>,
    /// The font atlas.
    pub font: FontAtlasDef,
    /// The pictures: none yet.
    pub images: ImageTable,
    /// The lights a map may be in: none, so the default, which changes
    /// nothing.
    pub lights: Lighting,
    /// The seed of the next stage's luck.
    pub seed: u64,
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
        let stage_map = gather(&mut errors, || {
            let problem = |e: String| vec![ContentError::new("stage.map", e)];
            let map = rules::parse_map("stage.map", text("stage.map")?).map_err(problem)?;
            rules::Stage::new(&map, 0).map_err(problem)?;
            Ok(map)
        });
        let font = gather(&mut errors, || {
            let png = file(FONT_PNG);
            FontAtlasDef::from_source(FONT_RON, text(FONT_RON)?, FONT_PNG, png, REQUIRED_GLYPHS)
        });
        match (english, keymap, palette, stage_map, font) {
            (Some(english), Some(keymap), Some(palette), Some(stage_map), Some(font)) => Ok(Self {
                lang: Lang::new(english, BTreeMap::new()),
                keymap,
                audio: AudioManifest::default(),
                palette,
                stage_map,
                font,
                images: ImageTable::default(),
                lights: Lighting::default(),
                seed: 0,
            }),
            _ => Err(errors),
        }
    }

    /// The seed of a new stage's luck: each stage the next one.
    pub fn next_seed(&mut self) -> u64 {
        let seed = self.seed;
        self.seed = self.seed.wrapping_add(1);
        seed
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

/// The game's context: its data, in its one layout, with `seed` the first
/// stage's luck.
///
/// # Panics
///
/// If the game's files don't load or its palette lacks a color, which its
/// tests rule out.
pub fn ctx(seed: u64) -> Ctx<Data> {
    let mut game = Data::load().unwrap_or_else(|errors| {
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        panic!("the game's files: {}", errors.join("\n"))
    });
    game.seed = seed;
    let mut ctx = Ctx::new(game).unwrap_or_else(|missing| panic!("colors: {missing:?}"));
    ctx.use_layout(LAYOUT);
    ctx
}

/// The game on `ctx`, on its stage, with the engine's modules its
/// features turn on (`Cargo.toml`) installed; or the one that can't be.
pub fn game(mut ctx: Ctx<Data>) -> Result<Game<Data>, ModuleError> {
    let stage = stage::StageScreen::new(&mut ctx);
    Game::new(ctx, Box::new(stage), GameHooks::default()).with_modules(shuyi::enabled_modules())
}

/// Every screen the game can show, overlays on their own, for the check
/// that its controls fit each target (`cargo shuyi check --target`): its
/// own and any of the engine's it opens. A new screen goes here too.
pub fn screens(ctx: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    vec![Box::new(stage::StageScreen::new(ctx))]
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
        assert!(data.images.images.is_empty(), "{:?}", data.images.images);
    }

    #[test]
    fn each_stage_takes_the_next_seed() {
        let mut ctx = ctx(41);
        assert_eq!(ctx.game.next_seed(), 41);
        assert_eq!(ctx.game.next_seed(), 42);
        assert_eq!(ctx.console, CONSOLE);
        assert_eq!(ctx.layout(), Some(LAYOUT));
        assert!(ctx.game.theme_def(()).is_none());
        assert_eq!(ctx.game.voice_variant(), OnlyClip);
        assert_eq!(OnlyClip.suffix(), "");
        let got = OnlyClip::split();
        assert!(got.is_empty(), "{got:?}");
    }
}
