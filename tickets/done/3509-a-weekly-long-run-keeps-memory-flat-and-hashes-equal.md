---
id: "3509"
title: A weekly long run checks that memory and live sounds stay flat over hours of frames and that a second run's hashes match
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-24
---

# 3509 — A weekly long run checks that memory and live sounds stay flat and that a second run's hashes match

## Context

The engine's blind-spot audit of 2026-11-23 found no long-session test.
Everything that measures a game today is short:

- The `bench` job (`.github/workflows/ci.yml`, about lines 1007-1078;
  ticket 3008, ADR-0081) plays Hello, Grid's eight scenarios
  (`examples/hello-grid/bench/*.bench.ron`) under Callgrind. Each is a
  few seconds of play. `bench-play` (`crates/cli/src/bench.rs`, `play`)
  reads the frames from the session's perf ring, which keeps the last
  `PERF_FRAMES` = 600 (`crates/ui/src/perf.rs`), and refuses a longer
  scenario.
- 3221 (open) pins the allocations of one steady frame. 3319 (open)
  compares the bench's instructions with main's.
- The `playtest` workflow (ticket 2021) plays each stage to a result.
- 2527 (open) is `shuyi soak`: a bot's random legal commands with a
  save and reload at every step, over the `TurnRules` state alone, with
  no frames, screens or sounds.

Nothing runs a game's frames for hours and asks whether the heap stays
flat, whether the sounds it keeps alive stay few, and whether the run
is still deterministic at the end. Slow leaks only show there: a cache
that grows with every map entered, a sound handle never released, an
atlas page added at run time (2222), a log that keeps every line. The
real-time games next on the roadmap (2739, 1147) run longer sessions
than a turn game, and players and store checks (Steam Deck now,
consoles later) find a leak first.

What exists to build on, checked on main at `74736aa8`:

- `shuyi_alloc_count::global::reading()` gives the program's
  `live_bytes`, `peak_bytes` and `allocs`, and `global::rows()` the same
  per memory tag (ADR-0046 §6). `crates/cli/src/contract/session.rs`
  reads them as `process_heap()` and `process_snapshot()`. The bench's
  tool program is built in the `shuyi-bench` profile, which counts the
  heap.
- `Session` (same file, about line 414) has `play(script)`,
  `idle(frames)`, `sounds_added()` and `heap_snapshot()`. It has no
  frame hash. `App::frame_hash` and `App::hash_frames`
  (`crates/platform/src/app.rs`, about line 1419) hash each frame
  without recording a session.
- `Harness::sounds_playing()` (`crates/platform/src/harness.rs`, about
  line 737) lists the sounds alive now.
- **A trap:** `Harness::note_frame` (about line 664) pushes every
  frame's audio requests onto `self.audio`, and the music commands onto
  `self.music` and `self.music_frames`. Over a million frames that is
  a leak of the test driver's own, and a long run that measured it
  would always fail. A session record (`Harness::start_recording`)
  grows each frame too.
- `security.yml` is the weekly pattern: a cron, and on failure one open
  issue with a label that later failures comment on.

The word "soak" is 2527's command. This ticket calls its runs **long
runs** and keeps the name free.

## Nick input

None.

## Scope

**In:**
- A long scenario: a `*.long.ron` file in a game's folder, read by
  `shuyi bench --long`. Its fields: `format_version`, `about`, `start`
  (as a bench scenario's), `script` (a bench scenario's script form),
  `repeat` (times the script plays), `idle` (frames with no input after
  each repeat), `warm_up` (frames not judged), `sample_every` (frames
  between samples) and `slack` (`heap_bytes`, default 64 KiB;
  `sounds`, default 0). A private `pub(crate)` type, as `Scenario` is:
  no public API change.
- `shuyi bench --long [--scenario <name>]`: plays each long scenario in
  the game's tool program (`bench-play --long`), never under Callgrind,
  with the heap counted. Every `sample_every` frames it records the
  frame, the live heap bytes, the live bytes of each memory tag, the
  sounds alive, and the frame's hash. It writes
  `dist/bench/long-report.json` and `long-report.md`.
- **The flatness rule.** After `warm_up`, the samples are split in
  quarters. The run fails when the highest live heap of the last
  quarter is above the highest of the first quarter by more than
  `slack.heap_bytes`, or the most sounds alive in the last quarter are
  more than the first quarter's most plus `slack.sounds`. The failure
  names the scenario, both numbers, and the three memory tags whose live
  bytes grew most.
