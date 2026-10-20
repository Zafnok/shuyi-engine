//! `App::frame` on the headless backend: a frame's order (input, actions,
//! the update and its steps, the audio, the frame presented), the loading
//! card, suspend and resume, quitting, and the system keyboard.

use alloc::rc::Rc;
use core::cell::RefCell;

use shuyi_ui::frame::glyph_buffer::{Cell, GlyphBuffer};
use shuyi_ui::input::{Chord, Key};
use shuyi_ui::screen::{FrameInput, Screen, Transition};
use shuyi_ui::storage::MemoryStorage;
use shuyi_ui::system_keyboard::TextAsk;
use shuyi_ui::toy::{self, Action, Toy, ToyCtx};
use shuyi_ui::{GameHooks, IdleMotion};

use super::*;
use crate::headless::{HeadlessPlatform, Presented, ScriptedKeyboard};

/// What a [`Steps`] screen saw in one update.
#[derive(Debug, Clone, PartialEq)]
struct Seen {
    actions: Vec<Action>,
    dt: f32,
    ticks: u32,
    answer: Option<TextAnswer>,
}

type Log = Rc<RefCell<Vec<Seen>>>;

/// A screen that records every update, draws the number of updates in its
/// first cell, plays a click on Confirm and asks for the system keyboard
/// on `CursorUp`.
struct Steps(Log);

impl Screen<Toy> for Steps {
    fn name(&self) -> &'static str {
        "steps"
    }

    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        if input.actions.contains(&Action::Confirm) {
            ctx.audio.play_sound("click");
        }
        if input.actions.contains(&Action::CursorUp) {
            ctx.system_keyboard.ask(TextAsk {
                prompt: "Name".into(),
                text: String::new(),
                max_chars: 8,
            });
        }
        if input.actions.contains(&Action::Cancel) {
            return Transition::Quit;
        }
        self.0.borrow_mut().push(Seen {
            actions: input.actions.clone(),
            dt: input.dt(),
            ticks: input.ticks(),
            answer: ctx.system_keyboard.take_answer(),
        });
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, buf: &mut GlyphBuffer) {
        let count = self.0.borrow().len() % 10;
        let glyph = char::from_digit(u32::try_from(count).unwrap_or(0), 10).unwrap_or('?');
        let white = Rgb::new(255, 255, 255);
        buf.set(0, 0, Cell::new(glyph, white, Rgb::new(0, 0, 0)));
    }
}

/// The toy's context showing only a [`Steps`], and its log.
fn steps_game() -> (Game<Toy>, Log) {
    let log = Log::default();
    let game = Game::new(
        toy::ctx(),
        Box::new(Steps(Rc::clone(&log))),
        GameHooks::default(),
    );
    (game, log)
}

/// The game `app` runs.
fn running(app: &App<Toy>) -> &Game<Toy> {
    app.game().unwrap_or_else(|| panic!("not running"))
}

fn press(key: Key) -> [RawInputEvent; 1] {
    [RawInputEvent::Down(Chord::plain(key))]
}

/// The first cell of what the platform was last shown.
fn shown_glyph(p: &HeadlessPlatform) -> Option<char> {
    let buffer = p.last_frame()?.buffer()?;
    buffer.get(0, 0).map(|cell| cell.glyph)
}

#[test]
fn a_frame_takes_input_then_acts_steps_plays_and_presents() {
    let (game, log) = steps_game();
    let mut app = App::running(game);
    let mut p = HeadlessPlatform::new();
    p.push_input(&press(Key::F));
    assert_eq!(app.frame(&mut p, Millis(1000)), FrameOutcome::Continue);
    // The first frame counts no time: no steps.
    let first = log.borrow()[0].clone();
    assert_eq!(first.actions, [Action::Confirm]);
    assert_eq!((first.dt, first.ticks), (0.0, 0));
    // Its click went to the device, and the frame drawn after the update
    // to the display.
    let click = SoundRequest::Sound {
        cue: "click".into(),
        volume: 1.0,
    };
    assert_eq!(p.speaker().requests(), [click]);
    assert_eq!(app.audio().len(), 1);
    assert_eq!(p.presented(), 1);
    assert_eq!(shown_glyph(&p), Some('1'));
    // A sixtieth of a second later: one step, the time to the millisecond.
    assert_eq!(app.frame(&mut p, Millis(1017)), FrameOutcome::Continue);
    let second = log.borrow()[1].clone();
    assert!(second.actions.is_empty());
    assert!((second.dt - 0.017).abs() < 1e-6, "{}", second.dt);
    assert_eq!(second.ticks, 1);
    assert!(p.speaker().requests().is_empty());
    assert!(app.audio().is_empty());
    assert_eq!((p.presented(), shown_glyph(&p)), (2, Some('2')));
}

