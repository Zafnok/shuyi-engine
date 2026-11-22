//! A key held while the window loses focus, or while the game sleeps,
//! through the Harness on the engine's headless backend: its release never
//! reaches the game, so the engine lets go of it, and the cursor stays
//! where the press left it. A pointer press held then is no click when let
//! go, and the pointer's place is forgotten.

use hello_grid::Hello;
use hello_grid::action::Action;
use shuyi::assets::keymap::{Chord, Key};
use shuyi::core::Pos;
use shuyi::platform::SystemEvent;
use shuyi::platform::harness::{FRAME_DT, Harness};
use shuyi::ui::RawInputEvent;

use crate::{stage, title};

/// The game on its stage, `Right` pressed in a frame with no release, and
/// where the press took the cursor.
fn right_held() -> (Harness<Hello>, Pos) {
    let mut h = title(1);
    h.keys("Enter Backspace");
    assert_eq!(h.screens(), ["title", "stage"]);
    let from = stage(&h).cursor.pos;
    h.frame(&[RawInputEvent::Down(Chord::plain(Key::Right))], FRAME_DT);
    let pressed = stage(&h).cursor.pos;
    assert_eq!(pressed, Pos::new(from.x + 1, from.y));
    assert!(h.game().input().is_held(Action::Right));
    (h, pressed)
}

#[test]
fn a_key_held_when_focus_is_lost_is_let_go_and_repeats_no_more() {
    let (mut h, pressed) = right_held();
    h.system(SystemEvent::FocusLost);
    assert!(!h.app().is_focused());
    assert!(!h.game().input().is_held(Action::Right));
    h.wait(1.0);
    assert_eq!(stage(&h).cursor.pos, pressed);
    // Back, and pressed again: it moves.
    h.system(SystemEvent::FocusGained);
    h.keys("Right");
    assert_eq!(stage(&h).cursor.pos, Pos::new(pressed.x + 1, pressed.y));
}

#[test]
fn a_key_let_go_while_asleep_is_let_go_after_the_resume() {
    let (mut h, pressed) = right_held();
    h.system(SystemEvent::Suspend);
    // Its release comes while the game sleeps, unheard.
    h.frame(&[RawInputEvent::Up(Key::Right)], FRAME_DT);
    h.system(SystemEvent::Resume);
    h.wait(1.0);
    assert!(!h.game().input().is_held(Action::Right));
    assert_eq!(stage(&h).cursor.pos, pressed);
}

/// The centre pixel of the title menu's first row, Play.
fn play_row(h: &Harness<Hello>) -> (i32, i32) {
    let hits = h.game().buffer().hits();
    let hit = hits
        .iter()
        .find(|hit| hit.item == 0)
        .unwrap_or_else(|| panic!("no row: {hits:?}"));
    (hit.rect.x + hit.rect.w / 2, hit.rect.y + hit.rect.h / 2)
}

#[test]
fn a_pointer_press_held_when_focus_is_lost_is_no_click() {
    let mut h = title(1);
    let (x, y) = play_row(&h);
    h.point(x, y);
    h.frame(&[RawInputEvent::PointerDown], FRAME_DT);
    h.system(SystemEvent::FocusLost);
    h.system(SystemEvent::FocusGained);
    h.frame(&[RawInputEvent::PointerUp], FRAME_DT);
    assert_eq!(h.screens(), ["title"]);
    // Its place is forgotten too: a press before it moves again is
    // nowhere, and chooses nothing.
    h.frame(&[RawInputEvent::PointerDown], FRAME_DT);
    h.frame(&[RawInputEvent::PointerUp], FRAME_DT);
    assert_eq!(h.screens(), ["title"]);
    // Once it moves, a click plays.
    h.click(x, y);
    assert_eq!(h.screens(), ["title", "stage"]);
}