- **The determinism rule.** `shuyi bench --long --twice` plays each long
  scenario again in a second process and fails at the first sample
  whose frame hash differs, naming its frame.
- A lean Harness: `Harness::keep_logs(false)` keeps only the last
  frame's audio and music and the current screens, so the driver holds
  a bounded amount however long it runs. Additive; the default keeps
  today's logs.
- `Session::frame_hash()`, defaulted to `None`; `Headless` turns
  `App::hash_frames` on for a long run and answers `App::frame_hash`.
  `Session::sounds_alive()`, defaulted to `0`; `Headless` answers
  `Harness::sounds_playing().len()`.
- Hello, Grid gets `examples/hello-grid/long/stage-loop.long.ron`: the
  stage played through moves, the foe's turns, the options menu and a
  dialogue, back to the title and into the stage again, repeated to at
  least 200,000 frames (about 55 minutes of play at 60 frames a second).
- The toy's fixtures: `crates/cli/toy/long/flat.long.ron` passes, and
  `leaks.long.ron`, with a toy option `--leak-bytes N` that keeps N more
  bytes every frame, fails naming its tag.
- A weekly workflow, `.github/workflows/long-run.yml`: Monday's cron and
  `workflow_dispatch`, `timeout-minutes: 90`, `contents: read`. It runs
  `cargo shuyi bench --long --twice` for Hello, Grid, puts the table in
  the job's summary, uploads the reports, and on failure opens one issue
  labelled `long-run` or comments on the open one, as `security.yml`
  does.
- The book (`site/content/learn/book/targets-and-export.md`) and
  `docs/targets/README.md`'s `shuyi bench` section say what a long
  scenario is and how a game adds one.
- **Either order with 2739:** if 2739 is done, add a long scenario for
  `examples/snake` (a seeded play that loses, restarts and loses again)
  and run it in `long-run.yml` beside Hello, Grid; if not, nothing to do
  (2739 adds it when it lands, as the docs say any game does).

**Out (do not do):**
- Random legal play with a reload at every step: 2527's `shuyi soak`.
- Per-frame allocation counts: 3221. Instructions against main: 3319.
- Handles the headless run cannot see: GPU textures, the sound device's
  voices, open files. They exist only in `shuyi-app` with a window, and
  this ticket runs headless.
- Wall time. A long run never measures it, as the bench never does
  (ADR-0081).
- A long run on every pull request. It is weekly and on demand only.
- A reusable workflow for games, as `playtest.yml` is. A game runs
  `cargo shuyi bench --long` in its own CI if it wants to.
- A long run on wasm32 or a one-bit target's conversion.

**Estimated size:** about 800 lines (code 450, tests 250, docs and data
100).

## Implementation steps

1. `crates/platform/src/harness.rs`: add `Harness::keep_logs(&mut self,
   on: bool) -> &mut Self`. With it off, `note_frame` replaces
   `self.audio` with the last frame's requests, clears `self.music` and
   `self.music_frames` before extending them, and keeps only the last
   entry of `self.stacks`. Document that `audio_log`, `music_log` and
   `screen_log` then hold the last frame only.
2. `crates/cli/src/contract/session.rs`: add `Session::frame_hash(&self)
   -> Option<StateHash>` and `Session::sounds_alive(&self) -> u32`, both
   with defaults, and `Session::set_long(&mut self)`, which turns the
   Harness's logs off and `App::hash_frames` on. Implement them on
   `Headless`.
3. `crates/cli/src/bench/long.rs` (new): `LongScenario` (the fields of
   the Scope, `#[serde(deny_unknown_fields)]`, `FORMAT_VERSION` 1),
   `find` for `*.long.ron` (the same folders `bench::find` skips),
   `Sample { frame, live_heap_bytes, tags: Vec<(String, u64)>,
   sounds_alive, hash }`, and `judge(samples, warm_up, slack) ->
   Result<(), Verdict>` with the flatness rule.
4. `crates/cli/src/bench.rs`: `--long` and `--twice` in `parse_args`;
   `bench-play --long <file> --out <file>` in `play_main`, which starts
   the session with `measured_start`, calls `set_long`, plays `script`
   then `idle` frames `repeat` times with `idle(1)` steps so it can
   sample every `sample_every` frames, and writes the samples. It never
   uses the perf ring, so the 600-frame limit doesn't apply. The outer
   half judges, compares the two runs' hashes with `--twice`, and writes
   the report. A failed rule exits 1, as an over-budget bench does
   (`bench::main` returns `u8::from(!report.over.is_empty())`); a game
   that fails to run keeps `codes::GAME_ERROR`.
