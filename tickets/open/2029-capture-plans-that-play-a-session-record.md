---
id: "2029"
title: "Capture plans that play a session record: the capture mode replays a recorded session and compares the window with the raster at the frames a plan names"
type: infra
milestone: After MVP
model: reasoning
effort: medium
status: todo
blocked_by: ["2018", "2518", "2519"]
nick_input: none
completed:
---

# 2029 — Capture plans that play a session record

## Context

2018 promised this ticket: "plans that play a session record (2518) in
place of steps". The 2026-10-24 backlog audit found it unwritten.

What is there once the blockers land:

- **2018's capture mode** (`crates/app/src/capture.rs`, behind
  `shuyi-app`'s `capture` feature) runs a plan
  (`shuyi_platform::capture::Plan`) of steps: keys, pad buttons, waits
  and captures. They are `capture::Step`s (`keys`, `pad`, `wait`,
  `capture`), or, when 2566 landed first, `capture::PlanStep`s over
  `shuyi_platform::script::Step` (2566 folds them).
  `Plan` is a struct with private fields, made only by `Plan::parse`
  and read through `steps()` and `tolerance()`; `Plan::frames()` gives
  a `PlannedFrame` per frame. The run drives `App::frame_after(platform,
  capture::FRAME_DT)`, so every run is the same. On each capture it
  reads the window with `get_screen_data()` (`readback::window`, the
  live link's read too) and compares it with the raster (2332) under
  the plan's `Tolerance`. The `render` CI job runs it under Xvfb.
- **2518's session record** (`shuyi_platform::session`) holds what the
  platform handed a game: input, system events, clock readings, music
  positions, keyboard answers, the store and the display. `Playback`
  plays one through `App::frame` at the recorded times, from the game's
  real `Start`, on a `HeadlessPlatform` built with `from_session`, and
  compares each frame's hash with the record's.
- **2519** moves Hello, Grid's start into its library
  (`hello_grid::start()`, stamped with `Start::made`) and keeps a golden
  session, `examples/hello-grid/tests/sessions/stage.session.ron` (the
  title, the options screen, the stage, a few moves). A test plays it,
  and `SHUYI_BLESS=1` blesses it.

The gap: a session record plays only headless. The window never shows
one. A plan of steps is written by hand and can't hold what a real
session holds: frame times, music positions, a keyboard answer, saved
settings. A recorded session reaches screens and looks that no
hand-written plan reaches, and nothing checks that the window draws
them as the raster does.

Turning a record into steps is lossy (2518's Out says so). So the
capture mode plays the record itself, as `Playback` does.

**Why 2519 is a blocker too.** The record must come from Hello, Grid's
real `Start`, which lives in its binary until 2519 moves it into the
library. And 2519's golden session is the record a plan needs, kept
current by its bless. A second record made here would be a second file
to bless.

Why it matters:

- **The game first.** A session recorded in a debug build of Visions of
  Shuyi becomes a check of the real window, not only of the headless
  frames.
- **Other 2D games** on `shuyi-app` get a window check from any record
  they keep, with a plan of a few lines.
- **The end goal** (2026-10-19): what an agent sees (the raster) must
  be what players see (the window), on frames players really reach.

## Nick input

None.

## Scope

**In:**
- **The plan's session form** (`shuyi_platform::capture`, behind the
  `capture` feature):
  - `Plan` gains a private field `session: Option<SessionPlan>` and a
    reader `Plan::session() -> Option<&SessionPlan>`. It is additive:
    2018 made `Plan`'s fields private, so only `Plan::parse` builds
    one (ADR-0039). The new
    `SessionPlan { file: String, captures: Vec<SessionCapture> }` and
    `SessionCapture { frame: u32, name: String }` are
    `#[non_exhaustive]` too, with `new` constructors;
  - in RON:

    ```ron
    (
        session: (
            file: "../tests/sessions/stage.session.ron",
            captures: [
                (frame: 40, name: "title"),
                (frame: 160, name: "options"),
                (frame: 420, name: "stage"),
            ],
        ),
    )
    ```

    `steps` and `session` are both `#[serde(default)]`, and `tolerance`
    works as in a step plan;
  - `Plan::parse` refuses a plan with steps and a session, or with
    neither; a session with no captures or an empty `file`; frame
    numbers that don't strictly increase (a repeated frame, or one out
    of order); and capture names that step captures can't have (the
    same characters, no repeats). Each error names the capture by its
    number and its text, as a step's error does;
  - a capture's frame is the number `Playback::next` answers and
    `Harness::play_to` takes: `frame: 40` is the picture frame 40
    presents;
  - `SessionPlan::marks(&self, frames: usize) ->
    Result<Vec<Option<&str>>, String>`: one entry per frame of a record
    of `frames` frames, the capture's name on the frames it names. A
    capture past the record's end is an error naming it and the
    record's length ("capture 3 (stage) is at frame 900, but the record
    has 812 frames");
  - `Plan::frames` gives no frames for a session plan, and its docs say
    so;
  - it stays `no_std` + `alloc`, so the `no-std` job's line for
    `--features capture` still builds it.
