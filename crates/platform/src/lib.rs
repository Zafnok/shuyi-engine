//! Shuyi Engine's platform boundary (ADR-0008 §2, ADR-0023): the
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
//! - the [`headless`] backend, which plays scripted input and records what
//!   it was shown, and the test `harness` over it (the `harness`
//!   feature).
//!
//! The backends: `shuyi-app` (macroquad: desktop, the web, the Steam Deck),
//! later `shuyi-playdate`, and private crates for closed consoles.
//! `docs/platform-backends.md` says how to write one. Nothing here touches
//! a device: no window, file, sound card or clock.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// What `std`'s prelude gives for free, for the `no_std` build.
mod prelude {
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
}

pub mod app;
pub mod audio;
#[cfg(any(test, feature = "harness"))]
pub mod harness;
pub mod headless;
pub mod platform;
pub mod text_input;
pub mod tick;

pub use app::{App, Card, FrameOutcome, Load, LookOf, Setup, Start, SystemHook};
pub use audio::{Audio, AudioFrame, Silent, SoundRequest};
pub use platform::{DisplayInfo, Frame, Look, Millis, Platform, Show, SystemEvent};
pub use shuyi_ui::storage::Storage;
pub use text_input::{TextAnswer, TextAsk, TextInput};
