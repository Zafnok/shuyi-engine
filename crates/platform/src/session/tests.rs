//! Session records on the toy game and the headless backend: what `App`
//! notes, the frame's hash, the text form, and playing a record back.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::rc::Rc;
use alloc::vec;
use core::cell::RefCell;

use proptest::prelude::*;
use shuyi_assets::keymap::Button;
use shuyi_core::made::ContentHash;
use shuyi_ui::frame_cap::{FrameCap, Pacing};
use shuyi_ui::input::pad::PadKind;
use shuyi_ui::input::{Chord, Key};
use shuyi_ui::screen::{FrameInput, Screen, Transition};
use shuyi_ui::storage::{MemoryStorage, StorageError};
use shuyi_ui::system_keyboard::TextAsk;
use shuyi_ui::toy::{self, Action, Toy, ToyCtx};
use shuyi_ui::{Game, GameHooks, KeyPrompt};

use super::*;
use crate::app::{App, Card, FrameOutcome, Setup};
use crate::audio::Audio;
use crate::headless::ScriptedKeyboard;
use crate::platform::{Look, Platform};

/// An empty font and image table: the toy draws neither.
static FONT: shuyi_assets::font::FontAtlasDef = shuyi_assets::font::FontAtlasDef {
    cell_w: 0,
    cell_h: 0,
    columns: 0,
    glyphs: BTreeMap::new(),
};
static IMAGES: shuyi_assets::image::ImageTable =
    shuyi_assets::image::ImageTable::new(BTreeMap::new());

/// The toy's look, for a start.
fn look(_: &Toy) -> Look<'_> {
    Look::new(&FONT, &IMAGES)
}

/// The start of `game`, made on the setup's context, with no first key to
/// wait for.
fn start_of(game: impl FnOnce(ToyCtx) -> Game<Toy> + 'static) -> Start<Toy> {
    Start::new(Card::text("Toy"), look, |setup| {
        Ok(game(setup.ctx(toy::ctx())))
    })
    .before_load(|setup| setup.key_prompt = KeyPrompt::Off)
}

/// The toy at its [`toy::Home`], hashing the frames it has seen.
fn home() -> Start<Toy> {
    start_of(toy::game_on).state_hash(|game| {
        let home = game.screen::<toy::Home>()?;
        Some(StateHash(u64::try_from(home.frames).ok()?))
    })
}

/// What a [`Probe`] saw in one update.
#[derive(Debug, Clone, PartialEq)]
struct Seen {
    actions: Vec<Action>,
    dt: f32,
    ticks: u32,
    answer: Option<TextAnswer>,
    music: Option<(String, f32)>,
    seed: u64,
    display: DisplayInfo,
    target: String,
    pacing: Option<Pacing>,
    /// What the store read at `key`.
    saved: Option<String>,
}

type Log = Rc<RefCell<Vec<Seen>>>;

/// A screen that logs all a game can see of the platform: Confirm asks for
/// the system keyboard, Cancel starts the music, and an answer writes to
/// the store.
struct Probe {
    log: Log,
    /// The display its load was handed.
    display: DisplayInfo,
}

impl Screen<Toy> for Probe {
    fn name(&self) -> &'static str {
        "probe"
    }

    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        if input.actions.contains(&Action::Confirm) {
            ctx.system_keyboard.ask(TextAsk {
                prompt: "Name".into(),
                text: String::new(),
                max_chars: 8,
            });
        }
        if input.actions.contains(&Action::Cancel) {
            ctx.audio.play_music("theme");
        }
        let answer = ctx.system_keyboard.take_answer();
        if let Some(TextAnswer::Typed(name)) = &answer {
            let wrote = ctx.storage.computer_mut().write("name", name);
            assert_eq!(wrote, Ok(()));
        }
        self.log.borrow_mut().push(Seen {
            actions: input.actions.clone(),
            dt: input.dt(),
            ticks: input.ticks(),
            answer,
            music: ctx
                .music_clock
                .as_ref()
                .map(|c| (c.cue.clone(), c.position)),
            seed: ctx.music_seed,
            display: self.display,
            target: ctx.platform.target.clone(),
            pacing: ctx.pacing(),
            saved: ctx.storage.computer().read("name").unwrap_or_default(),
        });
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut shuyi_ui::frame::glyph_buffer::GlyphBuffer) {}
}

/// A start of a [`Probe`] that logs to `log`.
fn probe_on(log: Log) -> Start<Toy> {
    Start::new(Card::text("Toy"), look, move |setup: Setup| {
        let display = setup.display;
        let probe = Probe { log, display };
        let ctx = setup.ctx(toy::ctx());
        Ok(Game::new(ctx, Box::new(probe), GameHooks::default()))
    })
    .before_load(|setup| setup.key_prompt = KeyPrompt::Off)
}

/// A start of a [`Probe`], and what it will see.
fn probe() -> (Start<Toy>, Log) {
    let log = Log::default();
    (probe_on(Rc::clone(&log)), log)
}

fn press(key: Key) -> Vec<RawInputEvent> {
    vec![RawInputEvent::Down(Chord::plain(key))]
}

/// The key of `action` in the toy's keymap, pressed.
fn press_action(action: Action) -> Vec<RawInputEvent> {
    let chord = toy::ctx().keymap.primary(action);
    vec![RawInputEvent::Down(
        chord.unwrap_or_else(|| panic!("{action:?} has no key")),
    )]
}

/// Records `start` on `platform`: a frame at each time, with its input.
fn record_on(
    start: Start<Toy>,
    platform: &mut HeadlessPlatform,
    frames: Vec<(u64, Vec<RawInputEvent>)>,
) -> (SessionRecord, App<Toy>) {
    let mut app = App::new(start);
    assert!(app.record());
    for (now, input) in frames {
        platform.push_input(&input);
        let _ = app.frame(platform, Millis(now));
    }
    let record = app.take_session();
    (record.unwrap_or_else(|| panic!("not recording")), app)
}

/// Frames 16 ms apart from 0, the first two the card and the load, with
/// `input` at the frames given (counted from 1).
fn timed(count: u64, input: &[(u64, Vec<RawInputEvent>)]) -> Vec<(u64, Vec<RawInputEvent>)> {
    (1..=count)
        .map(|frame| {
            let events = input.iter().find(|(at, _)| *at == frame);
            (
                (frame - 1) * 16,
                events.map_or_else(Vec::new, |(_, events)| events.clone()),
            )
        })
        .collect()
}

