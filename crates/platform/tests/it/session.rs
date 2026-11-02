//! The Harness on a game's real `Start`: its clock and its first frames,
//! recording a run, and playing a record to any frame.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use proptest::prelude::*;
use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::image::ImageTable;
use shuyi_core::StateHash;
use shuyi_core::made::{ContentHash, Made};
use shuyi_platform::SystemEvent;
use shuyi_platform::app::{Card, Start};
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_platform::platform::Look;
use shuyi_platform::session::{FORMAT_VERSION, Played, SessionError, SessionRecord, Stamps};
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::input::{Chord, Key};
use shuyi_ui::screen::{FrameInput, Screen, Transition};
use shuyi_ui::toy::{self, Action, Home, Toy, ToyCtx};
use shuyi_ui::{Game, GameHooks, KeyPrompt, RawInputEvent};

/// An empty font and image table: the toy draws neither.
static FONT: FontAtlasDef = FontAtlasDef {
    cell_w: 0,
    cell_h: 0,
    columns: 0,
    glyphs: BTreeMap::new(),
};
static IMAGES: ImageTable = ImageTable::new(BTreeMap::new());

fn look(_: &Toy) -> Look<'_> {
    Look::new(&FONT, &IMAGES)
}

/// A start of a game at `first`, with no first press to wait for.
fn start_at(first: impl FnOnce() -> Box<dyn Screen<Toy>> + 'static) -> Start<Toy> {
    Start::new(Card::text("Toy"), look, move |setup| {
        Ok(Game::new(setup.ctx(toy::ctx()), first(), toy::hooks()))
    })
    .before_load(|setup| setup.key_prompt = KeyPrompt::Off)
}

/// The toy at its [`Home`], hashing the frames it has seen, and saving them
/// when the system suspends it.
fn home() -> Start<Toy> {
    start_at(|| Box::new(Home::default()))
        .state_hash(|game| {
            let home = game.screen::<Home>()?;
            Some(StateHash(u64::try_from(home.frames).ok()?))
        })
        .on_system(|game, event| {
            if event == SystemEvent::Suspend {
                let frames = game.screen::<Home>().map_or(0, |home| home.frames);
                let _ = game
                    .ctx_mut()
                    .storage
                    .computer_mut()
                    .write("frames", &frames.to_string());
            }
        })
}

/// What a [`Probe`] saw in each update.
#[derive(Debug, Clone, PartialEq, Default)]
struct Seen {
    actions: Vec<Action>,
    /// The frames' times, in seconds.
    dts: Vec<f32>,
}

/// A screen that declares only Confirm, notes what reaches it, and starts
/// the music on Confirm.
struct Probe(Rc<RefCell<Seen>>);

impl Screen<Toy> for Probe {
    fn name(&self) -> &'static str {
        "probe"
    }

    fn actions(&self) -> Option<Vec<Action>> {
        Some(vec![Action::Confirm])
    }

    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        if input.actions.contains(&Action::Confirm) {
            ctx.audio.play_music("theme");
        }
        let mut seen = self.0.borrow_mut();
        seen.actions.extend_from_slice(&input.actions);
        seen.dts.push(input.dt());
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}
}

/// `millis` as a screen sees it.
fn seconds(millis: u64) -> f32 {
    Duration::from_millis(millis).as_secs_f32()
}

fn probe() -> (Start<Toy>, Rc<RefCell<Seen>>) {
    let seen = Rc::new(RefCell::new(Seen::default()));
    let screen = Rc::clone(&seen);
    (start_at(move || Box::new(Probe(screen))), seen)
}

/// Unwraps a Harness's start, which isn't `Debug`.
fn started<E: core::fmt::Display>(harness: Result<Harness<Toy>, E>) -> Harness<Toy> {
    harness.unwrap_or_else(|error| panic!("{error}"))
}

fn recording(start: Start<Toy>) -> Harness<Toy> {
    started(Harness::start_recording(start))
}

fn record_of(h: &Harness<Toy>) -> SessionRecord {
    h.session()
        .cloned()
        .unwrap_or_else(|| panic!("not recording"))
}

fn playing(start: Start<Toy>, record: &SessionRecord) -> Harness<Toy> {
    started(Harness::from_session(start, record, Stamps::Check))
}

/// What the toy's store holds at the end of its run.
fn store(h: Harness<Toy>) -> BTreeMap<String, String> {
    let storage = h.into_storage();
    let keys = storage.list().unwrap_or_default();
    keys.into_iter()
        .map(|key| {
            let value = storage.read(&key).ok().flatten().unwrap_or_default();
            (key, value)
        })
        .collect()
}