#[test]
fn a_second_of_frames_steps_sixty_times_and_a_late_one_four() {
    let (game, log) = steps_game();
    let mut app = App::running(game);
    let mut p = HeadlessPlatform::new();
    let mut now = 0;
    let _ = app.frame(&mut p, Millis(now));
    for ms in [16, 17, 17].iter().cycle().take(60) {
        now += ms;
        let _ = app.frame(&mut p, Millis(now));
    }
    let ticks: u32 = log.borrow().iter().map(|s| s.ticks).sum();
    assert_eq!(ticks, 60);
    // A frame half a second late runs at most four steps.
    let _ = app.frame(&mut p, Millis(now + 500));
    assert_eq!(log.borrow().last().map(|s| s.ticks), Some(4));
}

#[test]
fn suspended_ten_minutes_the_game_steps_at_most_once_after_resuming() {
    let (game, log) = steps_game();
    let mut app = App::running(game);
    app.set_system_hook(|game, event| {
        if event == SystemEvent::Suspend {
            // The game saves through its hook (here: into its storage).
            let _ = game.ctx_mut().storage.write("save", "where it was");
        }
    });
    let mut p = HeadlessPlatform::new();
    let mut now = 0;
    for _ in 0..10 {
        now += 17;
        let _ = app.frame(&mut p, Millis(now));
    }
    let updates = log.borrow().len();
    let presented = p.presented();
    p.push_system(SystemEvent::Suspend);
    // The device may go on calling for frames while it sleeps: nothing
    // steps, nothing is shown, and input is dropped.
    for _ in 0..3 {
        now += 17;
        p.push_input(&press(Key::F));
        assert_eq!(app.frame(&mut p, Millis(now)), FrameOutcome::Continue);
    }
    assert!(app.is_suspended());
    assert_eq!(log.borrow().len(), updates);
    assert_eq!(p.presented(), presented);
    assert!(p.speaker().requests().is_empty());
    let game = running(&app);
    assert_eq!(
        game.ctx().storage.read("save"),
        Ok(Some("where it was".into()))
    );
    // Ten minutes later it resumes: 36 000 steps' worth of time is
    // forgotten.
    now += 10 * 60 * 1000;
    p.push_system(SystemEvent::Resume);
    let _ = app.frame(&mut p, Millis(now));
    now += 17;
    let _ = app.frame(&mut p, Millis(now));
    assert!(!app.is_suspended());
    let after: Vec<u32> = log.borrow()[updates..].iter().map(|s| s.ticks).collect();
    assert_eq!(after.len(), 2);
    assert!(after.iter().sum::<u32>() <= 1, "{after:?}");
    // The first frame after the resume counts no time for the screens either.
    assert!(log.borrow()[updates].dt.abs() < f32::EPSILON);
}

#[test]
fn the_card_shows_first_then_the_load_runs_behind_it_then_the_game() {
    let (game, log) = steps_game();
    let game = RefCell::new(Some(game));
    let card = Card::text("A toy").colored(Rgb::new(0, 0, 0), Rgb::new(255, 255, 255));
    let start = Start::new(
        card.clone(),
        |toy: &Toy| toy.look(),
        move |setup| {
            assert_eq!(setup.display, HeadlessPlatform::DISPLAY);
            let game = game
                .borrow_mut()
                .take()
                .unwrap_or_else(|| panic!("loaded twice"));
            Ok(game)
        },
    );
    let mut app = App::new(start);
    let mut p = HeadlessPlatform::new();
    assert!(app.is_loading());
    // Frame 1: the card, and nothing else.
    let _ = app.frame(&mut p, Millis(0));
    assert_eq!(p.last_frame(), Some(&Presented::Card(card)));
    assert!(app.is_loading() && app.game().is_none());
    // Frame 2: the load, seconds long; then the game's first frame is shown,
    // drawn before any update.
    p.push_input(&press(Key::F));
    let _ = app.frame(&mut p, Millis(16));
    assert!(!app.is_loading());
    assert_eq!(shown_glyph(&p), Some('0'));
    assert!(
        log.borrow().is_empty(),
        "the input of the load frame is dropped"
    );
    // Frame 3, after a load that took 3.7 s: the game runs from now, not
    // from before the load.
    let _ = app.frame(&mut p, Millis(3716));
    let seen = log.borrow()[0].clone();
    assert!(seen.actions.is_empty());
    assert_eq!((seen.dt, seen.ticks), (0.0, 0));
    assert_eq!(shown_glyph(&p), Some('1'));
    assert_eq!(p.presented(), 3);
}

