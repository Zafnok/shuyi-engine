//! Controllers (ticket 0219, game ADR-0034): reads every connected pad's raw
//! state each frame, with gilrs on native and the browser's Gamepad API
//! (through `crates/cli/web/gamepad.js`) on web, and hands it to `shuyi-ui`'s [`Pads`],
//! which holds all the rules (sticks as directions, the Switch-style swap,
//! several pads as one). Like `keys.rs`, this only translates.

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

use shuyi_ui::RawInputEvent;
use shuyi_ui::input::{Pads, StickDef};

/// Every connected controller, polled once per frame.
pub struct PadInput {
    /// gilrs; the browser needs nothing kept.
    #[cfg(not(target_arch = "wasm32"))]
    source: native::Source,
    pads: Pads,
}

impl PadInput {
    /// Starts watching for controllers; `stick` is the keymap's stick
    /// thresholds. If the platform's controller support can't start, the
    /// game runs without controllers.
    pub fn new(stick: StickDef) -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            source: native::Source::new(),
            pads: Pads::new(stick),
        }
    }

    /// This frame's controller button releases, then presses. Pads plugged
    /// in or removed since the last frame are picked up here.
    pub fn poll(&mut self) -> Vec<RawInputEvent> {
        #[cfg(not(target_arch = "wasm32"))]
        let connected = self.source.read();
        #[cfg(target_arch = "wasm32")]
        let connected = web::read();
        RawInputEvent::from_pads(&mut self.pads, &connected)
    }
}
