---
id: "3225"
title: The options panel's rows scroll when they don't fit above the message
type: bug
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-19
---

# 3225 — The options panel's rows scroll when they don't fit above the message

## Context

1. **The game ticket:** `game#1301`
   (github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1301-options-list-scrolls-when-it-outgrows-the-screen.md).
2. **The job:** a long Options list scrolls to keep the focused row in
   view, so the message under the panel and the help line always show.
3. **No friction review:** a bug left by 3210 and 3212.
4. **The generic shape:** the options glyph skin
   (`crates/ui/src/screens/options/glyph.rs`) grows its panel with its
   rows and raises it, up to `TOP_MIN`, so the message fits under it.
   Past that the panel keeps growing: it runs into the message's row and
   the help line, and the message is not painted at all. The game's
   build with pictures has 30 lines (rows and group gaps) for the 26 a
   100 × 32 console holds above the message.
5. **Another game:** any game with many options (a farming game's
   accessibility settings, a platformer's video settings).

## Nick input

None.

## Scope

**In:** at `TOP_MIN` the panel grows no more; its rows scroll as the Key
bindings skin's do (the list moves just enough to keep the focused row
on the panel), with a `▲` or `▼` on the panel's right border where rows
are hidden. No API change; a game whose options already fit sees no
change.

**Out (do not do):** the screen's logic or view; any other skin.

## Implementation steps

1. `panel()` caps the height at the rows from `TOP_MIN` to the message's
   blank row.
2. `paint` draws the window of lines from `first_shown`, and the marks.
3. Tests: more lines than fit scroll, the focused row on the panel for
   every focus, the message's row free.

## Acceptance criteria

- [x] With more rows than fit, the message and the help line show.
- [x] The focused row is always on the panel.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `first_shown`; a 30-line view painted at each focus.

## Completion notes

`panel()` caps the panel at the rows from `TOP_MIN` to the message's
blank row; `paint` draws the window of lines `first_shown` picks (the
first line until the focused row passes the last line, then just enough
to keep it there, as the Key bindings skin does) and a `▲` / `▼` on the
right border where lines are hidden. Tests: `first_shown`, a 30-line
view scrolled at both ends with the message on row 30, and the focused
row on the panel for every focus.

No API change; a list that fits paints as before (the snapshots are
unchanged). The game's PR for `game#1301` moves the pin to this one.

Book: no chapter describes the options skin's layout.
