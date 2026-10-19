//! The Harness's scripts (ticket 1111): key, typing, controller and wait
//! scripts in frames of 1/60 s, and what it reports of the game.

use shuyi_ui::harness::{FRAME_DT, Harness, MAX_FRAMES};
use shuyi_ui::input::Button;
use shuyi_ui::toy::{Action, Home};
use shuyi_ui::{KeyPrompt, RawInputEvent};

use crate::{Log, recorder, toy};

fn actions(log: &Log) -> Vec<Vec<Action>> {
    log.borrow().iter().map(|s| s.actions.clone()).collect()
}

fn total_time(log: &Log) -> f32 {
    log.borrow().iter().map(|s| s.dt).sum()
}

fn all_one_frame_long(log: &Log) -> bool {
    log.borrow()
        .iter()
        .all(|s| (s.dt - FRAME_DT).abs() < f32::EPSILON)
}

/// How many times `action` came in the frames seen.
fn count(log: &Log, action: Action) -> usize {
    let log = log.borrow();
    log.iter()
        .flat_map(|s| s.actions.iter())
        .filter(|&&a| a == action)
        .count()
}

#[test]
fn keys_press_and_release_each_chord_over_two_frames() {
    let (mut h, log) = recorder();
    h.keys("  f\tShift+Space  Down ");
    assert_eq!(
        actions(&log),
        [
            vec![Action::Confirm],
            vec![],
            vec![Action::ToggleAutoEnd],
            vec![],
            vec![Action::CursorDown],
            vec![],
        ]
    );
    assert!(all_one_frame_long(&log));
}

#[test]
fn type_text_types_each_character_with_its_key() {
    let (mut h, log) = recorder();
    h.type_text("Hi é");
    let log = log.borrow();
    let text: Vec<&str> = log.iter().map(|s| s.text.as_str()).collect();
    // `H` with Shift and its release, `i` and its release, Space and its
    // release, and `é`, which has no key, alone.
    assert_eq!(text, ["H", "", "i", "", " ", "", "é"]);
    // Shift only for the capital.
    let chords: Vec<&[String]> = log.iter().map(|s| s.chords.as_slice()).collect();
    let none: &[String] = &[];
    assert_eq!(
        chords,
        [
            &["Shift+h".to_owned()][..],
            none,
            &["i".to_owned()][..],
            none,
            &["Space".to_owned()][..],
            none,
            none,
        ]
    );
    // Space is a key too: the sample game's End turn.
    assert_eq!(log[4].actions, [Action::EndTurn]);
}

#[test]
fn hold_repeats_then_releases() {
    let (mut h, log) = recorder();
    h.hold("Down", 0.5);
    // Press, then repeats at 300 ms and every 55 ms up to 500 ms (the
    // sample keymap's repeat).
    assert_eq!(count(&log, Action::CursorDown), 1 + 1 + (500 - 300) / 55);
    assert!((total_time(&log) - (0.5 + FRAME_DT)).abs() < 1e-4);
    // Released: waiting gives nothing more.
    log.borrow_mut().clear();
    h.wait(1.0);
    assert!(log.borrow().iter().all(|s| s.actions.is_empty()));
}

#[test]
fn pad_presses_and_releases_each_button_over_two_frames() {
    let (mut h, log) = recorder();
    h.pad(" South\tDpadDown  Start ");
    assert_eq!(
        actions(&log),
        [
            vec![Action::Confirm],
            vec![],
            vec![Action::CursorDown],
            vec![],
            vec![Action::EndTurn],
            vec![],
        ]
    );
    assert!(all_one_frame_long(&log));
}

#[test]
fn buttons_joined_by_plus_go_down_together_in_the_order_written() {
    let (mut h, log) = recorder();
    h.pad("East+DpadRight");
    let log = log.borrow();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].buttons, [Button::East, Button::DpadRight]);
    assert!(log[1].buttons.is_empty());
}

#[test]
fn hold_pad_repeats_with_the_keyboard_timings_then_releases() {
    let (mut h, log) = recorder();
    h.hold_pad("DpadDown", 0.5);
    assert_eq!(count(&log, Action::CursorDown), 1 + 1 + (500 - 300) / 55);
    assert!((total_time(&log) - (0.5 + FRAME_DT)).abs() < 1e-4);
    let (mut keys, keys_log) = recorder();
    keys.hold("Down", 0.5);
    assert_eq!(actions(&log), actions(&keys_log));
    log.borrow_mut().clear();
    h.wait(1.0);
    assert!(log.borrow().iter().all(|s| s.actions.is_empty()));
}

