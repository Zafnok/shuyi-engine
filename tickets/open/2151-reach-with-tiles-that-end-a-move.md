---
id: "2151"
title: "Reach with tiles that end a move when entered, and the cost of a drawn path with the step it fails at"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2151 — Reach with tiles that end a move, and a drawn path's cost

## Context

A request from the game.

1. **The game ticket:** game#1419, which puts the game's movement on this
   once it lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1419-movement-on-the-engines-stopping-reach.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** work out where a piece can move when some
   tiles may be entered but end the move there (a zone of control), and
   check an arrow the player drew tile by tile: its cost, or the step
   where it goes wrong and why.
3. **The friction numbers:** sweep game#1397 (2026-11-27). The game would
   keep about 40 lines: which tiles stop (next to an enemy that holds a
   stance), its step costs and its hostile tiles, as closures. It
   measures about 130 code lines in `crates/core/src/movement.rs` at its
   pin 230080d: a `Reach` that wraps `shuyi_core::path::Reach` with a
   `stops` list (about lines 169–240), the search that enters each
   stopping tile from its cheapest neighbour of the walk (about 352–382),
   and `path_cost` with its `PathError` (about 101–166, 400–450). About
   3×.
4. **The generic shape:** in `shuyi_core::path`:
   - `reachable_with_stops(width, height, start, budget, cost, stops)`:
     `cost(pos) -> Option<u32>` as `reachable` has it, and
     `stops(pos) -> bool` for tiles that may be entered but not left. The
     walk goes on from every other tile; each stopping tile is entered
     from its cheapest neighbour of the walk, the first in `Dir::ALL` on
     a tie, if that is within budget. The `Reach` it gives holds both:
     `cost`, `contains`, `path_to` and `tiles` cover the stopping tiles,
     `is_stop(pos)` says which they are.
   - `walk_cost(path, cost, stops) -> Result<u32, StepError>`: every step
     to a 4-way neighbour, each entered tile with a cost, no step leaving
     a stopping tile entered after the start; `StepError { index, kind }`
     with `kind` one of `Empty`, `NotAdjacent`, `Closed`, `Stopped`. The
     budget check stays the caller's (it knows its own error).
5. **An example from a game that isn't tactics:** none outside the grid;
   a board game with zones of control (a wargame's hex-less grid, a
   Wesnoth-like) is the user, so it is the tactics kit's.

## Nick input

None.

## Scope

**In:**
- `crates/core/src/path.rs`: `reachable_with_stops`, `Reach::is_stop`
  (and the stopping tiles inside the existing `Reach`), `walk_cost`,
  `StepError`, `StepErrorKind` (`#[non_exhaustive]`).
- **The tie order is a requirement:** a game's recorded battles replay
  only if the entry neighbour is "cheapest, then first in `Dir::ALL`",
  exactly as the game's `movement.rs` picks it today. A test pins it.
- `reachable` unchanged; additive: no break to the public API.

**Out (do not do):**
- What makes a tile stop or a tile closed (units, stances, sides): the
  caller's closures.
- Threat areas over this reach: `threat.rs` already takes a `Reach`;
  with stops inside it, its `strike_area` needs no change (check, and
  say so in the completion notes).

## Implementation steps

1. Add stopping tiles to `Reach` (a `TileSet` and their entries) without
   changing a plain `reachable`'s answer.
2. `reachable_with_stops` as the game's `search` does it after its
   `path::reachable` call.
3. `walk_cost` from the game's `path_cost`, minus its unit lookups.
4. Tests, and a property test: every tile `reachable_with_stops` gives
   has a `path_to` that `walk_cost` accepts at that tile's cost.
5. Docs: `skills/shuyi-stage-grid/SKILL.md`; the book's grid chapter.
6. The PR says the game's PR that moves the pin (game#1419) follows it.

## Acceptance criteria

- [ ] On a 5 × 5 open grid, budget 4, with the ring around (2, 2)
      stopping: those tiles are in the reach and end their paths, and
      no tile past them is reached through them.
- [ ] A stopping tile reachable from two neighbours at equal cost is
      entered from the first in `Dir::ALL`.
- [ ] `walk_cost` on a path that leaves a stopping tile fails with
      `kind: Stopped` and the index of that tile; a diagonal step with
      `NotAdjacent`; a closed tile with `Closed`.
- [ ] The property test passes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cases above.
- Property: reach and walk cost agree.

## Completion notes

*(Filled in by the session that completes the ticket.)*
