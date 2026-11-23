---
id: "3243"
title: "A range tint strength a game sets for each skin"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-23
---

# 3243 — A range tint strength a game sets for each skin

## Context

1. **The game ticket:** `game#1319`
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/1319-move-and-attack-ranges-easy-to-tell-apart.md`).
   A new player found the move and attack ranges hard to tell apart.
   Shown animated renders on 2026-11-23, Nick picked brighter colours
   and, over the bought pictures, tints at 150% of today's strength: at
   the strengths the range looks use (0.45 to 0.75), red over a bright
   green picture blends to rust, and he wanted it to read red.
2. **The job:** make a range's tint stronger (or weaker) on one skin
   than the look makes it, so it reads over bright pictures, while
   another skin keeps today's.
3. **Friction:** no friction review: a missing setting. The tint's
   strength is a set of constants today (`OVERLAY_BLEND`,
   `SHOWN_BLEND`, `GLINT_BASE`, …), the same for every skin.
4. **The generic shape:** `RangeShow` gets `strength_percent` (100 by
   default, today's) and `with_strength(percent)`; `RangeShow::strength`
   scales the tint by it, never past the range's color itself (1). A
   game builds each skin with the `RangeShow` it wants. Nothing of the
   game: ranges are already named data.
5. **A game that isn't this one:** any grid game drawn over bright
   pictures (a board game's legal moves over a painted board, a
   city-builder's zone of effect over a sunlit map) whose tint must be
   stronger there than on a dark glyph ground.

## Nick input

None (the strength was Nick's pick in the game ticket).

## Scope

**In:**
- `grid::ranges::RangeShow`: the field `strength_percent`,
  `with_strength`, and `strength` scaling by it, capped at 1. Both
  skins already take their tint from `strength`.

**Out (do not do):** which skin a game makes stronger, and by how much
(the game's); the flashes' and still glyph tints' constants.

No break: an addition (`RangeShow` is non-exhaustive). The game's PR
that moves the pin follows this one (`game#1343`).

**Estimated size:** about 60 lines (code 20, tests 20, ticket 20).

## Implementation steps

1. `grid/ranges.rs`: the field, `TINT`'s 100, `with_strength`,
   `strength` scaled and capped.
2. `grid/ranges/tests.rs`: the scale, the cap, a strength of 0, and
   nothing before the spread.

## Acceptance criteria

- [x] A look built as before tints as before (`strength_percent` 100).
- [x] 150 makes a tile's tint 1.5 times as strong, mid-spread too; 0
      none; a tint never passes 1.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `strength` at 100, 150, 50, 0 and 400 (capped), mid-spread and
  before the spread.

## Completion notes

- `RangeShow::strength_percent` and `RangeShow::with_strength`;
  `RangeShow::strength` scales by it and caps at 1. A skin built as
  before draws as before.
- Checked in the game's Chapter 1 battle at this change, the sprite
  skin at 100, 125 and 150 (the pictures stay local: the map is the
  game's private content); Nick picked 150.
- Docs: the rustdoc of the field, the method and `strength`; nothing
  else describes a tint's strength.

No break: new items only (`RangeShow` is non-exhaustive).

Book: none, the book's grid chapter doesn't show range tints' strength;
the new items are documented in their rustdoc.

Game: game#1343 moves the pin and makes the sprite skin's tints 150%.
