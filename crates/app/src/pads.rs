//! Controllers (ticket 0219, game ADR-0034): reads every connected pad's raw
//! state each frame, with gilrs on native and the browser's Gamepad API
//! (through `crates/cli/web/gamepad.js`) on web, and hands it to `shuyi-ui`'s [`Pads`],
//! which holds all the rules (sticks as directions, the Switch-style swap,
//! several pads as one). Like `keys.rs`, this only translates, and ignores
//! the pads while the window has no focus (`pad_events`).

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

use shuyi_ui::RawInputEvent;
use shuyi_ui::input::{PadId, PadKind, PadState, Pads, StickDef};

/// Every connected controller, polled once per frame.
pub struct PadInput {
    /// gilrs; the browser needs nothing kept.
    #[cfg(not(target_arch = "wasm32"))]
    source: native::Source,
    pads: Pads,
    /// Whether the window had focus at the last poll.
    focused: bool,
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
            focused: true,
        }
    }

    /// This frame's controller button releases, then presses, while the
    /// window has focus if `focused` (`pad_events`). Pads plugged in or
    /// removed since the last frame are picked up here.
    pub fn poll(&mut self, focused: bool) -> Vec<RawInputEvent> {
        // Read whatever the focus, so gilrs's queue keeps draining.
        #[cfg(not(target_arch = "wasm32"))]
        let connected = self.source.read();
        #[cfg(target_arch = "wasm32")]
        let connected = web::read();
        let was = core::mem::replace(&mut self.focused, focused);
        pad_events(&mut self.pads, &connected, (was, focused))
    }
}

/// The controller events of a frame `connected` describes, the window's
/// focus at the last frame and at this one being `focus`. A window without
/// focus hears no pad: every held button and stick direction goes up, and
/// nothing goes down. On the frame focus comes back, its presses are
/// dropped, so a button held across the change counts once it is let go
/// and pressed again, as a key does.
pub(crate) fn pad_events(
    pads: &mut Pads,
    connected: &[(PadId, PadKind, PadState)],
    (was, focused): (bool, bool),
) -> Vec<RawInputEvent> {
    // A pad left out counts as removed: its buttons go up.
    let heard = if focused { connected } else { &[] };
    let mut events = RawInputEvent::from_pads(pads, heard);
    if focused && !was {
        events.retain(|event| !matches!(event, RawInputEvent::PadDown(..)));
    }
    events
}

#[cfg(test)]
mod tests {
    use shuyi_ui::input::Button;

    use super::*;

    fn pads() -> Pads {
        Pads::new(StickDef {
            press_percent: 50,
            release_percent: 35,
        })
    }

    /// One Xbox pad holding `buttons`, its left stick at `stick`.
    fn pad(buttons: &[Button], stick: (f32, f32)) -> [(PadId, PadKind, PadState); 1] {
        let state = PadState {
            buttons: buttons.iter().copied().collect(),
            left_stick: stick,
            ..PadState::default()
        };
        [(0, PadKind::Xbox, state)]
    }

    const FOCUSED: (bool, bool) = (true, true);
    const LOST: (bool, bool) = (true, false);
    const AWAY: (bool, bool) = (false, false);
    const BACK: (bool, bool) = (false, true);

    #[test]
    fn losing_focus_lets_go_of_buttons_and_stick_directions_and_presses_none() {
        let mut p = pads();
        let held = pad(&[Button::South], (1.0, 0.0));
        let down = pad_events(&mut p, &held, FOCUSED);
        assert_eq!(down.len(), 2, "{down:?}");
        let mut up = pad_events(&mut p, &held, LOST);
        up.sort_by_key(|e| format!("{e:?}"));
        assert_eq!(
            up,
            [
                RawInputEvent::PadUp(Button::LeftStickRight),
                RawInputEvent::PadUp(Button::South),
            ]
        );
    }

    #[test]
    fn a_press_while_unfocused_gives_nothing() {
        let mut p = pads();
        assert_eq!(pad_events(&mut p, &pad(&[], (0.0, 0.0)), FOCUSED), []);
        let pressed = pad(&[Button::East], (0.0, 0.0));
        assert_eq!(pad_events(&mut p, &pressed, LOST), []);
        assert_eq!(pad_events(&mut p, &pressed, AWAY), []);
    }

    #[test]
    fn a_button_held_across_the_change_counts_once_let_go_and_pressed_again() {
        let mut p = pads();
        let held = pad(&[Button::South], (0.0, 0.0));
        let _ = pad_events(&mut p, &held, FOCUSED);
        let _ = pad_events(&mut p, &held, LOST);
        // Focus back, the button still held: no press.
        assert_eq!(pad_events(&mut p, &held, BACK), []);
        assert_eq!(pad_events(&mut p, &held, FOCUSED), []);
        let free = pad(&[], (0.0, 0.0));
        let up = pad_events(&mut p, &free, FOCUSED);
        assert_eq!(up, [RawInputEvent::PadUp(Button::South)]);
        let down = pad_events(&mut p, &held, FOCUSED);
        assert_eq!(down, [RawInputEvent::PadDown(Button::South, PadKind::Xbox)]);
    }
}
