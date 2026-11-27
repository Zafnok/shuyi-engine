//! Shuyi Engine's platform boundary (ADR-0008 §2, ADR-0024): the
//! [`Platform`] trait a backend implements, and [`App`], which runs one
//! frame of a game each time the backend's own loop calls
//! [`App::frame`] with its clock.
//!
//! - [`Platform`]: the display a [`Frame`] is presented on, the player's
//!   input, the system's events ([`SystemEvent`]), the game's
//!   [`Storage`], the sound device ([`Audio`]) and the system keyboard
//!   ([`TextInput`]);
//! - [`audio::player`]: the rules of playing a game's sounds and music, which
//!   every backend shares over its small [`Backend`](audio::player::Backend);
//! - [`App`]: the game's screens and input state ([`shuyi_ui::Game`]), its
//!   fixed simulation steps ([`tick`]), suspend and resume, and the
//!   loading phase ([`Start`], [`Card`]);
//! - [`BasicGame`] ([`basic`]): a game on the short path, described once
//!   for its binary, its tools and its tests, and the [`prelude`] such a
//!   game names;
//! - [`AppInfo`] ([`app_info`]): who a game is, its name, id, version,
//!   storage's name and window's icon ([`WindowIcon`]), as its `shuyi.ron`
//!   says;
//! - the [`headless`] backend, which plays scripted input and records what
//!   it was shown, and the test `harness` over it (the `harness`
//!   feature);
//! - [`player_files`]: where a file the game offered the player went, and
//!   the file the player picked ([`ChosenFile`]), as the system's events;
//! - [`crash`]: the report a backend's panic hook writes through the
//!   game's storage, and the last crash the game hears of at its next
//!   start ([`Setup::last_crash`]);
//! - [`script`]: the steps of a player's input a tool plays (keys,
//!   buttons, an action by name, waits), and the one plan of their frames
//!   that the Harness, the live link and a capture's plans play;
//! - [`session`]: a run recorded where the platform hands things to [`App`]
//!   ([`App::record`]), its text, and its playback on the headless backend;
//! - the live `link`'s wire and server (the `link` feature), which a
//!   backend's listener runs so a tool reaches a running debug build;
//! - `capture`'s plans (the `capture` feature): the input a backend's
//!   capture mode plays, and the frames it compares with the raster;
//! - `heap::snapshot` (the `heap` feature): the heap counter's rows as a
//!   heap snapshot, for a backend that counts the heap.
//!
//! The backends: `shuyi-app` (macroquad: desktop, the web, the Steam Deck),
//! `shuyi-playdate`, and private crates for closed consoles.
//! `docs/platform-backends.md` says how to write one. Nothing here touches
//! a device: no window, file, sound card or clock.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// What `std`'s prelude gives for free, for the `no_std` build.
mod alloc_prelude {
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
}

pub mod app;
pub mod app_info;
pub mod audio;
pub mod basic;
#[cfg(feature = "capture")]
pub mod capture;
pub mod crash;
pub mod dropped;
#[cfg(any(test, feature = "harness"))]
pub mod harness;
pub mod headless;
#[cfg(feature = "heap")]
pub mod heap;
#[cfg(feature = "link")]
pub mod link;
pub mod platform;
pub mod player_files;
pub mod script;
pub mod session;
pub mod text_input;
pub mod tick;

pub use app::{
    App, Card, CrashRecord, FrameOutcome, GameHash, Load, LoadInSteps, LoadStep, Loader, LookOf,
    Progress, Setup, Start, SystemHook,
};
pub use app_info::{AppInfo, WindowIcon};
pub use audio::{Audio, AudioFrame, Silent, SoundRequest};
pub use basic::BasicGame;
pub use dropped::DroppedFile;
pub use platform::{
    DisplayInfo, Frame, LoadSlice, Look, Millis, OneBitLook, Platform, Show, SystemEvent,
};
pub use player_files::{ChosenFile, Offered};
pub use shuyi_ui::storage::Storage;
pub use text_input::{TextAnswer, TextAsk, TextInput};

/// The variable `cargo shuyi run --watch` sets for the game it starts: the
/// folder of the game's files on disk, which a backend lays over the
/// embedded ones and watches (ADR-0071).
pub const WATCH_VAR: &str = "SHUYI_WATCH";

/// The variable `cargo shuyi run --watch` sets beside [`WATCH_VAR`] when
/// the game has a checkout of its private files (ADR-0111): their folder on
/// disk, which a backend reads in place of the game's embedded private
/// layer (`BasicGame::private`) and watches.
pub const WATCH_PRIVATE_VAR: &str = "SHUYI_WATCH_PRIVATE";

