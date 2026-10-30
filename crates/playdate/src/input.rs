//! The Playdate's buttons and crank as the engine's input: the SDK's button
//! bits are one controller of [`PadKind::Playdate`], and the crank two more
//! of its buttons, one press a notch. It only translates, as `shuyi-app`'s
//! `pads.rs` does; the only file in this crate that names buttons.
//!
//! A reports `East` and B `West`, as `targets/playdate.ron` lists them
//! (ADR-0008 §1).

use alloc::vec::Vec;

use shuyi_assets::keymap::{CrankDef, StickDef};
use shuyi_ui::RawInputEvent;
use shuyi_ui::input::Button;
use shuyi_ui::input::crank::{Crank, STEPS_PER_TURN};
use shuyi_ui::input::pad::{PadKind, PadState, Pads};

/// The SDK's `PDButtons` bits (`pd_api_sys.h`; `shim/shim.c` checks this
/// copy against the header), each with the binding position it reports.
pub const BUTTONS: [(u32, Button); 6] = [
    (0x01, Button::DpadLeft),
    (0x02, Button::DpadRight),
    (0x04, Button::DpadUp),
    (0x08, Button::DpadDown),
    (0x20, Button::East),
    (0x10, Button::West),
];

/// The button of the system menu's item in `slot`, if there is one.
pub fn menu_button(slot: u8) -> Option<Button> {
    Button::system_menu(slot)
}

/// What the player's choice of the system menu's item in `slot` is: a press
/// and a release of its button.
pub fn menu_press(slot: u8) -> Option<[RawInputEvent; 2]> {
    let button = menu_button(slot)?;
    Some([
        RawInputEvent::PadDown(button, PadKind::Playdate),
        RawInputEvent::PadUp(button),
    ])
}

/// The Playdate's one pad: never unplugged, no sticks.
const PAD: usize = 0;

/// The pad state of the buttons whose bits are set in `bits`.
fn state(bits: u32) -> PadState {
    PadState {
        buttons: BUTTONS
            .iter()
            .filter(|&&(bit, _)| bits & bit != 0)
            .map(|&(_, button)| button)
            .collect(),
        ..PadState::default()
    }
}

/// Degrees in a whole turn.
const DEGREES_PER_TURN: f32 = 360.0;

/// The float the SDK gives for the crank, turned into whole steps of a turn
/// here at the boundary: what rounds away is kept for the next frame, so a
/// slow turn still gets there, and no float travels further.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Steps {
    carry: f32,
}

impl Steps {
    /// The whole steps in `degrees` and what the last call left. A reading
    /// that isn't a number is no turn.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn of(&mut self, degrees: f32) -> i32 {
        if !degrees.is_finite() {
            return 0;
        }
        let per_degree = STEPS_PER_TURN as f32 / DEGREES_PER_TURN;
        let exact = degrees * per_degree + self.carry;
        // `as` saturates, and `Crank::turn_steps` clamps.
        let steps = exact as i32;
        self.carry = exact - steps as f32;
        steps
    }
}

/// The Playdate's buttons and crank, read once per frame.
#[derive(Debug, Clone)]
pub struct PadInput {
    pads: Pads,
    crank: Crank,
    steps: Steps,
}

impl PadInput {
    /// Nothing held yet, a crank's notch `crank` long.
    pub fn new(crank: CrankDef) -> Self {
        Self {
            // The Playdate has no stick, but `Pads` wants its thresholds.
            pads: Pads::new(StickDef::default()),
            crank: Crank::new(crank),
            steps: Steps::default(),
        }
    }

    /// This frame's events: the button releases, then presses, then the
    /// crank's notches. `current` is the bits held now and `pushed` those
    /// pressed since the last frame: a button pressed and let go between
    /// two frames counts as held for this one, so a quick tap is never lost
    /// while a frame takes long. `turned` is the crank's degrees since the
    /// last frame.
    pub fn poll(
        &mut self,
        current: u32,
        pushed: u32,
        turned: f32,
        docked: bool,
    ) -> Vec<RawInputEvent> {
        let held = state(current | pushed);
        let mut events =
            RawInputEvent::from_pads(&mut self.pads, &[(PAD, PadKind::Playdate, held)]);
        if docked {
            self.steps = Steps::default();
        }
        let steps = self.steps.of(turned);
        events.extend(RawInputEvent::from_crank_steps(
            &mut self.crank,
            steps,
            docked,
            PadKind::Playdate,
        ));
        events
    }

