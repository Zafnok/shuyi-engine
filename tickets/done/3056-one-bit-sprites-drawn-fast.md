---
id: "3056"
title: "One-bit sprites drawn fast: whole bytes for a picture shown pixel for pixel, and no division per pixel otherwise"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-28
---

# 3056 — One-bit sprites drawn fast

## Context

1. **The game ticket:** game#1041, the Playdate's combat picture
   ([`tickets/done/1041-playdate-combat-picture.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/1041-playdate-combat-picture.md)),
   and game#1078, which moves the pin past this ticket and measures
   again. Neither blocks this one.
2. **The job:** draw a backdrop across the screen and two big pictures
   moving over it, every frame, on the one-bit screen.
3. **The friction numbers:** a performance budget missed (game ADR-0092's
   signal), not a line count: the game's change was 317 lines against an
   estimate of 200 (1.6×). On the Playdate (Rev B, release build,
   2026-10-23), while a fight plays, a drawn frame takes **136–169 ms on
   average and up to 640 ms** to render and copy, against 33 ms at 30
   frames a second; the device drops frames and the audio hitches. The
   frame is a 400 × 110 backdrop sprite and two sprites of up to
   160 × 192 pixels, about 95,000 sprite pixels.
4. **The generic shape:** `crates/ui/src/frame/onebit.rs`'s
   `draw_sprite` works a pixel at a time: for every pixel it calls
   `Image::sample` (a 64-bit multiply and divide; the Cortex-M7 has no
   64-bit divide, so it is a library call), reads one bit and writes one
   bit (`Target::set`). The rest of the renderer already writes whole
   bytes (ticket 1055's work, game ADR-0077). A sprite that shows its
   picture **pixel for pixel** (each screen pixel one picture pixel, as
   a one-bit picture enlarged ahead of time to its shown size is,
   `shuyi_assets::bitmap`) can be drawn a byte at a time: the picture's
   row shifted to the screen's alignment, its mask, the opacity's and
   brightness's patterns and a solid paint's pattern each a byte (they
   repeat within 8 pixels), written as `out = out & !m | bits & m`. Any
   other sprite still needs a sample per column, but once per sprite (a
   column table), not once per pixel.
5. **Not only tactics:** a visual novel's character standing over its
   background, a platformer's boss, a title screen's backdrop on a
   one-bit target all draw a few big pictures every frame.

## Nick input

None.

## Scope

**In:**
- In `onebit.rs`, a fast path in `draw_sprite` for a sprite whose `src`
  maps onto its picture pixel for pixel (at the target's scale), with or
  without `flip_x`, a mask or not, any opacity, brightness and paint:
  whole bytes where it can, the pixels at its two ends as now.
- For every other sprite: each shown column's picture column worked out
  once per sprite (`Image::sample` per column, not per pixel), with no
  allocation (`Nothing here allocates`: a fixed array the width of the
  screen).
- The result is **equal to `reference`** (the plain renderer) for every
  sprite, as the renderer's tests already require.
- No public API change.
- **Either order with 2335** (turned sprites): if 2335 is done, the
  byte path and the column table take `flip_y` too (the byte path picks
  the picture's row from the bottom, as `flip_x` picks the column), and
  a sprite at an angle other than 0 skips both for 2335's `dest_pixel`
  path; the property tests add `flip_y` and angles. If not, nothing to
  do (2335 adds `flip_y` to whichever path exists, its clause says
  so).
- **Either order with 2337** (tint and blend modes): if 2337 is done,
  the byte path and the column table draw `Paint::Tinted` as `Image`,
  `Add` as the item's white pixels only (`out | bits & m`) and
  `Multiply` as its black ones only (`out & (bits | !m)`); the property
  tests add tinted, added and multiplied sprites. If not, nothing to do
  (2337 adds them to whichever path exists, its clause says so).

**Out (do not do):**
- Other items, the glyphs, `render_changed`'s rows.
- The game's measuring on the device (game#1078).
- A different picture format or packing (3007's, 2222's).

## Implementation steps

1. Read `crates/ui/src/frame/onebit.rs` (`draw_sprite`, `Image`,
   `Image::sample`, `shows`, `Target`), its `tests.rs`, `speed_tests.rs`
   and `reference.rs`; game ADR-0077 and ticket 1055 for how fills and
   glyphs write whole bytes.
2. A test first: the speed test of a frame like the game's fight (a
   400 × 110 sprite from a 400-pixel-wide picture of a 336-pixel PNG,
   and two 160 × 160 sprites, one flipped, both masked), timed before the
   change.
3. The column table for the general path.
4. The pixel-for-pixel test (exact integer arithmetic: sampling `d`
   lands on `start × size / source + d` for every `d` of the shown
   range) and the byte path, with the ends a pixel at a time.
5. Property tests: random sprites (positions not on a byte, clips,
   flips, masks, opacities, brightness, `Solid` and `Dimmed` paints,
   pixel-for-pixel and not) render equal to `reference`.
6. Record the speed test's before and after in the completion notes.

## Acceptance criteria

- [x] Every sprite renders equal to `reference` (property tests over the
      cases in step 5).
- [x] The fight-like frame of step 2 renders at least 4 times faster on
      the PC than before; the numbers are in the completion notes.
- [x] No public API change (CI's `api` job).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: pixel-for-pixel detection (on and off by one pixel of size), a
  flipped sprite's first and last byte, a sprite starting and ending
  inside a byte.
- Property: equality with `reference` over random sprites.
- Speed: the fight-like frame before and after (`speed_tests.rs`).

## Completion notes

- **What was done.** `draw_sprite` (`crates/ui/src/frame/onebit.rs`)
  now writes a byte of the frame at a time, for every sprite. Which
  picture column each shown screen column takes is worked out once a
  sprite (`Columns`):
  - `Columns::Run`, pixel for pixel: at scale 1 with `src` from a
    column of 0 or more, when the first and last shown columns are as
    far apart in the picture as on the screen. That is exact: the
    sampled columns are then floors of a line, stepping by 0 or 1 each
    or by 1 or more, so steps that add up to as many as there are are
    each 1. (A `src` from left of the picture is sampled rounding
    towards 0, which can make columns 0, 0, 2: no run.) A row's eight pixels then come out of the picture's row in one
    shift (`byte_at`), flipped with `reverse_bits`, its mask and the
    picture's edges (`on_picture`) the same way.
  - `Columns::Table` otherwise: a `[u16; 400]` on the stack, one
    `Image::sample` per column, then a table lookup per pixel. Nothing
    allocates.

  The opacity's and brightness's dithers are a byte a row
  (`shows_byte`, 4 rows each sprite), the solid and dimmed paints'
  patterns too, and each byte is written as `out & !m | bits & m`, its
  two end bytes masked to the shown pixels. The picture's row is
  still sampled once a row. `Target::set` is gone; `Image::pixel` is
  kept for `reference` only.
- **Speed** (this PC, `cargo test -p shuyi-ui --lib a_fight_draws_fast`,
  the least of 9 tries of 4 renders; the frame is the ticket's: a
  400 × 110 backdrop from a 400-pixel-wide picture of a 336-pixel PNG,
  and two masked 160 × 160 fighters enlarged twice, one flipped, neither
  on a byte):

  | build | before | after | plain renderer, same run |
  | --- | --- | --- | --- |
  | release | 1.07 ms | 60.7 µs | 805 µs |
  | test (debug) | 4.02 ms | 0.53 ms | 3.15 ms |

  13 times faster than the plain renderer in the same run (the fast one
  was as slow as it before), 17 times faster than before's measured
  number. `speed_tests/timing.rs` asserts the 4 times against the
  plain renderer on every test run (`std` only, as it reads the clock).
- **Tests.** `sprites_draw_as_the_reference_does`: random sprites only
  (positions off a byte, clips, flips, masks, opacities, brightness,
  `Image`, `Dimmed` and `Solid` paints, half of them pixel for pixel,
  a quarter of `src` sides reaching past the picture, four pictures
  with a new 45 × 7 one with and without a mask, many bytes a row)
  over grey
  or a backdrop's scene at zoom 1 to 4, in inks and in roles. The
  general property tests draw the new picture and pixel-for-pixel
  sprites too. Unit tests: pixel-for-pixel detection on and a pixel off
  (picture and screen), a clipped run, a run off the picture, and a
  scale-2 run and one from left of the picture that only land their
  ends right (`pixel_for_pixel_is_found_exactly`, and the latter drawn
  in `a_sprite_from_left_of_its_picture_draws_as_the_reference`); a
  flipped sprite's first and
  last byte; sprites starting and ending inside one byte;
  `a_fight_draws_as_the_reference` as the fighters move.
- **Deviations.** The byte path's ends are masked bytes, not pixels one at
  a time: the same pixels, and simpler. The general path writes bytes
  too (from the table), not pixels.
- **2335 and 2337** are open, so neither `flip_y`, angles nor the new
  paints were added. Their clauses now say what holds.
- **Docs.** None beyond the module's doc comment: no public item,
  format or rule changed, only how fast the one-bit renderer draws.
- **The game.** game#1078 moves the pin past this and measures on the
  device; no game PR exists yet.

No break.