/// The toy's look, for a test start.
trait LookOfToy {
    fn look(&self) -> Look<'_>;
}

impl LookOfToy for Toy {
    fn look(&self) -> Look<'_> {
        Look {
            font: &FONT,
            images: &IMAGES,
        }
    }
}

static FONT: std::sync::LazyLock<shuyi_assets::font::FontAtlasDef> =
    std::sync::LazyLock::new(Default::default);
static IMAGES: std::sync::LazyLock<shuyi_assets::image::ImageTable> =
    std::sync::LazyLock::new(Default::default);

#[test]
fn the_game_is_shown_with_its_look_and_its_background() {
    let start = Start::new(
        Card::text("A toy"),
        |toy: &Toy| toy.look(),
        |setup| Ok(toy::game_on(setup.ctx(toy::ctx()))),
    );
    let mut app = App::new(start);
    let mut p = RecordLook::default();
    let _ = app.frame(&mut p.headless, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    assert_eq!(p.looked, Some(true));
    let ctx = running(&app).ctx();
    let background = ctx.game.blank_cell(&ctx.palette).bg;
    assert!(matches!(
        p.headless.last_frame(),
        Some(Presented::Game { background: shown, .. }) if *shown == background
    ));
    // A running App a test made has no look.
    let mut app = App::running(toy::game());
    let _ = app.frame(&mut p, Millis(0));
    assert_eq!(p.looked, Some(false));
}

/// A headless backend that notes whether the game's frames came with
/// their look.
#[derive(Default)]
struct RecordLook {
    headless: HeadlessPlatform,
    looked: Option<bool>,
}

impl Platform for RecordLook {
    fn display(&self) -> DisplayInfo {
        self.headless.display()
    }

    fn present(&mut self, frame: &Frame<'_>) {
        if let Show::Game { look, .. } = frame.show {
            self.looked = Some(look.is_some());
        }
        self.headless.present(frame);
    }

    fn poll_input(&mut self, out: &mut Vec<RawInputEvent>) {
        self.headless.poll_input(out);
    }

    fn system_events(&mut self, out: &mut Vec<SystemEvent>) {
        self.headless.system_events(out);
    }

    fn storage(&mut self) -> Box<dyn Storage> {
        self.headless.storage()
    }

    fn audio(&mut self) -> &mut dyn crate::audio::Audio {
        self.headless.audio()
    }

    fn text_input(&mut self) -> Option<&mut dyn TextInput> {
        self.headless.text_input()
    }
}

#[test]
fn a_load_that_fails_shows_why_until_the_player_quits() {
    let start: Start<Toy> = Start::new(
        Card::text("A toy"),
        |toy: &Toy| toy.look(),
        |_| Err("files/font.ron:3: no glyphs".to_owned()),
    );
    let mut app = App::new(start);
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    for ms in [16, 33, 50] {
        assert_eq!(app.frame(&mut p, Millis(ms)), FrameOutcome::Continue);
        let errors = Presented::Errors("files/font.ron:3: no glyphs".into());
        assert_eq!(p.last_frame(), Some(&errors));
    }
    assert_eq!(app.errors(), Some("files/font.ron:3: no glyphs"));
    assert!(app.game().is_none() && !app.is_loading());
    p.push_system(SystemEvent::QuitRequested);
    assert_eq!(app.frame(&mut p, Millis(66)), FrameOutcome::Quit);
}

#[test]
fn closing_while_the_card_shows_quits_without_loading() {
    let start: Start<Toy> = Start::new(
        Card::text("A toy"),
        |toy: &Toy| toy.look(),
        |_| panic!("loaded after a quit"),
    );
    let mut app = App::new(start);
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    p.push_system(SystemEvent::QuitRequested);
    assert_eq!(app.frame(&mut p, Millis(16)), FrameOutcome::Quit);
    assert_eq!(app.frame(&mut p, Millis(33)), FrameOutcome::Quit);
    assert!(matches!(p.last_frame(), Some(Presented::Card(_))));
}

#[test]
fn the_setup_puts_the_platform_into_the_games_context() {
    let mut saved = MemoryStorage::new();
    let _ = saved.write("from-before", "yes");
    let start = Start::new(
        Card::text("A toy"),
        |toy: &Toy| toy.look(),
        |setup| {
            assert_ne!(setup.seed, 0);
            Ok(toy::game_on(setup.ctx(toy::first_launch())))
        },
    )
    .before_load(|setup| {
        setup.key_prompt = KeyPrompt::Off;
        setup.keyboard = Keyboard::Absent;
    });
    let mut app = App::new(start);
    let mut p = HeadlessPlatform::new().with_storage(Box::new(saved));
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(1_700_000_000_000));
    let ctx = running(&app).ctx();
    assert_eq!(ctx.idle_motion, IdleMotion::On);
    assert_eq!(ctx.key_prompt, KeyPrompt::Off);
    assert_eq!(ctx.keyboard, Keyboard::Absent);
    assert_ne!(ctx.music_seed, shuyi_ui::ctx::DEFAULT_MUSIC_SEED);
    assert_eq!(
        ctx.storage.computer().read("from-before"),
        Ok(Some("yes".into()))
    );
    assert!(app.take_warnings().is_empty());
}

