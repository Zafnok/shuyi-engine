# ADR-0139: A frame may fill the window at a fraction of a pixel, drawn sharp, from one placement the window and the check share

- **Status:** Accepted
- **Date:** 2026-11-23
- **Related tickets:** 2333; 3240 (the player's pick), 3241 (the capture
  of a filled frame); game#1320 (the request)
- **Amends:** ADR-0008 §1 (the `desktop` row's "any, integer scale"), and
  for the engine game ADR-0012's integer-only rule

## Context

A frame was placed only at the largest whole scale that fits, letterboxed
(`layout` in `crates/ui/src/frame/console.rs`, game ADR-0012). For a
game's 800 × 512 frame (100 × 32 cells of 8 × 16) that is 1× on a
1280 × 720 window: an 800 × 512 picture in bars, 71 % of the height.
Visions of Shuyi's owner reported exactly that (game#1320, 2026-11-22). A
1280 × 800 Steam Deck shows 64 %, a 1366 × 768 laptop 67 %, a 1440p
screen 71 %. A browser's canvas can be any size, so the web build is the
same.

`shuyi.ron` already let a target say `scale: fit`, and `check --target`
checked a `fit` frame's text at, say, 1.56×, while the window drew it at
1×: the check promised what the window didn't do (ticket 2333).

The benchmarks offer both: Godot's integer and fractional scale modes,
Unity's Pixel Perfect Camera (a whole scale into a texture, then
stretched), Construct's letterbox and letterbox integer scale.

Stretching a pixel picture by a fraction has three known looks:
nearest-pixel sampling makes columns of uneven widths (an 8-pixel glyph
is 11 pixels wide here and 12 there at 1.41×); plain linear filtering
blurs every edge over a whole source pixel; and "sharp bilinear" (draw
at a whole scale first, then filter linearly down or up to the size)
keeps each pixel's inside flat and blends its edge over at most one
window pixel.

On 1080p and 4K the whole scale already fills about 95 % of the height
(2× and 4× of 512 against 1080 and 2160). There a fraction would trade a
pixel-perfect picture for 5 % more of the screen.

## Decision

1. **One placement, in whole numbers**, in `shuyi_ui::frame::console`
   (`no_std`, no floats): `ScaleMode { Integer, Fit }` (`Integer` the
   default; serde names `integer` and `fit`, those `shuyi.ron` uses),
   `Factor { num, den }` (moved from the CLI), `scale(mode, frame,
   space)` and `place(mode, frame, window) -> Placement { scale,
   draw_scale, x, y, w, h }`:
   - `Integer`: the largest whole scale that fits, at least 1 (a
     too-small window crops the edges evenly, as before);
   - `Fit`: the largest fraction that fits, below 1 too, the limiting
     side equal to the window's and the other rounded down; **but the
     whole scale when it is at least `SNAP` (9 / 10) of the fraction**;
   - centred with `div_euclid`, a leftover odd pixel rounded down;
   - `draw_scale` is the whole scale the frame is drawn at before any
     stretch: the scale itself when whole, else it rounded up, at least
     1, lowered while a side would pass `MAX_DRAW_PX` (4096, a texture
     side nearly every GPU takes).

   `layout` and `Layout` keep their signatures and values: `layout` is
   `place(Integer, …)`. `Layout::frame_px` is `Placement::frame_px` at a
   whole scale; `Placement::frame_px` inverts a fraction too (frame x =
   (window x − x) × den / num, rounded down), so the pointer (ADR-0134)
   follows the picture.

2. **`check --target` uses the same function**: `project::Scale` is
   `ScaleMode`, and `scale_on` is `console::scale`, so the text is
   checked at the scale the window draws, the snap included. With the
   game's tools it compares each asked target's `scale` with the game's
   default (`GameTools::scale`) and fails a difference on
   `targets."<id>".scale`; without them, its limits say the scale wasn't
   compared.

3. **The game picks, the frame carries it**: `GameSettings::DEFAULT_SCALE`
   (an associated const, `Integer` unless the game says otherwise), set
   on `Frame::scale` by `present_game`, and on the loading card's frame,
   since the player's settings aren't read yet. A headless frame keeps
   it (`Presented::Game`'s `scale`, `Harness::scale`). A player's own pick
   is a setting of its own (ticket 3240); until then the game's default
   is what is drawn.

4. **`shuyi-app` draws it sharp**: a placement whose rectangle is the
   frame times a whole scale is drawn as before, straight into the
   window, pixel for pixel the same. Otherwise the frame is drawn at
   `draw_scale` into a render target (kept between frames, made again
   when its size changes, its texture filtered linearly), the backdrop's
   window on that target too, and the target is drawn into the
   placement's rectangle: each logical pixel's edge blends over one
   window pixel at most (two when `MAX_DRAW_PX` lowered `draw_scale`
   below the scale). The loading card's picture takes the same
   placement and the same path. The web build runs the same code.

5. **What stays whole**: logical pixels, glyph scales, a backdrop's zoom
   and camera zooms. Only the backend's last step scales by a fraction.
   `frame-png` and the raster stay at whole scales; a backend that shows
   its display 1:1 (the Playdate) ignores `Frame::scale`.

## Consequences

- A game that sets `DEFAULT_SCALE = Fit` fills most windows: an
  800 × 512 frame is 1125 × 720 on 1280 × 720, 1250 × 800 on a Steam
  Deck, 1200 × 768 on a 1366 × 768 laptop, 2250 × 1440 on 1440p; and
  stays a sharp 2× on 1080p and 4× on 4K.
- Its glyphs at a fraction have soft edges of one window pixel; the
  check and the window agree on its scale, so the text size it checks is
  what shows.
- Every other game sees nothing change: `Integer` is the default, and
  the integer path draws the same pixels.
- The render target costs one texture of the frame at `draw_scale` (an
  800 × 512 frame at 2×: 1600 × 1024 RGBA, 6.5 MB of video memory) and
  one more draw a frame, only while a fraction shows.
- `check --target` now fails a game whose tools report a scale other
  than its targets': `shuyi.ron` says the truth or the check says so.
- A filled frame's window isn't compared with the raster in CI until
  ticket 3241.

## Alternatives considered

- **Nearest-pixel stretching to the fraction** — uneven pixel widths
  make glyphs wobble as text scrolls; the benchmarks avoid it too.
- **Plain linear filtering of the 1× picture** — every edge blurs over a
  whole source pixel: an 8 × 16 glyph turns grey.
- **`Fit` that always fills, no snap** (ticket 2333 as first written) —
  1080p and 4K, the most common desktop screens, would lose their
  pixel-perfect picture for about 5 % of the height.
- **A console that grows by whole cells** (2376, parked, after 2323) —
  sharp at every size, but every screen of every game must lay out
  against a console of any size; the frame filled by a fraction asks
  nothing of the screens. Both can live together later.
- **A shader that does sharp bilinear in one pass** — needs a material
  per draw for every glyph and sprite, and shader code per backend; the
  render target is macroquad's own API and works on the web.