5. `crates/cli/src/toy.rs`: `--leak-bytes N` keeps N bytes more each
   frame under its own memory tag. Add the two toy fixtures.
6. `examples/hello-grid/long/stage-loop.long.ron`: write the loop from
   the stage's existing bench scripts (`unit-move`, `foe-turns`,
   `options-menu`, `dialogue`). Measure its time on the runner with
   `workflow_dispatch` and pick `repeat` so one run of the pair takes
   under 40 minutes, with at least 200,000 frames. Write the measured
   time in a comment in the file.
7. `.github/workflows/long-run.yml`: as in the Scope. Pin the actions by
   hash as `ci.yml` does; the run's URL goes in through `env`, as in
   `security.yml`. Run `zizmor` locally over it.
8. Docs: the long scenario's fields and rules in
   `docs/targets/README.md` under `shuyi bench`, a section of ADR-0081
   (the `write-adr` skill: an amendment, not a new ADR), and the book's
   page that covers `shuyi bench`
   (`site/content/learn/book/targets-and-export.md`).

## Acceptance criteria

- [x] A Harness with `keep_logs(false)` holds the same live heap bytes,
      within 4 KiB, after 10,000 idle frames of the toy as after 1,000.
- [x] `cargo shuyi bench --long` in `crates/cli/toy` passes `flat` and
      fails `leaks` with exit 1, naming `leaks`, both heap numbers and
      the toy's leak tag.
- [x] A scenario with `warm_up` of 1,000 frames that allocates 1 MB once
      at frame 500 and never again passes.
- [x] A scenario whose sounds alive rise by one every 1,000 frames fails
      naming the sound counts.
- [x] With `--twice`, a toy option that mixes the process id into its
      state fails at the first sample, naming its frame; without it,
      the pair passes.
- [x] `stage-loop.long.ron` plays at least 200,000 frames of Hello,
      Grid and passes, ~~and the job's summary shows its table~~ (not
      done: the job runs only once `long-run.yml` is on main; the table
      it posts was checked locally).
- [x] ~~`long-run.yml` runs on `workflow_dispatch`, passes on main~~,
      and zizmor finds nothing new in it (not done: a workflow is
      dispatched only from main; see the completion notes).
- [x] A long scenario with an unknown field is refused naming the field.
- [x] ~~The public API listing changes only by `Harness::keep_logs` (an
      addition)~~ (not done: `Session`, a public trait, gains three
      defaulted methods too; all additions, see Deviations).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `judge` on hand-made samples: flat, a rise inside the slack, a
  rise just past it (slack + 1 byte fails), a rise only inside
  `warm_up`, a sound count rise; the quarters with a sample count not a
  multiple of four. `LongScenario::read` on a good file and on each bad
  field.
- Unit: `Harness::keep_logs(false)` keeps the last frame's audio and
  screens and nothing older.
- Integration: the toy fixtures through `bench::play` and the outer
  half in-process (`crates/cli/src/bench/tests.rs`), with a short
  `repeat` so the test stays under a few seconds.
- Snapshot: `long-report.md` for the toy's two fixtures.

## Completion notes

What was done:

- `Harness::keep_logs(on)` (`crates/platform/src/harness.rs`): off, the
  Harness keeps only the last frame's audio requests, music commands and
  dropped plays, and the screens open now. The default keeps today's logs.
- `Session::set_long`, `Session::frame_hash` and `Session::sounds_alive`,
  each defaulted, and `Headless`'s answers (`keep_logs(false)` and
  `App::hash_frames(true)`; `App::frame_hash`; `sounds_playing().len()`).
- `crates/cli/src/bench/long.rs`: `LongScenario` (format 1,
  `deny_unknown_fields`), `Sample`, the inner half `play` (samples written
  as JSON lines as they are taken), `judge` (the flatness rule),
  `first_difference` (the determinism rule), the report and its table.
  `bench.rs`: `--long`, `--twice`, `bench-play --long`, `long_plan`,
  `measure_long` and the `LongPlayer` trait; `find` shares `find_ending`.
