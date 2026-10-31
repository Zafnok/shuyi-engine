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
#[cfg(any(test, feature = "harness"))]
pub mod harness;
pub mod headless;
#[cfg(feature = "heap")]
pub mod heap;
#[cfg(feature = "link")]
pub mod link;
pub mod platform;
#[cfg(any(feature = "link", feature = "capture"))]
mod press;
pub mod text_input;
pub mod tick;

pub use app::{App, Card, FrameOutcome, Load, LookOf, Setup, Start, SystemHook};
pub use app_info::{AppInfo, WindowIcon};
pub use audio::{Audio, AudioFrame, Silent, SoundRequest};
pub use basic::BasicGame;
pub use platform::{DisplayInfo, Frame, Look, Millis, OneBitLook, Platform, Show, SystemEvent};
pub use shuyi_ui::storage::Storage;
pub use text_input::{TextAnswer, TextAsk, TextInput};

/// What a game on the short path names (ADR-0036), and the start types a
/// game names when it leaves it. Anything else is named by its module's
/// path.
pub mod prelude {
    pub use shuyi_assets::keymap::{ActionSet, UiAction};
    pub use shuyi_assets::{Bundle, ContentError, Layer};
    pub use shuyi_ui::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
    pub use shuyi_ui::frame::{ConsoleSpec, EngineColor, Rgb};
    pub use shuyi_ui::module::{Hook, Install, Module};
    pub use shuyi_ui::screens::print_centred;
    pub use shuyi_ui::{
        BasicData, Ctx, FrameInput, Game, GameData, GameHooks, Screen, ScreenResult, Transition,
    };

    pub use crate::{AppInfo, BasicGame, Card, Look, Setup, Start};
}