/// The variable a debug build reads for the graphics tier to show in
/// place of its target's (`SHUYI_GRAPHICS=reduced`): a lesser target's
/// stills, previewed on the desktop
/// ([`PlatformFacts::with_graphics`](shuyi_ui::PlatformFacts::with_graphics)).
/// A build that ships never reads it.
pub const GRAPHICS_VAR: &str = "SHUYI_GRAPHICS";

/// What a game on the short path names (ADR-0036), and the start types a
/// game names when it leaves it. Anything else is named by its module's
/// path.
pub mod prelude {
    pub use shuyi_assets::bundle::Embedded;
    pub use shuyi_assets::keymap::{ActionSet, UiAction};
    pub use shuyi_assets::{Bundle, ContentError, Layer};
    pub use shuyi_core::{Bag, RandomSource};
    pub use shuyi_ui::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
    pub use shuyi_ui::frame::{ConsoleSpec, EngineColor, Rgb};
    pub use shuyi_ui::module::{Hook, Install, Module};
    pub use shuyi_ui::screens::print_centred;
    pub use shuyi_ui::settings::Background;
    pub use shuyi_ui::{
        BasicData, Ctx, FrameInput, Game, GameData, GameHooks, Screen, ScreenResult, Transition,
    };

    pub use crate::{AppInfo, BasicGame, Card, Look, Setup, Start, SystemEvent};

    /// The grid kit, by one path (ADR-0161): `use
    /// shuyi::prelude::grid::*;` in a screen that shows a map. Its camera,
    /// cursor, skins, scene and painters (`shuyi-ui`'s `grid`), the map
    /// file (`GridMap`, `MapText`) and its lights. Its `Grid` is the
    /// screen's layout, not the rules' `shuyi_core::Grid`: a file that
    /// names both names one by its path.
    pub mod grid {
        pub use shuyi_assets::grid_map::{self, GridMap, MapFeature, MapFormat, MapText};
        pub use shuyi_assets::lighting::Lighting;
        pub use shuyi_assets::tileset::{Look as TileLook, TerrainTiles};
        pub use shuyi_ui::grid::debug::{GRID, layout_gizmos};
        pub use shuyi_ui::grid::{
            Camera, Cursor, CursorStyle, CursorView, GlyphSkin, Grid, GridSkin, Looks, MapColors,
            MapScene, MapUnit, RangeName, SpriteSkin, TerrainGlyphs, Terrains, UnitPainter,
            WalkSpeeds, glyph, steer, walk,
        };
    }

    /// Styled text and the font, by one path (ADR-0161): `use
    /// shuyi::prelude::text::*;` in a screen that writes words. The styled
    /// text and its roles, a message's parse, the font's atlas and widths,
    /// and the languages.
    pub mod text {
        pub use shuyi_assets::font::{CellWidths, FontAtlasDef, FontAtlasRon};
        pub use shuyi_assets::lang::{English, Lang, LangCode, SourceLang, SourceText, text_width};
        pub use shuyi_assets::message::Message;
        pub use shuyi_ui::frame::onebit::Font;
        pub use shuyi_ui::screens::print_centred_styled;
        pub use shuyi_ui::text::{StyledText, TextRole, check_key_placeholders};
    }

    /// Sound, by one path (ADR-0161): `use shuyi::prelude::audio::*;`. The
    /// game's audio manifest and its cues, a screen's requests and the
    /// sound groups, and a voice clip's variant.
    pub mod audio {
        pub use shuyi_assets::audio::{
            AudioManifest, Bytes, CreditRef, CueRules, Files, SoundCue, format_problem, playable,
        };
        pub use shuyi_assets::voice::ClipVariant;
        pub use shuyi_ui::audio::{AudioConfig, AudioRequest, MenuSound, Samples, groups};
    }

    /// A game's tests, by one path (ADR-0161): `use
    /// shuyi::prelude::test::*;`. The Harness, with the `harness` feature
    /// (a game's tests get it through `shuyi-cli`), its session files, the
    /// playtest runner's checks and the autobalance search. The bots that
    /// play a turn game are in `shuyi_core::prelude`.
    pub mod test {
        #[cfg(all(feature = "harness", feature = "std"))]
        pub use crate::harness::assert_session;
        #[cfg(feature = "harness")]
        pub use crate::harness::{FRAME_DT, Harness, check_session};
        pub use crate::session::{SessionRecord, Stamps};
        pub use shuyi_core::bots::balance::Change;
        pub use shuyi_core::bots::playtest;
        pub use shuyi_core::bots::playtest::{
            Band, Check, Condition, Entrant, Found, Measure, Measures, MedianVerdict, PlayedTry,
            PlaytestError, PlaytestReport, RateVerdict, Tried, Tries, UnknownEntrant, bot_seed,
            judge, play_tries, play_try,
        };
        pub use shuyi_core::{BalanceSettings, Balanced, Knob, Stop, autobalance};
    }
}