/// Plays all of `record` on `start`: the app, the platform, and each frame's
/// answer.
fn play(
    start: Start<Toy>,
    record: &SessionRecord,
) -> (
    App<Toy>,
    HeadlessPlatform,
    Vec<Result<u32, FrameDifference>>,
) {
    let (start, _) =
        prepare(start, record, Stamps::Ignore).unwrap_or_else(|error| panic!("{error}"));
    let mut app = App::new(start);
    let mut platform = HeadlessPlatform::from_session(record);
    let mut playback = Playback::new(record);
    let answers = play_out(&mut playback, record, &mut app, &mut platform);
    (app, platform, answers)
}

/// Plays `record` through `playback`: one answer a frame, and a playback that
/// goes on past its record fails here rather than running for ever.
fn play_out(
    playback: &mut Playback,
    record: &SessionRecord,
    app: &mut App<Toy>,
    platform: &mut HeadlessPlatform,
) -> Vec<Result<u32, FrameDifference>> {
    let most = record.frames.len() + 1;
    let answers: Vec<_> = (0..most)
        .map_while(|_| playback.next(app, platform))
        .collect();
    assert_eq!(answers.len(), record.frames.len(), "one answer a frame");
    answers
}

/// Asserts every frame of `answers` played the same.
fn assert_same(answers: &[Result<u32, FrameDifference>]) {
    for (i, answer) in answers.iter().enumerate() {
        assert_eq!(*answer, Ok(u32::try_from(i + 1).unwrap_or(0)));
    }
}

/// What `app`'s store holds.
fn store(app: App<Toy>) -> BTreeMap<String, String> {
    let game = app.into_game().unwrap_or_else(|| panic!("not running"));
    let storage = game.into_ctx().storage.into_computer();
    let mut all = BTreeMap::new();
    assert!(read_store(storage.as_ref(), &mut all));
    all
}

fn stamp(game: &str) -> Made {
    Made::new(game, ContentHash(0x1a2b))
}

#[test]
fn recording_starts_before_the_first_frame_only() {
    let mut app = App::new(home());
    assert!(app.session().is_none());
    assert!(app.record());
    assert!(app.record(), "already on");
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(5));
    assert!(!app.record());
    assert_eq!(app.session().map(|s| s.start), Some(5));

    let mut late = App::new(home());
    let _ = late.frame(&mut p, Millis(0));
    assert!(!late.record());
    assert!(late.session().is_none());
    assert_eq!(late.frame_hash(), None, "not recording, not hashing");
}

#[test]
fn the_load_is_noted_and_a_backend_adds_its_part() {
    let mut saved = MemoryStorage::new();
    assert_eq!(saved.write("settings", "(speed: 3)"), Ok(()));
    let mut facts = PlatformFacts::default();
    "playdate".clone_into(&mut facts.target);
    let display = DisplayInfo::new(400, 240).one_bit();
    let mut p = HeadlessPlatform::new()
        .with_storage(Box::new(saved))
        .with_display(display)
        .with_facts(facts.clone());
    let made = stamp("demo 1.0");
    let mut app = App::new(home().made(made.clone()));
    assert!(app.record());
    if let Some(record) = app.session_mut() {
        record.voice_manifest = Some("voices".to_owned());
    }
    let _ = app.frame(&mut p, Millis(0));
    assert_eq!(app.session().map(|s| s.storage_complete), Some(false));
    let _ = app.frame(&mut p, Millis(16));
    let record = app.session().unwrap_or_else(|| panic!("not recording"));
    assert_eq!(record.made, Some(made));
    assert_eq!(record.display, display);
    assert_eq!(record.platform, facts);
    assert_eq!(
        record.storage.get("settings").map(String::as_str),
        Some("(speed: 3)")
    );
    assert!(record.storage_complete);
    assert_eq!(record.voice_manifest.as_deref(), Some("voices"));
    assert_eq!(record.frames.len(), 2);
}

#[test]
fn a_store_that_cannot_be_read_says_so() {
    #[derive(Debug)]
    struct Broken {
        keys: bool,
    }
    impl Storage for Broken {
        fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
            if key == "good" {
                return Ok(Some("value".into()));
            }
            Err(StorageError::Backend("no".into()))
        }
        fn write(&mut self, _: &str, _: &str) -> Result<(), StorageError> {
            Ok(())
        }
        fn delete(&mut self, _: &str) -> Result<(), StorageError> {
            Ok(())
        }
        fn list(&self) -> Result<Vec<String>, StorageError> {
            if self.keys {
                Ok(vec!["good".into(), "bad".into()])
            } else {
                Err(StorageError::Backend("no".into()))
            }
        }
    }
    let mut all = BTreeMap::new();
    assert!(!read_store(&Broken { keys: false }, &mut all));
    assert!(all.is_empty());
    assert!(!read_store(&Broken { keys: true }, &mut all));
    assert_eq!(all.len(), 1, "what could be read is kept");
    let mut record = SessionRecord::new();
    assert!(record.limits().iter().any(|l| l.contains("store")));
    record.storage_complete = true;
    assert_eq!(record.limits(), Vec::<String>::new());
}

#[test]
fn an_idle_frame_is_its_dt_alone_and_the_frames_hash_leaves_out_the_picture() {
    let (start, _) = probe();
    let mut p = HeadlessPlatform::new();
    let (record, _) = record_on(start, &mut p, timed(6, &[]));
    assert_eq!(record.frames[0].dt, 0);
    assert_eq!(record.frames[1].dt, 16);
    // The card and the load, then the game's first frame, whose screen has
    // opened: each has a hash of its own, and idle frames have none.
    assert!(record.frames[0].hash.is_some());
    assert!(record.frames[1].hash.is_some());
    assert_ne!(record.frames[0].hash, record.frames[1].hash);
    let idle = FrameRecord {
        dt: 16,
        ..FrameRecord::default()
    };
    assert_eq!(record.frames[3..], [idle.clone(), idle.clone(), idle]);
    let text = record.to_ron();
    assert!(text.contains("\n  (dt: 16),\n"), "{text}");
    // One frame per line.
    assert_eq!(text.matches("(dt: ").count(), 6, "{text}");
    assert_eq!(
        text.lines().filter(|l| l.contains("(dt: ")).count(),
        6,
        "{text}"
    );
}

