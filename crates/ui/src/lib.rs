//! Shuyi Engine's UI, independent of any window or platform (ADR-0004).
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
//! Later tickets add widgets and skins.
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
    pub use crate::float::Float;
}

pub mod audio;
pub mod ctx;
pub mod float;
pub mod frame;
pub mod game;
#[cfg(any(test, feature = "harness"))]
pub mod harness;
pub mod input;
pub mod profiles;
pub mod screen;
pub mod settings;
pub mod storage;
pub mod text;
#[cfg(any(test, feature = "toy"))]
pub mod toy;

pub use ctx::{Ctx, GameData, IdleMotion, KeyPrompt, Keyboard};
pub use game::{FrameOutput, Game, GameHooks, RawInputEvent, ScreenHook};
pub use screen::{FrameInput, Screen, ScreenStack, Transition};

/// Whether this build offers the developer's tools: debug builds, and
/// release builds with the `debug-tools` feature. Then the keys of the
/// actions no player can rebind (a debug key) stay out of the player's
/// reach; otherwise those keys are free to bind.
pub const DEBUG_TOOLS: bool = cfg!(any(debug_assertions, feature = "debug-tools"));
