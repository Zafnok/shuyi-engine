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
- **`shuyi bench [--target <id>] [--scenario <name>]`.** It plays each
  scenario through the headless backend (3003), including the conversion
  the target needs (the 1-bit frame on `playdate` once 3006 lands), and
  reports per scenario:
  - **instructions per frame** (the most, p99 and the mean), counted
    with Cachegrind (valgrind) on Linux. Use a crate under ADR-0002 §8
    (for example `iai-callgrind`; check its current name, version and
    license) or a small wrapper around `valgrind --tool=cachegrind`;
  - **peak heap**, from the counting allocator below;
  - **package size**, from `shuyi export` (3004) when it exists (for
    `web`, the size of the zip 3004 writes, with `index.html` at its
    root), or the asset bytes otherwise. A package over budget names its
    largest files and the files no load read, from `shuyi check`'s files
    report (2516) when it is done.
- **The counting allocator**, written as a reusable type (for example
  `CountingAlloc<A: GlobalAlloc>` around any inner allocator:
  `std::alloc::System` here, the Playdate SDK's allocator in 3010). It
  has a safe read of the live bytes and the peak, and a reset of the
  peak.
  - In this ticket only the bench binary installs it as
    `#[global_allocator]`. 2368 (parked) installs it in `shuyi-app`'s
    debug builds, and 3020 (parked) counts calls with it.
  - `GlobalAlloc` is an unsafe trait and the workspace's lints forbid
    `unsafe_code`, so it goes in a crate whose lints are the workspace's
    except `unsafe_code = "deny"`, with one `#[allow(unsafe_code)]`
    module, a `// SAFETY:` comment on every `unsafe` block, and a test
    like `shuyi-app`'s
    `the_lints_are_the_workspace_lints_except_unsafe_code`. A small
    crate of its own is one way; record the choice in the PR. A new
    crate that allows `unsafe` extends ADR-0002 §9, so name it in an ADR
    (the `write-adr` skill) as ADR-0021 §6 did for the platform crates.
    It never goes in `shuyi-core`, `shuyi-ui` or `shuyi-platform`.
  - It uses atomic read-modify-write only under
    `cfg(target_has_atomic = "ptr")`, with a single-threaded fallback,
    so 3020's targets can build it.
  - **Either order with 2906:** if 2906 is done, its measuring program
    (`examples/hello-grid/examples/asset_memory.rs`) already uses a
    counting allocator, written in a crate of its own as this bullet
    says, and 2906's ADR names that crate. Use it, and add to it what
    this ticket needs, instead of writing a second. If not, 2906 uses
    this one (its clause says so).
- **Budgets from the profile.** Each number is compared with the
  `budgets` of every `supported` target (or just `--target`). A scenario
  over budget fails, naming the scenario, the frame, the target and both
  numbers.
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
- allocations per frame, the `frame_cycles` budget and the emulator
  runner for the GBA and the DS (the parked 3020, ADR-0012 §7). Keep the
  counting allocator able to report per frame. 2368 (parked) shows it in
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
