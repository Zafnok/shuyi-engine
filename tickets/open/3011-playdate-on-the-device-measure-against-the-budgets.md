---
id: "3011"
title: "Playdate on the device: run the bench scenarios on Rev A or Rev B hardware, record frame times and memory, and calibrate the playdate budgets"
type: research
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["3007", "3008", "3009", "3010"]
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
- **The report** in `docs/targets/playdate.md`, per scenario:
  - frame time p50, p99 and the worst frame;
  - dropped frames (frames over 33.3 ms);
  - peak heap;
  - start-up to the title screen;
  - the `.pdx` size;
  - the hardware revision and the SDK version.
- **Calibration:**
  - `frame_instructions` becomes (measured device ms ÷ x86
    instructions) applied to 33.3 ms, with 20 % headroom, taking the
    scenario with the worst ratio;
  - `think_units_per_frame` (3009) is set the same way;
  - `status` becomes `supported` once every scenario holds 30 fps.

  The PR carries the measurements that justify every number it changes.
- **The report a shipped game keeps** (from the game's friction sweep,
  game#1153). Lift from the game's Playdate front end (`src/report.rs`,
  `src/pd/bench.rs`), which logs on every run, not only in a bench
  build: when the loading card and the first frame were shown, the
  slowest files to load (through the bundle's read hook), once a minute
  the frame times, and the stack's high-water mark (3028). The backend
  offers it; a game turns it on.
- **If a scenario misses 30 fps.** Write a ticket per cause found (for
  example, the 1-bit conversion, the scene build, or the bot), each with
  its measurement. Don't fix them here.

**Out (do not do):**
- optimizing (follow-up tickets);
- the game's own content (a game ticket runs the game's scenarios once
  its port starts, game 1025).

## Implementation steps

1. Add `--bench` to the exporter and the in-game scenario menu.
2. Run every scenario five times, and keep the median run of each.
3. Write the report and calibrate the profile.
4. Write the follow-up tickets.

## Acceptance criteria

- [ ] `docs/targets/playdate.md` has every scenario's numbers, with the
      hardware revision and the SDK version.
- [ ] The `playdate` profile's budgets are set from the measurements, and
      the PR shows the arithmetic.
- [ ] `status: supported` only if every scenario holds 30 fps.
      Otherwise, follow-up tickets exist for each miss.
- [ ] Nick's sign-off is recorded in the completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the gates. This ticket measures; the bench gate (3008)
  enforces the result.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
