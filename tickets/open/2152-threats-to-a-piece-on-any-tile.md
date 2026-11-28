---
id: "2152"
title: "What threatens a piece if it stood on a tile: each other piece searched once, searched again only when the moved piece could change its moves"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2151"]
nick_input: none
completed:
---

# 2152 — What threatens a piece on any tile

## Context

A request from the game.

1. **The game ticket:** game#1420, which puts the game's threat planner
   on this once it lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1420-threat-planner-on-the-engines-threat-queries.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** while the player hovers a move's
   destination, show which enemy pieces could strike the piece there,
   for every tile the cursor visits, without searching every enemy's
   moves again each time.
3. **The friction numbers:** sweep game#1397 (2026-11-27). The game would
   keep about 60 lines: its ranges, its stance that stops enemies, its
   pieces that never move, and the display's caches. It measures about
   280 code lines of `ThreatPlanner` and `ThreatAreas` in
   `crates/core/src/movement.rs` (about lines 668–961) at its pin
   230080d, plus about 40 of the three caches in
   `crates/ui/src/screens/battle/threat.rs` that exist to drive them.
   About 5×. It is in no engine ticket: `threat.rs`'s `Threats::reaches`
   and `safe` answer only for areas already worked out.
4. **The generic shape:** in `shuyi_core::threat`:
   - `ThreatPlanner<P>`, for one piece `target`: for each other piece,
     its moves searched once with `target` off the board, kept with its
     strike area. Asked for a tile `at`, a kept search settles it when it
     can: a tile outside the kept area is never threatened; a tile the
     kept moves never come next to (nor, if `target` stops pieces next
     to it, within two tiles of) changes nothing; a tile in range of a
     tile whose path keeps clear of `at` is threatened. Only otherwise is
     that one piece searched again with `target` on `at`.
   - `ThreatAreas<P>`: every piece's moves searched once with everything
     in place, answering for pieces on their own tiles, and lending its
     searches to a planner whose result would be the same.
   - Pieces that never move: their area from their tile only.
   - The searches are the caller's closure
     (`search(piece, absent: Option<P>) -> Reach`, from 2151's
     `reachable_with_stops`), and so are the ranges; this module only
     decides which searches can be reused.
5. **An example from a game that isn't tactics:** none outside the grid;
   any grid-tactics game that shows "who can hit me if I stand here"
   while the cursor moves wants it unchanged. It is the tactics kit's.

## Nick input

None.

## Scope

**In:**
- `crates/core/src/threat/planner.rs`: `ThreatPlanner`, `ThreatAreas`,
  generic over the piece key `P: Copy + Ord`; additive.
- The answers are exactly those of searching every piece again with
  `target` on `at` (the slow path), which a property test checks.

**Out (do not do):**
- Drawing the threats (the grid skin's ranges).
- A game's rules for what stops a piece or which pieces never move: its
  closures and flags.

**Estimated size:** about 760 lines (code 300, tests 400, docs 60).

## Implementation steps

1. The slow answer first, as a test helper: search every other piece
   with `target` on `at`, and test which strike areas hold `at`.
2. `ThreatAreas::new` and `threats_to`, from the game's.
3. `ThreatPlanner::new`/`with_areas` and `threats_at`, porting the game's
   `Threat::settles` and `moves_clear_of` rules with their reasons in the
   doc comments.
4. The property test: on random small boards and pieces, the planner
   agrees with the slow answer on every tile.
5. A count of the searches (the caller's closure counts its calls, so the
   number is exact on every machine): a 20 × 20 board with 30 pieces,
   every tile asked once, against the slow answer's count.
6. Docs: `skills/shuyi-stage-grid/SKILL.md`; the book's grid chapter.
7. The PR says the game's PR that moves the pin (game#1420) follows it.

## Acceptance criteria

- [ ] The property test passes over at least 500 random cases (seeded).
- [ ] On the 20 × 20 board, asking every tile once makes at most a fifth
      of the slow answer's searches.
- [ ] A piece that stops pieces next to it, and a piece that never
      moves, each have a unit test.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the settling rules one by one.
- Property: planner equals slow answer.
- Unit: the search count on the 20 × 20 board.

## Completion notes

*(Filled in by the session that completes the ticket.)*
