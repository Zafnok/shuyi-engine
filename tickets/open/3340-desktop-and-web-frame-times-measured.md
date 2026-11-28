---
id: "3340"
title: The desktop and web targets measure real frame times, on CI's runners first and then on a real machine
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: setup
completed:
---

# 3340 — The desktop and web targets measure real frame times

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§6 gives each target a frame time on its device. The `desktop` row says
p99 ≤ 16.7 ms. The `web` row says p99 ≤ 16.7 ms in Chrome on a
mid-range laptop. Neither was ever measured:

- `docs/targets/desktop.md` (its *Budgets* section, about line 201):
  "Not yet measured on a device: no ticket covers the desktop yet."
- `docs/targets/web.md` (*On the device*, about line 45): "Not yet
  measured on a device. No device ticket covers the web yet."

Every number on those pages is a proxy. CI's `bench` job (3008,
[ADR-0081](../../docs/adr/0081-the-bench-callgrind-dumps-at-each-frames-end-a-profile-of-its-own-and-scenarios-as-test-inputs.md))
counts instructions on the headless backend, whose `present` only keeps
the frame. Hello, Grid's heaviest frame is 394,501 instructions
(`idle-map-tiles`) against a budget of 20,000,000. The budget is
marked provisional, and `docs/targets/README.md` says a budget changes
only in a PR that carries a device measurement. So the budget can't move
until someone measures, and no ticket measures. The Playdate (3011) and
the Steam Deck (3013) have device tickets. The two targets most players
will use have none.

What already exists, read on 2026-11-23:

- **Frame times as data** (3063, ADR-0046). `shuyi_ui::perf::FrameTimes`
  has `update_us`, `draw_us`, `paint_us`, `present_us` (the swap and the
  wait for vsync or the cap) and `frame_us` (the whole frame, start to
  start). `App` records them only when `shuyi_ui::DEBUG_TOOLS` is on:
  `cfg!(any(debug_assertions, feature = "debug-tools"))`
  (`crates/ui/src/lib.rs:111`). So a release build with shuyi-ui's
  `debug-tools` feature measures, and a player's release build pays
  nothing.
- **The trace file** (2553). On native, `SHUYI_PERF=<path>` writes each
  frame's record as a JSON line (`crates/app/src/perf_file.rs`). The web
  has no trace file: `perf_file` is `cfg(not(target_arch = "wasm32"))`.
- **The spread** (`shuyi_ui::perf::spread`, `crates/ui/src/perf.rs`):
  p50, p99 and the most of a list of numbers.
- **Plans that play input in a real window** (the capture mode,
  ADR-0052, `crates/app/src/capture.rs`). A plan's input steps are a
  script's (`shuyi_platform::script::Step`), the same steps a bench
  scenario's `script` holds. But the capture mode steps exact frames
  with a fixed `FRAME_DT`, and `run_app` returns into `capture::run`
  before the pacer and the trace file start (`crates/app/src/lib.rs`,
  about line 592). Its frames are not the frames a player gets.
- **The bench scenarios** (`examples/hello-grid/bench/*.bench.ron`,
  eight of them). Each one has `start` arguments, a `script` and a frame
  count.
- **A window in CI.** The `render` job runs a real macroquad window under
  Xvfb with Mesa's software GL (`LIBGL_ALWAYS_SOFTWARE`). The
  `web-render` job runs the web build in headless Chromium through
  Playwright (`tools/web-check/check.py`).

So the pieces are there. What's missing is a mode that plays a scenario
at the real pace, a command that runs it and sums it up, and the runs
themselves. CI's runners come first: they need no hardware, and they
prove the whole path end to end. Their numbers are a software renderer's
on a shared machine, so they are recorded and never gated
(`docs/targets/README.md`, *The proxies, and why not wall time*). Then
the same command runs on a real machine with a real GPU: the Windows 11
machine the engine is developed on. Which other devices to buy or
borrow is Nick's call (3527).

These numbers also become the baseline that the parked renderer work
(3325, 3326, 3330, from the macroquad research in PR 532) is compared
with.

## Nick input

**Setup** (for step 8 only; steps 1 to 7 need nothing from Nick):

1. Leave the computer plugged in and unlocked, with nothing heavy
   running (no game, no video call, no build), for about 15 minutes.
2. The session opens a game window and a browser window several times.
   Don't move them or click in them until the session says it's done.
3. If you have more than one monitor, say which one the window should
   open on.