#[test]
fn frames_of_frame_dt_come_out_as_16_17_and_17_ms() {
    let mut h = recording(home());
    assert_eq!(h.frame_count(), 2, "the card and the load");
    for _ in 0..7 {
        h.frame(&[], FRAME_DT);
    }
    let record = record_of(&h);
    let dts: Vec<i64> = record.frames.iter().map(|f| f.dt).collect();
    assert_eq!(dts, [0, 16, 17, 17, 16, 17, 17, 16, 17]);
    assert_eq!(record.start, 0);
    assert!(!record.truncated, "a Harness from a start keeps recording");
}

#[test]
fn frames_shorter_than_a_millisecond_add_up() {
    let mut h = recording(home());
    for _ in 0..10 {
        h.frame(&[], 0.000_4);
    }
    let dts: Vec<i64> = record_of(&h).frames[2..].iter().map(|f| f.dt).collect();
    let total: i64 = dts.iter().sum();
    assert_eq!(total, 4, "from 16 ms to 20: {dts:?}");
    // Nonsense times pass no time, as on `frame_after`.
    h.frame(&[], f32::NAN).frame(&[], -1.0);
    let frames = record_of(&h).frames;
    assert_eq!(frames[frames.len() - 2].dt, 0);
    assert_eq!(frames[frames.len() - 1].dt, 0);
}

#[test]
fn a_harness_from_a_start_has_run_the_card_and_the_load() {
    let h = started(Harness::start(home()));
    assert_eq!(h.frame_count(), 2);
    assert!(!h.app().is_loading());
    assert_eq!(h.screens(), ["home"]);
    let log: Vec<(u32, Vec<&str>)> = vec![(0, vec![]), (2, vec!["home"])];
    assert_eq!(h.screen_log(), log);
    assert_eq!(h.session(), None, "not recording");
    assert_eq!(h.audio_log().len(), 0);
    assert_eq!(h.quit_frame(), None);
}

#[test]
fn a_load_that_fails_is_the_err_with_its_errors() {
    let failing = || Start::<Toy>::new(Card::text("Toy"), look, |_| Err("no files".into()));
    assert_eq!(Harness::start(failing()).err().as_deref(), Some("no files"));
    let recorded = Harness::start_recording(failing()).err();
    assert_eq!(recorded.as_deref(), Some("no files"));
}

#[test]
fn only_a_harness_on_a_running_game_holds_back_an_undeclared_action() {
    let cancel = RawInputEvent::Down(Chord::plain(Key::Escape));

    let (start, seen) = probe();
    let mut h = started(Harness::start(start));
    h.frame(&[cancel], FRAME_DT);
    assert_eq!(seen.borrow().actions, [Action::Cancel]);
    assert_eq!(h.undeclared(), []);

    let held = Rc::new(RefCell::new(Seen::default()));
    let game = Game::new(
        toy::ctx(),
        Box::new(Probe(Rc::clone(&held))),
        GameHooks::default(),
    );
    let mut h = Harness::new(game);
    h.frame(&[cancel], FRAME_DT);
    assert_eq!(held.borrow().actions.len(), 0);
    assert_eq!(h.undeclared(), [("probe", Action::Cancel)]);
}

#[test]
fn a_game_that_quits_says_so_at_its_frame() {
    let mut h = recording(home());
    h.keys("Escape");
    assert_eq!(h.quit_frame(), Some(3));
    let record = record_of(&h);
    let mut played = playing(home(), &record);
    assert_eq!(played.play_all().first_difference, None);
    assert_eq!(played.quit_frame(), Some(3));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Random keys, pad buttons, waits and a suspend and a resume: played
    /// back, no frame differs, and the game ends on the same screens with
    /// the same store.
    #[test]
    fn a_recorded_session_plays_back_the_same(
        ops in proptest::collection::vec((0u8..8, 0u16..=100), 0..40),
        suspend in 0usize..40,
        resume in 0usize..40,
    ) {
        let mut h = recording(home());
        for (i, &(op, millis)) in ops.iter().enumerate() {
            if i == suspend {
                h.system(SystemEvent::Suspend);
            }
            if i == resume {
                h.system(SystemEvent::Resume);
            }
            match op {
                0 => { h.keys("f"); }
                1 => { h.keys("Escape"); }
                2 => { h.keys("Up"); }
                3 => { h.pad("South"); }
                4 => { h.pad("DpadDown"); }
                _ => { h.frame(&[], f32::from(millis) / 1000.0); }
            }
        }
        let record = record_of(&h);
        prop_assert!(!record.truncated);
        let mut played = playing(home(), &record);
        let answer = played.play_all().clone();
        prop_assert_eq!(answer.first_difference, None);
        prop_assert_eq!(answer.frames, answer.of);
        prop_assert_eq!(answer.frames, h.frame_count());
        prop_assert_eq!(played.frame_count(), h.frame_count());
        prop_assert_eq!(played.screens(), h.screens());
        prop_assert_eq!(played.screen_log(), h.screen_log());
        prop_assert_eq!(played.quit_frame(), h.quit_frame());
        prop_assert_eq!(store(played), store(h));
    }
}

