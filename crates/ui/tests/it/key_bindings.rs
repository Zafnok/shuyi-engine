//! The player's key bindings through the Harness (ticket 1111, game ADR-0031):
//! a rebound key works at the next press, plain Escape always cancels,
//! and each layout's keys are saved and come back at the next launch.

use shuyi_platform::harness::Harness;
use shuyi_ui::input::{Chord, Layout, LayoutBindings};
use shuyi_ui::toy::{self, Action, LEFT_HANDED, RIGHT_HANDED, Toy};

use crate::toy as home;

fn chord(s: &str) -> Chord {
    Chord::parse(s).unwrap_or_else(|e| panic!("{e}"))
}

/// Rebinds `layout` with `edit` applied, as a key bindings screen would.
fn rebind(h: &mut Harness<Toy>, layout: Layout, edit: impl FnOnce(&mut LayoutBindings<Action>)) {
    let mut b = h.ctx().layout_bindings(layout);
    edit(&mut b);
    if let Err(e) = h.ctx_mut().set_layout_bindings(layout, b) {
        panic!("saving key bindings: {e}");
    }
}

/// Confirm's first slot on `g`.
fn confirm_on_g(b: &mut LayoutBindings<Action>) {
    assert_eq!(b.bind(Action::Confirm, 0, chord("g")), Ok(None));
}

/// The next launch, from `h`'s storage.
fn relaunch(h: Harness<Toy>) -> Harness<Toy> {
    Harness::new(toy::game_on(toy::launch(h.into_storage())))
}

#[test]
fn a_rebound_key_works_at_once_and_the_old_one_doesnt() {
    let mut h = home();
    rebind(&mut h, RIGHT_HANDED, confirm_on_g);
    h.keys("f");
    assert_eq!(h.screens(), ["home"]);
    h.keys("g");
    assert_eq!(h.screens(), ["home", "menu"]);
}

#[test]
fn escape_cancels_in_both_layouts() {
    for layout in [RIGHT_HANDED, LEFT_HANDED] {
        let mut ctx = toy::ctx();
        ctx.use_layout(layout);
        let mut h = Harness::new(toy::game_on(ctx));
        let confirm = h.ctx().keymap.chords_for(Action::Confirm)[0];
        h.keys(&confirm.to_string());
        assert_eq!(h.screens(), ["home", "menu"], "{layout}");
        h.keys("Escape");
        assert_eq!(h.screens(), ["home"], "{layout}");
    }
}

#[test]
fn escape_cancels_even_with_cancel_unbound() {
    let mut h = home();
    rebind(&mut h, RIGHT_HANDED, |b| b.clear(Action::Cancel, 0));
    h.keys("f d");
    assert_eq!(h.screens(), ["home", "menu"]);
    h.keys("Escape");
    assert_eq!(h.screens(), ["home"]);
}

#[test]
fn shift_escape_can_be_bound_and_plain_escape_still_cancels() {
    let mut h = home();
    rebind(&mut h, RIGHT_HANDED, |b| {
        assert_eq!(b.bind(Action::Confirm, 1, chord("Shift+Escape")), Ok(None));
    });
    h.keys("Shift+Escape");
    assert_eq!(h.screens(), ["home", "menu"]);
    h.keys("Escape");
    assert_eq!(h.screens(), ["home"]);
}

/// The toy game with the right-handed layout chosen and saved.
fn saved() -> Harness<Toy> {
    let mut h = home();
    h.ctx_mut()
        .choose_layout(RIGHT_HANDED)
        .unwrap_or_else(|e| panic!("{e}"));
    h
}

#[test]
fn custom_keys_survive_a_restart() {
    let mut h = saved();
    rebind(&mut h, RIGHT_HANDED, confirm_on_g);
    let mut h = relaunch(h);
    assert_eq!(h.ctx().layout(), Some(RIGHT_HANDED));
    h.keys("f");
    assert_eq!(h.screens(), ["home"]);
    h.keys("g");
    assert_eq!(h.screens(), ["home", "menu"]);
}

#[test]
fn each_layout_keeps_its_own_keys() {
    let mut h = home();
    rebind(&mut h, RIGHT_HANDED, confirm_on_g);
    // Left-handed: its own default keys.
    h.ctx_mut()
        .choose_layout(LEFT_HANDED)
        .unwrap_or_else(|e| panic!("{e}"));
    h.keys("g");
    assert_eq!(h.screens(), ["home"]);
    h.keys("j");
    assert_eq!(h.screens(), ["home", "menu"]);
    h.keys("k");
    // And back: the right-handed edit is still there, across a restart too.
    h.ctx_mut()
        .choose_layout(RIGHT_HANDED)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut h = relaunch(h);
    h.keys("f");
    assert_eq!(h.screens(), ["home"]);
    h.keys("g");
    assert_eq!(h.screens(), ["home", "menu"]);
}
