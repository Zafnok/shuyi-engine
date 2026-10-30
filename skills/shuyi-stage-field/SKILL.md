---
name: shuyi-stage-field
description: A real-time stage on Shuyi Engine, where things move every fixed step, hit tiles and boxes, and a camera follows. The engine doesn't build one yet. This skill says what exists, what is coming, and how a game that can't wait keeps its world ready for it. Use when a game needs real time (snake, a platformer, a top-down action game) or asks how the engine will run one. Not for a turn-based grid stage (shuyi-stage-grid), or for showing a turn game's moves (shuyi-animate).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
  waits_for: "1119 1120 1122"
---

# A real-time stage

**Not built yet.** Shuyi Engine plays turn-based stages today. Its
real-time stage is planned: `cargo shuyi modules` lists `fixed ticks`
(Time), `top-down free` and `side view` (Perspective) and `movers and
drawn clips` (Bodies and motion) as `parked`. When they are `ready`,
this skill gets its steps and a worked example. Until then, don't build
a game that needs them without telling the person who asked for it.

## When to use it

- Someone asks for a game that moves in real time: snake, a
  platformer, a top-down action game.
- A turn-based game needs a part that runs on its own clock.

## When not to

- A game played turn by turn on a grid: `shuyi-stage-grid`.
- Showing a move happen (a walk, a sprite's frames): `shuyi-animate`.

## What exists today

- **Fixed steps.** The platform turns real time into fixed simulation
  steps, 60 a second, at most 4 in one frame (a late frame slows the
  game down rather than piling steps up). A screen reads the steps due
  this frame as `FrameInput::ticks()`: 0 to 4, and 0 while the game is
  suspended.
- **Luck and hashes.** `SimRng`, kept in the state, and
  `shuyi_core::state_hash`, which is the same on every machine for a
  state whose maps are ordered.
- **Tile maps.** `Grid`, `Pos` and `Dir`, and the `.map` text format
  for a map's tiles (`shuyi-stage-grid`).
- **The tools.** `run`, `scene` and `frame-png` work on any screen.
  `replay` runs a record by the game's own code: a game's tools give it
  with `GameTools::replay`, a hash per step in a `ReplayRun`. The engine
  builds that run for a turn game (`replay::turns`); a real-time world
  builds its own.

## What is coming

A simulation stepped by those fixed steps, in fixed-point numbers
(whole numbers with a fraction of a pixel, never floats), with a record
of each step's input that replays to the same hash on every machine.
Beside it: tile collision and boxes, movers that step a pixel at a time
and say what they hit, triggers, a top-down controller as data, and a
camera that follows with a dead zone and parallax.

## Steps, meanwhile

A game that must move in real time before then keeps its world in the
shape the engine's simulation will take:

1. **Keep the world's rules in `crates/rules`**, pure, behind the same
   fence as a turn game's (`shuyi-gates`): no I/O, no clock, no floats,
   no luck but a `SimRng` in the state, `BTreeMap` for maps.
2. **Step the rules once per step due.** In the screen's `update`, call
   the rules' own `step` with this frame's input `input.ticks()` times.
   The rules never read `dt`: `input.dt()` is how long a frame took,
   which differs from machine to machine. Only a skin reads it, to show
   motion between steps.
3. **Count in whole numbers.** Positions in pixels, or in parts of a pixel
   (256 to the pixel) in an `i32`; speeds in those units per step. Time a rule
   with `shuyi_core::timer`'s `Countdown` and `Every` in the state, advanced by
   one in each step of `FrameInput::ticks()`: a cooldown of 30 steps is half a
   second.
4. **Hand the rules plain input.** Turn the frame's actions into a value
   of the rules' own (which way, which button pressed this step), so a
   test, a bot or a replay can give the same input without a screen.
5. **Keep a record.** The seed and each step's input are enough to play
   the world again. A test of the rules steps them from a record and
   compares `state_hash` at the end, and the game's `GameTools::replay`
   can give `cargo shuyi replay` a hash per step (`shuyi-debug-replay`).

## When it fails

Branch on the answer's `code`, never its message, and read its page
with `cargo shuyi explain <code>`:

- `SH2103`: the rules use what their fence bans: a float (count in whole
  numbers), a clock (count steps), a `HashMap` (use `BTreeMap`).
- `SH3001`: the game's tools don't offer an operation: `replay` before
  its `GameTools::replay` is written.
- `SH0003`: `shuyi add` named a module that isn't built yet.

## Worked example

None yet: it comes with the engine's real-time stage.
