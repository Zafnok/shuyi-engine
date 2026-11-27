//! The window losing focus: the game pauses, and a run waits in its pause
//! menu, in the tools' Harness and from the game's real start.

use shuyi::prelude::test::Harness;
use shuyi::prelude::{Background, SystemEvent, UiAction};

use crate::{at_title, stage};

#[test]
fn a_run_that_loses_focus_opens_its_pause_menu_and_doesnt_move() {
    let mut h = at_title(1);
    h.act(UiAction::Confirm);
    h.wait(0.2);
    // Another of the system's events opens no menu.
    h.system(SystemEvent::LowBattery);
    assert_eq!(h.screens(), ["title", "stage"]);
    let steps = stage(&h).snake.steps();
    let body = stage(&h).snake.body().to_vec();
    h.system(SystemEvent::FocusLost);
    assert_eq!(h.screens(), ["title", "stage", "pause"]);
    assert!(h.app().is_paused());
    h.wait(1.0);
    assert_eq!(stage(&h).snake.steps(), steps);
    assert_eq!(stage(&h).snake.body(), body);
    // Back, the run still waits in the menu until Resume.
    h.system(SystemEvent::FocusGained);
    h.wait(0.5);
    assert_eq!(h.screens(), ["title", "stage", "pause"]);
    assert_eq!(stage(&h).snake.steps(), steps);
    h.act(UiAction::Confirm);
    h.wait(0.2);
    assert!(stage(&h).snake.steps() > steps);
}

#[test]
fn the_title_that_loses_focus_opens_no_menu() {
    let mut h = at_title(1);
    h.system(SystemEvent::FocusLost);
    assert_eq!(h.screens(), ["title"]);
    assert!(h.app().is_paused());
}

#[test]
fn its_real_start_pauses_without_focus_and_opens_the_menu() {
    let mut h = Harness::start(snake::game().start()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(h.app().background(), Background::KEEP_RUNNING.paused());
    h.act(UiAction::Confirm);
    assert_eq!(h.screens(), ["title", "stage"]);
    h.system(SystemEvent::FocusLost);
    assert_eq!(h.screens(), ["title", "stage", "pause"]);
    assert!(h.app().is_paused());
}
