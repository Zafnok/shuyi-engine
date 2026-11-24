//! quad-snd's ALSA bindings, as `alsa-sys`'s (ADR-0142). quad-snd's Linux path needs
//! `quad-alsa-sys` and CPAL's needs `alsa-sys`, and each declares `links = "alsa"`,
//! which Cargo allows only once in a build, whatever the target. Both are bindings of
//! the same C library, `libasound`, made by `bindgen` from its headers, `alsa-sys`'s
//! the newer and the larger. So this takes quad-snd's crate's name, without the
//! `links`, and re-exports `alsa-sys`, which finds and links `libasound` for both. The
//! root `Cargo.toml` patches it in; ticket 3079 takes it out with quad-snd.
#![no_std]

pub use alsa_sys::*;
