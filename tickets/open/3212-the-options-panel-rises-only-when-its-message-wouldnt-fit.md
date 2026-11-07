---
id: "3212"
title: The options panel rises only when its message wouldn't fit
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3212 — The options panel rises only when its message wouldn't fit

## Context

1. **The game ticket:** `game#0859`, whose new Options row 3210 made room
   for (github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0859-auto-advance-when-the-voice-ends.md).
   Found by the game's pin move of 2026-11-07.
2. **The job:** a panel that already fits stays where it was.
3. **No friction review:** a bug in 3210.
4. **The generic shape:** 3210's `panel()` in
   `crates/ui/src/screens/options/glyph.rs` raises the panel by
   `h - room`, where `room = height - PANEL.y - BELOW_PANEL - 1`. The
   message needs only `panel.y + panel.h + 1 <= height - 2` (a blank row,
   the message, the help line: `BELOW_PANEL`), so `room` is one row
   short of what fits: a panel of exactly `room + 1` rows, which fitted
   at `PANEL.y` with its message before 3210, now rises one row. On a
   32-row frame a 23-line panel moves from row 3 to row 2 for nothing.
   (3210's test of 22 to 25 lines expects that rise, so it encodes the
   bug.)
5. **Another game:** any game whose options list is exactly that long.

## Nick input

None.

## Scope

**In:** the rise is `h - (height - PANEL.y - BELOW_PANEL)`, clamped as
now; the test's table corrected (23 lines at row 3, 24 at row 2, 25 at
row 1). No API change; the game's next pin move updates its snapshots.

**Out (do not do):** anything else of the layout.

## Implementation steps

1. Fix the rise in `panel()`; leave the `PANEL.h.min(room)` cap as it is.
2. Fix `a_taller_panel_rises_so_the_message_fits`'s table and add 26 lines
   (row 1, the message on row 30).

## Acceptance criteria

- [ ] A panel whose message fits at `PANEL.y` isn't moved (unit test).
- [ ] Taller panels rise one row a line, to row 1 (unit test).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the corrected table.

## Completion notes

*(Filled in by the session that completes the ticket.)*
