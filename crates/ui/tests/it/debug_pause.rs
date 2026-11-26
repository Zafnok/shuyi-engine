//! The debug view's pause, step and slow motion, and its readout
//! (ADR-0153), through the Harness on the toy game: a paused game updates
//! none of its screens and drops their input, a step runs one update of
//! 1/60 s, a game paused and stepped through a script ends where the
//! script played straight does, the debug tools' screens still work on a
//! paused game, slow motion updates one frame in `slow` and keeps the
//! input of the frames between, and the readout names the pause first.

use std::cell::Cell as Shared;
use std::rc::Rc;

use insta::assert_snapshot;
use proptest::prelude::*;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_ui::audio::AudioRequest;
use shuyi_ui::debug::gizmo::READOUT;
use shuyi_ui::frame::{GizmoShape, GlyphBuffer};
use shuyi_ui::input::{Chord, Key};
use shuyi_ui::module::{Hook, Install, Module};
use shuyi_ui::toy::grid::Field;
use shuyi_ui::toy::tick::DotStage;
use shuyi_ui::toy::{self, Action, Toy, ToyCtx};
use shuyi_ui::{FrameInput, Game, GameHooks, RawInputEvent, STEP_DT, Screen, Transition};

use super::{Log, Recorder};

/// The toy's grid field, with the toy's hooks: its Debug key opens the
/// gallery, a debug tools' screen.
fn field() -> Harness<Toy> {
    let game = Game::new(toy::ctx(), Box::new(Field::glyphs()), toy::hooks());
    Harness::new(game)
}

/// A [`Recorder`] with the toy's hooks, and its log: one entry an update.
fn recorder() -> (Harness<Toy>, Log) {
    let log = Log::default();
    let screen = Box::new(Recorder(Rc::clone(&log)));
    (
        Harness::new(Game::new(toy::ctx(), screen, toy::hooks())),
        log,
    )
}

fn down(name: &str) -> RawInputEvent {
    RawInputEvent::Down(Chord::parse(name).unwrap_or_else(|e| panic!("{e}")))
}

fn up(name: &str) -> RawInputEvent {
    RawInputEvent::Up(Key::from_name(name).unwrap_or_else(|| panic!("no key {name}")))
}

fn hash(h: &Harness<Toy>) -> shuyi_core::StateHash {
    h.game()
        .state_hash()
        .unwrap_or_else(|| panic!("the field has a hash"))
}

#[test]
fn a_step_runs_one_update_of_a_sixtieth_and_pausing_none() {
    let (mut h, log) = recorder();
    h.wait(0.05);
    let ran = log.borrow().len();
    assert_eq!(ran, 3);
    // The press that pauses runs no update, and no frame after it does.
    h.keys("F3").wait(0.5);
    assert!(h.ctx().debug_view.paused());
    assert_eq!(log.borrow().len(), ran);
    // A step: the frame of the press runs one update, whatever its own time.
    h.frame(&[down("F4")], 0.25).frame(&[up("F4")], FRAME_DT);
    assert_eq!(log.borrow().len(), ran + 1);
    let stepped = log.borrow().last().cloned().unwrap();
    assert!(
        (stepped.dt - STEP_DT).abs() < f32::EPSILON,
        "{}",
        stepped.dt
    );
    // Neither debug action reaches a screen.
    assert!(stepped.actions.is_empty(), "{:?}", stepped.actions);
    assert_eq!(h.ctx().debug_view.steps(), 1);
    // Running again.
    h.keys("F3");
    assert!(!h.ctx().debug_view.paused());
    assert_eq!(log.borrow().len(), ran + 3);
    // While the game runs, a step does nothing.
    h.keys("F4");
    assert_eq!(log.borrow().len(), ran + 5);
    assert!(log.borrow().iter().all(|seen| seen.actions.is_empty()));
}

