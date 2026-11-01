---
id: "3028"
title: "The Playdate backend runs the game on a stack of its own taken from the heap, and reports how much of it was used"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: ["3010"]
nick_input: none
completed: 2026-11-01
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

- [x] Host test: the high-water mark of a painted buffer after a
      simulated use of 3 KB reads 3 KB (±16 bytes).
- [x] The example built for the device runs its load on the heap stack
      (checked on the device in 3011, or in the Simulator's device
      build if it has one; say which in the PR).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (host): painting and the high-water mark; the nested-call guard.

## Completion notes

- `crates/playdate/src/stack.rs` holds the plain part, tested on the host:
  the high-water mark, the red zone, the top, and `run`, the guard on a nested
  call. `src/device/game_stack.rs` holds the switch, lifted from the game's
  `src/pd/game_stack.rs`. Every call from the SDK runs on the stack: `init`,
  the update, the events and the system menu.
- The size is `main!(start, stack_bytes = N)`, with `main!(start)` still
  meaning `stack::DEFAULT_BYTES` (64 KB, the game's size). `stack_high_water()`
  is the mark, `None` off the device.
- **Deviations from the game's code:** the overflow check reads only the 256
  bytes of the red zone after each call, where the game's scanned up to the
  deepest byte used, so the check costs no more on a stack of 64 KB than on a
  small one. The stack's bytes are read one at a time with `read_volatile`, so
  `stack_high_water()` is sound from inside the game, on the stack it reads.
- **Which check of the device criterion:** Hello, Grid's `.pdx` was exported
  and linked, and the switch is in its `pdex.elf` (`mov sp, r1`, `blx r2`,
  `mov sp, r4`). It has **not** run on a device: 3011 holds that check, as its
  report's high-water line.
- **The game PR that moves the pin:** game#1170, in the same session.

Book: none, `main!` keeps its form and the book names no stack; the new
argument and `stack_high_water()` are in `docs/targets/playdate.md`.

No break.
