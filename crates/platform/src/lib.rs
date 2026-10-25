//! Shuyi Engine's platform boundary (ADR-0008 §2, ADR-0024): the
//! [`Platform`] trait a backend implements, and [`App`], which runs one
//! frame of a game each time the backend's own loop calls
//! [`App::frame`] with its clock.
//!
//! - [`Platform`]: the display a [`Frame`] is presented on, the player's
//!   input, the system's events ([`SystemEvent`]), the game's
//!   [`Storage`], the sound device ([`Audio`]) and the system keyboard
//!   ([`TextInput`]);
//! - [`App`]: the game's screens and input state ([`shuyi_ui::Game`]), its
//!   fixed simulation steps ([`tick`]), suspend and resume, and the
//!   loading phase ([`Start`], [`Card`]);
//! - [`BasicGame`] ([`basic`]): a game on the short path, described once
//!   for its binary, its tools and its tests, and the [`prelude`] such a
//!   game names;
//! - the [`headless`] backend, which plays scripted input and records what
//!   it was shown, and the test `harness` over it (the `harness`
//!   feature);
//! - the live `link`'s wire and server (the `link` feature), which a
//!   backend's listener runs so a tool reaches a running debug build.
//!
//! The backends: `shuyi-app` (macroquad: desktop, the web, the Steam Deck),
//! later `shuyi-playdate`, and private crates for closed consoles.
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
pub mod audio;
pub mod basic;
#[cfg(any(test, feature = "harness"))]
pub mod harness;
pub mod headless;
#[cfg(feature = "link")]
pub mod link;
pub mod platform;
pub mod text_input;
pub mod tick;

pub use app::{App, Card, FrameOutcome, Load, LookOf, Setup, Start, SystemHook};
pub use audio::{Audio, AudioFrame, Silent, SoundRequest};
pub use basic::BasicGame;
pub use platform::{DisplayInfo, Frame, Look, Millis, Platform, Show, SystemEvent};
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

    pub use crate::{BasicGame, Card, Look, Setup, Start};
}
