//! A load whose steps wait for the platform (`Progress::Wait`, ADR-0128
//! §4): its frames and its card on the headless backend, which has no
//! slice, and a run recorded where it waits played back where nothing
//! waits.

use std::cell::Cell;

use shuyi_core::StateHash;
use shuyi_platform::app::{Card, Progress, Start};
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_platform::headless::Presented;
use shuyi_ui::KeyPrompt;
use shuyi_ui::toy::{self, Home, Toy};

use super::session::{look, playing, record_of, recording, started};

thread_local! {
    /// How many times the second file isn't there yet when a step asks for
    /// it: the page still fetching it. Each test sets its own, on its
    /// thread.
    static NOT_YET: Cell<u32> = const { Cell::new(0) };
}

/// The toy's load in steps: a unit a file decoded, two files, the second
/// not there until it has been asked for [`NOT_YET`] times; then the game,
/// at its home, its frames in its state hash.
fn waiting() -> Start<Toy> {
    let card = Card::text("Toy");
    Start::in_steps(card, look, |setup| {
        let mut ctx = Some(setup.ctx(toy::ctx()));
        let mut decoded = 0;
        move || {
            if decoded == 1 && NOT_YET.get() > 0 {
                NOT_YET.set(NOT_YET.get() - 1);
                return Progress::Wait;
            }
            if decoded < 2 {
                decoded += 1;
                return Progress::Of {
                    done: decoded,
                    of: 2,
                };
            }
            let game = ctx.take().map(toy::game_on);
            Progress::done(game.ok_or_else(|| "loaded twice".to_owned()))
        }
    })
    .before_load(|setup| setup.key_prompt = KeyPrompt::Off)
    .state_hash(|game| {
        let home = game.screen::<Home>()?;
        Some(StateHash(u64::try_from(home.frames).ok()?))
    })
}

#[test]
fn a_load_that_waits_twice_takes_three_frames_the_card_on_each() {
    NOT_YET.set(2);
    // The card's frame, then the load's first: a unit, and a wait.
    let mut h = started(Harness::start(waiting()));
    let card = Presented::Card(Card::text("Toy"));
    for frame in [2, 3] {
        assert_eq!(h.frame_count(), frame);
        assert!(h.app().is_loading(), "frame {frame}");
        assert_eq!(h.platform().last_frame(), Some(&card));
        assert_eq!(h.platform().card_count(), Some((1, 2)));
        h.frame(&[], FRAME_DT);
    }
    assert!(!h.app().is_loading());
    assert_eq!(h.screen_log(), [(0, vec![]), (4, vec!["home"])]);
    assert_eq!(h.app().load_steps(), 5, "two units, two waits, the game");
    assert_eq!(h.app().load_progress(), Some((2, 2)));
    assert_eq!(h.platform().card_count(), None, "the game's frame");
}

#[test]
fn a_run_recorded_with_waits_plays_back_where_nothing_waits() {
    NOT_YET.set(2);
    let mut h = recording(waiting());
    h.frame(&[], FRAME_DT).frame(&[], FRAME_DT);
    assert!(!h.app().is_loading());
    h.keys("f").wait(0.25);
    assert_eq!(h.screens(), ["home", "menu"]);
    let record = record_of(&h);
    // The card's frame, the load's three as one, then the game's.
    let frames = u32::try_from(record.frames.len()).unwrap_or(u32::MAX);
    assert_eq!(frames, h.frame_count() - 2);
    // On the headless backend the file is there at once.
    assert_eq!(NOT_YET.get(), 0);
    let mut played = playing(waiting(), &record);
    let answer = played.play_all().clone();
    assert_eq!(answer.first_difference, None, "{answer:?}");
    assert_eq!((answer.frames, answer.of), (frames, frames));
    assert_eq!(played.app().load_steps(), 3, "no wait");
    assert_eq!(played.screens(), ["home", "menu"]);
    assert_eq!(played.screen_log()[1], (2, vec!["home"]));
}
