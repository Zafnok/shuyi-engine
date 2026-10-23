//! [`BasicData`]: a ready-made [`GameData`] for a game on the short path
//! (ADR-0036). It reads the files every such game has from its bundle,
//! through the format registry ([`BasicData::formats`]): the English
//! screen text, the keymap, the palette with the engine's colours, the
//! font atlas and every picture. A part of the game's own rides along in
//! [`BasicData::own`]. It has no sounds, no voices (`()`), no settings of
//! the game's own (`()`) and one palette. A game that needs more writes
//! its own `GameData`, as before.

use crate::prelude::*;
use alloc::collections::BTreeMap;
use core::fmt::Debug;

use shuyi_assets::ContentError;
use shuyi_assets::audio::AudioManifest;
use shuyi_assets::bundle::Bundle;
use shuyi_assets::font::{FontAtlasDef, FontAtlasRon};
use shuyi_assets::format::Formats;
use shuyi_assets::image::ImageTable;
use shuyi_assets::keymap::{ActionSet, KeymapDef, KeymapRon};
use shuyi_assets::lang::{English, Lang};
use shuyi_assets::palette::{PaletteDef, PaletteRon};
use shuyi_assets::ron::Ron;

use crate::ctx::GameData;
use crate::frame::{ConsoleSpec, EngineColor};

/// The English screen text by key, in the bundle.
pub const ENGLISH_RON: &str = "en.ron";
/// The keymap file, in the bundle.
pub const KEYMAP_RON: &str = "keymap.ron";
/// The palette, in the bundle: at least the engine's colours.
pub const PALETTE_RON: &str = "palette.ron";
/// The font atlas's description, in the bundle.
pub const FONT_RON: &str = "font/atlas.ron";
/// The font atlas's picture, beside [`FONT_RON`].
pub const FONT_PNG: &str = "font/atlas.png";

/// The glyphs the font must have: printable ASCII, and the box and shape
/// glyphs the engine's widgets draw.
pub const BASIC_GLYPHS: &str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~·…─│┌┐└┘├┤┬┴┼═║╔╗╚╝▓▲▼◄►";

/// How a game reads its own part from its bundle, with every problem of
/// every file.
pub type Own<T> = fn(&Bundle) -> Result<T, Vec<ContentError>>;

/// A game's data on the short path: what the engine reads from the
/// standard files, the files themselves and the game's own part `T`. Its
/// actions are `A` (most simply `UiAction`), its colours the engine's.
#[derive(Debug)]
pub struct BasicData<A: ActionSet, T: Debug + 'static = ()> {
    lang: Lang,
    keymap: KeymapDef<A>,
    audio: AudioManifest,
    palette: PaletteDef,
    console: ConsoleSpec,
    /// The font atlas.
    pub font: FontAtlasDef,
    /// Every picture of the bundle but the font's.
    pub images: ImageTable,
    /// The bundle the data was read from: the pictures' bytes.
    pub bundle: Bundle,
    /// The game's own part.
    pub own: T,
}

/// What `loaded` gives, or `None` with its problems added to `errors`.
fn gather<T>(errors: &mut Vec<ContentError>, loaded: Result<T, Vec<ContentError>>) -> Option<T> {
    loaded.map_err(|e| errors.extend(e)).ok()
}

impl<A: ActionSet, T: Debug + 'static> BasicData<A, T> {
    /// The formats it reads with: the engine's standard ones, the palette
    /// with the engine's colours and the font atlas with
    /// [`BASIC_GLYPHS`] in their place, the keymap of `A` and the English.
    pub fn formats() -> Formats {
        let mut formats = Formats::standard();
        let colors: Vec<&str> = EngineColor::ALL.iter().map(|c| c.name()).collect();
        // Each replaces the standard one of its kind, or is the first of
        // its kind: none is refused.
        let _ = formats.replace(PaletteRon::requiring(colors));
        let _ = formats.replace(FontAtlasRon::requiring(BASIC_GLYPHS));
        let _ = formats.register(KeymapRon::<A>::new());
        let _ = formats.register(Ron::<English>::new());
        formats
    }

    /// The data in `bundle`, on `console`, with the game's own part read
    /// by `own`; or every problem of every file at once, each naming its
    /// file.
    pub fn load(
        bundle: Bundle,
        console: ConsoleSpec,
        own: Own<T>,
    ) -> Result<Self, Vec<ContentError>> {
        let formats = Self::formats();
        let mut errors = Vec::new();
        let english = gather(&mut errors, formats.load::<English>(&bundle, ENGLISH_RON));
        let keymap = gather(
            &mut errors,
            formats.load::<KeymapDef<A>>(&bundle, KEYMAP_RON),
        );
        let palette = gather(
            &mut errors,
            formats.load::<PaletteDef>(&bundle, PALETTE_RON),
        );
        let font = gather(&mut errors, formats.load::<FontAtlasDef>(&bundle, FONT_RON));
        let images = gather(
            &mut errors,
            ImageTable::load_with(&bundle, &formats, &[FONT_PNG]),
        );
        let own = gather(&mut errors, own(&bundle));
        match (english, keymap, palette, font, images, own) {
            (Some(english), Some(keymap), Some(palette), Some(font), Some(images), Some(own)) => {
                Ok(Self {
                    lang: Lang::new(english.0, BTreeMap::new()),
                    keymap,
                    audio: AudioManifest::default(),
                    palette,
                    console,
                    font,
                    images,
                    bundle,
                    own,
                })
            }
            _ => Err(errors),
        }
    }
}

impl<A: ActionSet, T: Debug + 'static> GameData for BasicData<A, T> {
    type Action = A;
    type Colors = EngineColor;
    type Voice = ();
    type Settings = ();

    fn lang(&self) -> &Lang {
        &self.lang
    }

    fn keymap_def(&self) -> &KeymapDef<A> {
        &self.keymap
    }

    fn audio_manifest(&self) -> &AudioManifest {
        &self.audio
    }

    fn palette_def(&self) -> &PaletteDef {
        &self.palette
    }

    fn console(&self) -> ConsoleSpec {
        self.console
    }
}

#[cfg(test)]
mod tests;
