---
id: "3008"
title: "Performance budgets: shuyi bench plays scenarios headlessly and gates instruction counts, peak memory and package size against every supported target"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["3001", "3003"]
nick_input: none
completed: 2026-11-05
---

# 3008 — Performance budgets and `shuyi bench`

## Context

Nick, 2026-10-15 (`docs/decisions.md`): "expected performance benchmarks
for game exports", and on the Playdate, "run at a good fps".
[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§6:

- each target profile has budgets;
- CI gates on proxies that are the same on every run (instruction
  counts, peak heap, package size), never on wall time;
- device tickets (3011, 3013) measure the real frame times and calibrate
  the proxies.

The engine provides the tool and the examples' scenarios. A game writes
its own scenarios over its own content, so Visions of Shuyi gets numbers
for its heaviest battle without any of it entering the engine.

**From 2206 (open):** Parked: the MIDI synth's cost is measured with `shuyi bench` here when this ticket's bench exists first; otherwise 2206 uses a plain timing test and says so. Nothing in this ticket changes for it.

## Nick input

None.

## Scope

**In:**
- **Scenarios.** A scenario is a Harness script (a `.bench.ron` file)
  that gives:
  - the setup (example, stage, seed);
  - the inputs to play;
  - the frames to measure (a range, or "every frame").

  **A scenario is a test input of the tool, not an asset kind**, so it
  gets no `Format` and isn't in the registry: ADR-0007 is about what a
  game loads, and a scenario is never in a shipped build. 2018 says the
  same of its capture plans, and 2518 of its session records. It is a
  `Scenario` struct (`crates/cli/src/bench.rs`, private to the tool),
  read from its file with `ron::from_str` as the CLI reads its other
  test inputs, with a `format_version: u32` (1) that refuses another
  value. A file that doesn't read is an error naming the file, the line
  and the column, as a RON error gives them. The name `*.bench.ron`
  stays: `shuyi bench` finds a game's scenarios by it.

  The examples get these scenarios:
  - an idle map;
  - the cursor sweeping the map;
  - a unit moving;
  - an attack with its animation;
  - a full menu;
  - a bot's turn with the most units on the map;
  - a dialogue with a portrait.

  **Either order with 2360:** if 2360 is done, add a scenario that
  scrolls the camera across a map drawn with its tile-layer items, so
  the numbers say whether culling alone keeps tile layers in the web
  and Playdate budgets (2360 leaves chunk caching to them); if not,
  nothing to do (2360 adds it).

  **From 2566 (done):** a scenario's inputs are a
  `shuyi_platform::script::Script` (the contract's steps in their serde
  form, `act` among them), never a step type of its own, and its setup
  is the game's start options as `RunInput::start` holds them. The bench
  plays them as `run` does (`Session::play`, then `Session::idle`) with
  the session's perf on (`Session::set_perf`), so its peak heap is the
  one `run --perf` reports (`HeapSummary::peak_bytes`,
  `crates/cli/src/contract/perf.rs`), not a second reading. The
  instruction counts stay this ticket's own: `run --perf` times frames
  on the clock, which is never a gate here.
- **`shuyi bench [--target <id>] [--scenario <name>]`.** It plays each
  scenario through the headless backend (3003), including the conversion
  the target needs. **From 3006 (done):** on a `color: OneBit` target
  (`playdate`) that is `to_one_bit` (`shuyi_ui::frame::onebit`,
  ADR-0026), with the game's pattern map, one-bit font and one-bit
  pictures, which the game hands over as it does for `frame-png
  --target` (`GameTools::frame_one_bit`). It reports per scenario:
  - **instructions per frame** (the most, p99 and the mean), counted
    with Cachegrind (valgrind) on Linux. Use a crate under ADR-0002 §8
    (for example `iai-callgrind`; check its current name, version and
    license) or a small wrapper around `valgrind --tool=cachegrind`;
  - **peak heap**, from the counting allocator below;
  - **package size**. **From 3004 (done):** it comes from `shuyi
    export`'s `export-report.json` (`contract::export::ExportOutput`),
    whose `sizes` list the package first (for `web`, the zip with
    `index.html` at its root), with a test. **Either order with
    2516:** if 2516 is done, a
    package over budget names its largest files and the files no load
    read, from `shuyi check`'s files report. If not, 2516 adds that (its
    own clause says so).