#[test]
fn wait_runs_frames_of_at_most_frame_dt() {
    let (mut h, log) = recorder();
    h.wait(0.1);
    assert!((total_time(&log) - 0.1).abs() < 1e-4);
    assert!(log.borrow().iter().all(|s| s.dt <= FRAME_DT));
    // Nothing, less or not a number: one empty frame each.
    log.borrow_mut().clear();
    h.wait(0.0).wait(-1.0).wait(f32::NAN);
    let dts: Vec<f32> = log.borrow().iter().map(|s| s.dt).collect();
    assert_eq!(dts, [0.0, 0.0, 0.0]);
}

#[test]
fn long_waits_are_refused() {
    let (mut h, log) = recorder();
    h.wait(32.0);
    let frames = log.borrow().len();
    let most = usize::try_from(MAX_FRAMES).unwrap_or(usize::MAX);
    assert!(frames > 1_900 && frames <= most, "{frames} frames");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        h.wait(34.0);
    }));
    assert!(result.is_err(), "a wait over MAX_FRAMES must panic");
}

#[test]
#[should_panic(expected = "bad chord in test script")]
fn a_bad_chord_panics() {
    toy().keys("Ctrl+x");
}

#[test]
#[should_panic(expected = "bad button in test script")]
fn a_bad_button_panics() {
    toy().pad("f");
}

#[test]
fn frame_runs_one_frame_of_the_events_given() {
    let (mut h, log) = recorder();
    let f = shuyi_ui::input::Chord::parse("f").unwrap_or_else(|e| panic!("{e}"));
    h.frame(&[RawInputEvent::Down(f)], 0.25);
    assert_eq!(actions(&log), [vec![Action::Confirm]]);
    assert!((total_time(&log) - 0.25).abs() < f32::EPSILON);
}

/// The toy game through the Harness: Confirm opens the menu over the home
/// screen, Cancel closes it, Cancel again quits.
#[test]
fn reports_the_screens_the_frame_and_a_quit() {
    let mut h = toy();
    assert_eq!(h.screens(), ["home"]);
    assert_eq!(h.top_screen(), "home");
    assert_eq!(
        h.snapshot(),
        h.game().buffer().to_snapshot(&h.ctx().palette)
    );
    assert!(h.snapshot().starts_with('H'), "{}", h.snapshot());
    h.keys("f");
    assert_eq!(h.screens(), ["home", "menu"]);
    assert!(h.snapshot().starts_with("HM"), "{}", h.snapshot());
    h.keys("d");
    assert_eq!(h.top_screen(), "home");
    assert!(!h.quit_requested());
    h.keys("d");
    assert!(h.quit_requested());
    // Home saw the press that opened the menu, the release after the
    // menu closed, and the press that quit.
    assert_eq!(h.screen::<Home>().map(|s| s.frames), Some(3));
}

#[test]
fn a_screen_on_the_stack_is_changed_through_the_harness() {
    let mut h = toy();
    if let Some(home) = h.screen_mut::<Home>() {
        home.frames = 40;
    }
    h.wait(0.0);
    assert_eq!(h.screen::<Home>().map(|s| s.frames), Some(41));
}

#[test]
fn at_prompt_waits_for_the_first_press() {
    let mut h = toy().at_prompt();
    assert_eq!(h.ctx().key_prompt, KeyPrompt::Waiting);
    h.wait(1.0);
    assert_eq!(h.ctx().key_prompt, KeyPrompt::Waiting);
    h.pad("South");
    assert_eq!(h.ctx().key_prompt, KeyPrompt::Pressed);
    let mut h = toy().at_prompt();
    h.keys("Up");
    assert_eq!(h.ctx().key_prompt, KeyPrompt::Pressed);
}

#[test]
fn the_storage_carries_over_to_the_next_launch() {
    let mut h = toy();
    h.ctx_mut()
        .change_settings(|s| s.game.speed = 7)
        .unwrap_or_else(|e| panic!("{e}"));
    let storage = h.into_storage();
    let h = Harness::new(shuyi_ui::toy::game_on(shuyi_ui::toy::launch(storage)));
    assert_eq!(h.ctx().settings().game.speed, 7);
}
