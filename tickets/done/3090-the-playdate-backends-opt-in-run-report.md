---
id: "3090"
title: "The Playdate backend's opt-in run report: when the loading card and the first frame were shown, the slowest reads, a minute's frame and stack figures, and the draw's phases"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: ["3010", "3028"]
nick_input: none
completed: 2026-11-04
---

# 3090 — The Playdate backend's opt-in run report

## Context

A request from the game: [game#1170](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1170-the-playdate-front-end-on-the-engines-backend.md)
moves the game onto `shuyi-playdate` and waits only for this. The
game/engine boundary audit of 2026-11-03 split it out of 3011, which needs
a device and is research; this part needs neither, as 3011's own test plan
said.

- **The job:** a shipped Playdate game says in its console, on every
  run, how its start went and how fast it runs, so a tester's log shows a
  slow start or a stutter without a bench build.
- **Friction numbers** (the game's `docs/engine/friction.md`, sweep
  1153): the Playdate front end, 100 lines estimated, 1,735 measured,
  17×; outcome engine (3003, 3010 and 3011 held the rest). The game's
  copy is `crates/playdate/src/report.rs` (228 production lines) and
  `src/pd/bench.rs` (84), driven from `src/pd/entry.rs`.
- **Generic shape:** the backend measures and reports; the game only
  turns it on and may name steps of its own load. No game concept.
- **Not tactics:** a crank fishing game wants the same lines from its
  testers: "card at 180 ms, first frame at 2,400 ms, slowest file
  `lake.map` 900 ms, 29.8 frames a second on `Lake` this minute".

## Nick input

None.

## Scope

**In** (all in `crates/playdate`, off unless a game turns it on):
- **The start:** when the loading card was shown and when the first frame
  of the running game reached the screen (ms after `init`), the load's
  length, the stack's high-water mark after it (`stack_high_water()`,
  3028; "measured on the device only" off it).
- **The reads:** a `shuyi_assets::bundle::watch` watcher timing each read
  to the next one: a count of reads and files, the paths the bundle lacks,
  and the N slowest. `report::mark(name)` lets a loader name a step that
  reads no file, so its time is its own. If a watcher is already set
  (`watch` gives it back), the report says so and goes on without reads.
- **Each minute:** the top screen (`Game::top_screen`), frames a second,
  min/avg/max of `App::frame` and of the draw and flush, the stack mark,
  and the music: the cue heard and whether its file player ran out of data
  (a new `Sdk::file_did_underrun`, the SDK's `didUnderrun`).
- **The draw's phases**, a second switch for measuring builds: the
  backend's one-bit draw through `onebit::render_marked`, avg/max per
  `onebit::Phase`, in the minute lines.
- **A report file**, if the game names one: the start lines and every read
  written to the data folder once the first frame is shown (the Simulator
  has no console to read).
- Lines go to `Sdk::log` as the driver writes App's lines, level `info`,
  category `platform` (3040). Off, nothing is measured or logged.
- New public types are `#[non_exhaustive]` with constructors (ADR-0039).
  The two `Sdk` methods have default bodies, so the change is additive.
  The game's PR that moves the pin follows this one (game#1170).

**Out (do not do):**
- Device runs, `--bench`, `--run --device` and calibration: 3011, which
  is blocked by this ticket and uses this report for its bench build.
- Rendering each frame again with the reference renderer to compare it,
  and pausing the music every other minute: the game's own bench
  experiments for its done tickets 1055 and 1047. The engine's
  `frame/onebit/speed_tests.rs` proves the two renderers draw the same.
- A log file or a sink of the other backends (3041).

## Implementation steps

1. Add `crates/playdate/src/report.rs`: `RunReport` (builder: `file`,
   `every_seconds` default 60, `slowest` default 10, `phases`), `Timing`
   (min/avg/max/count), the reads table and `mark`, and the line text;
   numbers grouped by thousands.
2. `Driver::with_report(start, sdk, RunReport)`; `Driver::new` keeps it
   off. In `update` and `after_frame`, note the card, the end of loading
   and the first changed frame of the running game, time `App::frame` and
   the draw with `Sdk::clock_us`, and log the minute.
3. Add `Sdk::stack_used()` (default `None`; the device answers
   `stack_high_water()`) and `Sdk::file_did_underrun` (default `false`;
   the device through a new `shim/shim.c` call of the file player's
   `didUnderrun`), with settable values on `fake.rs`.
4. `main!(start, report = expr)` and `main!(start, stack_bytes = n,
   report = expr)` arms in both `main!` macros; `device::init` takes it.
5. `docs/platform-backends.md`, Playdate: the report, its lines, its cost.

Expected size: about 300 production lines, 350 test lines.

## Acceptance criteria

- [x] A toy game on the fake SDK and clock, with the report on, logs the
      card and first-frame times, the load time, the stack line, the
      slowest reads with a named step, and a minute's lines on the
      minute, through public API only (`report_lines_on_the_fake`).
- [x] With the report off, a five-minute run on the fake logs no report
      line and never reads `clock_us` for it.
- [x] Phases on, the minute lines give every `onebit::Phase`; phases off,
      none.
- [x] A named report file holds the start lines and every read.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Timing`, the reads table (repeats, missing files, steps, ties by
  path), grouping, the clock's wrap across a minute.
- Integration (`driver/tests.rs`, fake SDK and clock): on, off, phases,
  a second watcher refused, the file, the underrun line.

## Completion notes

Done. `crates/playdate/src/report.rs` holds `RunReport` (builder: `file`,
`every_seconds`, `slowest`, `phases`), `Timing`, the reads table and
`report::mark`, and the lines; `Driver::with_report` turns it on, and
`main!(start, report = …)` / `main!(start, stack_bytes = n, report = …)`
pass it to `device::init`, which adds the stack's size for the stack line.
`Sdk` gained `stack_used` and `file_did_underrun`, both with default bodies;
the device answers them with `stack_high_water()` and a new `shim.c` call of
the file player's `didUnderrun`. The speaker keeps a list of its open file
players so the minute can ask each without touching a closed one.

Deviations:

- Times are whole microseconds from `Sdk::clock_us`, not floats as in the
  game's copy, and shown in milliseconds to a tenth.
- The bundle's watcher is process-wide and a plain function, so the reads
  live outside the driver: a static on the device, one per thread in the
  tests. A second report in one process reuses the watcher it set; a
  watcher set by anything else first is refused and the report says so.
- `Timing`'s line says `1,800 times` where the game's said `n = 1,800`: the
  engine's text lint reads a lone letter as a key.
- The docs are in `docs/targets/playdate.md`, *The run report*, the home the
  docs map gives the Playdate backend, not `docs/platform-backends.md`,
  which is about every backend.
- The off test compares the timer's reads of a five-minute run through the
  driver with the same frames on `App` and `Playdate` alone: the app reads
  the timer itself in a debug build, so "never" means "no more than it".

No break: every change to the public API is an addition.

Book: chapter 9 (`targets-and-export.md`) names `report = RunReport::new()`
in `main!` and links the Playdate's page.

Follow-ups: none. The game's pin moves with game#1170
(`Zafnok/visions-of-shuyi`), which deletes its own `report.rs` and
`pd/bench.rs` for this; 3011's bench build turns it on with phases.

Nick: nothing to decide.

