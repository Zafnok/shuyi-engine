//! The test Harness on the headless backend, driving the toy game
//! (`shuyi_ui::toy`) through `App::frame` as a device's backend does: what
//! the display is shown, the system's events, and quitting. The Harness's
//! scripts are tested in `shuyi-ui`'s `tests/it`, over the toy's screens.

use shuyi_platform::SystemEvent;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_platform::headless::Presented;
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::input::{PadKind, state::Device};
use shuyi_ui::screen::{FrameInput, Screen, Transition};
use shuyi_ui::settings::MAX_VOLUME;
use shuyi_ui::storage::Storage;
use shuyi_ui::toy::{self, Action, Home, Toy, ToyCtx};
use shuyi_ui::{Game, GameData, GameHooks};

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
    let home = |h: &Harness<Toy>| h.screen::<Home>().map_or(0, |home| home.frames);
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

#[test]
fn the_harness_reads_the_game_it_drives() {
    let mut h = Harness::new(toy::game());
    assert_eq!(h.top_screen(), "home");
    assert_eq!(h.screen::<Home>().map(|home| home.frames), Some(0));
    h.wait(FRAME_DT);
    assert_eq!(h.screen::<Home>().map(|home| home.frames), Some(1));
    if let Some(home) = h.screen_mut::<Home>() {
        home.frames = 41;
    }
    assert_eq!(h.screen::<Home>().map(|home| home.frames), Some(41));
    // The snapshot is the frame drawn, `H` in its corner.
    let snapshot = h.snapshot();
    assert_eq!(snapshot, h.game().buffer().to_snapshot(&h.ctx().palette));
    assert!(snapshot.contains('H'), "{snapshot}");
    // As the device in use, the same frame; as a pad, the same screen.
    let device = h.ctx().device;
    assert_eq!(h.snapshot_as(device), snapshot);
    assert!(h.snapshot_as(Device::Pad(PadKind::Xbox)).contains('H'));
    assert_eq!(h.ctx().device, device, "put back");
}

/// A screen that asks for the toy's theme until it sounds.
struct Theme;

impl Screen<Toy> for Theme {
    fn name(&self) -> &'static str {
        "theme"
    }

    fn update(&mut self, ctx: &mut ToyCtx, _: &FrameInput<Action>) -> Transition<Toy> {
        if ctx.music_clock.is_none() {
            ctx.audio.play_music("theme");
        }
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

#[test]
fn the_harness_says_what_music_sounds_and_how_loud() {
    let game = Game::new(toy::ctx(), Box::new(Theme), GameHooks::default());
    let mut h = Harness::new(game);
    h.ctx_mut()
        .change_settings(|s| {
            s.music_volume = MAX_VOLUME / 2;
            s.sound_volume = 0;
        })
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(h.music_clock(), None);
    h.wait(0.5);
    let clock = h.music_clock().unwrap_or_else(|| panic!("no music"));
    assert_eq!(clock.cue, "theme");
    assert!((clock.position - 0.5 + FRAME_DT).abs() < 0.01, "{clock:?}");
    let music = h.ctx().settings().music_factor();
    assert!(music > 0.0 && music < 1.0, "{music}");
    assert_eq!(h.volumes(), (music, 0.0));
}
