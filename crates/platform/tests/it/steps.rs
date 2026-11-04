//! `Harness::steps`: exactly the steps asked for, a frame of a step's time
//! each and the last in half frames, and a panic on a game that runs none.

use std::cell::RefCell;
use std::rc::Rc;

use super::*;

/// The steps each frame handed the screen, oldest first.
type Log = Rc<RefCell<Vec<u32>>>;

/// A screen that logs each frame's steps due.
struct Ticks(Log);

impl Screen<Toy> for Ticks {
    fn name(&self) -> &'static str {
        "ticks"
    }

    fn update(&mut self, _: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        self.0.borrow_mut().push(input.ticks());
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

/// The toy's context showing only a [`Ticks`], and its log.
fn ticks() -> (Harness<Toy>, Log) {
    let log = Log::default();
    let game = Game::new(
        toy::ctx(),
        Box::new(Ticks(Rc::clone(&log))),
        GameHooks::default(),
    );
    (Harness::new(game), log)
}

#[test]
fn steps_runs_exactly_the_steps_asked_for_in_about_as_many_frames() {
    let (mut h, log) = ticks();
    h.steps(10);
    let steps: u32 = log.borrow().iter().sum();
    assert_eq!(steps, 10);
    // A frame of a step's time each, and a half one or two for the last.
    assert!(h.frame_count() <= 12, "{} frames", h.frame_count());
    assert_eq!(h.app().ticks(), 1);
    // None asked for: no frame.
    let frames = h.frame_count();
    h.steps(0);
    assert_eq!(h.frame_count(), frames);
}

#[test]
fn the_last_step_comes_in_half_frames_so_it_never_overshoots() {
    let (mut h, log) = ticks();
    // A fifth of a step owed: a whole frame would run one step, two halves
    // run it in the second.
    h.frame(&[], FRAME_DT / 5.0);
    assert_eq!(h.app().ticks(), 0);
    let before = h.frame_count();
    h.steps(1);
    assert_eq!(h.frame_count() - before, 2);
    assert_eq!(log.borrow().iter().sum::<u32>(), 1);
}

#[test]
#[should_panic(expected = "1 of 1 steps still due")]
fn steps_on_a_suspended_game_fails_instead_of_hanging() {
    let (mut h, _) = ticks();
    h.system(SystemEvent::Suspend);
    h.steps(1);
}