/// The hash of one frame on `start`, after `input`.
fn hash_after(start: Start<Toy>, input: Vec<RawInputEvent>) -> Vec<Option<StateHash>> {
    let mut app = App::new(start);
    app.hash_frames(true);
    let mut p = HeadlessPlatform::new();
    let mut hashes = Vec::new();
    for (now, input) in timed(4, &[(3, input)]) {
        p.push_input(&input);
        let _ = app.frame(&mut p, Millis(now));
        hashes.push(app.frame_hash());
    }
    hashes
}

#[test]
fn the_frames_hash_reads_the_phase_the_screens_quitting_the_audio_and_the_games_hash() {
    let base = hash_after(home(), Vec::new());
    assert!(base.iter().all(Option::is_some));
    assert_eq!(base, hash_after(home(), Vec::new()), "the same each run");
    // The phase: the card, then the game.
    assert_ne!(base[0], base[1]);
    // A failed load, with its errors' text.
    let failed = |errors: &'static str| {
        let start: Start<Toy> = Start::new(Card::text("Toy"), look, move |_| Err(errors.into()));
        hash_after(start, Vec::new())
    };
    assert_ne!(failed("a")[1], base[1]);
    assert_ne!(failed("a")[1], failed("b")[1]);
    assert_ne!(failed("a")[0], failed("a")[1]);
    // The screens: Confirm opens the menu.
    let opened = hash_after(home(), press(Key::F));
    assert_eq!(base[1], opened[1]);
    assert_ne!(base[2], opened[2]);
    // The game's hash: it counts the frames, so each differs; without one,
    // the frames are the same.
    assert_ne!(base[2], base[3]);
    let plain = hash_after(start_of(toy::game_on), Vec::new());
    assert_eq!(plain[2], plain[3]);
    assert_ne!(plain[2], base[2]);
    // Quitting: Cancel at home.
    let quit = hash_after(start_of(toy::game_on), press_action(Action::Cancel));
    assert_ne!(plain[2], quit[2]);
    assert_eq!(quit[3], quit[2], "the quit frame again, in silence");
    // The audio: at the menu Confirm clicks, a frame that changes nothing
    // else; and the next is the same screens in silence.
    let mut app = App::new(start_of(toy::game_on));
    app.hash_frames(true);
    let mut p = HeadlessPlatform::new();
    let mut hashes = Vec::new();
    let up = vec![RawInputEvent::Up(Key::F)];
    let script = [
        (3, press(Key::F)),
        (4, up.clone()),
        (5, press(Key::F)),
        (6, up),
    ];
    for (now, input) in timed(6, &script) {
        p.push_input(&input);
        let _ = app.frame(&mut p, Millis(now));
        hashes.push(app.frame_hash());
    }
    assert_eq!(p.speaker().requests().len(), 0, "the last frame is silent");
    assert_ne!(hashes[4], hashes[3], "the click");
    assert_eq!(hashes[5], hashes[3], "the same in silence");
    // A music command is in the hash too.
    let (start, _) = probe();
    let mut app = App::new(start);
    app.hash_frames(true);
    let mut p = HeadlessPlatform::new();
    let mut hashes = Vec::new();
    for (now, input) in timed(5, &[(3, press_action(Action::Cancel))]) {
        p.push_input(&input);
        let _ = app.frame(&mut p, Millis(now));
        hashes.push(app.frame_hash());
    }
    assert_ne!(hashes[2], hashes[4], "the music commands");
    // Off again, none.
    app.hash_frames(false);
    let _ = app.frame(&mut p, Millis(100));
    assert_eq!(app.frame_hash(), None);
}

#[test]
fn a_recording_stops_at_its_cap_and_on_a_frame_after() {
    assert_eq!(MAX_SESSION_FRAMES, 216_000);
    let mut app = App::new(home());
    assert!(app.record());
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    let fill = |app: &mut App<Toy>, frames: usize| {
        if let Some(record) = app.session_mut() {
            record.frames.resize(frames, FrameRecord::default());
        }
    };
    fill(&mut app, MAX_SESSION_FRAMES - 1);
    let _ = app.frame(&mut p, Millis(16));
    let record = app.session().unwrap_or_else(|| panic!("not recording"));
    assert_eq!(record.frames.len(), MAX_SESSION_FRAMES);
    assert!(!record.truncated, "it holds every frame so far");
    assert!(app.frame_hash().is_some());
    let _ = app.frame(&mut p, Millis(32));
    let record = app.session().unwrap_or_else(|| panic!("not recording"));
    assert_eq!(record.frames.len(), MAX_SESSION_FRAMES);
    assert!(record.truncated);
    assert_eq!(app.frame_hash(), None, "it stopped hashing");
    // A later frame changes nothing.
    p.push_input(&press(Key::F));
    p.push_system(SystemEvent::FocusLost);
    let _ = app.frame(&mut p, Millis(48));
    assert_eq!(
        app.session().map(|s| (s.frames.len(), s.truncated)),
        Some((MAX_SESSION_FRAMES, true))
    );

    let mut app = App::new(home());
    assert!(app.record());
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame_after(&mut p, 0.016);
    let _ = app.frame(&mut p, Millis(32));
    let record = app
        .take_session()
        .unwrap_or_else(|| panic!("not recording"));
    assert_eq!(record.frames.len(), 1);
    assert!(record.truncated);
    assert!(record.limits().iter().any(|l| l.contains("stopped early")));
    assert!(app.session().is_none(), "taken");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Random keys, pad buttons, frames of 0 to 100 ms and a suspend and a
    /// resume: played back, no frame differs, and the game ends on the same
    /// screens with the same store.
    #[test]
    fn a_recorded_session_plays_back_the_same(
        ops in proptest::collection::vec(0u8..8, 0..40),
        waits in proptest::collection::vec(0u64..=100, 40),
        suspend in 3usize..40,
        resume in 4usize..45,
    ) {
        let mut p = HeadlessPlatform::new();
        let mut app = App::new(home());
        prop_assert!(app.record());
        let (mut now, mut frame) = (1000_u64, 0_usize);
        let mut run = |p: &mut HeadlessPlatform, app: &mut App<Toy>, input: &[RawInputEvent], wait: u64| {
            frame += 1;
            if frame == suspend {
                p.push_system(SystemEvent::Suspend);
            }
            if frame == resume {
                p.push_system(SystemEvent::Resume);
            }
            now += wait;
            p.push_input(input);
            let _ = app.frame(p, Millis(now));
        };
        run(&mut p, &mut app, &[], 0);
        run(&mut p, &mut app, &[], 16);
        for (op, wait) in ops.iter().zip(&waits) {
            let input: Vec<RawInputEvent> = match op {
                0 => press(Key::F),
                1 => vec![RawInputEvent::Up(Key::F)],
                2 => press_action(Action::Cancel),
                3 => press_action(Action::CursorUp),
                4 => vec![RawInputEvent::PadDown(Button::South, PadKind::Xbox)],
                5 => vec![RawInputEvent::PadUp(Button::South)],
                _ => Vec::new(),
            };
            run(&mut p, &mut app, &input, *wait);
        }
        let record = app.take_session().unwrap_or_else(|| panic!("not recording"));
        let screens = app.game().map(Game::screens);
        let saved = store(app);
        let (played, _, answers) = play(home(), &record);
        assert_same(&answers);
        prop_assert_eq!(played.game().map(Game::screens), screens);
        prop_assert_eq!(store(played), saved);
        // And from its text.
        prop_assert!(SessionRecord::from_ron(&record.to_ron()) == Ok(record));
    }
}

