//! A resume from a suspend lets go of what was held before it (ticket
//! 2411): a key let go while the game was suspended, which the game never
//! heard come up, neither repeats nor stays held after the resume, and a
//! key pressed in the first frame after it counts.

use std::rc::Rc;

use shuyi_platform::SystemEvent;
use shuyi_platform::harness::FRAME_DT;
use shuyi_ui::input::{Chord, Key};
use shuyi_ui::toy::{self, Action};
use shuyi_ui::{Game, GameHooks, RawInputEvent};

use super::{Log, Recorder, recorder};

/// The key `key` going down.
fn down(key: Key) -> RawInputEvent {
    RawInputEvent::Down(Chord::plain(key))
}

#[test]
fn a_key_let_go_while_suspended_stops_repeating_at_the_resume() {
    let (mut h, log) = recorder();
    // Down held long enough to repeat.
    h.frame(&[down(Key::Down)], FRAME_DT).wait(0.5);
    let repeats = log
        .borrow()
        .iter()
        .filter(|s| s.actions.contains(&Action::CursorDown))
        .count();
    assert!(repeats > 2, "{repeats}");
    // Suspended, Down comes up unheard, and the game resumes.
    h.system(SystemEvent::Suspend);
    h.frame(&[RawInputEvent::Up(Key::Down)], FRAME_DT);
    let before = log.borrow().len();
    h.system(SystemEvent::Resume).wait(1.0);
    let after = &log.borrow()[before..];
    assert_ne!(after, []);
    assert!(
        after
            .iter()
            .all(|s| s.actions.is_empty() && s.held.is_empty()),
        "{after:?}"
    );
}

#[test]
fn a_key_pressed_in_the_first_frame_after_a_resume_counts() {
    // The Harness runs a resume in a frame of its own, so this drives the
    // game as `App` does on a resume with a press in the same frame.
    let log = Log::default();
    let screen = Box::new(Recorder(Rc::clone(&log)));
    let mut game = Game::new(toy::ctx(), screen, GameHooks::default());
    let _ = game.frame(&[down(Key::Down)], FRAME_DT);
    game.set_resumed();
    let _ = game.frame(&[down(Key::Up)], FRAME_DT);
    let seen = log.borrow();
    let first = seen.last().unwrap_or_else(|| panic!("no frame"));
    assert_eq!(first.actions, [Action::CursorUp]);
    assert_eq!(first.held, [Action::CursorUp]);
}
