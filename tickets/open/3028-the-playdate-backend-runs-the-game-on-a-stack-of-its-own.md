---
id: "3028"
title: "The Playdate backend runs the game on a stack of its own taken from the heap, and reports how much of it was used"
type: feature
milestone: After MVP
model: opus-5.5
effort: medium
status: todo
blocked_by: ["3010"]
nick_input: none
completed:
---

# 3028 — The game on a stack of its own on the Playdate

## Context

- **The game ticket:** game#1170
  ([`1170-the-playdate-front-end-on-the-engines-backend.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1170-the-playdate-front-end-on-the-engines-backend.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153.
- **The job:** "run a game whose loading or thinking goes deeper than
  the device's small C stack".
- **Friction:** the game's Playdate front end, 1735 production lines
  against an estimate of 100 (17×), outcome 1. Most of the gap is
  already the work of 3003 and 3010. This part is in no ticket: the
  Playdate's C stack is about 10 KB, and the game's first device build
  crashed while loading; on a 256 KB stack taken from the heap the same
  code used 15,256 bytes (the game's ADR-0070, *Context* and §4). So
  the game's front end:
  - takes a stack from the heap at start and paints it with a known
    byte;
  - enters it with a few lines of Thumb assembly around every call from
    the update callback into the game, and comes back (`run(f)`, with a
    guard so a nested call doesn't start again from the top);
  - reports the high-water mark (the deepest painted byte touched);
  - on the Simulator, runs on the PC's stack and measures nothing.
  About 90 lines, device-only behind `cfg(target_os = "none")`. Any
  real game on the engine will need more than 10 KB of stack in a load
  or a search, so the backend should do this, not each game.
- **The generic shape:** `shuyi-playdate` runs `App::frame` (3003) on a
  stack of `stack_bytes` (in the game's Playdate config, with a
  default) taken from the SDK's allocator, and offers
  `stack_high_water() -> Option<usize>`.
- **Outside tactics:** a farming game's world load parsing a large save
  recursively; a platformer whose level loader recurses into nested
  rooms.

## Nick input

None.

## Scope

**In:**
- The stack switch around every call from the SDK's update callback
  (and the event handler's calls into the game) into the engine, in
  `shuyi-playdate`, from the game's `crates/playdate/src/pd/game_stack.rs`
  and `src/stack.rs` (read them at the game's `main`; they are tested
  on the host with a painted buffer).
- The size from the game's config; the high-water mark in the
  backend's API (`stack_high_water()`). 3011 waits for this ticket,
  and its on-device report logs the mark.
- `docs/targets/playdate.md`: why, the size the game measured, and how
  to pick one.

**Out (do not do):**
- Making the engine use less stack (a separate ticket per cause, if
  3011 finds one).
- Any game's front end.

**For the game:** additive. The game's front end moves onto the
backend in game#1170.

## Implementation steps

1. Read the game's two files and their tests, and ADR-0070 §4 of the
   game.
2. Move the switch and the painting into the backend, with the host
   tests.
3. The config field, the API and the docs.

## Acceptance criteria

- [ ] Host test: the high-water mark of a painted buffer after a
      simulated use of 3 KB reads 3 KB (±16 bytes).
- [ ] The example built for the device runs its load on the heap stack
      (checked on the device in 3011, or in the Simulator's device
      build if it has one; say which in the PR).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (host): painting and the high-water mark; the nested-call guard.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