- The toy's options `--leak-bytes N` (under the memory tag `toy-leak`),
  `--hum-every N` and `--pid-screen`, and four fixtures in
  `crates/cli/toy/long/`: `flat`, `leaks`, `hums` and `pid`. CI's `bench`
  job checks each in a process of its own: `flat` passes twice, `leaks`
  fails naming `toy-leak`, `hums` fails naming the counts, `pid` fails
  `--twice` at frame 100 and passes without it.
- `examples/hello-grid/long/stage-loop.long.ron`: title, the stage and its
  scene, two moves and the foe's turns, leaving, the options walked, and
  back to the title; 456 frames a loop, 2,200 loops, 1,003,200 frames.
  Locally (24 cores, Windows) one run took 25 s, and `--twice` passed:
  the live heap between 195,169 and 198,070 bytes all through, five
  distinct hashes among the samples, equal in both runs.
- `.github/workflows/long-run.yml`, Monday 07:00 UTC and
  `workflow_dispatch`, `timeout-minutes: 90`, `contents: read` (and
  `issues: write` for its job, as `security.yml`); zizmor finds nothing
  in it, and `ci.yml` keeps main's 6 low findings, none new.
- ADR-0145, which extends ADR-0081; `docs/targets/README.md` (*Long
  runs*, *In CI*); the book's *Size and speed*; the CLI's help.

Deviations:

- **A new ADR, not a section of ADR-0081.** The `write-adr` skill and
  `AGENTS.md` say an accepted ADR's decision is never rewritten: an
  extension is a new ADR with an `Extends:` line, and the old one's
  `Status:` names it. ADR-0145 does that.
- **The public API grows by four, not one.** `Session` is a public trait
  (`shuyi_cli::contract::session`), so the ticket's step 2 adds three
  defaulted methods to the listing beside `Harness::keep_logs`. All are
  additions; no game implements `Session` (searched below).
- **Samples are taken step by step.** A whole script plays many frames in
  one call, so samples taken only at its end fell on the same frame of
  the loop each time (one hash, one heap value over 200 samples). The
  run plays the script one step at a time and samples at the end of the
  first step that reaches each `sample_every` frames.
- **The samples go to a file as they are taken** (JSON lines), so the
  list never grows the heap it measures.
- **Hello, Grid's loop is a million frames, not 200,000.** The ticket's
  200,000 is a least; the pair takes about 50 s locally, far under 40
  minutes, so the run covers five times as much play. The measured time
  is in the file's comment. It is measured on this machine, not on a
  runner: a workflow dispatches only once its file is on `main`.
- **The sounds and warm-up criteria are checked two ways**: `judge` on
  samples made by hand (a megabyte at frame 500 under a warm-up of
  1,000; one sound more every 1,000 frames), and the toy's `hums`
  fixture played in this program and in CI.
- **`--target` and `--long` together are refused**, and `--twice` without
  `--long`: a long run gates no target's budget.
- **The integration tests are in `crates/cli/src/bench/long/tests.rs`**,
  not `bench/tests.rs`. In the test program the whole heap moves with
  the other tests' threads (a first full run hid the leak that way), so
  the leak test judges the bytes under `toy-leak` alone, and the
  `keep_logs` test counts under a memory tag of its thread's own. CI's
  `bench` job judges the whole heap, in a process of its own.
- **The table's test stands for the snapshot**: its samples are made by
  hand, as the toy's heap in this program is the whole test program's.
- **2739's clause is settled**: it now says it adds snake's long
  scenario and its step in `long-run.yml` (*From 3509 (done)*).

Not checked here:

- `long-run.yml` hasn't run on `workflow_dispatch`: it can only once the
  file is on `main`. Nick (or the next session) starts it with `gh
  workflow run long-run.yml` after the merge, and its summary shows the
  table; until it has, that box stays unticked.

Size: 1,964 counted lines, against the 800 estimated: the tests are
547 and the doc comments of `long.rs` most of its 581. Not split, as no
half lands alone under 1,500: the weekly workflow, Hello, Grid's file
and the docs are about 400 lines and need the command, and the command's
rules are tested by the toy's fixtures and the Harness's lean mode they
run on.

Searched the game: `gh search code --repo Zafnok/visions-of-shuyi`
for `impl Session for`, `bench-play` and `long.ron`: no hit.

No break. No `Behaviour:` line: a game sees nothing change unless it
asks for a long run.

Game: nothing to adopt, the long run is a tool a game may call.
