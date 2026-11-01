//! The test Harness on the headless backend, driving the toy game
//! (`shuyi_ui::toy`) through `App::frame` as a device's backend does: what
//! the display is shown, the system's events, and quitting; and a backend
//! of only the methods every backend must have (`bare`). The Harness's
//! scripts are tested in `shuyi-ui`'s `tests/it`, over the toy's screens.

mod bare;
#[cfg(feature = "std")]
mod reload;
mod sounds;

use shuyi_platform::SystemEvent;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_platform::headless::{HeadlessPlatform, Presented};
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::input::{Chord, Key};
use shuyi_ui::input::{PadKind, state::Device};
use shuyi_ui::log::{LogCategory, LogLevel};
use shuyi_ui::perf::HeapSnapshot;
use shuyi_ui::profiles::ProfileId;
use shuyi_ui::screen::{FrameInput, Screen, ScreenStack, Transition};
use shuyi_ui::settings::{MAX_VOLUME, SETTINGS_KEY};
use shuyi_ui::storage::{MemoryStorage, Storage};
use shuyi_ui::toy::{self, Action, Home, Toy, ToyCtx};
use shuyi_ui::{Game, GameData, GameHooks, RawInputEvent};

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

/// A screen that declares Confirm alone, but pops on Cancel too, and
/// with `peek` asks whether Cancel is held: a declaration that lies.
struct Forgetful {
    peek: bool,
}

impl Screen<Toy> for Forgetful {
    fn name(&self) -> &'static str {
        "forgetful"
    }

    fn actions(&self) -> Option<Vec<Action>> {
        Some(vec![Action::Confirm])
    }

    fn update(&mut self, _: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        if self.peek && input.is_held(Action::Cancel) {
            return Transition::Pop;
        }
        if input.actions.contains(&Action::Cancel) {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

/// The toy's home with a [`Forgetful`] screen over it, in the Harness.
fn forgetful(peek: bool) -> Harness<Toy> {
    let mut stack = ScreenStack::new(Box::new(Home::default()));
    stack.push(Box::new(Forgetful { peek }));
    Harness::new(Game::with_stack(toy::ctx(), stack, GameHooks::default()))
}

#[test]
fn a_screen_never_sees_an_action_it_does_not_declare_in_the_harness() {
    let mut h = forgetful(false);
    h.keys("Escape");
    // Cancel was held back, so the screen didn't pop: a test that
    // expected it to fails, and the log says why.
    assert_eq!(h.screens(), ["home", "forgetful"]);
    assert_eq!(h.undeclared(), [("forgetful", Action::Cancel)]);
    // Without the Harness's guard, the same screen pops.
    let mut stack = ScreenStack::new(Box::new(Home::default()));
    stack.push(Box::new(Forgetful { peek: false }));
    let mut game = Game::with_stack(toy::ctx(), stack, GameHooks::default());
    let _ = game.frame(&[RawInputEvent::Down(Chord::plain(Key::Escape))], FRAME_DT);
    assert_eq!(game.screens(), ["home"]);
    assert_eq!(game.undeclared_actions(), []);
}

#[test]
#[should_panic(
    expected = "screen \"forgetful\" asks whether Cancel is held, but doesn't declare it"
)]
fn a_screen_asking_about_an_action_it_does_not_declare_fails_its_harness_test() {
    forgetful(true).keys("Escape");
}

#[test]
fn the_harness_log_starts_with_what_the_game_logged_before_it() {
    let mut storage = MemoryStorage::new();
    let key = ProfileId::FIRST.key(SETTINGS_KEY);
    storage.write(&key, "nonsense").unwrap();
    let h = Harness::new(toy::game_on(toy::launch(Box::new(storage))));
    let lines: Vec<_> = h.log().lines().collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = lines[0];
    assert_eq!(
        (line.frame, line.level, line.category),
        (0, LogLevel::Warn, LogCategory::Storage)
    );
    assert!(line.text.starts_with("settings: unreadable: "), "{line:?}");
}

#[test]
fn a_line_logged_between_frames_carries_the_next() {
    let mut h = Harness::new(toy::game());
    for _ in 0..3 {
        h.frame(&[], FRAME_DT);
    }
    assert_eq!(h.frame_count(), 3);
    h.ctx_mut().log(LogLevel::Info, LogCategory::Game, "hello");
    h.frame(&[], FRAME_DT);
    let said: Vec<_> = h
        .log()
        .lines()
        .map(|l| (l.frame, l.text.as_str()))
        .collect();
    assert_eq!(said, [(4, "hello")]);
}

#[test]
fn the_harness_takes_its_platforms_heap_snapshots_at_its_frames() {
    let mut h = Harness::new(toy::game());
    assert_eq!(h.heap_snapshot(), None, "no fake memory, no counter");
    h.set_heap_snapshots(Some(|| HeapSnapshot::new(7, 9, Vec::new())));
    h.frame(&[], FRAME_DT);
    let set = h.heap_snapshot().expect("the counter's snapshot");
    assert_eq!((set.frame, set.live_bytes, set.peak_bytes), (1, 7, 9));
    let platform = HeadlessPlatform::new().with_fake_memory();
    let mut fake = Harness::on(toy::game(), platform);
    let first = fake.heap_snapshot().expect("the fake memory's snapshot");
    for _ in 0..2 {
        fake.frame(&[], FRAME_DT);
    }
    let later = fake.heap_snapshot().expect("and another");
    assert_eq!((first.frame, later.frame), (0, 2));
    assert_eq!(first.diff(&later).live_bytes, 32_768);
    h.set_heap_snapshots(None);
    assert_eq!(h.heap_snapshot(), None);
}
