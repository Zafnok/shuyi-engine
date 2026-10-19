//! Shuyi Engine's assets: the bundle of files a game embeds, laid in layers
//! the game gives ([`bundle`]), errors that name a file, line and column
//! ([`error`]), and RON files read with those errors ([`ron`]). On top of
//! them, the kinds of file every game has: images ([`image`]) and their
//! one-bit versions ([`bitmap`]), a font atlas
//! ([`font`]), named colours ([`palette`], [`ink`]), lights ([`lighting`]),
//! the audio manifest ([`audio`]) and the credits ([`credits`]); the `.map`
//! text format of a grid map ([`grid_map`]); keymap files, generic over a
//! game's actions ([`keymap`]); and the text of a story game:
//! languages ([`lang`]), a names table ([`names`]), dialogue scripts
//! ([`dialogue`]), voice clips by line ([`voice`]) and tips ([`tip`]); and
//! cinematics, timed shots of a game's own kinds ([`cinematic`]). Each
//! is a pure parser; the game names its files, says what its own rules add,
//! and reads them from its bundle.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// What `std`'s prelude gives for free, for the `no_std` build.
mod prelude {
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
    pub use alloc::{format, vec};
}

pub mod audio;
pub mod bitmap;
pub mod bundle;
pub mod cinematic;
pub mod credits;
pub mod dialogue;
pub mod error;
pub mod font;
pub mod grid_map;
pub mod image;
pub mod ink;
pub mod keymap;
pub mod lang;
pub mod lighting;
pub mod names;
pub mod palette;
pub mod ron;
pub mod tileset;
pub mod tip;
pub mod voice;

pub use bundle::{Bundle, Files, Layer};
pub use error::{ContentError, ContentErrors, line_of};
pub use grid_map::{GridMap, MapFeature, MapFormat, MapLook};
pub use ron::parse_ron;
