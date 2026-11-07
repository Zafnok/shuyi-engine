---
id: "3210"
title: The options panel rises when its rows leave no room for the message
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-06
---

# 3210 — The options panel rises when its rows leave no room for the message

## Context

1. **The game ticket:** `game#0859`, auto-advance when the voice ends
   (github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0859-auto-advance-when-the-voice-ends.md).
   It adds one row to the game's Options.
2. **The job:** a long options list still shows the message under it
   ("Defaults restored", a refused change's reason).
3. **No friction review:** a bug found by the game's tests.
4. **The generic shape:** the ready-made options screen's glyph skin
   (`crates/ui/src/screens/options/glyph.rs`) places its panel at
   `PANEL.y` (row 3) and grows it downward to fit its rows. With 23 or
   more lines (rows and group gaps) on a 32-row frame, the panel's
   bottom reaches the row the message is painted on, and the help line
   paints over the message, so the player never sees it. The three
   blank rows above the panel are unused.
5. **Another game:** any game whose options list is long, a farming
   game with many accessibility and audio rows.

## Nick input

None.

## Scope

**In:**
- `panel()` raises the panel by the rows it lacks, into the rows above
  `PANEL.y`, never above row 1 (`TOP_MIN`). A panel that fits is placed
  exactly as before.
- No public API change. The game's PR that moves the pin follows this
  one.

**Out (do not do):**
- Scrolling the rows when even row 1 isn't enough.
- Any other screen's layout.

## Implementation steps

1. `TOP_MIN` and the raised `y` in `panel()`.
2. A test of 22 to 25 lines: the top border, the bottom border, the
   message and the help line each where they belong.

## Acceptance criteria

- [x] A panel of 22 lines sits at row 3; 23 at row 2; 24 and 25 at row 1; the message is one blank row under the panel and the help line is untouched (unit test).
- [x] Panels that already fit are painted as before (the existing tests pass unchanged).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `a_taller_panel_rises_so_the_message_fits`.

## Completion notes

Done as planned: `panel()` lifts the panel by `h - room`, clamped to
`PANEL.y - TOP_MIN`; one unit test. The game's PR that moves the pin to
this change is Zafnok/visions-of-shuyi#563 (game ticket 0859).

Book: none, a layout detail of the ready-made options screen that no
chapter describes.
