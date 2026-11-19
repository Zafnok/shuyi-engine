---
id: "3226"
title: The options panel's value column leaves room for the longest label and its arrow
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-19
---

# 3226 — The options panel's value column leaves room for the longest label and its arrow

## Context

1. **The game ticket:** `game#1302`
   (github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1302-options-focused-arrow-covers-a-long-label.md).
2. **The job:** on the Options screen, a focused row's `◄` never covers
   the end of a long label.
3. **No friction review:** a bug.
4. **The generic shape:** the options glyph skin
   (`crates/ui/src/screens/options/glyph.rs`) puts every value at the
   fixed column `VALUE_DX` (30) and a focused value's `◄` two cells left
   of it, on the 25th cell of the label column. A label of 25 cells or
   more is covered by the arrow (or runs into the value). The value
   column should start far enough right for the longest label, a space
   and the arrow.
5. **Another game:** any game with a long option name, or a translation
   longer than the English (a farming game's "Show crop growth timers").

## Nick input

None.

## Scope

**In:** the value column is `VALUE_DX` at the least, and further right
when the view's longest label (in cells) needs it, the same for every row
so the values stay lined up. No API change; a game whose labels are 24
cells or fewer sees no change.

**Out (do not do):** the screen's logic or view; any other skin.

## Implementation steps

1. A `value_dx(view, buf)` from the longest label; `paint_row` takes it.
2. Tests: the longest label focused is painted whole, with its arrow
   after it; short labels keep the old column.

## Acceptance criteria

- [x] A focused 25-cell label is painted whole, its `◄` right after it.
- [x] Labels of 24 cells or fewer paint as before (snapshots unchanged).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a view with a 25-cell label, focused, painted whole.

## Completion notes

`value_dx` puts the values' column at `VALUE_DX`, or at the longest
label's width plus two cells from the label column when that is further
right (every row of the view, so it doesn't move as the list scrolls), and
`paint` hands it to `paint_row`. A focused label then ends right before its
`◄`, as a 24-cell label always did. Test:
`the_longest_label_focused_is_painted_whole` (24, 25 and 32 cells, the
other rows' values lined up, shorter labels on the old column).

Game: `game#1302` adopts it (its PR moves the pin).

No API change; labels of 24 cells or fewer paint as before (the snapshots
are unchanged). The column isn't capped by the panel's width: a label long
enough to push a slider past the border would need the panel to grow,
which no game needs yet.

Book: no chapter describes the options skin's layout.
