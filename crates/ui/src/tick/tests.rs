//! The latch with frames built by hand: taps, late frames, presses twice
//! before a step, keys held since before, clearing, and events handed on
//! once across a rewind.

use super::*;
use crate::input::sample::Action::{self, CursorLeft, CursorRight, CursorUp};

/// A simulation that keeps what each step saw, and says which actions
/// each step pressed.
#[derive(Debug, Default)]
struct Seen(Vec<SeatInput>);

impl TickSim for Seen {
    type Event = ActionId;

    fn step(&mut self, input: &TickInput) -> Vec<ActionId> {
        let seat = input.seat(0).cloned().unwrap_or_default();
        let pressed = seat.pressed.clone();
        self.0.push(seat);
        pressed
    }

    fn hash(&self) -> u64 {
        u64::try_from(self.0.len()).unwrap_or(u64::MAX)
    }
}

/// A frame of `ticks` steps that pressed `presses`, let go of `released`
/// and ends with `held`.
fn frame(
    ticks: u32,
    presses: &[Action],
    released: &[Action],
    held: &[Action],
) -> FrameInput<Action> {
    FrameInput::new(presses.to_vec(), 0.0, held.to_vec())
        .with_presses(presses.to_vec(), released.to_vec())
        .with_ticks(ticks)
}

/// The ids of `actions`.
fn ids(actions: &[Action]) -> Vec<ActionId> {
    actions.iter().map(|&a| a.id()).collect()
}

/// A step's seat input.
fn seat(pressed: &[Action], held: &[Action], released: &[Action]) -> SeatInput {
    SeatInput::new(ids(pressed), ids(held), ids(released))
}

/// Runs `frames` through a new driver.
fn run(frames: &[FrameInput<Action>]) -> Vec<SeatInput> {
    let mut driver = TickDriver::new();
    let mut sim = Seen::default();
    for input in frames {
        driver.update(&mut sim, input);
    }
    sim.0
}

#[test]
fn an_action_has_its_place_in_the_list_as_its_id() {
    assert_eq!(CursorLeft.id(), ActionId(0));
    assert_eq!(Action::Confirm.id(), ActionId(4));
    for action in Action::ALL {
        assert_eq!(Action::from_id(action.id()), Some(action));
    }
    assert_eq!(Action::from_id(ActionId(500)), None);
}

#[test]
fn a_tap_on_a_frame_with_no_step_is_pressed_in_the_next_and_released_after() {
    let steps = run(&[
        frame(0, &[CursorRight], &[CursorRight], &[]),
        frame(1, &[], &[], &[]),
        frame(1, &[], &[], &[]),
        frame(1, &[], &[], &[]),
    ]);
    assert_eq!(
        steps,
        [
            seat(&[CursorRight], &[], &[]),
            seat(&[], &[], &[CursorRight]),
            seat(&[], &[], &[]),
        ]
    );
}

#[test]
fn a_press_on_a_late_frame_is_pressed_in_its_first_step_only() {
    let steps = run(&[frame(4, &[CursorUp], &[], &[CursorUp])]);
    assert_eq!(steps.len(), 4);
    assert_eq!(steps[0], seat(&[CursorUp], &[CursorUp], &[]));
    assert!(steps[1..].iter().all(|s| *s == seat(&[], &[CursorUp], &[])));
    // A tap in a late frame: pressed in its first step, released in its
    // second.
    let steps = run(&[frame(3, &[CursorUp], &[CursorUp], &[])]);
    assert_eq!(steps[0], seat(&[CursorUp], &[], &[]));
    assert_eq!(steps[1], seat(&[], &[], &[CursorUp]));
}

#[test]
fn every_step_a_frame_runs_sees_what_it_holds_at_its_end() {
    let steps = run(&[
        frame(1, &[CursorLeft], &[], &[CursorLeft]),
        frame(2, &[CursorUp], &[], &[CursorLeft, CursorUp]),
    ]);
    assert_eq!(steps[1].held, ids(&[CursorLeft, CursorUp]));
    assert_eq!(steps[2].held, ids(&[CursorLeft, CursorUp]));
    assert_eq!(steps[1].pressed, ids(&[CursorUp]));
}

