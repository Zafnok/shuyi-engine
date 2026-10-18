//! **Hello, Grid**: Shuyi Engine's first example, a small game made from
//! the engine's crates alone. A title menu; options with two settings
//! (the text speed and the map's look) and the Key bindings screen; a
//! short scene in the engine's dialogue box; a 12 × 10 grid stage where
//! you walk your piece to the goal while a foe steps toward it, its rules
//! over the engine's `TurnRules`; and a screen that says how it ended.
//!
//! This crate is the game; `src/main.rs` hands it to the engine's platform
//! shell, and `examples/tools.rs` to the engine's tools. Its files are in
//! `files/`, embedded: the keys, the palette, the text, the scene, the
//! stage, the font atlas and the tileset.

pub mod action;
pub mod map;
pub mod rules;
pub mod screens;
pub mod settings;
pub mod skin;
pub mod sound;
#[cfg(feature = "tools")]
pub mod tools;

use std::collections::BTreeMap;

use include_dir::{Dir, include_dir};
use shuyi_assets::ContentError;
use shuyi_assets::audio::AudioManifest;
use shuyi_assets::dialogue::{DialogueTable, NoDirective, parse_dlg};
use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::grid_map::GridMap;
use shuyi_assets::image::{ImageId, ImageInfo, ImageTable};
use shuyi_assets::keymap::{KeymapDef, Layout};
use shuyi_assets::lang::Lang;
use shuyi_assets::lighting::Lighting;
use shuyi_assets::palette::PaletteDef;
use shuyi_assets::parse_ron;
use shuyi_assets::voice::ClipVariant;
use shuyi_core::TerrainId;
use shuyi_ui::frame::{ConsoleSpec, EngineColor};
use shuyi_ui::grid::SpriteSkin;
use shuyi_ui::{Ctx, Game, GameData, GameHooks};

pub use action::Action;
use rules::Feature;
pub use settings::{MapLook, Prefs};

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
/// glyphs of its map and the engine's widgets. `shuyi.ron`'s
/// `font_atlas.required_glyphs` is the same (a test keeps them equal).
pub const REQUIRED_GLYPHS: &str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~·…─│┌┐└┘├┤┬┴┼═║╔╗╚╝▓▲▼◄►";

/// The layout the game plays in: its keymap file's only one.
pub const LAYOUT: Layout = Layout::named("Arrows");

/// The game's console: 48 × 24 cells of 8 × 16 pixels.
pub const CONSOLE: ConsoleSpec = ConsoleSpec::text(48, 24);

/// The game's data and state, which its screens share.
#[derive(Debug)]
pub struct Hello {
    lang: Lang,
    keymap: KeymapDef<Action>,
    audio: AudioManifest,
    palette: PaletteDef,
    /// The scene before the stage.
    pub dialogue: DialogueTable,
    /// The stage's map.
    pub stage_map: GridMap<TerrainId, Feature>,
    /// The font atlas.
    pub font: FontAtlasDef,
    /// The pictures: `tiles.png`.
    pub images: ImageTable,
    /// The lights a map may be in: none, so the default, which changes
    /// nothing.
    pub lights: Lighting,
    /// The seed of the next stage's luck.
    pub seed: u64,
}

impl Hello {
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
        let dialogue = gather(&mut errors, || {
            let (parsed, errors) = parse_dlg::<NoDirective>("intro.dlg", text("intro.dlg")?);
            if !errors.is_empty() {
                return Err(errors);
            }
            let scenes = parsed.into_iter().map(|p| (p.scene.id.clone(), p.scene));
            Ok(DialogueTable {
                scenes: scenes.collect(),
                files: BTreeMap::new(),
            })
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
        let images = gather(&mut errors, images_table);
        match (english, keymap, palette, dialogue, stage_map, font, images) {
            (
                Some(english),
                Some(keymap),
                Some(palette),
                Some(dialogue),
                Some(stage_map),
                Some(font),
                Some(images),
            ) => Ok(Self {
                lang: Lang::new(english, BTreeMap::new()),
                keymap,
                audio: sound::manifest(),
                palette,
                dialogue,
                stage_map,
                font,
                images,
                lights: Lighting::default(),
                seed: 0,
            }),
            _ => Err(errors),
        }
    }

    /// `tiles.png`'s id.
    ///
    /// # Panics
    ///
    /// If the image table lacks it, which [`load`](Self::load) rules out.
    pub fn tiles(&self) -> ImageId {
        self.images
            .id(map::TILES_PNG)
            .unwrap_or_else(|| panic!("no {}", map::TILES_PNG))
    }

    /// The sprite skin, over `tiles.png`.
    pub fn sprite_skin(&self) -> SpriteSkin {
        map::sprites(self.tiles())
    }

    /// The seed of a new stage's luck: each stage the next one.
    pub fn next_seed(&mut self) -> u64 {
        let seed = self.seed;
        self.seed = self.seed.wrapping_add(1);
        seed
    }
}

/// The image table: `tiles.png`.
fn images_table() -> Result<ImageTable, Vec<ContentError>> {
    let png = file(map::TILES_PNG)
        .ok_or_else(|| vec![ContentError::new(map::TILES_PNG, "file not found")])?;
    let (width, height) = shuyi_assets::image::png_size(png)
        .ok_or_else(|| vec![ContentError::new(map::TILES_PNG, "not a valid PNG file")])?;
    Ok(ImageTable {
        images: [(map::TILES_PNG, ImageInfo { width, height })].into(),
    })
}

impl GameData for Hello {
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
pub fn ctx(seed: u64) -> Ctx<Hello> {
    let mut game = Hello::load().unwrap_or_else(|errors| {
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        panic!("the game's files: {}", errors.join("\n"))
    });
    game.seed = seed;
    let mut ctx = Ctx::new(game).unwrap_or_else(|missing| panic!("colors: {missing:?}"));
    ctx.use_layout(LAYOUT);
    ctx
}

/// The game on `ctx`, at its title.
pub fn game(ctx: Ctx<Hello>) -> Game<Hello> {
    Game::new(ctx, Box::new(screens::Title::new()), GameHooks::default())
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
    fn a_line_has_only_its_one_clip() {
        assert_eq!(OnlyClip.suffix(), "");
        assert!(OnlyClip::split().is_empty());
        assert!(OnlyClip.is_only());
    }

    #[test]
    fn each_stage_takes_the_next_seed() {
        let mut ctx = ctx(41);
        assert_eq!(ctx.game.next_seed(), 41);
        assert_eq!(ctx.game.next_seed(), 42);
        assert_eq!(ctx.game.seed, 43);
        assert_eq!(ctx.console, CONSOLE);
        assert_eq!(ctx.layout(), Some(LAYOUT));
        assert!(ctx.game.theme_def(()).is_none());
        assert_eq!(ctx.game.voice_variant(), OnlyClip);
    }
}
