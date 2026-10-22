---
id: "3040"
title: "One log for the engine and the game: lines with a level, a category and a frame, App keeps the last 500, and the contract's run and scene answers carry them"
type: feature
milestone: MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3040 — A log with levels and categories, kept by App and read through the contract

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Each of them has one log with levels, and most add
categories:

- Bevy's `LogPlugin` sends every message through `tracing`, with a level
  and a filter per module
  (`docs.rs/bevy/latest/bevy/log/struct.LogPlugin.html`);
- Godot writes `user://logs/` by default, and a game can add its own
  `Logger` (`godotengine.org/releases/4.5`);
- Unity writes `Player.log` for every build
  (`docs.unity3d.com/Manual/LogFiles.html`);
- Unreal has log categories, each with its own verbosity
  (`dev.epicgames.com/documentation/en-us/unreal-engine/logging-in-unreal-engine`).

What Shuyi has today:

- **Strings with no level.** `Ctx` gathers what went wrong while it read
  saved data in `warnings: Vec<String>` (`crates/ui/src/ctx.rs:232`): a
  voice manifest it can't use (line 454), key bindings it repaired
  (lines 594-608), settings it couldn't read (line 752) and the profiles'
  warnings (line 774). `Ctx::take_warnings` (line 614) hands them over
  with no level and no category.
- **Taken once.** `App` copies them after the game's load
  (`crates/platform/src/app.rs:488`), and `App::take_warnings` (line 557)
  hands them to the backend. A line given later, and every line in a
  Harness (which starts with `App::running` and never loads), reaches
  nobody.
- **macroquad's macros.** `shuyi-app` prints App's warnings with `warn!`
  (`crates/app/src/lib.rs:167-169`), and its own problems the same way
  (`backend.rs:47, 165, 194`, `render.rs:86, 266, 295`,
  `pads/native.rs:43`, `storage/mod.rs:31, 58-59`).
- **An undefined `log()`.** `docs/platform-backends.md:54` shows a
  backend's loop calling `log(&warning)`, which nothing defines.
- **No log in the contract.** `run`'s answer (`RunOutput`) carries the
  screens, the scene, the frame, the events and the sounds, and
  `scene`'s (`SceneOutput`) the scene; neither carries a line of a log
  (`crates/cli/schemas/run.output.json`, `scene.output.json`). ADR-0005
  names logs among what agents need, and 1133 (the MCP server, in the
  MVP list of `docs/ROADMAP.md`) promises that agents "read the scene
  and the logs".

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md), "a
second comparison"): the small protections go in the MVP, among them
"logs and error codes agents can read". This ticket is the logs; 2509 is
the error codes.

Why it matters. An agent working on Visions of Shuyi through the
contract, and soon through MCP (1133), can see that a run went wrong,
but not why: a settings file it couldn't read, a voice manifest it
refused. The same holds for every game made with the engine, and for the
end goal of 2026-10-19 (an agent makes a whole game in under an hour):
an agent fixes what it can read.

