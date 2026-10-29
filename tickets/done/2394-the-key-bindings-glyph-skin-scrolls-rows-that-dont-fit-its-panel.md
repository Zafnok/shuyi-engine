---
id: "2394"
title: The Key bindings screen's glyph skin scrolls rows that don't fit its panel
type: feature
milestone: MVP
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-29
---

# 2394 — The Key bindings screen's glyph skin scrolls rows that don't fit its panel

## Context

1. **The game ticket:** `game#1007`
   ([1007](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1007-world-map-nodes-travel-towns.md)),
   the game's world map, adds two actions in a key group of their own
   (jump to the army, a find list), and with them a third group of rows
   on the Key bindings screen.
2. **The job in one sentence:** a Key bindings screen with more actions
   than its panel has rows still shows every one, the list moving to the
   one the cursor is on.
3. **No friction review:** a missing feature. The glyph skin laid every
   group, action and Restore defaults one under the other with no limit,
   so on a full 100 × 32 console the game's new rows ran over the panel's
   bottom border and Restore defaults went off the panel.
4. **The generic shape:** a list of rows in a fixed panel that scrolls
   to keep its focused row in view, with marks for the rows hidden above
   and below.
5. **An example from a game that isn't tactics:** a farming game with
   tool, menu and photo-mode actions, or a racing game with driving,
   camera and replay actions, lists more rebindable actions than fit.

## Nick input

None.

## Scope

**In:**
- The glyph skin (`screens/key_bindings/glyph.rs`): when Restore defaults
  falls below the panel's last inner row, the list moves up just enough
  that the focused action (or Restore defaults) is on that row or above;
  rows outside the panel aren't drawn; `▲` on the right border at the first
  row and `▼` at the last say rows are hidden above or below. The choices
  box opens beside the focused row where it is shown.
- A list that fits draws as before.

**Out (do not do):**
- The screen's logic and view: they don't change (the scroll is the
  skin's, from the view's focus).
- The game's Playdate skin (it has its own).

## Acceptance criteria

- [x] A list that fits doesn't scroll and shows no marks.
- [x] A longer list starts at its top with `▼`; focusing Restore
      defaults puts it on the last row with `▲`; nothing is drawn over
      the panel's border.
- [x] A focused action below the panel is scrolled up to its last row,
      its slot bar there.
- [x] The book's chapter needs no change (the Key bindings screen's
      look isn't described there): `Book:` line below.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot / look: `glyph/tests.rs`, the three tests above on a view
  with its optional rows listed twice more.

## Completion notes

- `place` still lays the rows out from the top; `scroll` decides how far
  to move them from the view's focus, and `shown` drops the rows outside
  the panel. No state: the list is where the focus puts it.
- No break: nothing public changed.
- Book: no change, the book doesn't describe the screen's look row by row.
- The game's PR that moves the pin past this one is `game#1007`'s
  follow-up ticket (the game ships 1007 with its world-map rows left off
  the PC screen until then).
