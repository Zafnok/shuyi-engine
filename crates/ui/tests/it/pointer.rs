//! The pointer on the toy's lobby, an engine menu, through the Harness
//! (ticket 2405): a click gives what the keys give, hover follows motion
//! only, a press and a release on two rows choose nothing, a tap chooses,
//! and the pointer never names the device or opens the first-key screen.

use shuyi_platform::harness::Harness;
use shuyi_ui::input::{Device, PadKind};
use shuyi_ui::toy::{self, Lobby, Toy};
use shuyi_ui::{KeyPrompt, RawInputEvent};

/// The toy game at its lobby, the right-handed layout chosen.
fn lobby() -> Harness<Toy> {
    Harness::new(toy::lobby())
}

/// The centre pixel of the lobby's row `item`, from the frame's hit areas.
fn row(h: &Harness<Toy>, item: u32) -> (i32, i32) {
    let hit = h
        .game()
        .buffer()
        .hits()
        .iter()
        .find(|hit| hit.item == item)
        .unwrap_or_else(|| panic!("no row {item}"));
    (hit.rect.x + hit.rect.w / 2, hit.rect.y + hit.rect.h / 2)
}

fn focus(h: &Harness<Toy>) -> usize {
    h.screen::<Lobby>().map_or(usize::MAX, |l| l.menu.focus())
}

#[test]
fn a_click_on_a_row_gives_what_its_keys_give() {
    let mut keys = lobby();
    keys.keys("Down f");
    let mut clicked = lobby();
    clicked.click_text("Key bindings");
    assert_eq!(clicked.screens(), ["lobby", "key_bindings"]);
    assert_eq!(clicked.screens(), keys.screens());
    assert_eq!(clicked.sounds(), ["ui_move", "ui_select"]);
    assert_eq!(clicked.sounds(), keys.sounds());
}

#[test]
fn hover_moves_the_focus_only_on_a_frame_the_pointer_moved() {
    let mut h = lobby();
    let (x, y) = row(&h, 2);
    h.point(x, y);
    assert_eq!(focus(&h), 2);
    // The pointer stays over row 2: the keys move the focus on, and it
    // stays there.
    h.keys("Down");
    h.wait(0.5);
    h.point(x, y);
    assert_eq!(focus(&h), 3);
    // It moves within the row: the focus comes back.
    h.point(x + 1, y);
    assert_eq!(focus(&h), 2);
    assert_eq!(h.screens(), ["lobby"]);
}

#[test]
fn a_press_on_one_row_and_a_release_on_another_choose_nothing() {
    let mut h = lobby();
    let (x, y) = row(&h, 0);
    let (_, other) = row(&h, 2);
    h.point(x, y);
    h.frame(&[RawInputEvent::PointerDown], 0.016);
    h.point(x, other);
    h.frame(&[RawInputEvent::PointerUp], 0.016);
    assert_eq!(h.screens(), ["lobby"]);
    assert_eq!(focus(&h), 2, "hover still moved it");
    assert!(!h.sounds().contains(&"ui_select".to_owned()));
}

/// A tap: the finger comes down, then lifts, which leaves the frame in
/// the same poll as the release.
#[test]
fn a_tap_chooses_the_row_it_was_on() {
    let mut h = lobby();
    let (x, y) = row(&h, 3);
    h.frame(
        &[
            RawInputEvent::PointerMoved { x, y },
            RawInputEvent::PointerDown,
        ],
        0.016,
    );
    h.frame(
        &[RawInputEvent::PointerUp, RawInputEvent::PointerLeft],
        0.016,
    );
    assert!(h.quit_requested(), "Quit was chosen");
}

#[test]
fn the_wheel_and_a_pointer_that_left_choose_nothing() {
    let mut h = lobby();
    let (x, y) = row(&h, 3);
    h.point(x, y).leave_pointer();
    h.wheel(2);
    h.frame(&[RawInputEvent::PointerDown], 0.016);
    h.frame(&[RawInputEvent::PointerUp], 0.016);
    assert!(!h.quit_requested());
    assert_eq!(focus(&h), 3);
}

#[test]
fn a_pointer_press_ends_the_wait_for_a_key_and_names_no_device() {
    let mut h = lobby();
    h.use_pad(PadKind::Xbox).pad("DpadDown");
    assert_eq!(h.ctx().device, Device::Pad(PadKind::Xbox));
    h.ctx_mut().key_prompt = KeyPrompt::Waiting;
    let (x, y) = row(&h, 0);
    h.click(x, y);
    assert_eq!(h.ctx().key_prompt, KeyPrompt::Pressed);
    assert_eq!(
        h.ctx().device,
        Device::Pad(PadKind::Xbox),
        "help still names the pad"
    );
}

#[test]
#[should_panic(expected = "isn't shown")]
fn clicking_text_not_shown_panics() {
    lobby().click_text("Play");
}

/// A lost focus forgets the pointer's place, in the game and in the
/// Harness: a click where it was points there again first.
#[test]
fn a_click_after_a_lost_focus_points_again() {
    let mut h = lobby();
    let (x, y) = row(&h, 3);
    h.point(x, y);
    h.system(shuyi_platform::SystemEvent::FocusLost);
    h.click(x, y);
    assert!(h.quit_requested(), "Quit was chosen");
}
