//! Decoded sound: [`Pcm`] holds samples, a [`Source`] yields them in bounded
//! chunks, and optional adapters make a source of a file's bytes. It knows
//! nothing else of the engine, reads no file and opens no device.
//!
//! - [`pcm`]: holds [`Pcm`], its [`PcmFormat`], the [`Limits`] on what a decoder may
//!   produce, and sample width conversions.
//! - [`source`]: has the [`Source`] trait, [`PcmSource`] and [`read_all`].
//! - [`error`]: what can go wrong, [`Error`].
//! - `wav` (feature `wav`): `WavSource`, over the `hound` crate.
//! - `vorbis` (feature `vorbis`): `VorbisSource`, over the `lewton` crate, with
//!   a file's comments and page positions.
//!
//! Without features the crate is `no_std` with `alloc`. Samples are whole
//! numbers from end to end; the one float is a float WAV's own sample on its
//! way in.
//!
//! A malformed file gives an [`Error`], never a panic. The decoding libraries
//! are behind a guard that turns their panics into errors where the target can
//! unwind, which `wasm32-unknown-unknown` can't.

#![cfg_attr(not(any(feature = "std", test)), no_std)]
#![cfg_attr(not(test), deny(clippy::float_arithmetic))]

extern crate alloc;

pub mod error;
pub mod pcm;
pub mod source;

#[cfg(any(feature = "wav", feature = "vorbis"))]
mod guard;
#[cfg(feature = "vorbis")]
pub mod vorbis;
#[cfg(feature = "wav")]
pub mod wav;

pub use error::Error;
pub use pcm::{Channels, Limits, Pcm, PcmFormat};
pub use source::{PcmSource, Source, read_all};
