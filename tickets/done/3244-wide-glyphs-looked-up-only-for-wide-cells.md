---
id: "3244"
title: "A narrow cell costs no wide-glyph lookup: the renderers and is_wide skip the wide sets for it"
type: bug
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-24
---

# 3244 — A narrow cell costs no wide-glyph lookup

## Context

- **The game ticket:** game#0236
  ([`0236-wide-glyphs-the-first-wide-font-in-the-atlas.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0236-wide-glyphs-the-first-wide-font-in-the-atlas.md)),
  which puts the game's first wide font in its atlas (a wide set of
  174 glyphs). Its PR, Zafnok/visions-of-shuyi#828, failed the game's
  `bench` check.
- **The job:** draw a frame of Latin text as fast with a wide font in
  the atlas as without one.
- **Friction:** no friction review: a performance budget missed because
  of the engine. The Playdate's `battle-cursor-sweep` frame went from
  1,983,326 to 2,069,579 instructions, over its 2,000,000 budget; the
  desktop's frames grew by about 60,000.
- **The generic shape:** the frame's renderers and the glyph buffer look
  a character up in the atlas's wide sets only when it can be wide.
  - The one-bit renderer (`frame::onebit::Font::cell_rows`) looked every
    non-space cell up in the wide sets (`FontAtlasDef::wide_cell`, a
    search of each set) before it checked whether the next cell is a
    continuation; the raster (`frame::raster`) and `shuyi-app`'s
    renderer (`render.rs`) did the same in `atlas_cell`. With no wide
    glyph in the atlas the search was free, so 2301's tests and
    benchmarks never saw it.
  - `CellWidths::is_wide`, which the buffer calls for every character it
    prints, searched the set for every character.
- **Outside tactics:** any game with a wide font in its atlas and mostly
  Latin text on screen: a roguelike with a Japanese translation, a
  visual novel's English menus beside a Chinese one.

## Nick input

None.

## Scope

**In:** the three renderers look a cell up in the wide sets only when a
continuation follows it; `CellWidths::is_wide` answers a character
before the set's first wide one without a search.

**Out (do not do):** any change to what is drawn or measured; the atlas
format; the fallback for a wide glyph printed without its continuation
(it still draws the narrow glyph or the fallback, 2301).

For the game: no API change; the game's PR #828 moves its pin to this
PR's merge.

## Implementation steps

1. `onebit.rs`'s `cell_rows`, `raster.rs`'s and `render.rs`'s
   `atlas_cell`: test `wide` before the lookup.
2. `CellWidths` keeps the set's first character (`first_wide`) and
   `is_wide` compares with it before the search; `Default` is written by
   hand as `NARROW`, so the two stay equal.

## Acceptance criteria

- [x] A wide glyph still draws over its two cells, and one printed
      without its continuation still draws narrow (the existing one-bit,
      raster and render-card tests).
- [x] `is_wide` answers a character before the first wide one narrow,
      a set's member wide, a character between members narrow, and the
      wide punctuation by the language (unit test).
- [x] The game's Playdate `bench` frame is back under budget (its PR's
      CI).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `is_wide_answers_a_character_before_the_first_wide_one`
  (`crates/assets/src/font.rs`).
- The renderers' existing wide-glyph tests cover what is drawn.

## Completion notes

- **What was done.** The one-bit renderer, the raster and `shuyi-app`'s
  renderer check that a continuation follows a cell before they look it
  up in the wide sets. `CellWidths` keeps `first_wide`, the first
  character of its set (`char::MAX` when it has none), and `is_wide`
  answers a character before it narrow at once; `Default` is
  `CellWidths::NARROW`, written by hand, as a derived one would have set
  `first_wide` to `'\0'`.
- **Deviations.** None. No public API changes (`first_wide` is private;
  `Default` is still implemented).
- The game's bench numbers after the move are in its PR.

Game: Zafnok/visions-of-shuyi#828 (game#0236) moves the pin to this PR's merge.

Book: not updated: nothing a reader of the book does changes.
