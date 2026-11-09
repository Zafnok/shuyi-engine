//! A load in steps ([`Start::in_steps`]): the frames run as many steps as
//! the platform's slice holds, the card on the screen, timed by a clock a
//! test moves. Each test has a clock of its own, as the tests run at once.

use core::sync::atomic::{AtomicU64, Ordering};

use super::*;

/// A second, the Playdate's slice, in microseconds.
const SECOND_US: u64 = 1_000_000;

/// Moves `clock` on by `us`.
fn advance(clock: &AtomicU64, us: u64) {
    clock.fetch_add(us, Ordering::Relaxed);
}

/// A load of `count` steps, each moving `clock` on by `us`, the last
/// making the toy's game.
fn steps_of(count: u32, us: u64, clock: &'static AtomicU64) -> impl FnOnce(Setup) -> LoadStep<Toy> {
    move |setup| {
        let mut ctx = Some(setup.ctx(toy::ctx()));
        let mut left = count;
        Box::new(move || {
            advance(clock, us);
            left -= 1;
            if left > 0 {
                return Progress::More;
            }
            let game = ctx.take().map(toy::game_on);
            Progress::done(game.ok_or_else(|| "loaded twice".to_owned()))
        })
    }
}

/// The toy, loading in `count` steps of `us` each on `clock`.
fn in_steps(count: u32, us: u64, clock: &'static AtomicU64) -> Start<Toy> {
    Start::in_steps(
        Card::text("A toy"),
        |toy: &Toy| toy.look(),
        steps_of(count, us, clock),
    )
}

/// A slice of a second on the clock `now`.
fn a_second(now: fn() -> u64) -> LoadSlice {
    LoadSlice::new(SECOND_US, PerfClock::Real(now))
}

static THIRTY: AtomicU64 = AtomicU64::new(0);

fn thirty_now() -> u64 {
    THIRTY.load(Ordering::Relaxed)
}

/// The ticket's load: 30 steps of 100 ms on a slice of a second.
#[test]
fn a_load_in_steps_runs_a_slice_a_frame_behind_the_card() {
    let mut app = App::new(in_steps(30, 100_000, &THIRTY));
    let mut p = HeadlessPlatform::new().with_load_slice(a_second(thirty_now));
    let _ = app.frame(&mut p, Millis(0));
    assert_eq!(app.load_steps(), 0, "the card's frame runs none");
    let mut frames = Vec::new();
    for n in 1..=10 {
        let began = thirty_now();
        let _ = app.frame(&mut p, Millis(33 * n));
        frames.push((thirty_now() - began, app.load_steps()));
        if !app.is_loading() {
            break;
        }
        assert_eq!(p.last_frame(), Some(&Presented::Card(Card::text("A toy"))));
    }
    // Three frames of ten steps, none past the slice; then the game.
    let second = (SECOND_US, 10);
    assert_eq!(frames, [second, (SECOND_US, 20), (SECOND_US, 30)]);
    assert_eq!(running(&app).screens(), ["home"]);
    assert!(matches!(p.last_frame(), Some(Presented::Game { .. })));
}

static ONCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn without_a_slice_every_step_runs_in_one_frame() {
    let mut app = App::new(in_steps(30, 100_000, &ONCE));
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    assert_eq!(app.load_steps(), 30);
    assert_eq!(running(&app).screens(), ["home"]);
}

static SLOW: AtomicU64 = AtomicU64::new(0);

fn slow_now() -> u64 {
    SLOW.load(Ordering::Relaxed)
}

#[test]
fn a_step_longer_than_the_slice_runs_alone_in_its_frame() {
    let mut app = App::new(in_steps(3, 3 * SECOND_US, &SLOW));
    let mut p = HeadlessPlatform::new().with_load_slice(a_second(slow_now));
    let _ = app.frame(&mut p, Millis(0));
    for (n, steps) in [(1, 1), (2, 2)] {
        let _ = app.frame(&mut p, Millis(33 * n));
        assert_eq!((app.is_loading(), app.load_steps()), (true, steps));
    }
    let _ = app.frame(&mut p, Millis(99));
    assert_eq!((app.is_loading(), app.load_steps()), (false, 3));
}

