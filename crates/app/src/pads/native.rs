//! Pads on Windows, Linux and macOS through gilrs, whose bundled SDL
//! controller database makes Sony, Nintendo and generic pads report the
//! same buttons, by position, as an Xbox pad.

use gilrs::{Axis, Gilrs};
use macroquad::prelude::warn;
use shuyi_ui::input::{Button, PadId, PadKind, PadState};

/// The game's button for each gilrs button it reads. gilrs calls the
/// shoulder buttons `LeftTrigger` / `RightTrigger` and the triggers
/// `…Trigger2`; its `C`, `Z` and `Mode` (the logo button) aren't used.
const BUTTONS: [(gilrs::Button, Button); 16] = [
    (gilrs::Button::South, Button::South),
    (gilrs::Button::East, Button::East),
    (gilrs::Button::West, Button::West),
    (gilrs::Button::North, Button::North),
    (gilrs::Button::LeftTrigger, Button::LeftShoulder),
    (gilrs::Button::RightTrigger, Button::RightShoulder),
    (gilrs::Button::LeftTrigger2, Button::LeftTrigger),
    (gilrs::Button::RightTrigger2, Button::RightTrigger),
    (gilrs::Button::Select, Button::Select),
    (gilrs::Button::Start, Button::Start),
    (gilrs::Button::LeftThumb, Button::LeftStickPress),
    (gilrs::Button::RightThumb, Button::RightStickPress),
    (gilrs::Button::DPadUp, Button::DpadUp),
    (gilrs::Button::DPadDown, Button::DpadDown),
    (gilrs::Button::DPadLeft, Button::DpadLeft),
    (gilrs::Button::DPadRight, Button::DpadRight),
];

/// The platform's pads, or nothing if gilrs couldn't start.
pub struct Source {
    gilrs: Option<Gilrs>,
}

impl Source {
    /// Starts gilrs. On failure (e.g. Linux without access to udev) it logs
    /// why and reports no pads.
    pub fn new() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(gilrs) => Some(gilrs),
            Err(e) => {
                warn!("controllers: {}; playing without controller support", e);
                None
            }
        };
        Self { gilrs }
    }

    /// Every connected pad's kind and state right now.
    pub fn read(&mut self) -> Vec<(PadId, PadKind, PadState)> {
        let Some(gilrs) = &mut self.gilrs else {
            return Vec::new();
        };
        // gilrs updates its picture of each pad, and notices pads plugged
        // in or removed, as its events are taken.
        while gilrs.next_event().is_some() {}
        gilrs
            .gamepads()
            .map(|(id, pad)| {
                let held = BUTTONS.iter().filter(|(theirs, _)| pad.is_pressed(*theirs));
                let state = PadState {
                    buttons: held.map(|&(_, ours)| ours).collect(),
                    // gilrs sticks are up-positive; the game's are
                    // down-positive.
                    left_stick: (pad.value(Axis::LeftStickX), -pad.value(Axis::LeftStickY)),
                    right_stick: (pad.value(Axis::RightStickX), -pad.value(Axis::RightStickY)),
                };
                let kind =
                    PadKind::from_ids(pad.vendor_id().unwrap_or(0), pad.product_id().unwrap_or(0));
                (id.into(), kind, state)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_real_button_is_read_from_its_own_gilrs_button() {
        // The 16 real buttons, in order (the rest are stick directions).
        let ours: Vec<Button> = BUTTONS.iter().map(|&(_, ours)| ours).collect();
        assert_eq!(ours, Button::ALL[..16]);
        for (i, (theirs, _)) in BUTTONS.iter().enumerate() {
            assert!(
                BUTTONS[..i].iter().all(|(other, _)| other != theirs),
                "{theirs:?} is listed twice"
            );
            assert_ne!(*theirs, gilrs::Button::Unknown);
        }
    }

    /// The real platform backend starts (or fails cleanly) and can be read
    /// every frame, whatever is plugged into the machine running the test.
    #[test]
    fn starting_and_reading_never_panics() {
        let mut source = Source::new();
        for _ in 0..3 {
            for (_, _, state) in source.read() {
                assert!(state.left_stick.0.abs() <= 1.0 && state.left_stick.1.abs() <= 1.0);
            }
        }
    }

    #[test]
    fn gilrs_names_shoulders_and_triggers_its_own_way() {
        let ours = |theirs| {
            let found = BUTTONS.iter().find(|&&(t, _)| t == theirs);
            found.map(|&(_, ours)| ours)
        };
        assert_eq!(ours(gilrs::Button::LeftTrigger), Some(Button::LeftShoulder));
        assert_eq!(ours(gilrs::Button::LeftTrigger2), Some(Button::LeftTrigger));
        assert_eq!(
            ours(gilrs::Button::RightTrigger),
            Some(Button::RightShoulder)
        );
        assert_eq!(
            ours(gilrs::Button::RightTrigger2),
            Some(Button::RightTrigger)
        );
        assert_eq!(ours(gilrs::Button::LeftThumb), Some(Button::LeftStickPress));
        assert_eq!(ours(gilrs::Button::Mode), None);
    }
}
