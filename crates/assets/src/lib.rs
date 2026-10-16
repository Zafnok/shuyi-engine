//! Shuyi Engine's assets: the bundle of files a game embeds, laid in layers
//! the game gives ([`bundle`]), errors that name a file, line and column
//! ([`error`]), and RON files read with those errors ([`ron`]). On top of
//! them, the kinds of file every game has: images ([`image`]), a font atlas
//! ([`font`]), named colours ([`palette`], [`ink`]), lights ([`lighting`]),
//! the audio manifest ([`audio`]) and the credits ([`credits`]). Each is a
//! pure parser; the game names its files and reads them from its bundle.
//! The `.map` text format of a grid map is [`grid_map`].
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
pub mod bundle;
pub mod credits;
pub mod error;
pub mod font;
pub mod grid_map;
pub mod image;
pub mod ink;
pub mod lighting;
pub mod palette;
pub mod ron;

pub use bundle::{Bundle, Files, Layer};
pub use error::{ContentError, ContentErrors, line_of};
pub use grid_map::{GridMap, MapFeature, MapFormat, MapLook};
pub use ron::parse_ron;
