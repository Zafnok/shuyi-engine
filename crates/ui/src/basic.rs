//! [`BasicData`]: a ready-made [`GameData`] for a game on the short path
//! (ADR-0036). It reads the files every such game has from its bundle,
//! through the format registry ([`BasicData::formats`]): the English
//! screen text, the keymap, the palette with the engine's colours, the
//! font atlas, every picture, and the audio manifest when the bundle has
//! one ([`AUDIO_RON`]: the sounds its menus play). A part of
//! the game's own rides along in [`BasicData::own`]. It has no voices
//! (`()`), no settings of the game's own (`()`) and one palette. A game
//! that needs more writes its own `GameData`, as before.
//!
//! It remembers the files each of its text, keymap, palette and own part
//! read, and [`BasicData::reload`] reads again those whose files changed
//! on disk. [`BasicReload`], a module a game on the short path installs
//! with debug tools on (`shuyi_platform::BasicGame::game`), hands the
//! context what reloaded: the new text, colours and keys, and a game
//! started over at its first screen for a new own part (ADR-0164).

use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use core::fmt::{self, Debug};

use shuyi_assets::ContentError;
use shuyi_assets::audio::AudioManifest;
use shuyi_assets::bundle::Bundle;
use shuyi_assets::font::{FontAtlasDef, FontAtlasRon};
use shuyi_assets::format::Formats;
use shuyi_assets::hot::Reload;
use shuyi_assets::image::ImageTable;
use shuyi_assets::keymap::{ActionSet, KeymapDef, KeymapRon};
use shuyi_assets::lang::{English, Lang};
use shuyi_assets::pack::{Bake, Packed};
use shuyi_assets::palette::{PaletteDef, PaletteRon};
use shuyi_assets::ron::Ron;

use crate::ctx::{Ctx, GameData};
use crate::frame::{ConsoleSpec, EngineColor, Palette};
use crate::module::{Hook, Install, Module};
use crate::screen::Screen;
use crate::text::check_key_placeholders;

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
/// The audio manifest, in the bundle, if the game has sounds: its sound
/// files are named from the bundle's root (`sfx/move.sfx.ron`).
pub const AUDIO_RON: &str = "audio.ron";
/// What [`BasicData::reload`] names the game's own part by: it may read
/// any of the files, and its errors name each.
// check-text: not player text, a reload's name in the debug overlay
pub const OWN_PART: &str = "own part";

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
    /// How the game reads its own part, to read it again.
    read_own: Own<T>,
    /// The files each value that reloads read.
    reads: Reads,
}

/// The files each of [`BasicData`]'s values that reload read at its last
/// good decode, and at the failed ones since.
#[derive(Debug, Default)]
struct Reads {
    english: BTreeSet<String>,
    keymap: BTreeSet<String>,
    palette: BTreeSet<String>,
    own: BTreeSet<String>,
}

/// What `f` gives, and the files of a bundle it read (`recording`): none
/// without `std`, where no file changes as the game runs.
fn recorded<V>(f: impl FnOnce() -> V) -> (V, BTreeSet<String>) {
    #[cfg(feature = "std")]
    {
        shuyi_assets::bundle::recording(f)
    }
    #[cfg(not(feature = "std"))]
    {
        (f(), BTreeSet::new())
    }
}

/// How a value that read `reads` reloaded, from its new decode `decoded`,
/// which read `read`: a good one goes to `apply`, and `reads` becomes
/// `read`; a failed one keeps the value, and `reads` takes `read` too, as
/// a file the broken one names may be what is missing.
fn reload_with<V>(
    reads: &mut BTreeSet<String>,
    (decoded, read): (Result<V, Vec<ContentError>>, BTreeSet<String>),
    apply: impl FnOnce(V),
) -> Reload {
    match decoded {
        Ok(value) => {
            apply(value);
            *reads = read;
            Reload::Reloaded
        }
        Err(errors) => {
            reads.extend(read);
            Reload::Failed(errors)
        }
    }
}

/// The English of `bundle`, with its key placeholders checked.
fn read_english<A: ActionSet>(
    formats: &Formats,
    bundle: &Bundle,
) -> Result<English, Vec<ContentError>> {
    formats
        .load::<English>(bundle, ENGLISH_RON)
        .and_then(|english| {
            check_key_placeholders::<A>(bundle, ENGLISH_RON, &english.0).map(|()| english)
        })
}

/// What `loaded` gives, or `None` with its problems added to `errors`.
fn gather<T>(errors: &mut Vec<ContentError>, loaded: Result<T, Vec<ContentError>>) -> Option<T> {
    loaded.map_err(|e| errors.extend(e)).ok()
}

