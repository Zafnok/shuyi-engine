---
id: "3011"
title: "Playdate on the device: run the bench scenarios on Rev A or Rev B hardware, record frame times and memory, and calibrate the playdate budgets"
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3007", "3008", "3009", "3010", "3028"]
nick_input: setup
completed:
---

# 3011 — Playdate on the device

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§6. The `playdate` budgets are provisional:

- 30 fps held, with p99 ≤ 33.3 ms and no frame over 50 ms;
- 2 M x86 instructions per frame;
- 10 MB of heap;
- a 40 MB package;
- 5 s to start.

CI measures x86 instruction counts (3008), which are only a proxy. This
ticket runs the same scenarios on a real Playdate. It writes the real
numbers into `docs/targets/playdate.md`, and sets the instruction budget
from the measured ratio, so that from then on CI catches a change that
would drop the device below 30 fps. That is Nick's "run at a good fps",
as a number that a gate keeps.

## Nick input

**Setup:**

1. This needs a Playdate. If you don't have one, this ticket waits:
   buying one is your call (Panic sells it at play.date).
2. Connect it by USB, unlock it, and keep it on the home screen.
3. In *Settings → System*, read out the version and serial. Rev A and
   Rev B differ in memory speed, and the session records which one this
   is.

**Sign-off:** play the example on the device for two minutes, and say
whether it feels smooth. The numbers are the gate, but your eye is the
check on the numbers.

## Scope

**In:**
- **A bench build.** `shuyi export playdate --bench` builds the example
  with the bench scenarios (3008) built in. Each scenario runs from a
  menu, and the build logs per-frame times
  (`getElapsedTime` around `App::frame` and the display flush) and the
  peak heap over the SDK's console log.
- **Collecting the log** over USB, with the SDK's tools (the Simulator's
  device console, or `pdutil`; record which one works on Windows).
- **Run on the device from the computer.** `shuyi export playdate --run
  --device` (3004's run step, beside 3010's Simulator run) installs the
  `.pdx` and starts it over USB through the SDK's `pdutil` (its install
  and run commands, checked in the SDK's docs on the day). Then it
  prints the device's console log by the route *Collecting the log*
  finds. A missing device or SDK says how to fix it. The bench steps use
  it.
- **The report** in `docs/targets/playdate.md`, per scenario:
  - frame time p50, p99 and the worst frame;
  - dropped frames (frames over 33.3 ms);
  - peak heap;
  - the stack's high-water mark (3028's `stack_high_water()`), which
    also shows that the load ran on the heap stack (the device check
    3028's acceptance criteria leave here);
  - start-up to the title screen;
  - the `.pdx` size;
  - the hardware revision and the SDK version.
- **Calibration:**
  - `frame_instructions` becomes (measured device ms ÷ x86
    instructions) applied to 33.3 ms, with 20 % headroom, taking the
    scenario with the worst ratio;
  - `think_units_per_frame` (3009) is set the same way;
  - **From 3001 (done):** `targets/playdate.ron` is already `status:
    Supported`. 3001 set every profile but `switch`, `android` and
    `ios` so, before any device run; ADR-0008 §1 calls a target
    supported once its example runs on the device, which this ticket
    makes true. Keep it `Supported`. A scenario that misses 30 fps gets
    its follow-up ticket (below), not a demotion, which would drop the
    Playdate from 3008's gate.

  The PR carries the measurements that justify every number it changes.
- **The report a shipped game keeps** (from the game's friction sweep,
  game#1153). Lift from the game's Playdate front end (`src/report.rs`,
  `src/pd/bench.rs`), which logs on every run, not only in a bench
  build: when the loading card and the first frame were shown, the
  slowest files to load (through the bundle's read hook), once a minute
  the frame times, and the stack's high-water mark (3028). The backend
  offers it; a game turns it on. Its lines go out the way App's lines do
  on the Playdate (3010's sink, the SDK's console), with the category
  `platform` of 3040's log.
  **Consumer handoff, 2026-11-01 audit:** `game#1170` now explicitly
  waits for this report surface before adopting the backend. Expose the
  opt-in callbacks/observations needed for loading-card and first-frame
  timestamps, slow bundle reads, periodic frame and stack figures, and
  bench-phase observations. The game owns its labels and content; the
  backend owns clocks, measurements and output. The game's follow-up
  `game#1205` removes its superseded report only after adoption verifies
  equivalent observations. Do not count logs from the example's private
  bench code as a reusable shipped-game report API.
- **If a scenario misses 30 fps.** Write a ticket per cause found (for
  example, the 1-bit conversion, the scene build, or the bot), each with
  its measurement. Don't fix them here.

**Out (do not do):**
- optimizing (follow-up tickets);
- the game's own content (a game ticket runs the game's scenarios once
  its port starts, game 1025).

## Implementation steps

1. Add `--bench` and `--run --device` to the exporter, and the in-game
   scenario menu.
2. Run every scenario five times, and keep the median run of each.
3. Write the report and calibrate the profile.
4. Write the follow-up tickets.

## Acceptance criteria

- [ ] `shuyi export playdate --run --device` installs and starts the
      example on the device, and its log reaches the terminal
      (Completion notes).
- [ ] `docs/targets/playdate.md` has every scenario's numbers, with the
      hardware revision and the SDK version.
- [ ] The `playdate` profile's budgets are set from the measurements, and
      the PR shows the arithmetic.
- [ ] `targets/playdate.ron` stays `status: Supported`, and a
      follow-up ticket exists for each scenario that misses 30 fps.
- [ ] Nick's sign-off is recorded in the completion notes.
- [ ] A toy consumer enables the shipped-game report through public
      backend interfaces and receives loading/first-frame, slow-read,
      periodic frame/stack and bench-phase observations without its own
      device clock or SDK driver; the API and the `game#1170` handoff are
      documented. Disabled reporting adds no unsolicited periodic output.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Hardware runs above measure/calibrate performance; the bench gate
  (3008) enforces the result.
- Unit/integration: opt-in report callbacks using a fake SDK/clock and
  a toy consumer, plus disabled-report behavior. These verify the reusable
  interface independently of the hardware measurements.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