/// Any event a platform reports.
fn any_event() -> impl Strategy<Value = RawInputEvent> {
    let key = prop_oneof![
        Just(Key::F),
        Just(Key::D),
        Just(Key::Enter),
        Just(Key::Space)
    ];
    let button = prop_oneof![
        Just(Button::South),
        Just(Button::DpadUp),
        Just(Button::Start)
    ];
    let kind = prop::sample::select(PadKind::ALL.to_vec());
    prop_oneof![
        (key.clone(), any::<bool>())
            .prop_map(|(key, shift)| RawInputEvent::Down(Chord { key, shift })),
        key.prop_map(RawInputEvent::Up),
        any::<char>().prop_map(RawInputEvent::Text),
        (button.clone(), kind).prop_map(|(button, kind)| RawInputEvent::PadDown(button, kind)),
        button.prop_map(RawInputEvent::PadUp),
    ]
}

/// Any event the system says.
fn any_system() -> impl Strategy<Value = SystemEvent> {
    prop::sample::select(vec![
        SystemEvent::Suspend,
        SystemEvent::Resume,
        SystemEvent::FocusLost,
        SystemEvent::FocusGained,
        SystemEvent::QuitRequested,
        SystemEvent::LowBattery,
        SystemEvent::ToggleFullscreen,
        SystemEvent::CrankDocked,
        SystemEvent::CrankUndocked,
    ])
}

/// Any position a sound device may answer but NaN, which has no position.
fn any_position() -> impl Strategy<Value = f64> {
    use proptest::num::f64::{INFINITE, NEGATIVE, NORMAL, POSITIVE, SUBNORMAL, ZERO};
    POSITIVE | NEGATIVE | NORMAL | SUBNORMAL | ZERO | INFINITE
}

fn any_answer() -> impl Strategy<Value = TextAnswer> {
    prop_oneof![
        Just(TextAnswer::Cancelled),
        any::<String>().prop_map(TextAnswer::Typed)
    ]
}

fn any_frame() -> impl Strategy<Value = FrameRecord> {
    let keyboard = (
        proptest::option::of(any::<bool>()),
        proptest::option::of(any_answer()),
        proptest::option::of(any::<bool>()),
    )
        .prop_map(|(present, answer, opened)| KeyboardFrame {
            present,
            answer,
            opened,
        });
    let pacing = (15u16..1000, any::<u32>(), any::<bool>(), any::<u16>()).prop_map(
        |(fps, serial, vsync, measured)| {
            let mut pacing = Pacing::new(FrameCap::Fps(fps), serial);
            pacing.vsync = vsync;
            pacing.fps = measured;
            pacing
        },
    );
    (
        any::<i64>(),
        proptest::collection::vec(any_system(), 0..3),
        proptest::collection::vec(any_event(), 0..4),
        proptest::option::of(keyboard),
        proptest::option::of(any_position()),
        proptest::option::of(any::<String>()),
        proptest::option::of(pacing),
        proptest::option::of(any::<u64>()),
    )
        .prop_map(
            |(dt, system, input, keyboard, music, track, pacing, hash)| FrameRecord {
                dt,
                system,
                input,
                keyboard,
                music,
                track,
                pacing,
                hash,
            },
        )
}

fn any_record() -> impl Strategy<Value = SessionRecord> {
    (
        any::<u64>(),
        any::<bool>(),
        any::<bool>(),
        proptest::option::of(any::<u32>()),
        proptest::option::of(any::<String>()),
        proptest::collection::btree_map("[a-z0-9_-]{1,8}", any::<String>(), 0..3),
        proptest::collection::vec(any_frame(), 0..5),
    )
        .prop_map(
            |(start, truncated, storage_complete, reloaded_at, voice_manifest, storage, frames)| {
                let mut record = SessionRecord::new();
                record.start = start;
                record.truncated = truncated;
                record.storage_complete = storage_complete;
                record.reloaded_at = reloaded_at;
                record.voice_manifest = voice_manifest;
                record.storage = storage;
                record.frames = frames;
                record
            },
        )
}

proptest! {
    #[test]
    fn a_record_reads_back_from_its_text(record in any_record()) {
        let read = SessionRecord::from_ron(&record.to_ron());
        let read = read.unwrap_or_else(|error| panic!("{error}\n{}", record.to_ron()));
        prop_assert_eq!(&read, &record);
        // A music position, bit for bit (0.0 and -0.0 compare equal).
        for (read, frame) in read.frames.iter().zip(&record.frames) {
            prop_assert_eq!(read.music.map(f64::to_bits), frame.music.map(f64::to_bits));
        }
    }

    #[test]
    fn any_text_gives_a_record_or_an_error(text in any::<String>()) {
        let _ = SessionRecord::from_ron(&text);
    }

    /// A record's text, broken anywhere.
    #[test]
    fn a_broken_record_gives_an_error_or_a_record(
        record in any_record(),
        at in any::<proptest::sample::Index>(),
        with in any::<char>(),
    ) {
        let mut text: Vec<char> = record.to_ron().chars().collect();
        let i = at.index(text.len());
        text[i] = with;
        let text: String = text.into_iter().collect();
        let _ = SessionRecord::from_ron(&text);
    }
}