- **The counting allocator** is 3064's: `CountingAlloc<A: GlobalAlloc>`
  in `shuyi-alloc-count` (`crates/alloc-count`, a default crate), around
  any inner allocator (`std::alloc::System` here, the Playdate SDK's in
  3010), with `live_bytes`, `peak_bytes`, `reset_peak`, `take_allocs`
  and `counts`, atomic where the target has atomics and a
  single-threaded fallback where it hasn't.
  - **From 3064 (done):** its `global` feature is the
    `#[global_allocator]` of every debug build that links `shuyi-app` or
    `shuyi-cli`, and a second static in such a build would not link. So
    the bench reads that one in a debug build
    (`shuyi_alloc_count::global::reading()`), and declares a
    `CountingAlloc::new` of its own only in a build where the global one
    is compiled out (a release build, or `--cfg shuyi_no_heap_count`;
    `global::COUNTING` says which); record which in the PR. ADR-0046 §9
    names the crate as one more place ADR-0002 §9 allows `unsafe`.
  - 3020 (parked) counts calls with it (`take_allocs`).
  - 2906's measuring program (`examples/hello-grid/examples/asset_memory.rs`)
    reads the same crate; add to it what this ticket needs, never a
    second counter.
- **Budgets from the profile.** Each number is compared with the
  `budgets` of every `supported` target among those the game lists (or
  just `--target`). A scenario over budget fails, naming the scenario,
  the frame, the target and both numbers.
  - **From 3001 (done):** a game lists the targets it ships on in its
    `shuyi.ron` `targets` table, and `check --target all` checks those.
    The bench reads the same list: Hello, Grid lists `desktop`, `web`
    and `steam-deck`, so it isn't gated against the Playdate, whose
    screen it doesn't fit. ADR-0008 §6's "every `supported` profile"
    predates that table; say so in the PR. The acceptance criteria's
    fixtures pass `--target`.
  - **From 3001 (done):** `targets/desktop.ron` and `targets/web.ron`
    call their `frame_instructions` "Provisional: 3008's benchmarks
    calibrate it", and web's `package_bytes` "the download, measured by
    3008". No device ticket covers either target, and ADR-0008 §6
    changes a budget only in a PR with a device measurement. Record the
    examples' numbers beside those budgets in `docs/targets/desktop.md`
    and `web.md`, leave the budgets as they are, and rewrite the three
    comments so they no longer name this ticket (provisional, with the
    bench's numbers in `docs/targets/<id>.md`).
- **`bench-report.json`**, plus a Markdown table for PR comments.
- **CI.** `shuyi bench` on the examples on every PR, on Linux. Post the
  table as a job summary. Fail on any over-budget scenario.
- **`docs/targets/README.md`** explains the budgets, the proxies, why
  wall time isn't a gate, and how a game adds scenarios. Start
  `docs/targets/<id>.md` for each profile with ADR-0008's budget row and
  "not yet measured on a device".
- **The `run-gates` skill** notes that `shuyi bench` needs valgrind, so
  it runs in CI (and in WSL if Nick wants it locally). It isn't a local
  gate on Windows.

**Out (do not do):**
- measuring on devices (3011, 3013);
- the game's scenarios (a game ticket; write it and link it);
- profiling tools and flame graphs (a later ticket if wanted);
- 2705's research benchmarks (parked; its crate is outside the
  workspace). **Either order with 2705:** if 2705 is done, nothing to
  do (it counted instructions its own way and deleted its crate); if
  not, nothing to do (2705 writes its workload as a `.bench.ron`
  scenario when it runs, or counts another way if one can't drive it).
- allocations per frame, the `frame_cycles` budget and the emulator
  runner for the GBA and the DS (the parked 3020, ADR-0012 §7). Keep the
  counting allocator able to report per frame: 3064 reads it each frame
  for the perf traces, and 2368 (parked) shows it in
  the debug view.

## Implementation steps

1. Write the `Scenario` type (a test input read with serde, not a
   `Format`: see *Scenarios*) and its runner on the headless backend.
2. Add the counting allocator's crate (or take 2906's) and the
   Cachegrind measurement, with a per-frame marker. Use client requests, or one process per measured
   range if that is simpler; record which in the PR.