#[test]
fn a_press_taken_out_is_found_at_its_frame_and_the_game_stays_there() {
    let mut h = recording(home());
    h.wait(FRAME_DT).keys("f").wait(FRAME_DT * 3.0);
    assert_eq!(h.screens(), ["home", "menu"]);
    let mut record = record_of(&h);
    let at = record
        .frames
        .iter()
        .position(|f| {
            f.input
                .iter()
                .any(|e| matches!(e, RawInputEvent::Down(chord) if chord.key == Key::F))
        })
        .unwrap_or_else(|| panic!("no press"));
    let frame = u32::try_from(at + 1).unwrap_or(0);
    record.frames[at].input.clear();

    let mut played = playing(home(), &record);
    let answer = played.play_all().clone();
    let difference = answer
        .first_difference
        .unwrap_or_else(|| panic!("the same"));
    assert_eq!(difference.frame, frame);
    assert_ne!(difference.recorded, difference.played);
    assert_eq!(answer.frames, frame);
    assert_eq!(answer.of, h.frame_count());
    assert_eq!(played.frame_count(), frame, "left on that frame");
    assert_eq!(played.screens(), ["home"]);
    // Asked again, it stays.
    assert_eq!(played.play_all().frames, frame);
    assert_eq!(played.frame_count(), frame);
}

#[test]
fn play_to_stops_at_its_frame_and_at_the_records_end() {
    let mut h = recording(home());
    h.keys("f");
    let record = record_of(&h);
    let of = h.frame_count();
    let mut played = playing(home(), &record);
    assert_eq!(played.frame_count(), 0, "it plays nothing yet");
    assert_eq!(played.play_to(0).frames, 0);
    assert_eq!(played.play_to(3).frames, 3);
    assert_eq!(played.frame_count(), 3);
    assert_eq!(played.screens(), ["home", "menu"]);
    assert_eq!(played.play_to(2).frames, 3, "it doesn't go back");
    let answer = played.play_to(of + 100).clone();
    assert_eq!((answer.frames, answer.of), (of, of));
    assert_eq!(played.frame_count(), of);
    assert_eq!(played.play_all().frames, of, "nothing more to play");
    assert_eq!(played.frame_count(), of);
}

#[test]
fn play_to_before_the_load_leaves_the_harness_usable() {
    let mut h = recording(home());
    h.keys("f");
    let record = record_of(&h);
    let mut played = playing(home(), &record);
    assert_eq!(played.play_to(1).frames, 1);
    assert!(played.app().is_loading());
    assert_eq!(played.app().frame_number(), 1);
    assert_eq!(played.screen_log(), [(0, vec![])]);
    assert_eq!(played.screens(), Vec::<&str>::new());
    assert_eq!(played.top_screen(), "");
    assert_eq!(played.audio_log().len(), 0);
    // And it goes on from there.
    assert_eq!(played.play_to(2).first_difference, None);
    assert_eq!(played.screen_log(), [(0, vec![]), (2, vec!["home"])]);
}

#[test]
fn the_logs_fill_as_a_record_plays_as_they_did_as_it_was_recorded() {
    let mut h = recording(home());
    // Confirm opens the menu; Confirm there clicks.
    h.keys("f f");
    assert_eq!(h.sounds(), ["click"]);
    let record = record_of(&h);
    let mut played = playing(home(), &record);
    played.play_all();
    assert_eq!(played.sounds(), h.sounds());
    let frames = |h: &Harness<Toy>| h.audio_log().iter().map(|(f, _)| *f).collect::<Vec<_>>();
    assert_eq!(frames(&played), frames(&h));
    assert_eq!(played.screen_log(), h.screen_log());
}

/// What `from_session` makes of `record`, on `start`.
fn limits(start: Start<Toy>, record: &SessionRecord, stamps: Stamps) -> Vec<String> {
    let mut h = started(Harness::from_session(start, record, stamps));
    h.play_all().limits.clone()
}