/// What a random text rarely holds: what RON must escape.
#[test]
fn awkward_text_reads_back_from_its_text() {
    let awkward = [
        '\n',
        '\r',
        '\t',
        '\'',
        '"',
        '\\',
        '\0',
        '\u{7f}',
        '\u{2028}',
        '\u{1F600}',
    ];
    let mut record = SessionRecord::new();
    record.frames = awkward
        .iter()
        .map(|&c| FrameRecord {
            input: vec![RawInputEvent::Text(c)],
            track: Some(format!("a{c}b")),
            ..FrameRecord::default()
        })
        .collect();
    record.voice_manifest = Some(awkward.iter().collect());
    let text = record.to_ron();
    assert!(!text.contains('\r'), "the same line ending everywhere");
    let frames = text.lines().filter(|l| l.contains("(dt: "));
    assert_eq!(frames.count(), awkward.len(), "one frame per line: {text}");
    assert_eq!(SessionRecord::from_ron(&text), Ok(record));
    // A typed character is one character.
    let two = text.replacen("Text(\"\\n\")", "Text(\"ab\")", 1);
    assert_ne!(two, text);
    let error = SessionRecord::from_ron(&two);
    assert!(matches!(error, Err(SessionError::Text(_))), "{error:?}");
}

#[test]
fn another_version_is_refused_with_both_numbers_whatever_it_holds() {
    let mut record = SessionRecord::new();
    record.format_version = 2;
    let error = SessionRecord::from_ron(&record.to_ron());
    assert_eq!(error, Err(SessionError::Version(2)));
    let text = error.map(|_| ()).unwrap_err().to_string();
    assert!(
        text.contains("version 2") && text.contains("version 1"),
        "{text}"
    );
    // A later version may hold what this one can't read.
    let later = record.to_ron().replacen('(', "(new_field: 1, ", 1);
    assert_eq!(
        SessionRecord::from_ron(&later),
        Err(SessionError::Version(2))
    );
    // This version may not.
    record.format_version = FORMAT_VERSION;
    let unknown = record.to_ron().replacen('(', "(new_field: 1, ", 1);
    let error = SessionRecord::from_ron(&unknown);
    assert!(matches!(error, Err(SessionError::Text(_))), "{error:?}");
    // Its error says where.
    let broken = SessionRecord::from_ron("(format_version: ?)");
    let text = broken.map(|_| ()).unwrap_err().to_string();
    assert!(
        text.contains("can't be read") && text.contains("1:"),
        "{text}"
    );
}

#[test]
fn a_clock_that_goes_back_plays_back_the_same() {
    let (start, seen) = probe();
    let mut p = HeadlessPlatform::new();
    let times = [1000, 1016, 1032, 500, 516, 400, 416, 433];
    let frames = times.iter().map(|&now| (now, Vec::new())).collect();
    let (record, _) = record_on(start, &mut p, frames);
    assert_eq!(record.start, 1000);
    let dts: Vec<i64> = record.frames.iter().map(|f| f.dt).collect();
    assert_eq!(dts, [0, 16, 16, -532, 16, -116, 16, 17]);
    let (start, again) = probe();
    let (_, _, answers) = play(start, &record);
    assert_same(&answers);
    assert_eq!(*seen.borrow(), *again.borrow());
    assert_eq!(seen.borrow().len(), times.len() - 2);
}

#[test]
fn a_time_at_the_clocks_ends_plays_back_exactly() {
    let times = [u64::MAX, 3, u64::MAX - 1, 0];
    let (start, _) = probe();
    let mut p = HeadlessPlatform::new();
    let frames = times.iter().map(|&now| (now, Vec::new())).collect();
    let (record, _) = record_on(start, &mut p, frames);
    assert_eq!(record.start, u64::MAX);
    let mut playback = Playback::new(&record);
    assert_eq!(playback.now(), Millis(u64::MAX));
    let (start, _) = probe();
    let mut app = App::new(start);
    let mut platform = HeadlessPlatform::from_session(&record);
    for &now in &times {
        assert!(!playback.is_done());
        let answer = playback.next(&mut app, &mut platform);
        assert!(matches!(answer, Some(Ok(_))), "{answer:?}");
        assert_eq!(playback.now(), Millis(now));
    }
    assert!(playback.is_done());
    assert!(playback.next(&mut app, &mut platform).is_none());
}

#[test]
fn a_key_press_taken_out_is_found_at_its_frame() {
    let mut p = HeadlessPlatform::new();
    let frames = timed(10, &[(6, press(Key::F))]);
    let (mut record, _) = record_on(home(), &mut p, frames);
    let (_, _, answers) = play(home(), &record);
    assert_same(&answers);
    assert_eq!(answers.len(), 10);
    record.frames[5].input.clear();
    let (_, _, answers) = play(home(), &record);
    assert_same(&answers[..5]);
    let difference = answers[5].err().unwrap_or_else(|| panic!("no difference"));
    assert_eq!(difference.frame, 6);
    assert_ne!(difference.recorded, difference.played);
}

#[test]
fn a_frame_with_no_hash_has_the_last_one_written() {
    let start = || start_of(toy::game_on);
    let mut p = HeadlessPlatform::new();
    let frames = timed(8, &[(6, press(Key::F))]);
    let (mut record, _) = record_on(start(), &mut p, frames);
    // Idle frames write none, and match the last one written.
    assert!(record.frames[3].hash.is_none());
    assert!(record.frames[5].hash.is_some());
    // Without the press's hash, the frame is compared with the one before.
    let press_hash = record.frames[5].hash.take();
    let (_, _, answers) = play(start(), &record);
    assert_same(&answers[..5]);
    let difference = answers[5].err().unwrap_or_else(|| panic!("no difference"));
    assert_eq!(difference.frame, 6);
    assert_eq!(Some(difference.played.0), press_hash);
    // A record with no hashes at all compares nothing.
    for frame in &mut record.frames {
        frame.hash = None;
    }
    let (_, _, answers) = play(start(), &record);
    assert_same(&answers);
}

