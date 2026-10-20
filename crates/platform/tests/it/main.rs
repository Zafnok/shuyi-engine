//! The test Harness on the headless backend, driving the toy game
//! (`shuyi_ui::toy`) through `App::frame` as a device's backend does: what
//! the display is shown, the system's events, and quitting. The Harness's
//! scripts are tested in `shuyi-ui`'s `tests/it`, over the toy's screens.

use shuyi_platform::SystemEvent;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_platform::headless::Presented;
use shuyi_ui::GameData;
use shuyi_ui::storage::Storage;
use shuyi_ui::toy;

#[test]
fn each_frame_shows_the_display_what_the_screens_drew() {
    let mut h = Harness::new(toy::game());
    assert_eq!(h.platform().presented(), 0);
    h.keys("f");
    assert_eq!(h.screens(), ["home", "menu"]);
    assert_eq!(h.platform().presented(), h.frame_count());
    let shown = h.platform().last_frame().and_then(Presented::buffer);
    assert_eq!(shown, Some(h.game().buffer()));
    // A test's App has no look to paint with, and the background is the
    // game's blank cell's.
    let ctx = h.ctx();
    let blank = ctx.game.blank_cell(&ctx.palette).bg;
    assert!(matches!(
        h.platform().last_frame(),
        Some(Presented::Game { background, fullscreen: false, .. }) if *background == blank
    ));
}

#[test]
fn the_window_shortcut_asks_for_full_screen_from_the_next_frame() {
    let mut h = Harness::new(toy::game());
    h.wait(FRAME_DT);
    assert!(!h.fullscreen());
    h.system(SystemEvent::ToggleFullscreen);
    assert!(h.fullscreen());
    assert!(h.ctx().settings().fullscreen);
    h.system(SystemEvent::ToggleFullscreen);
    assert!(!h.fullscreen());
}

#[test]
fn the_games_hook_hears_the_system_and_a_suspend_stops_the_screens() {
    let mut h = Harness::new(toy::game()).with_system_hook(|game, event| {
        if event == SystemEvent::Suspend {
            let _ = game.ctx_mut().storage.write("suspended", "saved");
        }
    });
    let home = |h: &Harness<toy::Toy>| h.screen::<toy::Home>().map_or(0, |home| home.frames);
    h.wait(0.5);
    let frames = home(&h);
    h.system(SystemEvent::Suspend);
    assert_eq!(h.ctx().storage.read("suspended"), Ok(Some("saved".into())));
    h.keys("f").wait(1.0);
    assert_eq!(home(&h), frames);
    assert_eq!(h.screens(), ["home"]);
    h.system(SystemEvent::Resume).keys("f");
    assert_eq!(h.screens(), ["home", "menu"]);
}

#[test]
fn quitting_stops_the_audio_and_the_frames_say_so() {
    let mut h = Harness::new(toy::game());
    h.keys("Escape");
    assert!(h.quit_requested());
    assert_eq!(h.quit_frame(), Some(1));
    assert!(h.platform().speaker().stopped());
    assert!(h.app().has_quit());
    // The player closing the window quits too.
    let mut h = Harness::new(toy::game());
    h.wait(FRAME_DT).system(SystemEvent::QuitRequested);
    assert_eq!(h.quit_frame(), Some(2));
    assert!(!h.quit_requested(), "the game itself didn't ask");
}
