---
id: "3067"
title: "One clock for shuyi-app's frame: App::frame's time and the perf clock read the pacer's monotonic clock, not the system's time"
type: bug
milestone: After MVP
model: implementation
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-10-28
---

# 3067 — One clock for `shuyi-app`'s frame

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). `shuyi_app::run`'s loop reads the time from three places
each frame, and two of them are the system's time, which can jump (a
clock set by hand, a time sync):

- **The pacer:** `pacing::seconds()` (`crates/app/src/pacing.rs:291-307`).
  On native it is an `Instant`, counted from its first call. 2321 put it
  there on purpose; its completion notes say the pacer reads an
  `Instant` "since `miniquad::date::now` reads the system's time there,
  which can jump". `run` reads it at the top of each frame
  (`let start = pacing::seconds();`, `crates/app/src/lib.rs:363`).
- **`App::frame`'s time:** a few lines further down, `run` reads the
  clock again for the frame it hands `App`:
  `let now = Millis::from_secs_f64(miniquad::date::now());`
  (`lib.rs:375`). On native, `miniquad::date::now()` is
  `SystemTime::now()` since 1970 (miniquad 0.4.11, `src/lib.rs:72-79`).
  That `now` is everything `App` measures (`Clock::at`,
  `crates/platform/src/app.rs:278`): the screens' `dt`, the fixed steps
  due, and the time the audio player keeps (`Audio::music_playing` and
  `Audio::play` in `crates/app/src/backend.rs:44-56`, so the music's
  position and a voice clip's lateness, `audio/voice.rs:170`).
- **The perf clock:** `Macroquad::clock` gives `PerfClock::Real(now_us)`
  (`backend.rs:347-349`), and `now_us` (`backend.rs:404-410`) reads
  macroquad's `get_time()`, which is `miniquad::date::now()` minus the
  start (macroquad 0.4.16, `src/time.rs:35-39`): the system's time
  again. `PerfClock::Real` promises a clock "never going back"
  (`crates/ui/src/perf.rs:64-66`), and `docs/platform-backends.md:270`
  says the same of `shuyi-app`'s.

What a jump does today. `App` already counts a clock that goes back as
no time (`Millis::since` saturates; ADR-0024 §3), and `TickClock` owes
at most `MAX_STEPS` = 4 steps (`crates/platform/src/tick.rs:13`), so a
game never runs away. But:

- a jump forward is one frame whose `dt` is the whole jump: the screens'
  timers and animations skip ahead, the music's reported position leaps,
  and a voice clip still loading is given up as late;
- a jump back is a frame of no time, then the music's position (`now`
  minus when the track started) reads 0 until the clock catches up, and
  a voice clip counts as playing that much longer;
- a perf record's times can go backwards, against `PerfClock::Real`'s
  promise (`micros` in `app.rs:376` saturates them to 0).

`now_us` can't be tested today either: `get_time()` needs macroquad's
context, which a unit test has none of.

On the web all three are the same clock: `miniquad::date::now()` there
is the bundle's `Date.now() / 1e3` (`crates/cli/web/mq_js_bundle.js`),
and `pacing::seconds` reads it too. So the web is consistent today, if
not monotonic; see **Out**.

## Nick input

None.

## Scope

**In:**
- **One reading a frame.** `run` hands `App::frame` the reading it
  hands the pacer: `Millis::from_secs_f64(start)` in place of
  `Millis::from_secs_f64(miniquad::date::now())`. Where the clock counts
  from doesn't matter to `App`, which only takes differences
  (`Millis`'s docs, ADR-0024 §3).
- **The perf clock on the same clock.** `now_us` reads
  `pacing::seconds()` in place of `get_time()`, in whole microseconds as
  before.
- **Docs that name the clock:**
  - `pacing::seconds`'s doc comment says it is the loop's one clock (the
    pacer, `App::frame`'s time and the perf clock), an `Instant` on
    native and the page's `Date.now()` on the web;
  - `now_us`'s doc comment says it reads that clock;
  - `Millis::from_secs_f64`'s doc (`crates/platform/src/platform.rs:30-34`)
    stops naming `miniquad::date::now` as the clock a backend reads; it
    says a backend whose clock gives seconds as an `f64`, and that the
    clock should never go back;
  - `docs/platform-backends.md`: the `clock` section (line 270) says
    `shuyi-app`'s reads the same clock as its frame's time, not
    macroquad's `get_time`; and step 2 of *What one frame does* (the
    clock, line 129) gains one sentence: read the frame's time from a
    clock that never goes back, since the system's time jumps, and give
    `clock` that same one.
- **Either order with 3041:** if 3041 is done, check that its log
  line's seconds (`log_sink`, counted from `install`) read
  `pacing::seconds()`, and change them to it if they read
  `miniquad::date::now()`; if not, nothing to do (3041 reads
  `pacing::seconds()`, its clause says so).
- **Either order with 3043:** if 3043 is done, every call of
  `app.frame` in `run` (the frame its lifecycle plugin asks for, due or
  not, as well as the usual one) hands it the time from that frame's
  one `pacing::seconds()` reading; if not, nothing to do (3043 keeps to
  that one reading, its clause says so).
- **For the game:** no API change. `pacing` and `now_us` are private,
  `Millis` only changes its docs, and `shuyi_app::run` keeps its
  signature. The game gets steadier frame times at its next pin bump
  with no code change. No `Breaking:` line (ADR-0038 §6).

**Out (do not do):**
- **A monotonic clock on the web.** There `miniquad::date::now()` is
  `Date.now()`, which can jump too, and `pacing::seconds` reads it.
  `performance.now()` needs a JS plugin of our own (game ADR-0034, no
  `wasm-bindgen`). This ticket keeps the web on its one clock, as today;
  3068, blocked by this one, moves `pacing::seconds`'s web branch onto
  `performance.now()` through `clock.js`.
- **The audio's random seed** (`miniquad::date::now().to_bits()`,
  `backend.rs:170`): a seed should differ each launch, which the
  system's time does and an `Instant` from the start doesn't. It stays.
