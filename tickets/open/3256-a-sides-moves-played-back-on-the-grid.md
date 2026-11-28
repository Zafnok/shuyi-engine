---
id: "3256"
title: "The grid kit plays back a computer side's move: the camera pans, the piece is marked, it walks its path with a step cue, and a held key hurries it"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3256 — A computer side's move played back on the grid

## Context

A request from the game.

1. **The game ticket:** game#1417, which puts the game's enemy-phase
   playback on this once it lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1417-enemy-phase-playback-from-the-grid-kit.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** when a computer-controlled side moves a
   piece, the screen slides the camera to it, marks it for a moment,
   walks it along its path a tile at a time with a sound per tile
   entered, and plays all of it faster while the player holds a key.
3. **The friction numbers:** sweep game#1397 (2026-11-27). The game would
   keep about 40 lines: its pacing numbers, its unit list and what
   follows the move (its combat playback). It measures 182 code lines in
   `crates/ui/src/screens/battle/ai_phase.rs` at its pin 230080d
   (`Pacing`, `AiAction`: the phase times, two `HurryClock`s made fresh
   every tick, `tiles_entered`, `walker_pos`, the camera lerp), on top of
   this kit's `grid::walk::{steps, gait}` and `HurryClock`. About 4.5×.
4. **The generic shape:** a tactics-kit `grid::playback::MovePlayback`:
   a pure function of a path, a camera origin to pan from and to, a
   `MovePacing` (pan seconds, mark seconds, walk tiles per second, held
   walk tiles per second, hurry factor) and its clock. It answers, at any
   time: the camera origin (a straight slide, a tile at a time, using
   `shuyi_core::ease::Lerp`), whether the mark shows, whether the piece
   is walking, its `Gait` (facing, frame, offset), the tile it stands on,
   how many tiles the next `tick(dt, held)` enters (for step cues), and
   `done()`. `repan` changes the pan's ends without changing its time
   (the view was resized). It holds no units: the screen draws its own
   pieces at `tile()` with `gait()`.
5. **An example from a game that isn't tactics:** none outside the grid:
   a board game's or a roguelike's computer turn is the same sequence, so
   it belongs in the tactics kit, not the engine's core.

## Nick input

None.

## Scope

**In:**
- `crates/ui/src/grid/playback.rs`: `MovePlayback` and `MovePacing`
  (`#[non_exhaustive]`, with a constructor and `MovePacing::at_speed`
  scaling the pan and the mark, as the game's `Pacing::at_speed`);
  re-exported from `grid`.
- The hurry rule kept exactly: the pan and the mark hurry until the walk
  starts; the walk hurries whatever is left of the frame (the game's
  `advanced`, `ai_phase.rs` about line 167). A bad `dt` counts as 0.
- Additive: no break to the public API.

**Out (do not do):**
- Choosing the move (the bots and planners) and what follows it (an
  attack's playback): the game's.
- Following the walker with the camera after the pan: the screen's
  camera already does (`grid::camera`).

## Implementation steps

1. Port `AiAction`'s clock, times, steps, gait and camera into
   `MovePlayback`, generic over nothing: a path of `Pos`, the two camera
   origins and the pacing.
2. Make `HurryClock` able to scale a `dt` with no state of its own
   (`HurryClock::scaled(dt, held, factor)` or the like) if the port needs
   one; if `hurry.rs` already has it, use it.
3. Tests: total time, the mark's window, step counts at the end of a
   walk whatever rounding leaves, `tiles_entered` summing to the path's
   steps over any split of `dt`, hurrying across the walk's start.
4. Docs: `skills/shuyi-stage-grid/SKILL.md` and `skills/shuyi-animate/SKILL.md`
   name it; the book's grid chapter.
5. The PR says the game's PR that moves the pin (game#1417) follows it.

## Acceptance criteria

- [ ] A 4-tile path with pan 0.25 s, mark 0.2 s and 8 tiles/s takes
      0.825 s; `marked()` is true from 0.25 s to 0.45 s only.
- [ ] Over any split of the same total `dt`, `tiles_entered` adds up to
      3 for that path, held or not.
- [ ] With the key held, the pan plays `fast` times faster and the walk
      at the held speed, and a frame that crosses the walk's start
      splits as the game's `ai_phase.rs` tests expect.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the times, the steps and the camera at chosen moments.
- Property: `tiles_entered` over random splits of `dt` sums to the
  path's steps.

## Completion notes

*(Filled in by the session that completes the ticket.)*
