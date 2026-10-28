---
id: "3008"
title: "Performance budgets: shuyi bench plays scenarios headlessly and gates instruction counts, peak memory and package size against every supported target"
type: infra
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["3001", "3003"]
nick_input: none
completed:
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

  **Either order with 2566:** if 2566 is done, a scenario's inputs are a
  `shuyi_platform::script::Script` (the contract's steps in their serde
  form, `act` among them), never a step type of its own, and its setup
  is the game's start options as `RunInput::start` holds them. The bench
  plays them as `run` does (`Session::play`, then `Session::idle`) with
  the session's perf on (`Session::set_perf`), so its peak heap is the
  one `run --perf` reports (`HeapSummary::peak_bytes`,
  `crates/cli/src/contract/perf.rs`), not a second reading. The
  instruction counts stay this ticket's own: `run --perf` times frames
  on the clock, which is never a gate here. If not, the inputs are
  `frame_png::Script`'s steps, played and measured the same way, and
  2566 moves them onto `script::Step` with the rest (its own clause
  says so).
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
  - **package size**. **Either order with 3004:** if 3004 is done, it
    comes from `shuyi export`'s `export-report.json` (for `web`, the
    size of the zip 3004 writes, with `index.html` at its root); if not,
    it is the asset bytes, and 3004 switches it to the export's sizes
    (its clause says so). **Either order with 2516:** if 2516 is done, a
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

1. Write the scenario format (a `Format`, ADR-0007) and its runner on the
   headless backend.
2. Add the counting allocator's crate (or take 2906's) and the
   Cachegrind measurement, with a per-frame marker. Use client requests, or one process per measured
   range if that is simpler; record which in the PR.
3. Compare against the budgets, and write the report and the CI job.
4. Write the examples' scenarios and the docs.
5. Write the game ticket ("benchmark scenarios for our heaviest battle,
   menus and dialogue") in the game repository, and link it.

## Acceptance criteria

- [ ] Two runs of `shuyi bench` on the same commit give the same
      instruction counts (CI checks two runs).
- [ ] A fixture scenario that allocates 11 MB fails on `playdate`
      (budget 10 MB) and passes on `desktop`.
- [ ] A fixture that burns instructions in a loop fails on `playdate`,
      naming the frame.
- [ ] The CI job posts the table and fails on an over-budget scenario.
- [ ] `docs/targets/` exists with one file per profile.
- [ ] The game ticket exists and is linked.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the budget comparison and the report.
- Unit, if this ticket writes the counting allocator: the live bytes,
  the peak and its reset on known allocations, through an instance
  called directly (not the global one, which other test threads use).
- Integration: `shuyi bench` on an example in CI, twice, with the same
  counts.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
