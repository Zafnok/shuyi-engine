//! Shuyi Engine's UI, independent of any window or platform (game ADR-0004).
//! So far:
//!
//! - the [`frame`] every screen draws into (a console of glyph cells the
//!   game sizes, pictures, a backdrop, colors by name, the text form the
//!   snapshot tests read and the one-bit renderer);
//! - [`input`]: a game's actions from keys and controller buttons through
//!   a keymap, held keys and repeat, the player's saved bindings and the
//!   names of keys in help text;
//! - [`screen`]s on a stack, the context they share ([`Ctx`], with the
//!   game's part in it, [`GameData`]) and the frame loop ([`Game`]), with
//!   the player's [`settings`] and [`profiles`], [`storage`], [`audio`]
//!   requests and the music, and [`text`] placeholders;
//! - the test `harness` that drives a game without a window
//!   (the `harness` feature), and a `toy` game to try things on
//!   (the `toy` feature).
//!
//! - [`widgets`] screens are built from: a menu and word wrap;
//! - [`screens`] any game can use, with the game's rows: the layout
//!   picker and the Key bindings screen;
//! - [`cinematic`]s: a timeline of a game's shots, each frame drawn from
//!   the time alone;
//! - the tactics kit's [`grid`] map: a scene of the visible tiles generic
//!   over the game's units, the glyph and sprite skins that paint it (the
//!   game's units by its own painter), the tile camera, the virtual
//!   cursor, a path that follows it and how long a walk is shown.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// What `std`'s prelude gives for free, for the `no_std` build.
mod prelude {
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
    pub use alloc::{format, vec};

    #[cfg(not(feature = "std"))]
    pub use crate::float::{Float, Round};
}

pub mod audio;
pub mod cinematic;
pub mod ctx;
pub mod debug;
pub mod dialogue;
pub mod float;
pub mod frame;
pub mod game;
pub mod grid;
#[cfg(any(test, feature = "harness"))]
pub mod harness;
pub mod input;
pub mod profiles;
pub mod screen;
pub mod screens;
pub mod settings;
pub mod storage;
pub mod text;
pub mod tips;
#[cfg(any(test, feature = "toy"))]
pub mod toy;
pub mod widgets;

pub use ctx::{Ctx, GameData, IdleMotion, KeyPrompt, Keyboard};
pub use game::{FrameOutput, Game, GameHooks, RawInputEvent, ScreenHook};
pub use screen::{FrameInput, Screen, ScreenResult, ScreenStack, Transition};

/// Whether this build offers the developer's tools: debug builds, and
/// release builds with the `debug-tools` feature. Then the keys of the
/// actions no player can rebind (a debug key) stay out of the player's
/// reach; otherwise those keys are free to bind.
pub const DEBUG_TOOLS: bool = cfg!(any(debug_assertions, feature = "debug-tools"));