#[test]
fn by_default_the_setup_waits_for_a_key_on_a_keyboard() {
    let start = Start::new(
        Card::text("A toy"),
        |toy: &Toy| toy.look(),
        |setup| {
            assert_eq!(setup.voice_manifest, None);
            assert_eq!(
                (setup.key_prompt, setup.keyboard),
                (KeyPrompt::Waiting, Keyboard::Present)
            );
            Ok(toy::game_on(setup.ctx(toy::ctx())))
        },
    );
    let mut app = App::new(start);
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(1));
    let ctx = running(&app).ctx();
    assert_eq!(ctx.key_prompt, KeyPrompt::Waiting);
}

#[test]
fn a_game_that_quits_stops_its_audio_and_keeps_its_last_frame() {
    let (game, log) = steps_game();
    let mut app = App::running(game);
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    p.push_input(&press(Key::Escape));
    assert_eq!(app.frame(&mut p, Millis(17)), FrameOutcome::Quit);
    assert!(app.has_quit() && p.speaker().stopped());
    let updates = log.borrow().len();
    let shown = p.last_frame().cloned();
    p.push_input(&press(Key::F));
    assert_eq!(app.frame(&mut p, Millis(34)), FrameOutcome::Quit);
    assert_eq!(log.borrow().len(), updates);
    assert_eq!(p.last_frame().cloned(), shown);
    assert_eq!(p.presented(), 3);
}

#[test]
fn a_quit_the_player_asks_for_lets_the_game_save_first() {
    let (game, log) = steps_game();
    let mut app = App::running(game);
    app.set_system_hook(|game, event| {
        let note = format!("{event:?}");
        let _ = game.ctx_mut().storage.write("heard", &note.to_lowercase());
    });
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    p.push_system(SystemEvent::QuitRequested);
    assert_eq!(app.frame(&mut p, Millis(17)), FrameOutcome::Quit);
    assert_eq!(log.borrow().len(), 1, "no update after the quit");
    let game = running(&app);
    assert_eq!(
        game.ctx().storage.read("heard"),
        Ok(Some("quitrequested".into()))
    );
    // Nothing more is heard once it has quit.
    p.push_system(SystemEvent::LowBattery);
    let _ = app.frame(&mut p, Millis(34));
    let game = running(&app);
    assert_eq!(
        game.ctx().storage.read("heard"),
        Ok(Some("quitrequested".into()))
    );
}

#[test]
fn focus_and_battery_go_to_the_game_and_the_window_shortcut_flips_fullscreen() {
    let mut app = App::running(toy::game());
    app.set_system_hook(|game, event| {
        let heard = game
            .ctx()
            .storage
            .read("heard")
            .ok()
            .flatten()
            .unwrap_or_default();
        let _ = game
            .ctx_mut()
            .storage
            .write("heard", &format!("{heard}{event:?};").to_lowercase());
    });
    let mut p = HeadlessPlatform::new();
    for event in [SystemEvent::FocusLost, SystemEvent::LowBattery] {
        p.push_system(event);
    }
    let _ = app.frame(&mut p, Millis(0));
    assert!(!app.is_focused());
    assert!(!matches!(
        p.last_frame(),
        Some(Presented::Game {
            fullscreen: true,
            ..
        })
    ));
    p.push_system(SystemEvent::FocusGained);
    p.push_system(SystemEvent::ToggleFullscreen);
    let _ = app.frame(&mut p, Millis(17));
    assert!(app.is_focused());
    assert!(matches!(
        p.last_frame(),
        Some(Presented::Game {
            fullscreen: true,
            ..
        })
    ));
    let game = running(&app);
    let heard = game.ctx().storage.read("heard").ok().flatten();
    assert_eq!(
        heard.as_deref(),
        Some("focuslost;lowbattery;focusgained;togglefullscreen;")
    );
}

