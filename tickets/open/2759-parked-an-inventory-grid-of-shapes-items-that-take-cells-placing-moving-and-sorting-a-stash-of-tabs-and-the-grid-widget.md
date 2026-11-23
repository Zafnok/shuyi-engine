---
id: "2759"
title: "Parked: An inventory grid of shapes: items that take cells, placing, moving and sorting, a stash of tabs, and the grid widget"
type: feature
milestone: "Engine: action RPGs"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: blocked
blocked_by: ["2746", "2354", "2142"]
nick_input: none
completed:
---

# 2759 — An inventory grid of shapes

> **Parked** (Nick, 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0034](../../docs/adr/0034-action-rpgs-a-hero-in-real-time-loot-rolled-from-affix-tables-skills-with-cooldowns-and-packs-of-monsters.md)
§1–§2. Diablo's inventory is a grid where a sword takes one by three
cells and a chest piece two by three; the player fits them in. A stash
holds more, in tabs. 2746's inventory is a list with a limit; this adds
the grid as another kind of limit, so the same items and commands work.

**Who uses it** (ADR-0033 §17's bar for engine work): action RPGs,
survival horror (Resident Evil's case), extraction shooters, inventory
puzzle games (Backpack Hero), crawlers that want it.

It builds on 2746 (`Item`, `Inventory`, commands) and 2354 (the grid
cursor, for moving with a pad).

**From 2405 (done):** items are picked up and dropped with the
pointer: a click picks an item up and a click drops it, no drag (2405
left drag out), through the hit areas the grid widget's painter
records (`GlyphBuffer::hit`, `crates/ui/src/frame/hit.rs`) and
`PointerInput::clicked`.

3233 adds a pointer drag (`PointerInput::drag`, `DragAndDrop`). When
this ticket starts it may take drags too, beside the click to pick up
and click to drop.

**From 2142 (through 2746):** the items placed in the grid are
`shuyi_core::items::Stack`s, and `Grid { w, h }` is a limit beside
2142's `Capacity` kinds, kept in `shuyi-rpg`.

**Either order with 2409:** if 2409 is done, each cell's hit item is
2409's `grid_item(row, col)`; if not, the painter records each cell
with an item of its own, and 2409 moves them onto `grid_item` (its
clause with 2759 says so).

## Nick input

None.

## Scope

**In:**
- **Shapes**: `items.ron` (2746) gains `size: (w, h)` per kind (default
  1×1); rectangles only.
- **`GridInventory`** in `shuyi_rpg::grid_inventory`: a `w × h` grid of
  cells, each item at a top-left cell; `fits(item, at)`, `place`,
  `take`, `move_to`, `swap` (placing over exactly one item swaps it to
  the hand), `auto_place` (the first fit, column by column, as the data
  says), `sort` (by size, then kind, then rolls, packing tightly; a
  greedy rule, documented);
  - `Inventory`'s limit becomes `Count(n)` or `Grid { w, h }`, and
    2746's commands work on either;
  - stacks merge when an item is placed on its own kind.
- **A stash**: named tabs of `GridInventory`, kept in the profile
  (shared by every character of the profile, as `party.ron` or the
  game says), with move-between commands.
- **The widget** (behind `draw`): the grid, items as their picture
  spanning their cells, the cursor (2354), the item in hand, a place
  highlight green or red by `fits`, and a tooltip slot the game fills
  (2758's names and rolls if present).
- All moves are commands, so they replay.

**Out (do not do):**
- shapes other than rectangles, and rotating items;
- equipment slots (2746 has them; the widget may draw them beside the
  grid in the example);
- trading between players.

## Implementation steps

1. Sizes in the format; `GridInventory` and its rules.
2. The limit kinds in 2746's `Inventory`.
3. The stash and its storage.
4. The widget, Harness tests and snapshots.

## Acceptance criteria

- [ ] No two items ever share a cell (a property test of random
      commands).
- [ ] `sort` keeps every item and never needs more cells than before.
- [ ] A Harness test moves an item into the stash and back with the pad.
- [ ] `frame-png` snapshots of the grid with items in hand and placed.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: fits, place, swap, auto-place, sort.
- Property: no overlap; counts kept.
- Integration: Harness and storage round trip.
- Snapshot: the widget.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
