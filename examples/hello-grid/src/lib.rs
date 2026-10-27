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
//! stage, the font atlas and the tileset. It reads them through the
//! engine's format registry ([`formats`]), so the same loader reads them
//! from a pack that `cargo shuyi bake` made of them ([`Hello::from_pack`],
//! [`bake`]).

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
use std::sync::OnceLock;

use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize};
use shuyi::assets::ContentError;
use shuyi::assets::audio::AudioManifest;
use shuyi::assets::bundle::{Bundle, Layer};
use shuyi::assets::dialogue::{DialogueFile, DialogueTable};
use shuyi::assets::font::{FontAtlasDef, FontAtlasRon};
use shuyi::assets::format::{AssetKind, Formats};
use shuyi::assets::grid_map::{GridMap, MapText};
use shuyi::assets::image::{ImageId, ImageTable};
use shuyi::assets::keymap::{KeymapDef, KeymapRon, Layout};
use shuyi::assets::lang::Lang;
use shuyi::assets::lighting::Lighting;
use shuyi::assets::pack::{Bake, Pack};
use shuyi::assets::palette::{PaletteDef, PaletteRon};
use shuyi::assets::ron::Ron;
use shuyi::assets::voice::ClipVariant;
use shuyi::core::{Made, TerrainId};
use shuyi::ui::frame::{ConsoleSpec, EngineColor};
use shuyi::ui::grid::SpriteSkin;
use shuyi::ui::{Ctx, Game, GameData, GameHooks};

pub use action::Action;
use rules::Feature;
pub use settings::{MapLook, Prefs};

/// The game's files, embedded.
static FILES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/files");

/// The embedded file at `path` (relative to `files/`), if there is one.
pub fn file(path: &str) -> Option<&'static [u8]> {
    FILES.get_file(path).map(include_dir::File::contents)
}

/// The game's files as a bundle layer, shown under `files`
/// (`files/palette.ron`).
pub fn layer() -> Layer {
    Layer::dir("files", &FILES)
}

/// The game's English screen text, by key: `en.ron`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct English(pub BTreeMap<String, String>);

impl AssetKind for English {
    const NAME: &'static str = "english";
}

/// The game's formats: the engine's standard ones, its palette with the
/// engine's colours and its font atlas with the glyphs it draws, and its
/// own kinds (its keys, its stage and its English).
pub fn formats() -> Formats {
    let mut formats = Formats::standard();
    let colors: Vec<&str> = EngineColor::ALL.iter().map(|c| c.name()).collect();
    // Each replaces the standard one of its kind, or is the first of its
    // kind: none is refused.
    let _ = formats.replace(PaletteRon::requiring(colors));
    let _ = formats.replace(FontAtlasRon::requiring(REQUIRED_GLYPHS));
    let _ = formats.register(KeymapRon::<Action>::new());
    let _ = formats.register(MapText::<TerrainId, Feature>::new(
        rules::FORMAT,
        rules::terrain_named,
    ));
    let _ = formats.register(Ron::<English>::new());
    formats
}

/// What a pack of the game says it was baked for: the game and its
/// version, since its own kinds change only with it.
pub const PACK_STAMP: &str = concat!("hello-grid ", env!("CARGO_PKG_VERSION"));

/// What made this build, for its records: [`PACK_STAMP`] and the
/// fingerprint of the files it ships, computed once.
pub fn made() -> Made {
    static MADE: OnceLock<Made> = OnceLock::new();
    MADE.get_or_init(|| Made::new(PACK_STAMP, Bundle::new(vec![layer()]).fingerprint()))
        .clone()
}

/// What `cargo shuyi bake` bakes of the game: every text file it loads.
/// Its pictures stay as they are: the platform draws them, and the font
/// atlas's format reads its picture beside it.
pub fn bake() -> Bake {
    Bake::new(PACK_STAMP)
        .asset::<English>("en.ron")
        .asset::<KeymapDef<Action>>("keymap.ron")
        .asset::<PaletteDef>("palette.ron")
        .asset::<DialogueFile>("intro.dlg")
        .asset::<GridMap<TerrainId, Feature>>("stage.map")
        .asset::<FontAtlasDef>(FONT_RON)
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
        Self::load_from(&Bundle::new(vec![layer()]), &formats())
    }

    /// The game's data from pack `bytes` (embedded with `include_bytes!`),
    /// which `cargo shuyi bake` made of its files, or every problem found:
    /// the same loader, over the pack.
    pub fn from_pack(bytes: &'static [u8]) -> Result<Self, Vec<ContentError>> {
        let problem = |e: String| vec![ContentError::new("pack", e)];
        let pack = Pack::open(bytes, PACK_STAMP).map_err(|e| problem(e.to_string()))?;
        let mut formats = formats();
        bake()
            .register(&mut formats)
            .map_err(|e| problem(e.to_string()))?;
        Self::load_from(&Bundle::new(vec![pack.into_layer("pack")]), &formats)
    }

    /// The game's data from `bundle`, read with `formats`, or every
    /// problem found.
    pub fn load_from(bundle: &Bundle, formats: &Formats) -> Result<Self, Vec<ContentError>> {
        let mut errors = Vec::new();
        let english = gather(&mut errors, || formats.load::<English>(bundle, "en.ron"));
        let keymap = gather(&mut errors, || {
            formats.load::<KeymapDef<Action>>(bundle, "keymap.ron")
        });
        let palette = gather(&mut errors, || {
            formats.load::<PaletteDef>(bundle, "palette.ron")
        });
        let dialogue = gather(&mut errors, || {
            let file = formats.load::<DialogueFile>(bundle, "intro.dlg")?;
            let scenes = file
                .scenes
                .into_iter()
                .map(|p| (p.scene.id.clone(), p.scene));
            Ok(DialogueTable::new(scenes.collect()))
        });
        let stage_map = gather(&mut errors, || {
            let map = formats.load::<GridMap<TerrainId, Feature>>(bundle, "stage.map")?;
            rules::Stage::new(&map, 0)
                .map_err(|e| vec![ContentError::new(bundle.display_path("stage.map"), e)])?;
            Ok(map)
        });
        let font = gather(&mut errors, || {
            formats.load::<FontAtlasDef>(bundle, FONT_RON)
        });
        let images = gather(&mut errors, || {
            ImageTable::load_with(bundle, formats, &[FONT_PNG])
        });
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
                lang: Lang::new(english.0, BTreeMap::new()),
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
        let got = OnlyClip::split();
        assert!(got.is_empty(), "{got:?}");
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