/// Plays `frames` straight on the toy's grid field, or paused from the
/// start and stepped through them, each step after `extra` paused frames
/// with nothing (cycled), and gives the hash after the last.
fn played(frames: &[Vec<RawInputEvent>], paused: Option<&[u8]>) -> shuyi_core::StateHash {
    let mut h = field();
    match paused {
        None => {
            for events in frames {
                h.frame(events, FRAME_DT);
            }
        }
        Some(extra) => {
            h.keys("F3");
            for (i, events) in frames.iter().enumerate() {
                let waits = extra.get(i % extra.len().max(1)).copied().unwrap_or(0);
                for _ in 0..waits {
                    h.frame(&[], FRAME_DT);
                }
                let mut stepped = events.clone();
                stepped.push(down("F4"));
                h.frame(&stepped, FRAME_DT);
                h.frame(&[up("F4")], FRAME_DT);
            }
            assert!(h.ctx().debug_view.paused());
            assert_eq!(h.ctx().debug_view.steps() as usize, frames.len());
        }
    }
    hash(&h)
}

/// A press then a release of each of `keys`, a frame each.
fn taps(keys: &[&str]) -> Vec<Vec<RawInputEvent>> {
    keys.iter()
        .flat_map(|&k| [vec![down(k)], vec![up(k)]])
        .collect()
}

#[test]
fn the_grid_game_paused_and_stepped_through_a_script_ends_as_the_script_played_straight() {
    // Pick up the piece, walk it right, and let it walk, then move on.
    let mut frames = taps(&["f", "Right", "Right", "f"]);
    frames.extend(std::iter::repeat_n(Vec::new(), 30));
    frames.extend(taps(&["Down", "d"]));
    let straight = played(&frames, None);
    assert_eq!(played(&frames, Some(&[0])), straight);
    assert_eq!(played(&frames, Some(&[3, 0, 1])), straight);
    // And the script moved the game: a paused game that never steps
    // doesn't end there.
    let mut h = field();
    h.keys("F3");
    for events in &frames {
        h.frame(events, FRAME_DT);
    }
    assert_ne!(hash(&h), straight);
}

