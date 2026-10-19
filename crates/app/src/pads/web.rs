//! Pads on the web: the browser's Gamepad API, through the `shuyi_gamepad`
//! miniquad plugin in `engine/crates/cli/web/gamepad.js` (our own code, loaded by
//! `engine/crates/cli/web/index.html`). Only pads the browser reports in its "standard"
//! mapping are listed, so buttons and axes mean the same on every pad.

use shuyi_ui::input::{PadId, PadKind, PadState};

/// The plugin's version, which the JS loader compares with the `version`
/// in `engine/crates/cli/web/gamepad.js` (a `shuyi-cli` test keeps them equal). Bump both when the
/// functions below change.
const PLUGIN_VERSION: u32 = 3;

// The game's only `unsafe` (ADR-0034): declaring the functions
// `engine/crates/cli/web/gamepad.js` provides, and exporting the version the loader asks for.
// They take and return plain numbers, and the JS answers 0 for a pad or
// axis that isn't there, so calling them is safe.
#[allow(unsafe_code)]
unsafe extern "C" {
    /// Reads the connected standard-mapping pads; returns how many. The
    /// other functions index into that list until the next call.
    safe fn shuyi_pad_poll() -> u32;
    /// The browser's own index of pad `pad` (stable while it is plugged in).
    safe fn shuyi_pad_index(pad: u32) -> u32;
    /// The pad's USB vendor id, or 0 if the browser doesn't tell.
    safe fn shuyi_pad_vendor(pad: u32) -> u32;
    /// The pad's USB product id, or 0 if the browser doesn't tell.
    safe fn shuyi_pad_product(pad: u32) -> u32;
    /// Bit `i` is set while the pad's `buttons[i]` is pressed, `i < 16`.
    safe fn shuyi_pad_buttons(pad: u32) -> u32;
    /// The pad's `axes[axis]`: 0–3 are the left stick's x and y, then the
    /// right stick's (down is positive).
    safe fn shuyi_pad_axis(pad: u32, axis: u32) -> f32;
}

/// What the JS loader calls to check the plugin's version.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn shuyi_gamepad_crate_version() -> u32 {
    PLUGIN_VERSION
}

/// Every connected standard-mapping pad's kind and state right now. There
/// is nothing to start first: browsers list a pad once one of its buttons
/// is pressed while the page has focus.
pub fn read() -> Vec<(PadId, PadKind, PadState)> {
    (0..shuyi_pad_poll())
        .map(|pad| {
            let id = usize::try_from(shuyi_pad_index(pad)).unwrap_or(0);
            let vendor = u16::try_from(shuyi_pad_vendor(pad)).unwrap_or(0);
            let product = u16::try_from(shuyi_pad_product(pad)).unwrap_or(0);
            let axes = [0, 1, 2, 3].map(|axis| shuyi_pad_axis(pad, axis));
            let state = PadState::from_standard(shuyi_pad_buttons(pad), axes);
            (id, PadKind::from_ids(vendor, product), state)
        })
        .collect()
}