## Scope

**In:**
- **A measure mode in `shuyi-app`.** A `measure` feature, which turns on
  `shuyi-ui/debug-tools`. With it, `SHUYI_MEASURE=<plan>` plays a measure
  plan through the normal loop, with the pacer, vsync and the trace file
  as a player's build has them. The plan is a script's steps
  (`script::Step`) and a time to hold after the last step. The game's
  `dt` is the real one, not `FRAME_DT`. Storage is in memory and the
  audio is on, as a player hears it. When the plan ends, the run exits 0.
  A plan that can't be read exits 2 with the reason. Without the feature
  the variable is never read.
- **The web half of the mode.** A web build with `measure` runs
  `measure.ron` beside the page, as the capture mode runs `capture.ron`.
  It logs each frame's record as a console line after a prefix
  (`shuyi-measure: `), and `shuyi-measure: done` at the end.
  `tools/web-check/check.py` gets a `measure` command. It serves the
  page, collects those lines, and also records the page's own
  `requestAnimationFrame` timestamps over the same frames, as a check on
  the engine's clock. It writes both as JSON lines. The page's clock is
  coarse (browsers round `performance.now`), so the report gives the
  resolution it saw.
- **`cargo shuyi bench --wall`.** It builds the game's window program in
  the release profile with the `measure` feature (`--target web`: the web
  export with it). It turns each bench scenario into a measure plan under
  `dist/bench/wall/`, runs it with `SHUYI_PERF` set (the web through
  `check.py measure`), and reads the trace. It writes `wall-report.json`
  (every frame) and `wall-report.md` (the table) to `dist/bench`. For each
  scenario the table gives:
  - the frame interval (`frame_us`): p50, p99 and the worst;
  - the dropped frames: intervals over 1.5 times the run's median;
  - the busy time (`frame_us` minus `present_us`): p50, p99 and the
    worst, which is the frame's real cost under vsync;
  - its parts (`update_us`, `draw_us`, `paint_us`) at p99;
  - the start-up: from spawning the program to its first frame's record;
  - the machine: OS, CPU, GPU and renderer, the refresh rate and the
    browser and its version, read where the system gives them, and
    "unknown" otherwise.

  It compares the frame interval's p99 and the start-up with the
  target's numbers and says pass or miss, but its exit code is 0 for a
  miss. Wall time is never a gate. It exits 1 only when a run fails. A
  game without the `measure` feature gets an error naming the line to
  add to its `Cargo.toml`.
- **Hello, Grid and the template** gain the `measure` feature, beside
  `capture` and `link`.
