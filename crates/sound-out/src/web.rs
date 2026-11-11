//! The browser's output: `crates/cli/web/sound.js`, a miniquad plugin of our own, as
//! `gamepad.js` is. The web build has no `wasm-bindgen` (its loader is macroquad's) and
//! no threads, so the callback runs on the page's one thread, when the plugin asks.
//!
//! **The contract**, which `sound.js`'s comment says from its side:
//!
//! - The plugin calls `shuyi_sound_fill(frames)`, and copies the frames from the
//!   pointer it gets back, 16-bit, interleaved, before it does anything else.
//! - It posts them to an `AudioWorklet` as a transferred buffer, and the worklet's
//!   ring holds them. The worklet says how many frames it has played, and the plugin
//!   keeps a bounded lead of blocks over that. So nothing is shared but messages: it
//!   needs a secure context, as a worklet does, and neither cross-origin isolation,
//!   special headers nor atomics.
//! - For the first second the context plays after it starts, or after a hold lets go,
//!   the lead is twice its least: a page is busiest then, and the device takes several
//!   blocks a callback, so a page late there would cut the first sound.
//! - A worklet with nothing to play sounds silence and counts it: the mix is only
//!   asked for what will be heard, so a late page delays the sound and loses nothing.
//! - Until the first gesture, or where a page has no worklet, a timer asks for the
//!   frames that time has passed and discards them, so that what plays keeps its place
//!   on the player's clock.
//! - From the gesture the `AudioContext` is starting, and the mix waits for it: a sound
//!   asked for on the gesture is heard whole, as late as the device is to start. One
//!   that hasn't started within a few seconds goes back to the timer.
//!
//! A panic stops the program here, since `wasm32-unknown-unknown` doesn't unwind.

use std::cell::RefCell;

use crate::sys::web as sys;
use crate::{Callback, OutputConfig, OutputError};

thread_local! {
    /// The callback and the block `sound.js` reads from.
    static OPEN: RefCell<Option<(Callback, Vec<i16>, usize)>> = const { RefCell::new(None) };
}

/// See the [module docs](self).
pub(crate) struct WebOutput {
    rate: u32,
}

impl WebOutput {
    /// Opens the plugin's output, or says this page can't. With `discard` it never
    /// makes a sound.
    pub(crate) fn open(
        config: OutputConfig,
        callback: Callback,
        discard: bool,
    ) -> Result<Self, OutputError> {
        let channels = usize::from(config.channels.count());
        let rate = sys::shuyi_sound_open(config.rate, config.frames, u32::from(discard));
        if rate == 0 {
            return Err(OutputError::Unavailable(
                "this page has no AudioWorklet: it takes HTTPS or localhost".to_owned(),
            ));
        }
        OPEN.with(|open| {
            *open.borrow_mut() = Some((callback, Vec::new(), channels));
        });
        Ok(Self { rate })
    }

    /// An output that does nothing: nothing was there to open.
    pub(crate) const fn idle(rate: u32) -> Self {
        Self { rate }
    }

    pub(crate) const fn rate(&self) -> u32 {
        self.rate
    }

    /// The page has one context, so it needs nothing of this.
    #[allow(clippy::unused_self)]
    pub(crate) fn hold(&self, on: bool) {
        sys::shuyi_sound_hold(u32::from(on));
    }
}

impl Drop for WebOutput {
    fn drop(&mut self) {
        OPEN.with(|open| open.borrow_mut().take());
    }
}

/// `frames` frames of the callback's, for `sound.js`: where they are.
pub(crate) fn fill(frames: u32) -> *const i16 {
    OPEN.with(|open| {
        let mut open = open.borrow_mut();
        let Some((callback, block, channels)) = open.as_mut() else {
            return core::ptr::null();
        };
        block.resize(usize::try_from(frames).unwrap_or(0) * *channels, 0);
        if let Ok(mut callback) = callback.try_lock() {
            callback(block);
        } else {
            block.fill(0);
        }
        block.as_ptr()
    })
}

/// Asks the browser's own decoder about a file: a number to ask [`vetted`] about. A
/// file it refuses never reaches a library that could panic on it, and stop the program.
pub fn vet(bytes: &[u8]) -> u32 {
    sys::vet(bytes)
}

/// What the browser made of a file [`vet`] asked about: `None` while it decodes.
pub fn vetted(number: u32) -> Option<bool> {
    match sys::shuyi_sound_vetted(number) {
        0 => None,
        1 => Some(true),
        _ => Some(false),
    }
}