- `App`'s `Clock` and `TickClock` in `shuyi-platform`: they already
  count a clock going back as no time. No cap on `dt` here.
- 3042's focus handling: it restarts `App`'s clock on `FocusGained` as
  `Resume` does, inside `App`, and doesn't read the time in `run`.
  Nothing to settle.
- 2519's changes to `run` (`App::record` before the first frame): they
  don't read the clock. Its Context quotes the `miniquad::date::now()`
  line this ticket changes, as a description; no work follows from it.
- Other backends' clocks: 3010's Playdate timer, and parked 3034's
  replacement for macroquad, read their own.

## Implementation steps

1. `crates/app/src/lib.rs`, `run`: replace
   `let now = Millis::from_secs_f64(miniquad::date::now());` with
   `let now = Millis::from_secs_f64(start);`, with a comment that it is
   the pacer's reading. On the web path, `start` is read before the skip
   check, so a frame that runs counts from the last frame that ran, as
   today.
2. `crates/app/src/backend.rs`, `now_us`: read `crate::pacing::seconds()`
   in place of `get_time()`; keep the conversion through
   `Duration::try_from_secs_f64` and the `map_or(0, …)`. Update its doc
   comment.
3. `crates/app/src/pacing.rs`: update `seconds`'s doc comment as Scope
   says.
4. `crates/platform/src/platform.rs` and `docs/platform-backends.md`:
   the doc changes in Scope.
5. Settle the 3041 and 3043 clauses with what is done.
6. Run the gates (the `run-gates` skill).

## Acceptance criteria

- [x] `grep -rn "date::now\|get_time()" crates/app/src` finds only
      `pacing::seconds`'s web branch (`pacing.rs`) and the audio seed
      (`backend.rs`, `.to_bits()`).
- [x] In `run`, `pacing::seconds()` is read once before `app.frame`, and
      `app.frame` is handed that reading.
- [x] `now_us` reads `pacing::seconds()`, and the tests below pass.
- [x] `docs/platform-backends.md` no longer says `shuyi-app`'s clock
      reads `get_time`.
- [x] The public API listing doesn't change (CI's `api` job).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/app/src/backend.rs`, its `tests` module): `now_us` never
  goes back over two reads; and it is on the pacer's clock: with
  `a = pacing::seconds()`, `u = now_us()`, `b = pacing::seconds()` read
  in that order, `a * 1e6 <= u as f64 + 1.0` and
  `u as f64 <= b * 1e6 + 1.0`. These run without a window, which
  `get_time()` couldn't.
- The existing tests pass unchanged, among them
  `the_pacers_clock_never_goes_back` in `pacing.rs` and `App`'s tests in
  `crates/platform/src/app/tests.rs`.
- Manual (Windows, the `run` skill): hello-grid runs and steps as
  before; in a build with debug tools, run with `SHUYI_PERF=<path>` and
  the trace's frame times look as before (none negative, none huge).

## Completion notes

`run` hands `App::frame` the pacer's `pacing::seconds()` reading, and
`now_us` (the perf clock) reads `pacing::seconds()` too, so the frame's
time, the audio's time and the perf records all come from one clock that
never goes back on native. Docs updated (`pacing::seconds`, `now_us`,
`Millis::from_secs_f64`, `docs/platform-backends.md`). Two unit tests
for `now_us` run without a window. 3041, 3043 and 3068 were open, so
their clauses were rewritten as what now holds. The web stays on
`Date.now()` (3068 moves it). No deviations, no follow-ups, no game
change.

The manual hello-grid run with `SHUYI_PERF` was not done here (no window in this session); the unit tests cover the clock.

No break (no public API change).