#[test]
fn presses_in_one_frame_keep_their_order() {
    let steps = run(&[frame(
        1,
        &[CursorUp, CursorLeft],
        &[],
        &[CursorLeft, CursorUp],
    )]);
    assert_eq!(steps[0].pressed, ids(&[CursorUp, CursorLeft]));
}

#[test]
fn an_action_pressed_twice_before_a_step_waits_and_nothing_is_lost() {
    // Two taps in a frame with no step, then a step a frame.
    let steps = run(&[
        frame(
            0,
            &[CursorRight, CursorRight],
            &[CursorRight, CursorRight],
            &[],
        ),
        frame(1, &[], &[], &[]),
        frame(1, &[], &[], &[]),
        frame(1, &[], &[], &[]),
        frame(1, &[], &[], &[]),
    ]);
    assert_eq!(
        steps,
        [
            seat(&[CursorRight], &[], &[]),
            seat(&[CursorRight], &[], &[CursorRight]),
            seat(&[], &[], &[CursorRight]),
            seat(&[], &[], &[]),
        ]
    );
    // Let go and pressed again before a step: released and pressed in it.
    let steps = run(&[
        frame(1, &[CursorUp], &[], &[CursorUp]),
        frame(0, &[CursorUp], &[CursorUp], &[CursorUp]),
        frame(1, &[], &[], &[CursorUp]),
    ]);
    assert_eq!(steps[1], seat(&[CursorUp], &[CursorUp], &[CursorUp]));
}

#[test]
fn a_second_key_on_a_held_action_presses_nothing() {
    // Another key on Up while Up is held, let go while the first holds it.
    let steps = run(&[
        frame(1, &[CursorUp], &[], &[CursorUp]),
        frame(1, &[CursorUp], &[], &[CursorUp]),
        frame(1, &[], &[], &[CursorUp]),
        frame(1, &[], &[CursorUp], &[]),
    ]);
    let pressed: usize = steps.iter().map(|s| s.pressed.len()).sum();
    let released: usize = steps.iter().map(|s| s.released.len()).sum();
    assert_eq!((pressed, released), (1, 1));
    assert_eq!(steps[3], seat(&[], &[], &[CursorUp]));
}

#[test]
fn a_key_held_since_before_the_stage_is_held_but_never_pressed_or_released() {
    let steps = run(&[
        frame(1, &[], &[], &[CursorLeft]),
        frame(1, &[], &[CursorLeft], &[]),
        frame(1, &[], &[], &[]),
    ]);
    assert_eq!(steps[0], seat(&[], &[CursorLeft], &[]));
    assert!(steps[1..].iter().all(SeatInput::is_empty), "{steps:?}");
}

#[test]
fn a_cleared_latch_forgets_presses_no_step_had_but_keeps_releases_of_those_it_had() {
    let mut driver = TickDriver::new();
    let mut sim = Seen::default();
    // Up pressed and given to a step; Left pressed on a frame with no step.
    driver.update(&mut sim, &frame(1, &[CursorUp], &[], &[CursorUp]));
    driver.update(
        &mut sim,
        &frame(0, &[CursorLeft], &[], &[CursorUp, CursorLeft]),
    );
    driver.clear();
    // Both let go while covered; the stage is back.
    driver.update(&mut sim, &frame(2, &[], &[], &[]));
    assert_eq!(sim.0[1], seat(&[], &[], &[CursorUp]));
    assert_eq!(sim.0[2], seat(&[], &[], &[]));
    // A frame after a suspend forgets too.
    driver.update(&mut sim, &frame(0, &[CursorRight], &[CursorRight], &[]));
    driver.update(&mut sim, &frame(1, &[], &[], &[]).with_resumed(true));
    assert!(sim.0[3].is_empty(), "{:?}", sim.0[3]);
}

