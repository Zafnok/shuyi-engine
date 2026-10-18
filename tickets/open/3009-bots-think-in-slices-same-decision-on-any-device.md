---
id: "3009"
title: "Bots think in slices: spread a bot's search over frames by work units, so the frame rate holds and the decision is the same on every device"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["game#1131", "3008"]
nick_input: none
completed:
---

# 3009 — Bots think in slices

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§6. Bots (the tactics kit's enemy AI, carved in the game's 1103–1117)
decide in one call today. On a desktop that is fine. On a 168 MHz
Playdate, a turn with many enemies could freeze the screen for seconds.
The game's 1025 names "the enemy AI's thinking time" as the first thing
to measure.

Two rules hold together:

- **The frame rate holds.** The Playdate budget is 30 fps while a bot
  thinks.
- **The decision never depends on the device.** A bot that stopped
  "when time ran out" would choose differently on a slow device, and
  replays, lockstep and bug reports would break (ADR-0002 §2). So
  slicing counts work units, never time. A slow device reaches the same
  decision, only later.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::think`:**
  - `trait Think { type Decision; fn step(&mut self, units: u32) ->
    Progress<Self::Decision> }`, where `Progress` is `Working` or
    `Done(decision)`;
  - a "unit" is whatever the bot counts: a node visited or a move scored.
    The bot defines it, and it never depends on time.
- **The tactics kit's bot implements `Think`.** Its total work for a
  decision is fixed by its settings (for example, a search depth and a
  node limit), never by how many slices it gets. A test proves that
  `step(1)` repeated and `step(u32::MAX)` give the same decision on
  every fixture.
- **A frame-time budget in `App`.** `shuyi-platform`'s `App` gives a
  thinking bot `units_per_frame` from the target profile (a new
  `budgets.think_units_per_frame`; calibrated by 3011 and 3013). The
  screen keeps animating meanwhile: the cursor blinks, and a "thinking"
  sign appears after 0.5 s.
- **Headless and replays.** The Harness runs bots to completion in one
  call (`step(u32::MAX)`), so tests stay fast and records don't change.
- **A bench scenario** (3008): a bot turn with the most units, measured
  per frame against the `playdate` budget.

**Out (do not do):**
- threads (the Playdate has one core; the web has none we use);
- making the bot smarter or cheaper (game tickets);
- time-based limits of any kind;
- the real-time strategy AI (parked 2710, 2714, 2715, ADR-0015 §8). It
  will use `Think` for decisions that recur every few steps, so keep
  `Think` free of anything that assumes one decision per turn.

## Implementation steps

1. Add `Think` and `Progress` with docs and an example.
2. Turn the tactics bot's search into a resumable state machine. Its
   state lives in the `Think` value, not on the call stack.
3. Add the `App` budget and the profile field (3001's format gets a
   version bump with a default).
4. Add the equivalence property test and the bench scenario.

## Acceptance criteria

- [ ] On every bot fixture, slicing by 1, 7, 100 and unlimited units gives
      the same decision (property test with random slice sizes).
- [ ] Replays recorded before the change replay the same.
- [ ] The bot-turn bench scenario stays under the `playdate`
      instruction budget per frame.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: slice-size independence.
- Integration: replay fixtures; the bench scenario.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