- **Either order with 2566:** if 2566 is done, the step form's steps
  are `capture::PlanStep`s over `script::Step` (2566 folded 2018's
  `capture::Step` into them), and `Plan::frames` takes the keymap; the session form doesn't change, and `Plan::frames`
  still gives no frames for a session plan. If not, nothing to do (2566
  moves the step form's steps onto `script::Step` and leaves the
  session form alone).
- **Playback on a platform other than `HeadlessPlatform`**
  (`shuyi_platform::session`). If 2518's `Playback::next` takes
  `&mut HeadlessPlatform` only, give it a trait instead:
  `session::Replays: Platform` with `fn push_frame(&mut self, frame:
  &FrameRecord)`. `HeadlessPlatform` implements it with its own
  `push_frame`. The Harness's calls don't change.
- **The start check outside the Harness.** If the stamp check and the
  voice manifest of `Harness::from_session` live only in the Harness
  (behind `harness`), move them to one public function both call, for
  example `session::prepare<G: GameData>(start: Start<G>, record:
  &SessionRecord, stamps: Stamps) -> Result<(Start<G>, Vec<String>),
  SessionError>`. It runs 2111's `Made::check` between the record's
  stamp and the start's (2111 is done), applies the
  record's voice manifest with `Start::before_load`, and gives the
  lines `Played::limits` would give (no stamp, a truncated record, an
  incomplete store).
- **The capture mode plays a session plan**
  (`crates/app/src/capture.rs`):
  - **the record:** read from `file`, resolved against the plan's
    folder by a pure helper (an absolute path is kept as it is), with
    `macroquad::file::load_string` as the plan is, and parsed with
    `SessionRecord::from_ron`. A missing file or a parse error exits 2,
    naming the file. So does an error from `SessionPlan::marks`;
  - **the stamp:** `session::prepare(start, &record, Stamps::Check)`,
    or the check 2518 left public. A refused stamp exits 2 with 2111's
    `Mismatch` text. A plan never ignores stamps;
  - **the platform:** a private `SessionWindow` holding
    `HeadlessPlatform::from_session(&record)` and the window drawing of
    2018's capture mode. `present` draws, and captures on the frames
    `marks` names, as for a step plan. It never switches the window to
    full screen, whatever the frame asks (a record may hold the
    shortcut). Every other `Platform` method is the headless
    platform's: the recorded display, input, system events, store (a
    `MemoryStorage`), music positions and keyboard. Real keys,
    controllers and focus are never read. It implements `Replays` by
    passing each frame to the headless platform;
  - **the loop:** `App::new` on the prepared start, then one
    `Playback::next(&mut app, &mut platform)` per macroquad frame, at
    the recorded times (never `frame_after`), to the record's end;
  - **a differing frame** stops the run with exit 2. Its line, in
    `capture.txt` and on stderr, reads `frame <n> differs: recorded
    <hex>, played <hex>`. The captures before it stay written;
  - the close button stops the run with exit 2: a quit can't be handed
    to the game without changing its frames. A capture on a frame that
    isn't the game's (the card or the errors) exits 2, as for a step
    plan;
  - **what it writes:** the files and `capture.txt` lines of a step
    plan, each line also giving the capture's frame. The lines from
    `prepare` go first, each starting `limit: `;
  - **exit codes** as 2018's: 0 when every capture passes, 1 when one
    fails, 2 when the plan can't run;
  - on wasm32 a session plan is refused with an error, logged as a
    plan that doesn't parse is.