impl<A: ActionSet, T: Debug + 'static> BasicData<A, T> {
    /// The formats it reads with: the engine's standard ones, the palette
    /// with the engine's colours and the font atlas with
    /// [`BASIC_GLYPHS`] in their place, the keymap of `A` and the English;
    /// and the baked form of each kind [`bake`](Self::bake) bakes, so the
    /// same loader reads a pack of the files ([`Packed`]).
    pub fn formats() -> Formats {
        let mut formats = Formats::standard();
        let colors: Vec<&str> = EngineColor::ALL.iter().map(|c| c.name()).collect();
        // Each replaces the standard one of its kind, or is the first of
        // its kind: none is refused.
        let _ = formats.replace(PaletteRon::requiring(colors));
        let _ = formats.replace(FontAtlasRon::requiring(BASIC_GLYPHS));
        let _ = formats.register(KeymapRon::<A>::new());
        let _ = formats.register(Ron::<English>::new());
        let _ = formats.register(Packed::<English>::new());
        let _ = formats.register(Packed::<PaletteDef>::new());
        let _ = formats.register(Packed::<FontAtlasDef>::new());
        formats
    }

    /// What a pack of the files bakes, stamped `stamp` (ADR-0023): the
    /// English, the palette and the font atlas, read from their binary
    /// form with no text parsed. Every other file goes in as its own bytes:
    /// the pictures, which the platform draws; the keymap, whose actions
    /// `A` need not have a binary form; and the game's own part, whose
    /// kinds are its own.
    pub fn bake(stamp: impl Into<String>) -> Bake {
        Bake::new(stamp)
            .asset::<English>(ENGLISH_RON)
            .asset::<PaletteDef>(PALETTE_RON)
            .asset::<FontAtlasDef>(FONT_RON)
    }

    /// The data in `bundle`, on `console`, with the game's own part read
    /// by `own`; or every problem of every file at once, each naming its
    /// file. The audio manifest is [`AUDIO_RON`]'s, checked with its sound
    /// files, or empty if the bundle has none.
    pub fn load(
        bundle: Bundle,
        console: ConsoleSpec,
        own: Own<T>,
    ) -> Result<Self, Vec<ContentError>> {
        let formats = Self::formats();
        let mut errors = Vec::new();
        let mut reads = Reads::default();
        let (english, read) = recorded(|| read_english::<A>(&formats, &bundle));
        let english = gather(&mut errors, english);
        reads.english = read;
        let (keymap, read) = recorded(|| formats.load::<KeymapDef<A>>(&bundle, KEYMAP_RON));
        let keymap = gather(&mut errors, keymap);
        reads.keymap = read;
        let (palette, read) = recorded(|| formats.load::<PaletteDef>(&bundle, PALETTE_RON));
        let palette = gather(&mut errors, palette);
        reads.palette = read;
        let font = gather(&mut errors, formats.load::<FontAtlasDef>(&bundle, FONT_RON));
        let audio = if bundle.has(AUDIO_RON) {
            gather(
                &mut errors,
                formats.load::<AudioManifest>(&bundle, AUDIO_RON),
            )
        } else {
            Some(AudioManifest::default())
        };
        let images = gather(
            &mut errors,
            ImageTable::load_with(&bundle, &formats, &[FONT_PNG]),
        );
        let (part, read) = recorded(|| own(&bundle));
        let part = gather(&mut errors, part);
        reads.own = read;
        match (english, keymap, palette, font, images, audio, part) {
            (
                Some(english),
                Some(keymap),
                Some(palette),
                Some(font),
                Some(images),
                Some(audio),
                Some(part),
            ) => Ok(Self {
                lang: Lang::new(english.0, BTreeMap::new()),
                keymap,
                audio,
                palette,
                console,
                font,
                images,
                bundle,
                own: part,
                read_own: own,
                reads,
            }),
            _ => Err(errors),
        }
    }

    /// Reads again each of the English, the keymap, the palette and the
    /// game's own part a file of which is among `changed` (paths in the
    /// bundle), with the checks [`load`](Self::load) makes: the English's
    /// key placeholders, the palette's engine colours ([`formats`](Self::formats)
    /// refuses a palette that lacks one). A good decode
    /// replaces the value; a failed one keeps the old value and gives its
    /// errors, and reads again the next time one of its files changes.
    /// Each value read again is in the list, in that order, named by its
    /// file ([`ENGLISH_RON`], [`KEYMAP_RON`], [`PALETTE_RON`]), or by
    /// [`OWN_PART`]; one no file of which changed is not. The font atlas
    /// and the pictures don't reload here: a changed picture's pixels
    /// redraw on their own, and a new picture or atlas needs a restart.
    /// Without `std`, no read is recorded, and nothing reloads.
    pub fn reload(&mut self, changed: &[String]) -> Vec<(&'static str, Reload)> {
        let touched = |reads: &BTreeSet<String>| changed.iter().any(|path| reads.contains(path));
        let (english, keymap, palette, own) = (
            touched(&self.reads.english),
            touched(&self.reads.keymap),
            touched(&self.reads.palette),
            touched(&self.reads.own),
        );
        let mut reloads = Vec::new();
        if !(english || keymap || palette || own) {
            return reloads;
        }
        let formats = Self::formats();
        let bundle = &self.bundle;
        if english {
            let decoded = recorded(|| read_english::<A>(&formats, bundle));
            let reload = reload_with(&mut self.reads.english, decoded, |english| {
                self.lang = Lang::new(english.0, BTreeMap::new());
            });
            reloads.push((ENGLISH_RON, reload));
        }
        if keymap {
            let decoded = recorded(|| formats.load::<KeymapDef<A>>(bundle, KEYMAP_RON));
            let reload = reload_with(&mut self.reads.keymap, decoded, |keymap| {
                self.keymap = keymap;
            });
            reloads.push((KEYMAP_RON, reload));
        }
        if palette {
            let decoded = recorded(|| formats.load::<PaletteDef>(bundle, PALETTE_RON));
            let reload = reload_with(&mut self.reads.palette, decoded, |palette| {
                self.palette = palette;
            });
            reloads.push((PALETTE_RON, reload));
        }
        if own {
            let decoded = recorded(|| (self.read_own)(bundle));
            let reload = reload_with(&mut self.reads.own, decoded, |part| self.own = part);
            reloads.push((OWN_PART, reload));
        }
        reloads
    }
}

