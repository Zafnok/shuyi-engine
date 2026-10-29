//! The frames of a key or button press, as `Harness::keys` and
//! `Harness::pad` play them: the one copy the live link's plays and the
//! capture's plans share.

use crate::alloc_prelude::*;
use alloc::vec;

use shuyi_ui::RawInputEvent;
use shuyi_ui::input::{Button, Chord, PadKind};

/// The buttons of one pad token, `"South"` or `"South+East"`.
pub(crate) fn buttons(token: &str) -> Result<Vec<Button>, String> {
    token.split('+').map(Button::parse).collect()
}

/// `chord` down in one frame and its key up in the next.
pub(crate) fn keys(chord: Chord) -> [Vec<RawInputEvent>; 2] {
    [
        vec![RawInputEvent::Down(chord)],
        vec![RawInputEvent::Up(chord.key)],
    ]
}

/// `buttons` down together on a generic pad in one frame, and up in the
/// next.
pub(crate) fn pad(buttons: &[Button]) -> [Vec<RawInputEvent>; 2] {
    let kind = PadKind::default();
    [
        buttons
            .iter()
            .map(|&b| RawInputEvent::PadDown(b, kind))
            .collect(),
        buttons.iter().map(|&b| RawInputEvent::PadUp(b)).collect(),
    ]
}