- **No recording in a capture run.** 2519's `run` records a session in
  debug builds. A capture run, of either form, doesn't: the pure
  decision in `crates/app/src/sessions.rs` (whether to record) gains
  the capture case, with a test. A capture run reads and writes nothing
  outside `SHUYI_CAPTURE_OUT`, as 2018 says, so nothing lands under
  `<data dir>/sessions/`.
- **Hello, Grid:**
  - `examples/hello-grid/capture/session.ron`, naming the golden
    session (`../tests/sessions/stage.session.ron`) and three captures,
    each named after the screen on top at its frame (the last of
    `Game::screens`): the title, the options screen, and the stage
    after a move. Pick the frames by playing the record with
    `Harness::play_to`;
  - a test beside the golden session's test parses the plan with
    `Plan::parse`, plays the record with
    `Harness::from_session(hello_grid::start(), &record,
    Stamps::Check)` to each capture's frame, and checks that the top
    screen's name is the capture's name. When the golden session is
    recorded again and its frames move, this test says which capture
    to move. Hello, Grid's dev-dependency on itself turns on its
    `capture` feature (2018) for `Plan::parse`.
- **The `render` job** (2018), under `xvfb-run` like its other steps:
  - runs hello-grid (`--features capture`) with
    `SHUYI_CAPTURE=examples/hello-grid/capture/session.ron` twice, into
    `target/capture/hello-grid-session` and
    `target/capture/hello-grid-session-again`, and checks with `cmp`
    that the two runs' `.window.png` and `.raster.png` files are equal;
  - copies the record into `$RUNNER_TEMP` with its first written frame
    hash changed (`sed`), and the plan with `file` pointing at the
    copy. It checks that hello-grid exits 2 on that plan and that its
    `capture.txt` has a `differs` line;
  - writes the new `capture.txt` files into the step summary like the
    others; the artifact upload already takes `target/capture/`;
  - before merging, re-run `render` three times on the PR. If the
    session steps don't pass every time with the same `capture.txt`,
    they move to a job of their own outside `ci-result`, the PR says
    why, and a ticket (`write-ticket`) makes them stable. A gate never
    starts flaky (`AGENTS.md` rule 6).
- **Docs** (the `document` skill):
  - beside 2018's how-to in `shuyi-app`'s docs: the session form, with
    Hello, Grid's plan as the example; and that a stale record (another
    stamp, or a differing frame) exits 2 until it is blessed or
    recorded again (2519);
  - `docs/platform-backends.md`'s *Session records* (2518) gains a
    line: a capture plan can play a record in the real window;
  - the `run-gates` skill's line on `render` gains the session plan's
    command.
- **No new ADR.** The session form keeps 2018's ADR (a plan is a test
  input of the backend, not an asset kind) and 2518's (a record is a
  run's log, not an asset kind, and plays through `App::frame` at its
  recorded times).
- **For the game:** `Plan` gains a private field and a reader,
  additively. If
  `Playback::next` becomes generic and CI's `api` job calls that a
  break, the completion notes get a `Breaking:` line (ADR-0038), and
  the game's PR that moves the pin follows. The game uses neither
  today.

**Out (do not do):**
- The web build: 2019 checks it with step plans. A session plan on
  wasm32 is refused.
- Recording sessions in real builds, `shuyi run --session`, MCP, and
  blessing the golden session: 2519, done first.
