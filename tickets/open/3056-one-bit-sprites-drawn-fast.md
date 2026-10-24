---
id: "3056"
title: "One-bit sprites drawn fast: whole bytes for a picture shown pixel for pixel, and no division per pixel otherwise"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] Every sprite renders equal to `reference` (property tests over the
      cases in step 5).
- [ ] The fight-like frame of step 2 renders at least 4 times faster on
      the PC than before; the numbers are in the completion notes.
- [ ] No public API change (CI's `api` job).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: pixel-for-pixel detection (on and off by one pixel of size), a
  flipped sprite's first and last byte, a sprite starting and ending
  inside a byte.
- Property: equality with `reference` over random sprites.
- Speed: the fight-like frame before and after (`speed_tests.rs`).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