#[test]
fn played_limits_say_what_the_playing_cant_promise() {
    let made = Made::new("demo 1.0", ContentHash(0x1a2b));
    let stamped = || home().made(made.clone());
    let mut h = recording(stamped());
    h.keys("f");
    let record = record_of(&h);
    assert_eq!(record.made.as_ref(), Some(&made));
    assert!(record.storage_complete);
    assert_eq!(
        limits(stamped(), &record, Stamps::Check),
        Vec::<String>::new()
    );

    let has = |lines: Vec<String>, part: &str| {
        assert!(lines.iter().any(|l| l.contains(part)), "{part}: {lines:?}");
    };
    has(limits(stamped(), &record, Stamps::Ignore), "ignored");
    has(limits(home(), &record, Stamps::Check), "start has no stamp");
    let mut truncated = record.clone();
    truncated.truncated = true;
    has(
        limits(stamped(), &truncated, Stamps::Check),
        "stopped early",
    );
    let mut incomplete = record.clone();
    incomplete.storage_complete = false;
    has(limits(stamped(), &incomplete, Stamps::Check), "the store");
}

#[test]
fn another_build_and_another_version_are_refused() {
    let made = Made::new("demo 1.0", ContentHash(0x1a2b));
    let mut h = recording(home().made(made));
    h.keys("f");
    let mut record = record_of(&h);
    let other = home().made(Made::new("demo 2.0", ContentHash(0x1a2b)));
    let refused = Harness::from_session(other, &record, Stamps::Check).err();
    assert!(
        matches!(refused, Some(SessionError::Made(_))),
        "{refused:?}"
    );
    record.format_version = FORMAT_VERSION + 1;
    let refused = Harness::from_session(home(), &record, Stamps::Ignore).err();
    assert_eq!(refused, Some(SessionError::Version(FORMAT_VERSION + 1)));
}

#[test]
fn scripts_run_on_after_the_records_last_frame() {
    let (start, seen) = probe();
    let mut h = recording(start);
    h.keys("f");
    let record = record_of(&h);
    let end: i64 = record.frames.iter().map(|f| f.dt).sum();
    assert!(end > 0);

    let (start, again) = probe();
    let mut played = playing(start, &record);
    let answer = played.play_all().clone();
    assert_eq!(answer.first_difference, None);
    assert!(
        !answer.limits.iter().any(|l| l.contains("speaker")),
        "no script has run: {:?}",
        answer.limits
    );
    assert_eq!(*seen.borrow(), *again.borrow());

    played.keys("f");
    assert_eq!(played.frame_count(), record_frames(&record) + 2);
    // The clock ran on: a frame's whole length since the record's last, then
    // the next, not the time from 0.
    let dts = again.borrow().dts.clone();
    assert_eq!(dts[dts.len() - 2..], [seconds(16), seconds(17)]);
    assert_eq!(again.borrow().actions, [Action::Confirm, Action::Confirm]);
    let answer = played.play_all();
    assert_eq!(answer.frames, answer.of, "the record's, not the script's");
    assert!(
        answer.limits.iter().any(|l| l.contains("headless speaker")),
        "{:?}",
        answer.limits
    );
    assert!(!answer.limits.iter().any(|l| l.contains("didn't play")));
    // The music is the speaker's own now, and sounds.
    assert!(played.music_clock().is_some());
}

fn record_frames(record: &SessionRecord) -> u32 {
    u32::try_from(record.frames.len()).unwrap_or(0)
}

#[test]
fn a_clock_at_its_end_goes_on_from_the_record() {
    let (start, _) = probe();
    let mut h = recording(start);
    h.keys("f");
    let mut record = record_of(&h);
    // Too near its end for its microseconds to fit in a `u64`.
    record.start = u64::MAX - 1000;
    let (start, again) = probe();
    let mut played = playing(start, &record);
    assert_eq!(played.play_all().first_difference, None);
    played.keys("f");
    let dts = again.borrow().dts.clone();
    assert_eq!(dts[dts.len() - 2..], [seconds(16), seconds(17)]);
}

#[test]
fn a_script_before_the_records_end_takes_over_and_says_so() {
    let mut h = recording(home());
    h.keys("f f");
    let record = record_of(&h);
    let mut played = playing(home(), &record);
    played.play_to(3);
    played.keys("Up");
    let answer: Played = played.play_all().clone();
    assert_eq!(answer.frames, 3);
    assert_eq!(played.frame_count(), 5);
    let left = record_frames(&record) - 3;
    let line = format!("{left} of its frames didn't play");
    assert!(
        answer.limits.iter().any(|l| l.contains(&line)),
        "{answer:?}"
    );
    // Once only.
    played.keys("Up");
    assert_eq!(played.play_all().limits.len(), answer.limits.len());
}

#[test]
fn a_harness_with_no_record_plays_nothing() {
    let mut h = Harness::new(toy::game());
    assert_eq!(h.play_all(), &Played::default());
    assert_eq!(h.frame_count(), 0);
    assert_eq!(h.session(), None);
}