#[test]
fn a_session_plays_back_with_its_saved_store_from_the_record_alone() {
    let mut saved = MemoryStorage::new();
    assert_eq!(saved.write("name", "Before"), Ok(()));
    let (start, seen) = probe();
    let keyboard = ScriptedKeyboard::new([TextAnswer::Typed("Ada".into())]);
    let mut p = HeadlessPlatform::new()
        .with_storage(Box::new(saved))
        .with_keyboard(keyboard);
    let frames = timed(6, &[(3, press_action(Action::Confirm))]);
    let (record, app) = record_on(start, &mut p, frames);
    let wrote = store(app);
    assert_eq!(wrote.get("name").map(String::as_str), Some("Ada"));
    assert_eq!(
        record.storage.get("name").map(String::as_str),
        Some("Before"),
        "as it was handed over"
    );
    let (start, again) = probe();
    let (played, _, answers) = play(start, &record);
    assert_same(&answers);
    assert_eq!(*seen.borrow(), *again.borrow());
    assert_eq!(again.borrow()[0].saved.as_deref(), Some("Before"));
    assert_eq!(store(played), wrote, "the playback's writes are its own");
    assert_eq!(
        record.storage.get("name").map(String::as_str),
        Some("Before")
    );
}

#[test]
fn a_keyboards_answer_plays_back_with_no_keyboard_given() {
    let (start, seen) = probe();
    let keyboard = ScriptedKeyboard::new([TextAnswer::Typed("Ada".into())]);
    let mut p = HeadlessPlatform::new().with_keyboard(keyboard);
    let frames = timed(6, &[(3, press_action(Action::Confirm))]);
    let (record, _) = record_on(start, &mut p, frames);
    let present = record.frames[2].keyboard.as_ref();
    assert_eq!(present.and_then(|k| k.present), Some(true));
    assert_eq!(present.and_then(|k| k.opened), Some(true));
    let answered = record.frames[3].keyboard.as_ref();
    assert_eq!(
        answered.and_then(|k| k.answer.clone()),
        Some(TextAnswer::Typed("Ada".into()))
    );
    assert_eq!(answered.and_then(|k| k.present), None, "unchanged");
    assert_eq!(record.frames[4].keyboard, None);

    let (start, again) = probe();
    let (played, platform, answers) = play(start, &record);
    assert_same(&answers);
    assert!(platform.keyboard().is_none());
    assert_eq!(*seen.borrow(), *again.borrow());
    let typed = again
        .borrow()
        .iter()
        .filter_map(|s| s.answer.clone())
        .collect::<Vec<_>>();
    assert_eq!(typed, [TextAnswer::Typed("Ada".into())]);
    assert_eq!(store(played).get("name").map(String::as_str), Some("Ada"));
}

#[test]
fn a_recorded_keyboard_is_there_on_its_frames_and_answers_as_recorded() {
    let ask = TextAsk {
        prompt: String::new(),
        text: String::new(),
        max_chars: 1,
    };
    let mut p = HeadlessPlatform::from_session(&SessionRecord::new());
    let mut frame = FrameRecord::default();
    p.push_frame(&frame);
    assert!(p.text_input().is_none(), "not there until said");
    frame.keyboard = Some(KeyboardFrame {
        present: Some(true),
        answer: Some(TextAnswer::Cancelled),
        opened: Some(false),
    });
    p.push_frame(&frame);
    let keyboard = p.text_input().unwrap_or_else(|| panic!("not there"));
    assert_eq!(keyboard.poll(), Some(TextAnswer::Cancelled));
    assert_eq!(keyboard.poll(), None, "its answer once");
    assert!(!keyboard.open(&ask), "a refusal, as recorded");
    frame.keyboard = Some(KeyboardFrame {
        opened: Some(true),
        ..KeyboardFrame::default()
    });
    p.push_frame(&frame);
    let keyboard = p.text_input().unwrap_or_else(|| panic!("not there"));
    assert_eq!(keyboard.poll(), None);
    assert!(keyboard.open(&ask));
    assert!(!keyboard.open(&ask), "one answer for one ask");
    frame.keyboard = None;
    p.push_frame(&frame);
    let keyboard = p.text_input().unwrap_or_else(|| panic!("not there"));
    assert!(!keyboard.open(&ask), "no answer written, none given");
    frame.keyboard = Some(KeyboardFrame {
        present: Some(false),
        ..KeyboardFrame::default()
    });
    p.push_frame(&frame);
    assert!(p.text_input().is_none());
    // Over, the backend's own keyboard is back.
    p.end_session();
    assert!(p.text_input().is_none());
    let mut own = HeadlessPlatform::new().with_keyboard(ScriptedKeyboard::default());
    own.push_frame(&FrameRecord::default());
    assert!(own.text_input().is_none(), "the record's in its place");
    own.end_session();
    assert!(own.text_input().is_some());
}

#[test]
fn a_refused_open_plays_back_as_a_cancel() {
    let (start, seen) = probe();
    let mut app = App::new(start);
    assert!(app.record());
    // A keyboard that is there and refuses: the recorded one, as told.
    let mut p = HeadlessPlatform::new();
    for (now, input) in timed(5, &[(3, press_action(Action::Confirm))]) {
        let mut frame = FrameRecord {
            input,
            ..FrameRecord::default()
        };
        if now == 32 {
            frame.keyboard = Some(KeyboardFrame {
                present: Some(true),
                answer: None,
                opened: Some(false),
            });
        }
        p.push_frame(&frame);
        let _ = app.frame(&mut p, Millis(now));
    }
    let record = app
        .take_session()
        .unwrap_or_else(|| panic!("not recording"));
    let opened = record.frames[2].keyboard.as_ref().and_then(|k| k.opened);
    assert_eq!(opened, Some(false));
    let (start, again) = probe();
    let (_, _, answers) = play(start, &record);
    assert_same(&answers);
    assert_eq!(*seen.borrow(), *again.borrow());
    let cancelled = Some(TextAnswer::Cancelled);
    assert!(seen.borrow().iter().any(|s| s.answer == cancelled));
}