#[test]
fn a_press_a_step_had_is_let_go_first_when_a_tap_follows_a_clear() {
    let mut driver = TickDriver::new();
    let mut sim = Seen::default();
    // Up given to a step, let go on a frame with no step, then cleared:
    // its release is forgotten, but Up is still down for the steps.
    driver.update(&mut sim, &frame(1, &[CursorUp], &[], &[CursorUp]));
    driver.update(&mut sim, &frame(0, &[], &[CursorUp], &[]));
    driver.clear();
    // A tap: the steps see the first press let go, then the tap.
    driver.update(&mut sim, &frame(2, &[CursorUp], &[CursorUp], &[]));
    assert_eq!(
        sim.0[1..],
        [
            seat(&[CursorUp], &[], &[CursorUp]),
            seat(&[], &[], &[CursorUp])
        ]
    );
}

#[test]
fn repeats_among_a_frames_actions_never_reach_a_step() {
    // Right held long enough to repeat twice: three actions, one press.
    let input = FrameInput::new(vec![CursorRight; 3], 0.0, vec![CursorRight])
        .with_presses(vec![CursorRight], vec![])
        .with_ticks(1);
    let steps = run(&[input]);
    assert_eq!(steps, [seat(&[CursorRight], &[CursorRight], &[])]);
}

#[test]
fn events_are_handed_on_once_with_their_step_and_a_rewind_never_hands_them_again() {
    let mut driver = TickDriver::new();
    let mut sim = Seen::default();
    let tap = || frame(1, &[CursorUp], &[CursorUp], &[]);
    let idle = || frame(1, &[], &[], &[]);
    let mut handed = Vec::new();
    for input in [tap(), idle(), idle(), idle(), tap()] {
        handed.extend(driver.update(&mut sim, &input));
    }
    assert_eq!(handed, [(1, CursorUp.id()), (5, CursorUp.id())]);
    // Back to step 2: steps 3 to 5 run again, their events kept back (step
    // 5's, the last handed on, included), and step 6 is new.
    driver.rewind_to(2);
    assert_eq!((driver.steps(), driver.handed()), (2, 5));
    let mut again = Vec::new();
    for input in [idle(), idle(), tap(), tap()] {
        again.extend(driver.update(&mut sim, &input));
    }
    assert_eq!(again, [(6, CursorUp.id())]);
    // A rewind forward changes nothing.
    driver.rewind_to(10);
    assert_eq!(driver.steps(), 6);
    // Catching up hands on every step's events, in order.
    let mut driver = TickDriver::new();
    let late = frame(4, &[CursorUp, CursorLeft], &[CursorUp, CursorLeft], &[]);
    let events = driver.update(&mut Seen::default(), &late);
    assert_eq!(events, [(1, CursorUp.id()), (1, CursorLeft.id())]);
}

#[test]
fn a_recording_keeps_what_each_step_saw_and_forgets_the_steps_after_a_rewind() {
    let made = shuyi_core::Made::new("seen", shuyi_core::ContentHash(0));
    let mut driver = TickDriver::new().recording(TickRecord::new(made, 3));
    let mut sim = Seen::default();
    driver.update(&mut sim, &frame(3, &[CursorUp], &[], &[CursorUp]));
    let record = driver
        .record()
        .cloned()
        .unwrap_or_else(|| panic!("no record"));
    let inputs: Vec<&TickInput> = record.inputs().collect();
    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs[0].seats, [seat(&[CursorUp], &[CursorUp], &[])]);
    assert_eq!(record.runs().len(), 2);
    driver.rewind_to(1);
    assert_eq!(driver.record().map(TickRecord::len), Some(1));
    assert_eq!(driver.take_record().map(|r| r.seed()), Some(3));
    assert!(driver.record().is_none());
}