/// One frame of a script: a key of the field's going down or up.
fn any_frame() -> impl Strategy<Value = Vec<RawInputEvent>> {
    let key = prop::sample::select(vec!["f", "d", "Up", "Down", "Left", "Right"]);
    prop_oneof![
        key.clone().prop_map(|k| vec![down(k)]),
        key.prop_map(|k| vec![up(k)]),
        Just(Vec::new()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn any_script_paused_and_stepped_through_ends_as_played_straight(
        frames in prop::collection::vec(any_frame(), 0..40),
        extra in prop::collection::vec(0u8..3, 1..5),
    ) {
        prop_assert_eq!(played(&frames, Some(&extra)), played(&frames, None));
    }
}

#[test]
fn a_confirm_pressed_while_paused_changes_nothing_then_or_after_the_game_runs_again() {
    let mut h = field();
    h.keys("F3");
    let before = hash(&h);
    h.keys("f").wait(0.2);
    assert_eq!(hash(&h), before);
    // Unpaused: the press was dropped, not kept for later.
    let (mut again, log) = recorder();
    again.keys("F3").keys("f").keys("F3").wait(0.1);
    assert!(log.borrow().iter().all(|seen| seen.actions.is_empty()));
    h.keys("F3");
    assert!(h.screen::<Field>().unwrap().selected.is_none());
}

#[test]
fn the_debug_key_opens_the_gallery_on_a_paused_game_and_cancel_closes_it() {
    let mut h = field();
    h.wait(0.1).keys("F3");
    let before = hash(&h);
    h.keys("F2");
    assert_eq!(h.top_screen(), "gallery");
    // The gallery updates, with the frame's input, on a paused game.
    h.keys("d");
    assert_eq!(h.top_screen(), "field");
    assert!(h.ctx().debug_view.paused());
    assert_eq!(hash(&h), before);
    // Cancel on the field itself is dropped.
    h.keys("Right").keys("d");
    assert_eq!(hash(&h), before);
}

#[test]
fn slow_motion_of_four_runs_ten_updates_in_forty_frames_each_of_the_frames_time() {
    let (mut h, log) = recorder();
    h.ctx_mut().debug_view.set_slow(4);
    for _ in 0..40 {
        h.frame(&[], FRAME_DT);
    }
    assert_eq!(log.borrow().len(), 10);
    assert!(
        log.borrow()
            .iter()
            .all(|seen| (seen.dt - FRAME_DT).abs() < f32::EPSILON)
    );
}

#[test]
fn a_confirm_tapped_between_slow_motions_updates_reaches_the_next_one_once() {
    let (mut h, log) = recorder();
    h.ctx_mut().debug_view.set_slow(4);
    // Frames 1 and 2 of 4 run nothing: the tap waits for frame 4.
    h.frame(&[down("f")], FRAME_DT).frame(&[up("f")], FRAME_DT);
    assert_eq!(log.borrow().len(), 0);
    h.frame(&[], FRAME_DT).frame(&[], FRAME_DT);
    assert_eq!(log.borrow().len(), 1);
    assert_eq!(log.borrow()[0].actions, [Action::Confirm]);
    for _ in 0..8 {
        h.frame(&[], FRAME_DT);
    }
    let confirms = log
        .borrow()
        .iter()
        .filter(|seen| seen.actions.contains(&Action::Confirm))
        .count();
    assert_eq!((log.borrow().len(), confirms), (3, 1));
}

#[test]
fn a_pause_pressed_in_slow_motion_acts_at_once_and_drops_what_slow_motion_kept() {
    let (mut h, log) = recorder();
    h.ctx_mut().debug_view.set_slow(4);
    h.frame(&[down("f")], FRAME_DT);
    h.frame(&[up("f"), down("F3")], FRAME_DT);
    assert!(h.ctx().debug_view.paused());
    h.frame(&[up("F3")], FRAME_DT).wait(0.2);
    h.keys("F3").wait(0.2);
    assert!(!log.borrow().is_empty());
    assert!(log.borrow().iter().all(|seen| seen.actions.is_empty()));
}

/// A module that counts its `before_update` calls.
struct Counted(Rc<Shared<u32>>);

impl Module<Toy> for Counted {
    fn name(&self) -> &'static str {
        "counted"
    }

    fn install(&mut self, install: &mut Install<'_, Toy>) {
        install.hook(Hook::BeforeUpdate);
    }

    fn before_update(&mut self, _: &mut ToyCtx, dt: f32) {
        assert!(dt > 0.0, "a module runs only with time passing");
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn a_paused_game_holds_its_modules_and_a_step_runs_them_once() {
    let calls = Rc::new(Shared::new(0));
    let game = Game::new(toy::ctx(), Box::new(Field::glyphs()), toy::hooks())
        .with_module(Counted(Rc::clone(&calls)))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut h = Harness::new(game);
    h.wait(0.05);
    assert_eq!(calls.get(), 3);
    h.keys("F3").wait(0.5);
    assert_eq!(calls.get(), 3);
    h.keys("F4");
    assert_eq!(calls.get(), 4);
    // Under the gallery, on a paused game, the modules still wait.
    h.keys("F2").wait(0.1).keys("d");
    assert_eq!(calls.get(), 4);
}

#[test]
fn with_debug_tools_off_the_pause_and_the_step_do_nothing() {
    let (mut h, log) = recorder();
    h.ctx_mut().debug_tools = false;
    h.keys("F3").wait(0.05).keys("F4");
    assert!(!h.ctx().debug_view.paused());
    assert_eq!(log.borrow().len(), 7);
    assert!(log.borrow().iter().all(|seen| seen.actions.is_empty()));
    // Paused from outside, with debug tools off, the game still runs.
    h.ctx_mut().debug_view.set_paused(true);
    h.wait(0.05);
    assert_eq!(log.borrow().len(), 10);
    // The same over `act`: the actions never reach a screen.
    h.act(Action::DebugPause).act(Action::DebugStep);
    assert!(log.borrow().iter().all(|seen| seen.actions.is_empty()));
}

#[test]
fn act_pauses_steps_and_drops_the_paused_games_other_actions() {
    let (mut h, log) = recorder();
    h.act(Action::DebugPause);
    assert!(h.ctx().debug_view.paused());
    let ran = log.borrow().len();
    h.act(Action::Confirm);
    assert_eq!(log.borrow().len(), ran);
    // A step asked for runs in the next frame.
    h.act(Action::DebugStep);
    assert_eq!(log.borrow().len(), ran + 1);
    // Running again, an action reaches the screen.
    h.act(Action::DebugPause);
    let ran = log.borrow().len();
    h.act(Action::Confirm);
    assert_eq!(log.borrow()[ran].actions, [Action::Confirm]);
}

#[test]
fn the_pause_holds_the_sounds_with_a_handle_and_a_step_doesnt_let_them_go() {
    let mut h = field();
    h.keys("F3").keys("F4").keys("F4").keys("F3");
    let pauses: Vec<_> = h
        .audio_log()
        .into_iter()
        .filter(|(_, r)| matches!(r, AudioRequest::PauseSounds | AudioRequest::ResumeSounds))
        .map(|(_, r)| r.clone())
        .collect();
    assert_eq!(
        pauses,
        [AudioRequest::PauseSounds, AudioRequest::ResumeSounds]
    );
}

/// The readout's labels, in order.
fn readout(h: &Harness<Toy>) -> Vec<String> {
    h.game()
        .buffer()
        .gizmos()
        .iter()
        .filter_map(|g| match &g.shape {
            GizmoShape::Label { text, .. } => Some(text.as_str().to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_readout_names_the_pause_then_the_top_screens_lines() {
    let mut h = field();
    h.show_debug(READOUT.id());
    assert_eq!(readout(&h), ["cursor 2,3", "piece 2,3", "clock 0 ms"]);
    h.keys("F3");
    assert_eq!(readout(&h)[0], "paused");
    h.keys("F4").keys("F4");
    assert_eq!(readout(&h)[..2], ["paused, step 2", "cursor 2,3"]);
    assert_snapshot!(h.snapshot());
    // Its own channel: off, none of it.
    h.ctx_mut().debug_view.set(READOUT.id(), false);
    h.wait(0.0);
    assert_eq!(readout(&h), Vec::<String>::new());
}

#[test]
fn a_step_of_a_tick_stage_is_one_fixed_step_and_its_readout_names_it() {
    let game = Game::new(toy::ctx(), Box::new(DotStage::new()), GameHooks::default());
    let mut h = Harness::new(game);
    h.show_debug(READOUT.id()).steps(5);
    let steps = |h: &Harness<Toy>| h.screen::<DotStage>().unwrap().driver.steps();
    assert_eq!(steps(&h), 5);
    h.keys("F3");
    let paused = steps(&h);
    h.wait(0.5);
    assert_eq!(steps(&h), paused);
    // A long frame steps once all the same.
    h.frame(&[down("F4")], 0.25).frame(&[up("F4")], FRAME_DT);
    assert_eq!(steps(&h), paused + 1);
    assert_eq!(
        readout(&h)[..2],
        ["paused, step 1", &format!("step {}", paused + 1)]
    );
}

/// A screen that does nothing, drawn as nothing.
struct Still;

impl Screen<Toy> for Still {
    fn name(&self) -> &'static str {
        "still"
    }

    fn update(&mut self, _: &mut ToyCtx, _: &FrameInput<Action>) -> Transition<Toy> {
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

#[test]
fn a_screen_with_no_readout_shows_only_the_pause() {
    let mut h = Harness::new(Game::new(toy::ctx(), Box::new(Still), GameHooks::default()));
    h.show_debug(READOUT.id());
    assert_eq!(readout(&h), Vec::<String>::new());
    h.keys("F3");
    assert_eq!(readout(&h), ["paused"]);
}