This ticket gives the engine's lines a shape and a home, and hands them
to agents. The shipped build's sinks (a log file, the panic message, the
browser console, a filter from the environment) are 3041, after the MVP.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill, the next free number) for the
  engine's log:
  1. a log line is plain data: a level, a category, the frame and the
     text. It is kept per `App`, never in a global, so two Harnesses on
     two test threads never see each other's lines, and each line knows
     its frame;
  2. the core never logs (`shuyi-core` has no log), and `shuyi-assets`
     returns errors and never logs. `shuyi-ui` logs through `Ctx`, and
     `shuyi-platform` through `App`;
  3. a log is output only: no engine or game code reads it to decide
     anything. Tests, tools and backends read it;
  4. a log line is not player text: it is English, never translated, and
     `check-text` lets it through;
  5. a backend hands App's lines to its own sinks. `shuyi-app` does it
     through the `log` crate (3041): MIT OR Apache-2.0, `no_std`, already
     in `Cargo.lock` through gilrs, which writes to it, and the crate
     macroquad's `log-rs` feature turns macroquad's own macros into
     (without that feature they go through miniquad's own printer). The
     `tracing` crate stays out, as in 2325: its spans are 2325's job,
     and it brings a larger dependency tree. A global facade of our own
     was weighed and dropped, for point 1.
- **`shuyi_ui::log`**, a new module (`crates/ui/src/log.rs`, its tests
  in `crates/ui/src/log/tests.rs`), `no_std` + `alloc`:
  - `LogLevel`: `Error`, `Warn`, `Info`, `Debug`, ordered from the most
    serious (`Error < Debug` by `Ord`), with `name()` (`"error"`,
    `"warn"`, `"info"`, `"debug"`) and `from_name(&str) -> Option<Self>`;
  - `LogCategory`: `Assets`, `Audio`, `Input`, `Screens`, `Storage`,
    `Platform`, `Game`, with `name()` (`"assets"` to `"game"`) and
    `from_name(&str) -> Option<Self>`. `Game` is for a game's own lines;
  - `LogLine { frame: u32, level: LogLevel, category: LogCategory,
    text: String }`;
  - `LOG_LINES: usize = 500`;
  - `LogRing` (a `VecDeque` from `alloc`): `new(capacity)`;
    `push(line)`, which drops the oldest line and counts it when full;
    `lines()`, the kept lines oldest first; `dropped() -> u32` and
    `add_dropped(n)`; a sequence number per line (`u64`, from 0, never
    reused), with `next_seq()` and `since(seq)`, the kept lines whose
    number is `seq` or more, so a reader asks for the lines after the
    last one it took; `take() -> (Vec<LogLine>, u32)`, every kept line
    and the dropped count, leaving it empty with 0 dropped;
  - `LogFilter`: a level for every category (default `Info`), and a
    level per category that overrides it; `allows(category, level)`.
    Reading a filter from text is 3041's.
  - The two enums are `#[non_exhaustive]`, as `Card` is. **Either order
    with 2013:** if it is done, follow its rules for public types; if
    not, 2013 covers these with the rest.
- **`Ctx` logs:**
  - `Ctx::log(&mut self, level: LogLevel, category: LogCategory, text:
    impl Into<String>)` keeps the line, with `frame` 0, until something
    takes it, in a `LogRing` of `LOG_LINES`: the oldest dropped first
    and counted;
  - `Ctx::take_log(&mut self) -> (Vec<LogLine>, u32)`: the waiting lines
    and how many were dropped;
  - the `warnings` field goes. Each place that pushed to it calls `log`:
    the voice manifest at `Warn`, `Audio`; key bindings at `Warn`,
    `Input`; settings and profiles at `Warn`, `Storage`. The text stays
    as it is today;
  - `Ctx::take_warnings` stays for the game, whose tests and own
    Playdate front end call it (checked with `gh search code` on
    2026-10-20): it takes every waiting line and returns the text of
    those at `Warn` or `Error`, as today.
- **`App` keeps the log:**
  - a frame counter, `App::frame_number()`: 0 before the first frame,
    then one more for each call to `frame` or `frame_after`, so it equals
    the Harness's `frame_count`;
  - a `LogRing` of `LOG_LINES`, filtered by `App::set_log_filter`
    (default: `Info` and up);
  - at the end of every frame (a suspended one too), and once in
    `App::running`, it takes the game's waiting lines (`Ctx::take_log`)
    and stamps them with `frame_number()`, so lines from before the
    first frame carry 0. It adds the count Ctx dropped to the ring's.
    The take of the game's warnings after its load
    (`crates/platform/src/app.rs:488`) and the `warnings` field go;
  - its own line: a failed load logs its errors once, at `Error`,
    category `Platform`;
  - `App::log(&self) -> &LogRing` (every kept line) and
    `App::take_new_log(&mut self) -> Vec<LogLine>` (the ring's lines
    `since` the sequence number App kept at the last call, for a
    backend's sinks). `App::take_warnings` goes.
- **The Harness:** `Harness::log()` returns the App's ring.
- **The contract:**
  - `LogEntry { frame: u32, level: String, category: String, text:
    String }` in `crates/cli/src/contract/session.rs`, with the derives
    and `deny_unknown_fields` that `Sound` (line 83) has; the level and
    category are the names above;
  - `Session` (line 92) gains `log(&self) -> Vec<LogEntry>` and
    `log_dropped(&self) -> u32`, with default bodies (no lines, 0) so a
    game's own `Session` keeps building. `Headless` writes them over
    `Harness::log`, and `FakeSession` (`crates/cli/src/contract/fake.rs:24`,
    its `impl Session` at line 52) over two new fields, `log` and
    `log_dropped`, empty in `FakeSession::on`;
  - `RunOutput` and `SceneOutput` (`crates/cli/src/contract/run.rs:36`
    and `:58`) gain `log: Vec<LogEntry>`;
  - when lines were dropped, `limits` (`run.rs:129`) adds "the log keeps
    the last 500 lines: N earlier line(s) were dropped";
  - `run::text` (line 278) prints the `Warn` and `Error` lines after the
    events, and `scene_output_text` (line 318) after its first line;
  - the schemas regenerated, `contract::VERSION` raised by one (it is 4
    at `crates/cli/src/contract.rs:52` today), and the line in
    `crates/cli/schemas/versions.json`, as `docs/contract.md`, *The
    schemas*, says. 2509 (problem codes) already raised the version for
    its own fields, so this raises it once more from there.
- **`check-text`:** `log` joins `QUIET` (`crates/cli/src/check_text.rs:186`),
  so a literal inside `ctx.log(…)` is never counted (`QUIET` grows to 15
  names). The `// check-text: not player text` markers at `ctx.rs:453`
  and `:751`, above literals that now sit inside `self.log(…)`, go.
  Those at lines 126, 599 and 747 mark literals outside a `log` call
  and stay.
- **`shuyi-app` prints App's lines as before:** its loop
  (`crates/app/src/lib.rs:167-169`) prints `app.take_new_log()` after
  each frame with macroquad's macro for the line's level (`error!`,
  `warn!`, `info!`, `debug!`), as `"{category}: {text}"`. The `error!`
  in `Macroquad::draw_errors` (`backend.rs:163-166`) moves to where
  `broken` is set (`backend.rs:149`), so a failed load is logged once, by
  App, and a renderer that can't start once, by the backend; the
  `logged` field goes. The backend's other lines stay as they are until
  3041.
- **Docs:**
  - `docs/platform-backends.md`: the loop (line 54) hands each line of
    `app.take_new_log()` to the device's log; under `present` (line 119),
    App logs a failed load, so a backend needn't;
  - `docs/contract.md`: the `run` and `scene` rows say they answer with
    the log, and a short *The log* section says what a line holds, that
    frames count from 1 with 0 for before the first frame, and the limit
    of 500 lines; and *The answer*'s example (a `scene` answer) gains
    `"log": []` in its `result` and the new version in `contract` (2017
    adds a test that parses that example as an `Answer<SceneOutput>`).
    **Either order with 2017:** if it is done, its test in
    `crates/cli/tests/it/docs.rs` fails until the example has `log`; if
    not, the example is right when 2017 starts.
- **Size:** if the diff, fixtures and snapshots aside, passes about 600
  changed lines, `LogFilter` and `App::set_log_filter` move to 3041,
  which reads a filter from text anyway. App then keeps every level
  until 3041, and the PR adds them to 3041's scope and says so.

**Out (do not do):**
- the shipped build's sinks: a log file, the panic message, the browser
  console, `SHUYI_LOG` and reading a filter from text (3041);
- moving `shuyi-app`'s own lines (render, audio, pads, storage) off
  macroquad's macros (3041);
- a filter in the contract's input (`run --log debug`): a follow-up if
  agents need `Debug` lines;
- the MCP server's tools (1133 passes the field on);
- problem codes and help lines on the contract's errors (2509);
- timing spans and the frame profiler (2325, parked);
- a log channel in the debug view (a follow-up once 2315's channels
  exist), crash reports (3046) and session records (2518);
- the log in the debug console (2356) and read from a running windowed
  build (2521);
- a Playdate sink (3010 sends App's lines to the SDK's console).

## Implementation steps

1. Read ADR-0005, ADR-0022 and ADR-0024; `crates/ui/src/ctx.rs` and
   `crates/ui/src/profiles.rs`; `crates/platform/src/app.rs`, its tests
   and `harness.rs`; `crates/cli/src/contract/run.rs`, `session.rs` and
   `fake.rs`; `crates/cli/src/check_text.rs`. Check whether 2013, 2017
   and 2509 are done.
2. Write the ADR.
3. Write `shuyi_ui::log` (`pub mod log;` in `crates/ui/src/lib.rs`), its
   unit tests and the `LogRing` property test.
4. Move `Ctx`'s warnings onto `Ctx::log` and add `Ctx::take_log`,
   keeping `take_warnings`'s tests passing unchanged. Remove the two
   check-text markers named above.
5. App's frame counter, ring, filter, `log` and `take_new_log`, its
   failed-load line, and `Harness::log`. Move App's tests of
   `take_warnings` (`crates/platform/src/app/tests.rs:400` and
   `:628-633`) onto `log` and `take_new_log`.
6. The contract: `LogEntry`, the session's methods, the answers, the
   text and the limit. Run `cargo shuyi schema --out crates/cli/schemas`,
   raise `contract::VERSION` and add the line the schemas test asks for
   to `versions.json`.
7. `check-text`'s `QUIET`, `shuyi-app`'s loop and `draw_errors`, and the
   two docs.
8. Run the gates. Put the `rg` output and the API change in the PR
   description.

## Acceptance criteria

- [ ] A Harness over the toy, whose storage holds `nonsense` under the
      first profile's settings key (as
      `unreadable_settings_give_the_defaults_and_a_warning` in
      `crates/ui/src/ctx/tests.rs` sets it up), has one line in
      `Harness::log()`: frame 0, `Warn`, `Storage`, its text starting
      with `settings: unreadable: `.
- [ ] A line logged through `harness.ctx_mut().log(…)` after frame 3 and
      before frame 4 carries frame 4.
- [ ] The ring keeps the last 500 of 600 lines and counts 100 dropped.
- [ ] `App::take_new_log` returns each line once, in order, across calls.
- [ ] With the filter at `Warn`, an `Info` line never enters the ring.
- [ ] A failed load leaves one `Error` line, category `Platform`, with its
      errors.
- [ ] `Session::log` of a `Headless` session over that Harness returns
      the line as a `LogEntry` with `frame` 0, `level` `"warn"` and
      `category` `"storage"`.
- [ ] `run`'s and `scene`'s answers over a `FakeSession` with two lines
      carry both in `log`, and each answer's JSON reads back as the same
      `Answer<RunOutput>` or `Answer<SceneOutput>`. With `log_dropped`
      at 3, the limits hold "the log keeps the last 500 lines: 3 earlier
      line(s) were dropped".
- [ ] The schemas tests (`crates/cli/tests/it/schemas.rs`) pass with the
      new version and its fingerprint in `versions.json`.
- [ ] `docs/contract.md`'s example answer has `"log": []` in `result`
      and the new `contract::VERSION` in `contract`.
- [ ] `check-text` counts no literal inside `ctx.log(…)` (a unit test in
      `check_text.rs`).
- [ ] `Ctx` has no `warnings` field, and the existing tests of
      `Ctx::take_warnings` pass unchanged.
- [ ] Outside tests, only `App`, the Harness, the contract and backends
      call `App::log`, `App::take_new_log` or `Ctx::take_log` (the `rg`
      output in the PR description).
- [ ] `shuyi-ui` and `shuyi-platform` still build for
      `thumbv7em-none-eabihf` (CI's `no-std` job).
- [ ] The PR says the API change for the game's pin bump:
      `App::take_warnings` is gone (`App::take_new_log` replaces it);
      `Ctx::take_warnings` stays.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `LogRing` (order, capacity, the dropped count, `take`, sequence
  numbers after it wraps) and `LogFilter::allows`, and the names of
  levels and categories both ways (`crates/ui/src/log/tests.rs`);
  `Ctx::log`, `take_log` and `take_warnings`
  (`crates/ui/src/ctx/tests.rs`); App's frame stamps, the filter,
  `take_new_log` and the failed load's line
  (`crates/platform/src/app/tests.rs`); the Harness's frame stamps and
  the toy's settings line (`crates/platform/tests/it/main.rs`); `QUIET`
  and a literal
  inside `ctx.log(…)` (`crates/cli/src/check_text.rs`).
- Property (`proptest`, already a dev-dependency of `shuyi-ui`): for any
  sequence of `push` and `since` reads on a `LogRing` of small
  capacity, a reader that keeps `next_seq()` after each read gets every
  line still kept exactly once, in the order pushed.
- Integration: the `Headless` toy session's log
  (`crates/cli/src/contract/session/tests.rs`, `toy` module); `run`'s
  and `scene`'s answers with a `FakeSession` that has lines, and the
  dropped limit (`crates/cli/src/contract/run/tests.rs`); the
  regenerated schemas.

## Completion notes

*(Filled in by the session that completes the ticket.)*
