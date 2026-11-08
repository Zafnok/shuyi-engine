---
id: "3097"
title: A game's load runs over several update callbacks on the Playdate
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3097 — A game's load runs over several update callbacks on the Playdate

## Context

1. **The game ticket:** game#1170
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1170-the-playdate-front-end-on-the-engines-backend.md`)
   moves the game's Playdate front end onto this engine's backend
   (`shuyi-playdate`). It is blocked on 2388 and 3096 today.
2. **The job:** start a game whose data takes longer than 10 seconds to
   load on the device, with the loading card up, without the Playdate OS
   ending it.
3. **No friction review:** a gap found by the retro of 2026-11-08 (game
   PRs #513–#580), reading the code.
   - The Playdate OS ends a game whose update callback hasn't returned
     for 10 seconds.
   - The game hit exactly that in game#1086: "run loop stalled for more
     than 10 seconds". Its device loads data about 600–1,300 times slower
     than the Simulator, and the game's data alone took 15,351 ms there.
   - The game fixed it in its own front end (game #537, game ADR-0111):
     `Boot`, a step machine run one step per update callback. Each step
     stops after one second and goes on in the next.
   - The engine's backend has no such thing. `Load<G>` is a one-shot
     `Box<dyn FnOnce(Setup) -> Result<Game<G>, String>>`
     (`crates/platform/src/app.rs`), and `shuyi-playdate`'s driver runs it
     inside one frame (`App::new(start)`, `crates/playdate/src/driver.rs`).
   - So game#1170's cutover would likely bring the stall back. That is
     an inference from the code, not a measurement.
4. **The generic shape:** a load in steps. The game gives its load as a
   series of steps; the backend runs as many as fit in a time slice per
   update callback, with the loading card on screen and a progress
   report, and starts the game when the last one returns.
5. **A game that isn't tactics:** a farming game that reads a large
   world and its saves at start, or any game with many dialogue files,
   on a slow handheld.

## Nick input

None.

## Scope

**In:**
- A stepwise load beside the one-shot one: for example a `Load::Steps`
  whose steps are `FnMut(&mut Loading) -> Progress`, or an iterator of
  boxed steps. `App` runs it on every backend. The PC and the web may run
  all the steps in one frame. The Playdate's driver runs steps until a
  time slice is used, about one second by default and set by the target
  profile, then returns from the callback.
- The load report (3090) gains the per-callback times, in the shape
  game#1086 added ("Start, ms per update callback: …").
- The one-shot `Load<G>` keeps working: either additive (ADR-0039), or
  with a `Breaking:` line and a migration block. Game#1170's PR moving
  the pin follows.
- A Harness test with a fake clock: a load of 30 steps of 100 ms each
  runs in three or more callbacks, none over the slice.

**Out (do not do):**
- Splitting the game's load into steps (game#1170 does that, from its
  `Boot`).
- Loading one chapter at a time (game#1068's subject).

## Implementation steps

1. Read the game's `crates/playdate/src/boot.rs` and game ADR-0111 (via
   `gh api repos/Zafnok/visions-of-shuyi/contents/…`) for the step
   shape that worked on the device.
2. Design the type in `crates/platform/src/app.rs`, and write an ADR
   (`write-adr`) since it is a public API shape.
3. Run the steps from `crates/playdate/src/driver.rs`, with the slice
   read from the target profile, which has a 10-second limit.
4. Add the report line in `crates/playdate/src/report.rs`.
5. Write the game's adopter note in the completion notes: what 1170 does
   with its `Boot` steps.

## Acceptance criteria

- [ ] A Playdate driver test with a fake clock shows a 3-second load run
      over at least 3 callbacks, each under the slice.
- [ ] A one-shot `Load<G>` game still starts unchanged (the existing
      driver tests).
- [ ] The load report prints the per-callback times.
- [ ] Game#1170's ticket names this ticket in its `blocked_by` (the
      game's PR from the retro adds it).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the step runner's slice logic with a fake clock.
- Integration: the Playdate driver's tests with the fake SDK.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