#[test]
fn the_music_plays_back_from_the_record_whatever_the_speaker_says() {
    let (start, seen) = probe();
    let mut p = HeadlessPlatform::new();
    p.speaker_mut().set_load_delay(0.05);
    let frames = timed(16, &[(3, press_action(Action::Cancel))]);
    let (record, _) = record_on(start, &mut p, frames);
    assert!(record.frames.iter().any(|f| f.music.is_some()));
    let first = record.frames.iter().find(|f| f.music.is_some());
    assert_eq!(
        first.and_then(|f| f.track.as_deref()),
        Some("theme"),
        "written when it starts"
    );
    let tracks = record.frames.iter().filter(|f| f.track.is_some()).count();
    assert_eq!(tracks, 1, "and only then");

    let (start, again) = probe();
    let mut app = App::new(start);
    let mut platform = HeadlessPlatform::from_session(&record);
    platform.speaker_mut().set_load_delay(5.0);
    let mut playback = Playback::new(&record);
    assert_same(&play_out(&mut playback, &record, &mut app, &mut platform));
    assert_eq!(*seen.borrow(), *again.borrow());
    assert!(again.borrow().iter().any(|s| s.music.is_some()));
    // Played, the speaker answers for itself: 5 s of load to go.
    assert_eq!(platform.speaker().music_playing(playback.now()), None);
}

#[test]
fn silence_is_written_as_an_empty_track() {
    let mut recorder = Recorder::new(None);
    recorder.begin(0);
    recorder.music(None);
    assert_eq!(recorder.frame.track, None, "silent from the start");
    recorder.music(Some(("a", 0.5)));
    assert_eq!(recorder.frame.track.as_deref(), Some("a"));
    assert_eq!(recorder.frame.music, Some(0.5));
    recorder.end(&[], StateHash(1));
    recorder.begin(1);
    recorder.music(Some(("a", 0.75)));
    assert_eq!(recorder.frame.track, None, "unchanged");
    recorder.end(&[], StateHash(1));
    recorder.begin(2);
    recorder.music(None);
    assert_eq!(recorder.frame.track.as_deref(), Some(""));
    assert_eq!(recorder.frame.music, None);
    recorder.end(&[], StateHash(1));
    let frames = &recorder.record.frames;
    assert_eq!(frames.len(), 3);
    let mut p = HeadlessPlatform::new();
    p.push_frame(&frames[0]);
    assert_eq!(p.speaker().music_playing(Millis(0)), Some(("a", 0.5)));
    p.push_frame(&frames[1]);
    assert_eq!(p.speaker().music_playing(Millis(0)), Some(("a", 0.75)));
    p.push_frame(&frames[2]);
    assert_eq!(p.speaker().music_playing(Millis(0)), None);
}

#[test]
fn stamps_are_checked_and_the_limits_say_what_isnt() {
    let made = stamp("demo 1.0");
    let mut record = SessionRecord::new();
    record.storage_complete = true;
    record.made = Some(made.clone());
    let limits = |result: Result<(Start<Toy>, Vec<String>), SessionError>| {
        result.map_or_else(|error| panic!("{error}"), |(_, limits)| limits)
    };
    // The same build: nothing to say.
    let same = prepare(home().made(made.clone()), &record, Stamps::Check);
    assert_eq!(limits(same), Vec::<String>::new());
    // Another build: refused, naming both.
    let other = stamp("demo 2.0");
    let refused = prepare(home().made(other.clone()), &record, Stamps::Check);
    let error = refused.err().unwrap_or_else(|| panic!("not refused"));
    assert!(matches!(error, SessionError::Made(_)));
    let text = error.to_string();
    assert!(
        text.contains("demo 1.0") && text.contains("demo 2.0"),
        "{text}"
    );
    // Ignored: it plays, and says so; even the same build.
    let ignored = limits(prepare(home().made(other), &record, Stamps::Ignore));
    assert_eq!(ignored.len(), 1);
    assert!(ignored[0].contains("ignored"), "{ignored:?}");
    // No stamp on either side: it plays, and says so.
    let no_start = limits(prepare(home(), &record, Stamps::Check));
    assert_eq!(no_start.len(), 1);
    assert!(no_start[0].contains("start has no stamp"), "{no_start:?}");
    record.made = None;
    let no_record = limits(prepare(home().made(made), &record, Stamps::Check));
    assert_eq!(no_record.len(), 1);
    assert!(
        no_record[0].contains("record has no stamp"),
        "{no_record:?}"
    );
}

#[test]
fn a_records_voice_manifest_reaches_the_load() {
    let found = Rc::new(RefCell::new(None));
    let seen = Rc::clone(&found);
    let start: Start<Toy> = Start::new(Card::text("Toy"), look, move |setup: Setup| {
        *seen.borrow_mut() = setup.voice_manifest.clone();
        Ok(toy::game_on(setup.ctx(toy::ctx())))
    });
    let mut record = SessionRecord::new();
    record.voice_manifest = Some("voices".to_owned());
    record.frames = vec![FrameRecord::default(), FrameRecord::default()];
    let (_, _, answers) = play(start, &record);
    assert_eq!(answers.len(), 2);
    assert_eq!(found.borrow().as_deref(), Some("voices"));
}

#[test]
fn a_session_plays_back_on_the_display_and_target_it_was_recorded_on() {
    let mut facts = PlatformFacts::default();
    "playdate".clone_into(&mut facts.target);
    let display = DisplayInfo::new(400, 240).one_bit();
    let (start, seen) = probe();
    let mut p = HeadlessPlatform::new()
        .with_facts(facts)
        .with_display(display);
    let (record, _) = record_on(start, &mut p, timed(4, &[]));
    let (start, again) = probe();
    let (_, _, answers) = play(start, &record);
    assert_same(&answers);
    assert_eq!(*seen.borrow(), *again.borrow());
    assert_eq!(again.borrow()[0].target, "playdate");
    assert_eq!(again.borrow()[0].display, display);
}

#[test]
fn the_pacing_a_game_was_handed_plays_back() {
    let mut pacing = Pacing::new(FrameCap::Fps(30), 2);
    pacing.fps = 29;
    let (start, seen) = probe();
    let mut app = App::new(start);
    assert!(app.record());
    let mut p = HeadlessPlatform::new();
    for now in [0, 16] {
        let _ = app.frame(&mut p, Millis(now));
    }
    // Reported while the game loads it would wait; here it runs. The next
    // report, the same, writes nothing; then one that differs does.
    app.set_pacing(pacing);
    let _ = app.frame(&mut p, Millis(32));
    app.set_pacing(pacing);
    let _ = app.frame(&mut p, Millis(48));
    let mut later = pacing;
    later.fps = 30;
    app.set_pacing(later);
    let _ = app.frame(&mut p, Millis(64));
    let record = app
        .take_session()
        .unwrap_or_else(|| panic!("not recording"));
    let written: Vec<Option<Pacing>> = record.frames.iter().map(|f| f.pacing).collect();
    assert_eq!(written, [None, None, Some(pacing), None, Some(later)]);
    let (start, again) = probe();
    let (_, _, answers) = play(start, &record);
    assert_same(&answers);
    assert_eq!(*seen.borrow(), *again.borrow());
    assert_eq!(again.borrow().last().and_then(|s| s.pacing), Some(later));
}