#[test]
fn a_screen_asks_for_the_system_keyboard_and_hears_back_in_a_later_frame() {
    let (game, log) = steps_game();
    let mut app = App::running(game);
    let keyboard = ScriptedKeyboard::new([TextAnswer::Typed("Ada".into())]);
    let mut p = HeadlessPlatform::new().with_keyboard(keyboard);
    let _ = app.frame(&mut p, Millis(0));
    let present = |app: &App<Toy>| {
        app.game()
            .is_some_and(|g| g.ctx().system_keyboard.is_present())
    };
    assert!(present(&app));
    // `CursorUp` asks; the keyboard opens after that frame.
    let up = toy::ctx().keymap.primary(Action::CursorUp);
    let up = up.unwrap_or_else(|| panic!("CursorUp has no key"));
    p.push_input(&[RawInputEvent::Down(up)]);
    let _ = app.frame(&mut p, Millis(17));
    let asks = p.keyboard().map(|k| k.asks().len());
    assert_eq!(asks, Some(1));
    // The answer comes before the next update.
    let _ = app.frame(&mut p, Millis(34));
    let answers: Vec<Option<TextAnswer>> = log.borrow().iter().map(|s| s.answer.clone()).collect();
    assert_eq!(answers, [None, None, Some(TextAnswer::Typed("Ada".into()))]);
    // Without a keyboard, the ask is refused at once.
    let (game, _) = steps_game();
    let mut app = App::running(game);
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    assert!(!present(&app));
}

#[test]
fn a_keyboard_that_cannot_open_cancels_the_ask() {
    /// A system keyboard the system never lets open.
    struct Refusing;
    impl TextInput for Refusing {
        fn open(&mut self, _: &TextAsk) -> bool {
            false
        }
        fn poll(&mut self) -> Option<TextAnswer> {
            Some(TextAnswer::Typed("unasked".into()))
        }
    }
    struct WithRefusing(HeadlessPlatform, Refusing);
    impl Platform for WithRefusing {
        fn display(&self) -> DisplayInfo {
            self.0.display()
        }
        fn present(&mut self, frame: &Frame<'_>) {
            self.0.present(frame);
        }
        fn poll_input(&mut self, out: &mut Vec<RawInputEvent>) {
            self.0.poll_input(out);
        }
        fn system_events(&mut self, out: &mut Vec<SystemEvent>) {
            self.0.system_events(out);
        }
        fn storage(&mut self) -> Box<dyn Storage> {
            self.0.storage()
        }
        fn audio(&mut self) -> &mut dyn crate::audio::Audio {
            self.0.audio()
        }
        fn text_input(&mut self) -> Option<&mut dyn TextInput> {
            Some(&mut self.1)
        }
    }
    let (game, log) = steps_game();
    let mut app = App::running(game);
    let mut p = WithRefusing(HeadlessPlatform::new(), Refusing);
    // An answer nobody asked for is not handed on.
    let _ = app.frame(&mut p, Millis(0));
    let up = toy::ctx().keymap.primary(Action::CursorUp);
    p.0.push_input(&[RawInputEvent::Down(up.unwrap_or_else(|| panic!("no key")))]);
    let _ = app.frame(&mut p, Millis(17));
    let _ = app.frame(&mut p, Millis(34));
    let answers: Vec<Option<TextAnswer>> = log.borrow().iter().map(|s| s.answer.clone()).collect();
    assert_eq!(answers, [None, None, Some(TextAnswer::Cancelled)]);
}

#[test]
fn a_driver_with_its_own_clock_gives_the_screens_its_exact_frame_times() {
    let (game, log) = steps_game();
    let mut app = App::running(game);
    let mut p = HeadlessPlatform::new();
    let sixtieth = 1.0 / 60.0;
    for _ in 0..120 {
        let _ = app.frame_after(&mut p, sixtieth);
    }
    let log = log.borrow();
    assert!(log.iter().all(|s| (s.dt - sixtieth).abs() < f32::EPSILON));
    assert!(log.iter().all(|s| s.ticks == 1));
    // Nonsense counts as no time.
    drop(log);
    let _ = app.frame_after(&mut p, f32::NAN);
}

#[test]
fn a_seed_from_the_clock_differs_launch_to_launch() {
    assert_ne!(mix(1_700_000_000_000), mix(1_700_000_000_001));
    assert_ne!(mix(0), 0);
}
