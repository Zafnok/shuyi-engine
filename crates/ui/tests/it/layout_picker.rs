//! The first-launch layout picker through the Harness (ticket 1111): while
//! no layout is chosen the first key opens the game's first-key screen
//! (the toy's `Picker`) and does nothing else; the layout picked is used
//! at once and found again at the next launch.

use shuyi_ui::Keyboard;
use shuyi_ui::harness::Harness;
use shuyi_ui::toy::{self, LEFT_HANDED, Picker, RIGHT_HANDED, Toy};

use crate::first_launch;

/// A first launch with the picker open.
fn picker() -> Harness<Toy> {
    let mut h = first_launch();
    h.keys("z");
    h
}

#[test]
fn the_first_key_opens_the_picker_and_does_nothing_else() {
    let mut h = first_launch();
    assert_eq!(h.ctx().layout(), None);
    // `f` would open the menu with a layout; here it only opens the picker.
    h.keys("f");
    assert_eq!(h.screens(), ["home", "picker"]);
    assert_eq!(h.screen::<Picker>().map(|p| p.focus), Some(0));
    assert!(h.snapshot().starts_with("H P"), "{}", h.snapshot());
    assert!(h.snapshot().contains("Right hand"), "{}", h.snapshot());
}

#[test]
fn a_controller_button_doesnt_open_it_and_acts() {
    let mut h = first_launch();
    h.pad("South");
    assert_eq!(h.screens(), ["home", "menu"]);
}

/// The keys then are the layout picker's (`f` confirms there too).
#[test]
fn without_a_keyboard_it_never_opens() {
    let mut ctx = toy::first_launch();
    ctx.keyboard = Keyboard::Absent;
    let mut h = Harness::new(toy::game_on(ctx));
    h.keys("z");
    assert_eq!(h.screens(), ["home"]);
    h.keys("f");
    assert_eq!(h.screens(), ["home", "menu"]);
}

#[test]
fn it_works_with_either_hand() {
    // Up/Down or w/s choose; f, j, Enter or Space pick.
    for (script, layout) in [
        ("f", RIGHT_HANDED),
        ("Down Enter", LEFT_HANDED),
        ("s j", LEFT_HANDED),
        ("s w Space", RIGHT_HANDED),
        ("Up Up j", RIGHT_HANDED),
    ] {
        let mut h = picker();
        h.keys(script);
        assert_eq!(h.screens(), ["home"], "{script}");
        assert_eq!(h.ctx().layout(), Some(layout), "{script}");
    }
}

#[test]
fn it_cannot_be_cancelled() {
    let mut h = picker();
    h.keys("d k Escape Backspace");
    assert_eq!(h.top_screen(), "picker");
    assert!(!h.quit_requested());
}

#[test]
fn the_layout_picked_is_used_at_once_and_at_the_next_launch() {
    let mut h = picker();
    h.keys("s j");
    // Left-handed: `j` confirms, `f` does nothing.
    h.keys("f");
    assert_eq!(h.screens(), ["home"]);
    h.keys("j");
    assert_eq!(h.screens(), ["home", "menu"]);
    let mut h = Harness::new(toy::game_on(toy::launch(h.into_storage())));
    assert_eq!(h.ctx().layout(), Some(LEFT_HANDED));
    h.keys("z");
    assert_eq!(h.screens(), ["home"], "a later launch has no picker");
}
