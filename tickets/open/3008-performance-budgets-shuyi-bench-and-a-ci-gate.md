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
  - **peak heap**, from a counting `GlobalAlloc` wrapper installed in
    the bench binary only;
  - **package size**, from `shuyi export` (3004) when it exists, or the
    asset bytes otherwise.
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
- profiling tools and flame graphs (a later ticket if wanted).

## Implementation steps

1. Write the scenario format (a `Format`, ADR-0007) and its runner on the
   headless backend.
2. Add the counting allocator and the Cachegrind measurement, with a
   per-frame marker. Use client requests, or one process per measured
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
- Integration: `shuyi bench` on an example in CI, twice, with the same
  counts.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