#[test]
fn a_load_in_steps_that_fails_shows_why() {
    let start: Start<Toy> = Start::in_steps(
        Card::text("A toy"),
        |toy: &Toy| toy.look(),
        |_| {
            let mut steps = 0;
            move || {
                steps += 1;
                if steps < 3 {
                    Progress::More
                } else {
                    Progress::done(Err("data/stage.ron: missing".to_owned()))
                }
            }
        },
    );
    let mut app = App::new(start);
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    assert_eq!(app.errors(), Some("data/stage.ron: missing"));
    assert_eq!(app.load_steps(), 3);
    let errors = Presented::Errors("data/stage.ron: missing".into());
    assert_eq!(p.last_frame(), Some(&errors));
}

static BEFORE: AtomicU64 = AtomicU64::new(0);

fn before_now() -> u64 {
    BEFORE.load(Ordering::Relaxed)
}

#[test]
fn the_setup_is_given_once_and_after_the_backends_part() {
    let start = in_steps(30, 100_000, &BEFORE).before_load(|setup| {
        setup.key_prompt = KeyPrompt::Off;
    });
    let mut saved = MemoryStorage::new();
    let _ = saved.write("from-before", "yes");
    let mut app = App::new(start);
    let mut p = HeadlessPlatform::new()
        .with_storage(Box::new(saved))
        .with_load_slice(a_second(before_now));
    for n in 0..4 {
        let _ = app.frame(&mut p, Millis(1000 + 33 * n));
    }
    let ctx = running(&app).ctx();
    assert_eq!(ctx.key_prompt, KeyPrompt::Off);
    assert_eq!(
        ctx.music_seed,
        mix(1033),
        "the seed of the first step's frame"
    );
    assert_eq!(
        ctx.storage.computer().read("from-before"),
        Ok(Some("yes".into()))
    );
}

static TIMED: AtomicU64 = AtomicU64::new(0);

fn timed_now() -> u64 {
    TIMED.load(Ordering::Relaxed)
}

#[test]
fn the_load_is_timed_over_its_frames() {
    let mut app = App::new(in_steps(30, 100_000, &TIMED));
    let mut p = clocked().with_load_slice(a_second(timed_now));
    for n in 0..3 {
        let _ = app.frame(&mut p, Millis(33 * n));
    }
    // The fake clock reads 100 and 300 around the first slice, then 600
    // and 1000 around the second, and 1500 and 2100 around the last.
    assert_eq!(app.load_us(), Some(200 + 400));
    let _ = app.frame(&mut p, Millis(99));
    assert!(!app.is_loading());
    assert_eq!(app.load_us(), Some(200 + 400 + 600));
}

static RECORDED: AtomicU64 = AtomicU64::new(0);

fn recorded_now() -> u64 {
    RECORDED.load(Ordering::Relaxed)
}

/// Where a slice ends depends on the clock, which a playback doesn't
/// repeat: a recorded run loads in one frame, and plays back the same.
#[test]
fn a_recorded_or_hashed_run_takes_no_slice() {
    let mut app = App::new(in_steps(30, 100_000, &RECORDED));
    assert!(app.record());
    let mut p = HeadlessPlatform::new().with_load_slice(a_second(recorded_now));
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    assert_eq!((app.is_loading(), app.load_steps()), (false, 30));
    let mut app = App::new(in_steps(30, 100_000, &RECORDED));
    app.hash_frames(true);
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    assert_eq!((app.is_loading(), app.load_steps()), (false, 30));
}

#[test]
fn a_load_at_once_is_one_step() {
    let mut app = App::new(Start::new(
        Card::text("A toy"),
        |toy: &Toy| toy.look(),
        |setup| Ok(toy::game_on(setup.ctx(toy::ctx()))),
    ));
    let mut p = HeadlessPlatform::new().with_load_slice(LoadSlice::new(0, PerfClock::fake()));
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    assert_eq!((app.is_loading(), app.load_steps()), (false, 1));
}