#[test]
fn system_events_and_dropped_input_are_recorded_and_played() {
    let (start, seen) = probe();
    let mut app = App::new(start);
    assert!(app.record());
    let mut p = HeadlessPlatform::new();
    p.push_input(&press(Key::F));
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    p.push_system(SystemEvent::Suspend);
    p.push_input(&press(Key::F));
    let _ = app.frame(&mut p, Millis(32));
    p.push_system(SystemEvent::Resume);
    p.push_input(&press_action(Action::Confirm));
    let _ = app.frame(&mut p, Millis(9000));
    let _ = app.frame(&mut p, Millis(9016));
    let record = app
        .take_session()
        .unwrap_or_else(|| panic!("not recording"));
    assert_eq!(
        record.frames[0].input,
        press(Key::F),
        "dropped, and kept here"
    );
    assert_eq!(record.frames[2].system, [SystemEvent::Suspend]);
    assert_eq!(record.frames[2].input, press(Key::F));
    assert_eq!(record.frames[3].system, [SystemEvent::Resume]);
    let (start, again) = probe();
    let (played, _, answers) = play(start, &record);
    assert_same(&answers);
    assert!(!played.is_suspended());
    assert_eq!(*seen.borrow(), *again.borrow());
    assert!(again.borrow()[0].actions.contains(&Action::Confirm));
}

/// A game that quits plays back to the same frame, and a game whose load
/// failed plays back its errors.
#[test]
fn quitting_and_a_failed_load_play_back() {
    let mut p = HeadlessPlatform::new();
    let frames = timed(5, &[(3, press_action(Action::Cancel))]);
    let (record, app) = record_on(home(), &mut p, frames);
    assert!(app.has_quit());
    let (played, _, answers) = play(home(), &record);
    assert_same(&answers);
    assert!(played.has_quit());
    assert_eq!(played.frame_number(), 5);

    let failing = || Start::<Toy>::new(Card::text("Toy"), look, |_| Err("no files".into()));
    let mut p = HeadlessPlatform::new();
    let (record, app) = record_on(failing(), &mut p, timed(3, &[]));
    assert_eq!(app.errors(), Some("no files"));
    let (played, _, answers) = play(failing(), &record);
    assert_same(&answers);
    assert_eq!(played.errors(), Some("no files"));
    let (_, _, answers) = play(
        Start::new(Card::text("Toy"), look, |_| Err("other".into())),
        &record,
    );
    assert_same(&answers[..1]);
    assert_eq!(answers[1].err().map(|d| d.frame), Some(2));
}

#[test]
fn an_hour_of_idle_frames_with_music_is_under_16_megabytes() {
    let mut record = SessionRecord::new();
    record.frames = (0..MAX_SESSION_FRAMES)
        .map(|i| FrameRecord {
            dt: 17,
            music: Some(f64::from(u32::try_from(i).unwrap_or(0)) * 0.016_666_666_666_666_666),
            ..FrameRecord::default()
        })
        .collect();
    let bytes = record.to_ron().len();
    assert!(bytes < 16 * 1024 * 1024, "{bytes} bytes");
}

#[test]
fn a_finished_playback_leaves_the_game_to_the_platform() {
    let mut p = HeadlessPlatform::new();
    let (record, _) = record_on(home(), &mut p, timed(3, &[]));
    let mut app = App::new(home());
    let mut platform = HeadlessPlatform::from_session(&record);
    let mut playback = Playback::new(&record);
    assert_eq!(playback.now(), Millis(0));
    assert_same(&play_out(&mut playback, &record, &mut app, &mut platform));
    assert_eq!(playback.now(), Millis(32));
    // A driver goes on from there, on its own input.
    platform.push_input(&press(Key::F));
    assert_eq!(app.frame(&mut platform, Millis(48)), FrameOutcome::Continue);
    assert_eq!(app.game().map(Game::screens), Some(vec!["home", "menu"]));
}

/// Files changed while recording: the first such frame is noted, since
/// their new bytes aren't in the record.
#[cfg(feature = "std")]
#[test]
fn a_reload_is_noted_at_its_first_frame_and_the_limits_warn_of_it() {
    let layer = shuyi_assets::bundle::Layer::owned("disk".to_owned(), BTreeMap::new());
    let layer = layer.unwrap_or_else(|error| panic!("{error:?}"));
    let mut p = HeadlessPlatform::new();
    let mut app = App::new(home());
    assert!(app.record());
    for now in [0, 16, 32] {
        let _ = app.frame(&mut p, Millis(now));
    }
    assert_eq!(app.session().and_then(|s| s.reloaded_at), None);
    p.change_file(layer, "a.ron", b"one".to_vec());
    let _ = app.frame(&mut p, Millis(48));
    p.change_file(layer, "a.ron", b"two".to_vec());
    let _ = app.frame(&mut p, Millis(64));
    let record = app
        .take_session()
        .unwrap_or_else(|| panic!("not recording"));
    let told = shuyi_ui::DEBUG_TOOLS.then_some(4);
    assert_eq!(record.reloaded_at, told);
    if told.is_some() {
        let limits = record.limits();
        assert!(limits.iter().any(|l| l.contains("frame 4")), "{limits:?}");
    }
}

#[test]
fn the_errors_say_what_they_hold() {
    let error = SessionError::Text("1:2: nope".to_owned()).to_string();
    assert!(error.contains("1:2: nope"), "{error}");
    let mismatch = stamp("a").check(&stamp("b"));
    let mismatch = mismatch.err().unwrap_or_else(|| panic!("the same"));
    assert_eq!(
        SessionError::Made(mismatch.clone()).to_string(),
        mismatch.to_string()
    );
}