- Turning a record into key and wait steps: lossy (2518's Out).
- Ignoring stamps in a plan (`Stamps::Ignore`): a stale record is
  blessed or recorded again.
- Captures of a range of frames (every N frames): a plan names single
  frames.
- New session records: the plan uses 2519's golden session. A game
  adds its own records and plans; the templates get none.
- The render card: it has no game, so no record.
- A hash or a snapshot of the window's picture: each capture is
  compared with the raster of the same frame (`AGENTS.md` rule 9).

## Implementation steps

1. Read 2018's ADR, `shuyi_platform::capture`,
   `crates/app/src/capture.rs` and the `render` job; 2518's ADR,
   `shuyi_platform::session` (`Playback`,
   `HeadlessPlatform::from_session`) and `Harness::from_session`;
   2519's `crates/app/src/sessions.rs`, `hello_grid::start()` and the
   golden session's test.
2. Add the session form to `Plan`, and `SessionPlan::marks`, with their
   unit and property tests.
3. If 2518 didn't already, make `Playback::next` take a `Replays`
   platform and move the start check to `session::prepare`; the
   Harness calls both.
4. Write `SessionWindow` and the session loop in
   `crates/app/src/capture.rs`, the exit codes and the `capture.txt`
   lines, and the capture case of the recording decision. Run Hello,
   Grid's plan on your own desktop.
5. Pick the plan's frames with `Harness::play_to`, and write
   `examples/hello-grid/capture/session.ron` and its test.
6. Add the `render` job's steps, and re-run the job three times.
7. Write the docs, and run the gates.

Size: about 500 changed lines. The plan's form and `marks` with their
tests about 200, `Playback` and `prepare` about 60, the capture mode
about 150, Hello, Grid's plan and test about 50, the job about 30, the
docs about 30.

## Acceptance criteria

- [ ] Under Xvfb on the CI runner, hello-grid with
      `SHUYI_CAPTURE=examples/hello-grid/capture/session.ron` plays the
      golden session to its end, writes its three captures and
      `capture.txt`, and exits 0.
- [ ] Two runs of the session plan write byte-identical `.window.png`
      and `.raster.png` files (`cmp` in the job).
- [ ] A copy of the record with one frame hash changed makes hello-grid
      exit 2, and its `capture.txt` names the frame and both hashes
      (the job).
- [ ] `Plan::parse` refuses a plan with steps and a session, or with
      neither; a session with no captures, an empty `file`, frames that
      don't strictly increase, or a bad or repeated name. Each error
      names the capture.
- [ ] `SessionPlan::marks` marks exactly the named frames; a capture
      past the record's end is an error naming the capture and the
      record's length.
- [ ] A record whose stamp differs from the start's is refused with
      2111's `Mismatch` text, by the function `Harness::from_session`
      uses too.
- [ ] `Playback` plays a toy record on a test platform that wraps a
      `HeadlessPlatform` with no differing frame.
- [ ] A capture run, of either form, records no session (a test of the
      decision in `sessions.rs`).
- [ ] Hello, Grid's test finds each capture of `session.ron` on the
      screen it is named after.
- [ ] `cargo build -p shuyi-platform --no-default-features --features
      capture --target thumbv7em-none-eabihf --locked` passes, in CI's
      `no-std` job.
- [ ] `render` passed three re-runs with the same `capture.txt`, or the
      PR says why its session steps are outside `ci-result`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Plan::parse` of the session form (a good plan, and each
  error); `SessionPlan::marks` (the marks, a capture past the end, an
  empty record); the record's path against the plan's folder (relative,
  with `..`, absolute); the `capture.txt` line of a differing frame;
  the recording decision with a capture plan.
- Property: for random session plans and record lengths at or past the
  last capture, `marks` has one entry per frame and one name per
  capture.
- Integration: `Playback` through a wrapping platform on the toy game
  (`shuyi_ui::toy`); the stamp refusal; Hello, Grid's captures on their
  screens; the `render` job (CI).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