/// The screen a game on the short path starts on, and starts over at when
/// its own part reloads.
type FirstOf<A, T> = fn(&mut Ctx<BasicData<A, T>>) -> Box<dyn Screen<BasicData<A, T>>>;

/// The module that reloads a game on the short path's own files as they
/// change on disk (`cargo shuyi run --watch`), with no code of the game's:
/// its English, keymap, palette and own part ([`BasicData::reload`]). It
/// hands the context what reloaded:
///
/// - the English: nothing more, as the next frame shows the new text;
/// - the palette: the context's colours ([`Ctx::palette`]);
/// - the keymap: the keys of the layout in use, with the player's own
///   bindings kept ([`Ctx::refresh_keymap`]);
/// - the own part: the game starts over at its first screen on the same
///   context ([`Ctx::restart`]), as no screen built from the old part (a
///   map) can follow the new one. The settings, the profile, the layout
///   and the player's bindings stay; every screen's state goes.
///
/// Every outcome goes to [`Ctx::note_reload`], so a broken file shows its
/// errors over the frame while the old value stays, until a good save.
/// `shuyi_platform::BasicGame::game` installs it, before the game's own
/// modules, in a build with debug tools on.
pub struct BasicReload<A: ActionSet, T: Debug + 'static> {
    first: FirstOf<A, T>,
}

impl<A: ActionSet, T: Debug + 'static> BasicReload<A, T> {
    /// The module, starting the game over at the screen `first` makes when
    /// its own part reloads.
    pub fn new(first: FirstOf<A, T>) -> Self {
        Self { first }
    }
}

impl<A: ActionSet, T: Debug + 'static> Debug for BasicReload<A, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicReload").finish_non_exhaustive()
    }
}

impl<A: ActionSet, T: Debug + 'static> Module<BasicData<A, T>> for BasicReload<A, T> {
    fn name(&self) -> &'static str {
        "basic_reload"
    }

    fn install(&mut self, install: &mut Install<'_, BasicData<A, T>>) {
        install.hook(Hook::Reloaded);
    }

    fn reloaded(&mut self, ctx: &mut Ctx<BasicData<A, T>>, changed: &[String]) {
        for (id, reload) in ctx.game.reload(changed) {
            if reload == Reload::Reloaded {
                match id {
                    KEYMAP_RON => ctx.refresh_keymap(),
                    PALETTE_RON => {
                        // Its format refuses one that lacks a colour.
                        if let Ok(palette) = Palette::new::<EngineColor>(&ctx.game.palette) {
                            ctx.palette = palette;
                        }
                    }
                    OWN_PART => ctx.restart(self.first),
                    // The English: the screens read the text as they draw,
                    // and its languages' wide sets can't change (the source
                    // language is the default one, with no packs).
                    _ => {}
                }
            }
            ctx.note_reload(id, &reload);
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