- **A weekly CI workflow** (`.github/workflows/wall-times.yml`, weekly
  and on `workflow_dispatch`). It runs `bench --wall` for Hello, Grid on
  `ubuntu-latest` under Xvfb (the `render` job's setup) and on the web in
  headless Chromium (the `web-render` job's setup). It uploads
  `wall-report.*` as an artifact and adds the table to the job summary.
  It fails only when a run fails, never on a number. Actions are pinned
  by digest, credentials are not persisted, and each job has a
  `timeout-minutes`, as the other workflows do.
- **Either order with 3336** (CI launches exported packages on Windows
  and macOS): if 3336 is done, the workflow also runs `bench --wall` on
  `windows-latest` and `macos-latest`, with 3336's way of opening a
  window there. If not, nothing to do (3336 adds the measured run to its
  Windows and macOS legs).
- **Either order with 3507** (the web build in Firefox and WebKit): if
  3507 is done, `check.py measure` takes `--browser`, and the workflow
  and the device run measure in each browser `check.py` drives. If not,
  nothing to do (3507 adds `--browser` to `check.py measure`).
- **Either order with 3341** (draw calls, texture switches and uploads,
  and a stress scenario): if 3341 is done, its stress scenario is among
  the scenarios measured, and the report gives its p99. If not, nothing
  to do (3341 runs `bench --wall` on its stress scenario and writes the
  numbers on the target pages).
- **The device run** (step 8). On the Windows 11 machine the engine is
  built on, run `bench --wall` for the desktop and for the web in
  Chromium, each three times. Write the numbers, the machine and the
  date on `docs/targets/desktop.md` and `docs/targets/web.md`, in place
  of the "Not yet measured" lines. The web row names "a mid-range
  laptop": write what this machine is beside it, and that it isn't one,
  if it isn't.
- **Calibration**, by the rule in `docs/targets/README.md`. From the
  heaviest scenario's busy time and its instruction count (CI's `bench`
  job, same commit), write the time per million instructions on this
  machine. Change `frame_instructions` (20 M, provisional) only if that
  ratio says 20 M instructions would not fit in half of a 16.7 ms frame.
  Otherwise keep it and keep it provisional, and say why on the page:
  one machine, and a frame whose cost is mostly painting. Change
  `startup_ms` only if the measured start-up misses it.
- **Start-up on desktops and the web.** Measure `startup_ms` (from
  launch to the game's first frame) on the machine, and write it on
  `docs/targets/desktop.md` and `docs/targets/web.md`. Then set each
  profile's `startup_instructions` (3342) from the measured ratio of
  the start's instructions to its time, as ADR-0008 §6 sets
  `frame_instructions`.
- **Either order with 3342** (the start-up's instruction budget): if
  3342 is done, recalibrate `startup_instructions` on `desktop` and
  `web` from the measurement. If not, record the measured start-up and
  the ratio on the pages, and 3342 derives its provisional budget as it
  says.
- **Either order with 3527** (Nick's decision on platforms and the
  devices to test on): if 3527 is done, write a follow-up ticket (After
  MVP, `33xx`, with the `write-ticket` skill) that runs these steps on
  each device its answer names (a Mac, a Linux laptop, a low-end
  Windows laptop, a phone's browser). If not, nothing to do (3527's
  answer writes that ticket).
- **A miss gets a ticket.** A p99 or a start-up over its number on the
  real machine gets a follow-up ticket with the numbers. No fix here.
- New types: the measure plan (`shuyi_platform::measure::Plan`, or a
  name settled in the work) may grow, so it is `#[non_exhaustive]` with
  a constructor (ADR-0039). The report's JSON has `deny_unknown_fields`
  like the bench's.

**Out (do not do):**
- Any gate on a wall time. CI's numbers are recorded only.
- Counting draw calls, texture switches and uploads, and the stress
  scenario (3341).
- Counting the start's instructions in the bench (3342). This ticket
  times the start on a device; 3342 counts it in CI.
- Idle redraw and input latency (3345).
- Launching exported packages on Windows and macOS in CI (3336).
- Firefox and WebKit (3507).
- The Steam Deck (3013) and the Playdate (3011), which have their own
  device tickets.
- A frame profiler in the debug view (2325, parked) and GPU timestamps
  (2382, parked).
- Buying or borrowing a device (3527).
- Optimising anything.
- `docs/platform-backends.md`: PR 532 edits it. The mode's docs go in
  `crates/app/src/lib.rs`'s module docs and `docs/targets/`.

**Estimated size:** about 1,800 lines (code 850, tests 600, docs 350).

**Split:** past 900 lines, split off the web half (the web side of the
measure mode, `check.py measure` and the workflow's web job: parts of
steps 2, 4 and 7) into a new ticket (After MVP, `33xx`); this ticket
then measures the desktop only and the new one does the web's device
run.

**Shared with capture** (the redundancy sweep of 2026-11-28): `crates/app/src/capture.rs`'s
plan request (an environment variable on native, a file beside the page
on the web), its plan loading, `cannot_run`/`finish` and the prefix and
done console protocol become one small plan-mode helper with the names
as parameters. Measure and capture both use it.
`tools/web-check/check.py`'s measure mode reuses capture's
wait-for-done and console collection. The two modes stay separate
features.

## Implementation steps

1. **The plan.** In `shuyi-platform`, add the measure plan: script steps
   (`script::Step`, parsed as `capture::Plan` parses them) and
   `hold_s`, the seconds to keep running after the last step (default 10,
   at most `capture::MAX_WAIT`). Its input is given to the game by frame
   number, as the capture mode gives it, but the frames run at the real
   pace.
2. **The mode.** In `crates/app`, add the `measure` feature
   (`["shuyi-ui/debug-tools"]`) and a `measure` module. `run_app` reads
   `SHUYI_MEASURE` on native and `measure.ron` on the web. Unlike
   `capture::run`, it stays in the normal loop: the pacer, `next_frame`
   and the trace file run as they do for a player. Only storage (in
   memory) and the input (the plan's; the keyboard and pads aren't read)
   change. On the web, write each frame's `PerfLine` to the console
   after `shuyi-measure: `. Exit 0 at the end of the hold on native, and
   log `shuyi-measure: done` on the web.
3. **The features.** Add `measure = ["window", "shuyi-app/measure"]` to
   `examples/hello-grid/Cargo.toml`, and `measure =
   ["shuyi-app/measure"]` to
   `templates/shuyi-default/base/crates/game/Cargo.toml.tmpl` with its
   comment. The template job checks the template.
4. **`check.py measure <folder> --out <file>`.** Serve the page, read the
   `shuyi-measure: ` lines until `done` (with a timeout), and record
   `requestAnimationFrame` timestamps over the same span. Write both as
   JSON lines.
5. **`bench --wall`** in `crates/cli/src/bench.rs` (or a `bench/wall.rs`
   module beside `callgrind.rs`). Build, write each scenario's plan, run
   it, read the trace, and sum it up with `shuyi_ui::perf::spread`.
   Write `wall-report.json` and `wall-report.md`. Pass the scenario's
   `start` arguments as `bench` passes them to the tool program. Read
   the machine's facts. Update the contract's and the MCP tool's
   description if `bench` is listed there (`crates/cli/src/contract.rs`,
   `crates/cli/src/mcp.rs`).
6. **Docs.** `docs/targets/README.md` gets a section on `bench --wall`:
   what it measures, why it is never a gate, and how to run it on a
   device. `crates/app/src/lib.rs`'s module docs name the `measure`
   feature and `SHUYI_MEASURE`.
7. **The workflow.** Write `.github/workflows/wall-times.yml` as the
   Scope says. Run it once with `workflow_dispatch` on the branch, and
   copy its tables into `docs/targets/desktop.md` and
   `docs/targets/web.md` under a heading that says they are CI's
   software renderer, not a device.
8. **The device run.** With Nick's setup done, run `cargo shuyi bench
   --wall` and `cargo shuyi bench --wall --target web` three times each
   on the Windows 11 machine, in Hello, Grid. Write the numbers, then
   calibrate as the Scope says.
9. **Follow-ups.** Write a ticket for each miss. If 3527 is done, write
   the ticket for the devices it names.

## Acceptance criteria

- [ ] A Hello, Grid build with `measure` and `SHUYI_MEASURE` set plays
      the plan in a window at the real pace and exits 0. Its trace has
      one record per frame. A build without the feature ignores the
      variable.
- [ ] On the web, `check.py measure` reads a record per frame and the
      page's own frame timestamps, and the report gives the clock's
      resolution.
- [ ] `cargo shuyi bench --wall` writes `wall-report.json` and
      `wall-report.md` with each listed number for all eight scenarios,
      on the desktop and on the web.
- [ ] A p99 over 16.7 ms is reported as a miss, and the exit code is
      still 0. A run that fails exits 1, naming the scenario.
- [ ] A game without the `measure` feature gets an error naming the
      line to add.
- [ ] The workflow ran on the branch, and its run is linked in the
      completion notes. It fails on a failed run, never on a number.
- [ ] `docs/targets/desktop.md` and `docs/targets/web.md` have CI's
      numbers, marked as a software renderer, and the real machine's,
      with the machine, the browser and the date. Neither page says "no
      ticket covers" any more.
- [ ] The calibration is written on each page: the time per million
      instructions, and whether `frame_instructions` and `startup_ms`
      changed, and why. The measured start-up is written on each page,
      with its ratio of instructions to time, and, if 3342 is done,
      `startup_instructions` on `desktop` and `web` is set from it.
- [ ] Each miss on the real machine has a follow-up ticket.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the measure plan parses its steps and `hold_s`, and refuses a
  hold over `MAX_WAIT` and an unknown field. The frame on which each
  step's input is given is the same as the capture mode's for the same
  steps.
- Unit: the summary of a hand-written trace: p50, p99 and the worst of
  the interval and the busy time, a dropped frame at 1.5 times the
  median and none just under it, and the start-up. A trace with no
  frames is an error, not a zero.
- Unit: the `wall-report.md` table for two scenarios, as a snapshot.
- Unit: the error for a game without the feature, and the miss line
  with exit 0.
- Integration (Linux under Xvfb, in the workflow): Hello, Grid's
  `cursor-sweep` scenario played by `bench --wall`, with one record per
  frame and a frame count within 10 % of the hold's length times the
  frame cap.
- Python: `check.py measure` against a page whose console lines are
  written by a fixture (as `check.py fixtures` does).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