    /// The crank's notch from now on: the game's keymap file says it.
    pub fn set_crank(&mut self, crank: CrankDef) {
        self.crank = Crank::new(crank);
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use shuyi_assets::target::TargetProfile;

    use super::*;

    const LEFT: u32 = 1;
    const UP: u32 = 4;
    const B: u32 = 16;
    const A: u32 = 32;

    fn pad() -> PadInput {
        PadInput::new(CrankDef::default())
    }

    fn down(button: Button) -> RawInputEvent {
        RawInputEvent::PadDown(button, PadKind::Playdate)
    }

    fn up(button: Button) -> RawInputEvent {
        RawInputEvent::PadUp(button)
    }

    /// The presses and releases of `button`, `times` times.
    fn notches(button: Button, times: usize) -> Vec<RawInputEvent> {
        (0..times)
            .flat_map(|_| [down(button), up(button)])
            .collect()
    }

    #[test]
    fn a_is_east_b_is_west_and_the_d_pad_is_the_d_pad() {
        let mut p = pad();
        assert_eq!(p.poll(A, A, 0.0, true), [down(Button::East)]);
        assert_eq!(
            p.poll(B | UP | LEFT, B | UP | LEFT, 0.0, true),
            [
                up(Button::East),
                down(Button::West),
                down(Button::DpadUp),
                down(Button::DpadLeft),
            ]
        );
        assert_eq!(state(0b11_1111).buttons.iter().count(), 6);
        assert_eq!(state(1 << 6), PadState::default(), "no other bits");
    }

    /// Whatever the buttons report, the profile lists: its check and the
    /// device agree.
    #[test]
    fn every_button_is_one_the_profile_lists() {
        let profile = TargetProfile::from_source(
            "targets/playdate.ron",
            "playdate",
            include_str!("../../../targets/playdate.ron"),
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
        for (_, button) in BUTTONS {
            assert!(
                profile.input.buttons.contains(&button),
                "{button:?} isn't in {:?}",
                profile.input.buttons
            );
        }
    }

    #[test]
    fn a_held_button_is_one_press_until_it_is_let_go() {
        let mut p = pad();
        assert_eq!(p.poll(A, A, 0.0, true), [down(Button::East)]);
        assert_eq!(p.poll(A, 0, 0.0, true), []);
        assert_eq!(p.poll(0, 0, 0.0, true), [up(Button::East)]);
        assert_eq!(p.poll(0, 0, 0.0, true), []);
    }

    #[test]
    fn a_tap_between_two_frames_is_not_lost() {
        let mut p = pad();
        assert_eq!(p.poll(0, B, 0.0, true), [down(Button::West)]);
        assert_eq!(p.poll(0, 0, 0.0, true), [up(Button::West)]);
    }

    /// The ticket: 90° at the default 45° a notch.
    #[test]
    fn a_crank_turn_of_90_degrees_gives_two_notches_forward() {
        let mut p = pad();
        assert_eq!(p.poll(0, 0, 90.0, false), notches(Button::CrankForward, 2));
        assert_eq!(p.poll(0, 0, -45.0, false), notches(Button::CrankBack, 1));
    }

    /// A turn too small to be a step in any one frame still adds up.
    #[test]
    fn a_slow_turn_gets_there_in_the_same_notches() {
        let mut p = pad();
        let mut events = vec![];
        for _ in 0..450 {
            events.extend(p.poll(0, 0, 0.2, false));
        }
        assert_eq!(events, notches(Button::CrankForward, 2));
    }

    #[test]
    fn a_docked_crank_gives_nothing_and_forgets_a_part_notch() {
        let mut p = pad();
        assert_eq!(p.poll(0, 0, 44.0, false), []);
        assert_eq!(p.poll(0, 0, 90.0, true), []);
        assert_eq!(p.poll(0, 0, 44.0, false), []);
    }

    #[test]
    fn the_keymap_sets_the_notch() {
        let mut p = pad();
        p.set_crank(CrankDef { notch_degrees: 90 });
        assert_eq!(p.poll(0, 0, 90.0, false), notches(Button::CrankForward, 1));
    }

    #[test]
    fn a_broken_reading_is_no_turn() {
        let mut p = pad();
        assert_eq!(p.poll(0, 0, f32::NAN, false), []);
        assert_eq!(p.poll(0, 0, f32::INFINITY, false), []);
        assert_eq!(p.poll(0, 0, 45.0, false), notches(Button::CrankForward, 1));
    }
}
