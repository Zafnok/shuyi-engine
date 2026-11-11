---
id: "3308"
title: Filling a rectangle keeps the items it misses as they were
type: bug
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-10
---

# 3308 — Filling a rectangle keeps the items it misses as they were

## Context

1. **The game ticket:** game#1261
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1261-filling-a-rectangle-leaves-the-items-it-doesnt-touch-alone.md`).
2. **The job:** a menu opened over a map full of sprites costs the
   frame about 800,000 instructions more than it should, because every
   row it fills walks and rebuilds every sprite the map drew.
3. **Friction numbers:** no friction review: a cost found by the game's
   `bench` job (frame 43 of its `battle-attack` scenario, 2,305,584
   instructions against a budget of 2,000,000; Callgrind put 913,299 of
   them in `GlyphBuffer::cut_items`).
4. **The generic shape:** `GlyphBuffer::fill_rect` (and everything that
   calls `cut_items`) leaves an item whose visible part and overhang the
   replaced cells miss as it is, with no new `Vec` and no rebuild, and
   does nothing to the item list when no item is met.
5. **An example from a game that isn't tactics:** a farming game's
   inventory panel drawn over a field of crop sprites: each row of the
   panel fills a rectangle that meets few of the crops.

`cut_items` rebuilt `self.items` on every call: for each item,
`subtract(item.visible(), hole)` returned a new `Vec` and the item was
rebuilt with `with_visible`, also when the item didn't meet the hole.

## Nick input

None.

## Scope

**In:**
- `cut_items` (`crates/ui/src/frame/glyph_buffer.rs`) skips the items
  whose visible part meets neither the hole nor their overhang, and
  returns without touching the list when none do.
- A unit test: an item away from the fill is kept as it was, and a fill
  that meets nothing keeps the list itself.

No API change. The game's PR that moves the pin follows this one and
measures its `bench` numbers.

**Out (do not do):**
- Any other drawing cost (the menu widget's paint, the map skin).
- The one-bit drawing.

## Implementation steps

1. Change `cut_items`.
2. The test; the existing `GlyphBuffer` and sprite tests and property
   tests pass unchanged.

## Acceptance criteria

- [x] An item away from a filled rectangle is kept as it was, and a fill
      that meets no item keeps the list (unit test).
- [x] Every existing `GlyphBuffer` test and property test passes
      unchanged.
- [x] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the item kept as it was; the list kept when nothing is met.
- Property: the existing `fill_rect` and sprite property tests.
- Snapshot / integration: none new; every snapshot unchanged.

## Completion notes

`cut_items` finds the first item the hole (or that item's overhang)
meets and returns at once when there is none, so a fill over cells with
no items keeps the item list itself. Otherwise it builds the new list
once: the items before the first one met are copied, an item the hole
misses is copied as it was, and only the items met go through
`subtract` and `with_visible`. The result is the same list as before, in
the same order; the existing unit and property tests pass unchanged.

Test: `a_fill_keeps_the_items_it_misses_as_they_were`.

Book: no change; this is an internal cost, nothing a game calls changes.

No API change. The game's PR that moves the pin and measures the
`bench` numbers is game#1261's.
