---
id: "2311"
title: "Glyph drawing helpers: a box with a title in its border, a meter bar, and text centred in or cut to a width"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-19
---

# 2311 — Glyph drawing helpers

## Context

- **The game ticket:** game#1165
  ([`1165-the-games-skins-on-the-engines-drawing-helpers.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1165-the-games-skins-on-the-engines-drawing-helpers.md))
  waits for this one. Found by the game's first engine-friction sweep,
  game#1153.
- **The job:** "paint a panel with a title, a bar that shows how full
  something is, and a label centred in its place".
- **Friction:** found across eight of the game's glyph skins (its main
  screen's skin, 651 lines against 500, 1.3×, and its panels at 1.1× to
  1.2×), outcome 1. Ranked sixth of the sweep's findings. In the game:
  - a `█`/`░` bar loop written 7 times;
  - a label centred by `i32::try_from(text.chars().count())` 16 times;
  - a framed box with a title set into its top border 3 times
    (`draw_box`, then a `fill_rect` of spaces, then `format!(" {} ")`
    printed over the border);
  - two helpers that re-write `GlyphBuffer::print_fg` plus
    `screens::cut` because the game didn't find them. That part is the
    game's to fix, but it says the helpers are hard to find.
- **The generic shape:** on `GlyphBuffer` (or in a small
  `frame::draw` module):
  - `panel(rect, style, title: Option<&str>, fg, bg)`: clear, frame, title
    in the top border;
  - `meter(x, y, cells, filled: u32, of: u32, full: Cell, empty: Cell)`:
    rounds the same way everywhere (a bar that is not empty shows at
    least one full cell, a bar that is not full at least one empty
    one; check the game's copies for the rule they share and document
    it);
  - `print_centred(rect_or_row, text, fg)` and `print_cut(x, y, width,
    text, fg)` (the latter is `print_fg` plus `cut`, named so it is
    found).
- **Outside tactics:** a farming game's stamina bar and its shop
  window; a platformer's boss health bar and its pause panel.

## Nick input

None.

## Scope

**In:**
- The four helpers, documented, each with a doc example.
- The engine's own skins use them where they repeat these lines today
  (options, Key bindings, the layout picker, the menu), with no change
  to any snapshot.
- A line in the frame module's docs listing the drawing helpers, so a
  game finds them.

**Out (do not do):**
- Wide glyphs (2301) and line breaking (2302); a centred line counts
  cells as `print_fg` does today.
- One-bit patterns (3006).
- Any game's skins.

**For the game:** additive. The game's PR that moves its pin follows
this one (game#1165).

## Implementation steps

1. Read the engine's skins for the same lines (`grep -rn "draw_box\|chars().count()" crates/ui/src`).
2. Write the helpers with unit tests of a look (cells).
3. Use them in the engine's skins; snapshots unchanged.

## Acceptance criteria

- [x] Unit (look): a meter at 0, 1, half, n-1 and n of n; a title
      longer than the box is cut; centring an odd and an even width.
- [x] The engine's snapshots are unchanged.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit tests of a look, reading cells.

## Completion notes

- New module `frame::draw` (`crates/ui/src/frame/draw.rs`), methods on
  `GlyphBuffer`:
  - `panel(rect, &PanelLook, title: Option<&str>)`: clears, frames, sets
    the title in the top border two cells in, cut so the border keeps a
    line and a corner each side;
  - `meter(rect, filled, of, full, empty)`, with `meter_fill(cells,
    filled, of)` for a meter drawn some other way (a pixel bar);
  - `print_centred(rect, text, fg)` and `print_cut(x, y, width, text, fg)`,
    both keeping the background like `print_fg`.
  - `cut` moved here from `screens`, which re-exports it (no break).
- `frame`'s docs list the helpers.
- The options, Key bindings, layout picker and menu skins (and Key
  bindings' choices box) paint their boxes with `panel`. Every snapshot is
  unchanged.
- **Deviations:**
  - `panel` takes a `PanelLook` (style, border, title, text and
    background colours) instead of `fg, bg`. The engine's panels clear to
    the text colour but frame in the border colour and title in a
    highlight, and the snapshots read every cell's colour.
  - `meter` takes a `Rect` (every row is a bar) instead of `x, y, cells`,
    which keeps it under clippy's argument limit.
  - The game's copies don't share one rounding rule: its HP bars round
    half up (`hp_fill`) and its EXP bars round down (`bar_cells`), and
    neither keeps a cell for "almost empty" or "almost full". `meter_fill`
    rounds half up and then applies the ticket's rule (not empty: at least
    one full cell; not full: at least one empty one; one cell: rounding
    alone). Moving the game onto it changes a bar at its edges, which is
    game#1165's to accept.
- `screens::print_centred` (prints with a background, centred on the
  whole buffer) is kept as it is; the engine's help lines still use it.
- **For the game:** additive. The game PR that moves the pin is
  game#1165's (not opened yet).
- Follow-up tickets: none.
