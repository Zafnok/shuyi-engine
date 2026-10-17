//! Shuyi Engine's UI, independent of any window or platform (ADR-0004).
//! So far the [`frame`] every screen draws into (a console of glyph cells
//! the game sizes, pictures, a backdrop, colors by name, the text form the
//! snapshot tests read and the one-bit renderer) and its input
//! ([`input`]): a game's actions from keys and controller buttons through
//! a keymap, held keys and repeat, the player's saved bindings and the
//! names of keys in help text. Later tickets add screens, widgets and
//! skins.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// What `std`'s prelude gives for free, for the `no_std` build.
mod prelude {
    pub use alloc::borrow::ToOwned;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
    pub use alloc::{format, vec};

    #[cfg(not(feature = "std"))]
    pub use crate::float::Float;
}

pub mod float;
pub mod frame;
pub mod input;

/// Whether this build offers the developer's tools: debug builds, and
/// release builds with the `debug-tools` feature. Then the keys of the
/// actions no player can rebind (a debug key) stay out of the player's
/// reach; otherwise those keys are free to bind.
pub const DEBUG_TOOLS: bool = cfg!(any(debug_assertions, feature = "debug-tools"));