3. Compare against the budgets, and write the report and the CI job.
4. Write the examples' scenarios and the docs.
5. Write the game ticket ("benchmark scenarios for our heaviest battle,
   menus and dialogue") in the game repository, and link it.

## Acceptance criteria

- [x] Two runs of `shuyi bench` on the same commit give the same
      instruction counts (CI checks two runs).
- [x] A fixture scenario that allocates 11 MB fails on `playdate`
      (budget 10 MB) and passes on `desktop`.
- [x] A fixture that burns instructions in a loop fails on `playdate`,
      naming the frame.
- [x] The CI job posts the table and fails on an over-budget scenario.
- [x] `docs/targets/` exists with one file per profile.
- [x] The game ticket exists and is linked.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the budget comparison and the report.
- Unit, if this ticket writes the counting allocator: the live bytes,
  the peak and its reset on known allocations, through an instance
  called directly (not the global one, which other test threads use).
- Integration: `shuyi bench` on an example in CI, twice, with the same
  counts.

## Completion notes

**What was done.**

- `shuyi bench [--target <id>] [--scenario <name>] [--out <dir>]
  [--no-instructions]` (`crates/cli/src/bench.rs`, private to the tool),
  with its inner half `bench-play`, run in the game's tool program.
  `Scenario` is read with serde from `*.bench.ron` (`format_version` 1,
  `about`, `start`, `script`, `frames`, `measure`), never a `Format`; an
  error names the file, the line and the column.
- **Instructions:** Callgrind (Valgrind's, beside Cachegrind), one
  process per scenario, with `--dump-before` on the frame's heap reading
  (`contract::session::process_heap`, now `#[inline(never)]`): one dump
  per frame. Neither client requests nor one process per measured range,
  as step 2 offered: a client request is `unsafe` or C in the CLI, and a
  process per frame costs the whole start each time. ADR-0081.
- **Peak heap:** the bench reads 3064's global counter
  (`shuyi_alloc_count::global::reading()`, through `Session::set_perf`,
  so it is `run --perf`'s `peak_bytes`). It declares no `CountingAlloc`
  of its own: it builds the tool program in a cargo profile of its own,
  `shuyi-bench` (the release profile with `debug-assertions` on and its
  symbols kept, given with `--config`), where `global::COUNTING` is true.
  A game built with `shuyi_no_heap_count` can't have its instructions
  counted (no frame marker); `--no-instructions` measures the rest.
- **One-bit targets:** `HeadlessPlatform::set_present_hook` and
  `Harness::set_present_hook` run a `PresentHook` on each frame
  presented, and `GameTools::session_on(target, options)` starts the game
  on a target's console. Hello, Grid's and the toy's convert each frame
  to one bit there (`to_one_bit`, `onebit::render`).
- **Package size** from each `dist/export/*/export-report.json` whose
  `profile` has a `package_bytes` budget, with a test.
- **Budgets:** each `Supported` target the game's `shuyi.ron` lists (or
  `--target` alone). ADR-0008 §6's "every `supported` profile" predates
  3001's `targets` table, so a game is gated only where it ships.
  Hello, Grid now lists the Playdate too (3010), so it is gated there
  (2 M instructions, 10 MB heap).
- `bench-report.json` (every frame's numbers) and `bench-report.md`.
- **CI:** the `bench` job (after `export`, with its web package): Hello,
  Grid twice, every frame's count compared, the table in the summary,
  then the toy's fixtures (`crates/cli/toy/bench/`: `--burn-frame` and
  `--hold-mb`, new toy start options).
- **Docs:** `docs/targets/README.md`, a page per profile (new: `web`,
  `steam-deck`, `switch`, `android`, `ios`; a *Budgets* section added to
  `desktop` and `playdate`), the targets' comments, ADR-0081, the docs
  map, `contract.md`'s `GameTools` table, the book's *Size and speed*,
  the `shuyi-perf` skill (1.4) and the `run-gates` skill.

- **The examples' numbers** (CI's first `bench` run, 2026-11-05), beside
  the budgets in `docs/targets/desktop.md`, `web.md` and `playdate.md`,
  the budgets unchanged: Hello, Grid's heaviest frame takes 394,501
  instructions on the colour targets (20 M budget) and 907,519 on the
  Playdate with its one-bit conversion (2 M); its heap peaks at 158.5 KB;
  its web zip is 993.4 KB (25 MB). Both runs counted every frame the same.

**Deviations.**

- Hello, Grid has no attack and no portraits: its scenarios are
  `idle-map`, `idle-map-tiles`, `cursor-sweep`, `unit-move`,
  `walk-animation` (the move's animation, standing in for the attack's),
  `foe-turns` (the bot's turns; two units is its most), `options-menu`
  and `dialogue` (the opening scene, without a portrait).
- 2360 isn't done: its scenario is its own (its clause now says so).
- 2566 merged while this was open: a scenario's `script` is its
  `shuyi_platform::script::Script` (`frame_png`'s re-export), read in its
  serde form (`act` among the steps) and played with `Session::play`,
  then `Session::idle`, as its clause says.
- 2516 isn't done: a package over budget names the package only; 2516
  adds its largest and unread files (its clause now says so).
- 2906's `asset_memory.rs` doesn't exist yet: nothing to add there.

**Follow-ups.** The game's ticket: Zafnok/visions-of-shuyi#553 writes
game ticket 1244 (benchmark scenarios for its heaviest battle, menus and
dialogue), blocked by this one; its PR moves the pin.

**For Nick.** CI now fails a pull request that makes a frame too heavy
for a target the example ships on, by numbers that are the same on
every run; the real devices calibrate them later (3011, 3013).

No break. The public API grows: `shuyi_platform::headless::PresentHook`,
`HeadlessPlatform::set_present_hook`, `Harness::set_present_hook` and
`GameTools::session_on` (a method with a default).
