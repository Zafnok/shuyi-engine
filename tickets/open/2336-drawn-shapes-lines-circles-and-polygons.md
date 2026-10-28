---
id: "2336"
title: "Drawn shapes for every game: lines, circles and polygons as frame items, filled or outlined, by one pixel rule in every renderer"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2335"]
nick_input: none
completed:
---

# 2336 — Drawn shapes: lines, circles and polygons

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engine GameMaker. Most of them let a game
draw shapes without art:

- Godot: `draw_line`, `draw_circle`, `draw_polygon` and
  `draw_polyline` on any `CanvasItem`
  (https://docs.godotengine.org/en/stable/classes/class_canvasitem.html);
- Bevy: 2D meshes from its shape primitives (`Circle`, `Rectangle`,
  `RegularPolygon` and others), and gizmos for lines and circles
  (https://docs.rs/bevy/latest/bevy/math/primitives/index.html);
- Unity: placeholder sprites (circle, square, triangle and others) and
  a line renderer;
- GameMaker: `draw_line_width`, `draw_circle` and
  `draw_primitive_begin`. PICO-8 (`line`, `circ`, `circfill`) and LÖVE
  (`love.graphics.circle`, `love.graphics.polygon`) do the same.

Shuyi today (`crates/ui/src/frame/glyph_buffer.rs`):

- `Item` (line 305) is a `Rect` (an `Overlay`, line 144: a filled,
  axis-aligned rectangle) or a `Sprite`;
- glyph cells draw box lines in whole cells (`draw_box`, `BoxStyle`);
- nothing in the crates draws a line at an angle, a circle or a
  polygon;
- four renderers draw items, each in a `draw_items`: `frame-png`'s
  `Painter` (`crates/cli/src/frame_png.rs`, line 536), `shuyi-app`'s
  `Renderer` (`crates/app/src/render.rs`, line 229), and the 1-bit
  renderer (`crates/ui/src/frame/onebit.rs`, line 876) and its
  reference (`onebit/reference.rs`, line 237).

Plans:

- 2315 (after the MVP) adds `Item::Gizmo` with one-pixel lines and
  circle outlines, but only when debug tools are on, in debug colours
  from `EngineColor`. It is not a game's drawing item;
- 2703's `Strip` (parked) makes thick lines for procedural animation,
  but no circles or polygons;
- 2217 (parked) says "If the frame has no triangle items by then, add
  them", for textured triangles;
- 3103 (parked) has an integer triangle rasterizer for 3D.

Nick, 2026-10-20 (`docs/decisions.md`): the features most 2D games
expect come after the MVP, not parked, and some take "the turn-game
half of a parked ticket", including "rotated sprites and drawn shapes
from 2703". 2335 is the rotated sprites; this ticket is the shapes.

Why it matters:

- **The game first.** Visions of Shuyi asks for none of this (no open
  game ticket draws a circle or a polygon): its marks are rectangles
  and its pictures are sprites. Nothing it draws changes.
- **Other 2D games:** an aim line or a laser, a range circle around a
  unit, Breakout's ball, a radar sweep, a ship drawn as a polygon in the
  Asteroids style, placeholders before any art exists.
- **The end goal** (2026-10-19; measured on turn-based games until the
  launch): an agent with no art draws a game from shapes in a few
  lines, and the same shapes draw on the desktop, the web and the
  Playdate (in 1-bit patterns, as rectangles are).

It follows 2335's ADR: every renderer decides each console pixel from
its centre, in integers, through one function in `shuyi-ui`. So
`shuyi-app`, `frame-png` and the 1-bit renderers draw the same pixels by
construction. `shuyi-app` does not use macroquad's `draw_line`,
`draw_circle` or `draw_poly`, which work in floats at screen pixels.

It is blocked by 2335 because 2335's ADR writes the pixel rule this
ticket follows.

## Nick input

None. `docs/decisions.md` (2026-10-20) puts drawn shapes after the MVP,
not parked.

## Scope

**In:**
- **`shuyi_ui::frame::shape`** (`crates/ui/src/frame/shape.rs`,
  declared in `frame.rs`), `no_std`, no floats, and no allocation while
  drawing (the 1-bit renderer allocates nothing):
  - `Shape { kind: ShapeKind, color: Rgb, layer: Layer, clip: PxRect }`,
    deriving what `Item` derives (`Copy`, `Clone`, `PartialEq`, `Eq`,
    `Debug`, `Hash`);
  - `ShapeKind`:
    - `Line { from: (i32, i32), to: (i32, i32), width: u8 }`;
    - `Circle { center: (i32, i32), radius: u16, fill: Fill }`;
    - `Polygon { points: [(i32, i32); MAX_POINTS], len: u8, fill: Fill }`,
      with `MAX_POINTS = 16`. A fixed array keeps `Item` `Copy` and the
      drawing free of allocation. Points past `len` are `(0, 0)`, so
      `Eq` and `Hash` only see the real ones;
  - `Fill::Solid` or `Fill::Outline` (one pixel wide);
  - constructors: `Shape::line(from, to, width, color, layer)`,
    `Shape::circle(center, radius, fill, color, layer)`, and
    `Shape::polygon(points: &[(i32, i32)], fill, color, layer) ->
    Option<Shape>` (`None` for fewer than 3 points or more than 16).
    Each sets `clip` to `bounds()`;
  - `Shape::bounds() -> PxRect`: a box holding every console pixel it
    can cover, clamped to `-(1 << 30)..(1 << 30)` on each axis so its
    width and height fit in `i32` (no buffer reaches that far);
  - `Shape::for_each_run(&self, f: impl FnMut(i32, i32, i32))`: calls
    `f(y, x0, x1)` for runs of covered pixels inside `clip`, `x0..x1`
    half-open. Runs may overlap (a circle's rows, an outline's
    corners); every renderer paints them opaque, so that doesn't
    matter.
- **The pixel rule.** The module's doc comment is its spec. In console
  pixels, integers only:
  - A point names a pixel: `(x, y)` is that pixel's centre. Distances
    are measured between pixel centres.
  - **A line of width 1** is Bresenham's, from `from` to `to`, both
    ends drawn: the all-octant form whose error term starts at
    `dx + dy` (`dx = |x1 − x0|`, `dy = −|y1 − y0|`). A line far longer
    than `clip` must not walk its whole length: start at the first step
    inside `clip`, with the minor coordinate there computed in closed
    form (one floor division, in `i128`), so the pixels are those of a
    full walk, and stop where it leaves `clip`.
  - **A line of width `2k + 1`** (3 or more): pixel `P` is covered when
    it is less than `k + ½` from the segment `AB`, so the ends are
    round. Where `P` projects onto the segment
    (`0 ≤ (P − A)·(B − A) ≤ |B − A|²`) that is
    `4·cross(P − A, B − A)² < (2k + 1)²·|B − A|²`; past an end `E` it
    is `4·|P − E|² < (2k + 1)²`. Work in `i128`: the right side is
    always below 2^82, so a left side whose `checked_mul` overflows is
    not covered. Each row is one run, because the shape is convex: seed
    it at the floor or the ceiling of the row's point nearest the
    segment (where the row crosses it, or level with the nearer end),
    and find both ends by binary search on the test.
  - A line through pixel centres is an odd number of pixels across, so
    a width of 0 counts as 1 and an even width as the next odd one, in
    `Shape::line` and in `for_each_run` (a shape built by hand draws the
    same). The snapshot shows the width drawn.
  - **A circle of radius `r`** is `2r + 1` pixels across. Its outline is
    the midpoint circle whose decision value starts at `1 − r`. Filled,
    each row covers every pixel from the outline's leftmost to its
    rightmost pixel in that row, so the fill contains the outline. Both
    come from one walk of one octant (at most `r` steps): each step
    gives its eight mirrored pixels, or its four mirrored runs.
  - **A filled polygon** uses the even-odd rule with half-open edges.
    On row `y`, each edge with `min(y0, y1) ≤ y < max(y0, y1)` crosses
    it at an exact fraction, rounded up to a whole pixel (a ceiling
    division in `i128`). With the rounded crossings sorted
    (`c0 ≤ c1 ≤ …`, in a fixed array of at most 16), the runs are
    `c(2i)..c(2i+1)`. So:
    - the polygon `(x, y)`, `(x + w, y)`, `(x + w, y + h)`,
      `(x, y + h)` covers exactly the pixels of `Overlay` rectangle
      `(x, y, w, h)`;
    - two polygons that share an edge never both cover a pixel;
    - a filled polygon leaves out the pixels on its right and bottom
      edges, which its outline covers.

    Self-crossing polygons are allowed.
  - **An outlined polygon** is its width-1 lines from point to point,
    closing the loop.
- **The frame** (`glyph_buffer.rs`):
  - `Item::Shape(Shape)`;
  - `GlyphBuffer::add_shape(shape)` cuts `clip` to `bounds()` and to
    the buffer, and drops a shape whose `clip` ends empty;
  - `GlyphBuffer::shapes()`, like `overlays()` and `sprites()`;
  - `Item::layer`, `visible` (the `clip`), `with_visible` and
    `overhang` (`None`) cover it, so `cut_items` cuts a shape's `clip`
    where cells are replaced;
  - `fade` scales a shape's colour as it scales a rectangle's;
  - `blit` moves a shape's points and clips its `clip`, as it moves a
    sprite's `dest`. A shape whose points no longer fit in `i32` is
    dropped;
  - `frame` re-exports `Shape`, `ShapeKind` and `Fill`, and the module
    docs of `frame.rs` and `glyph_buffer.rs` name shapes beside
    rectangles and sprites;
  - every exhaustive `match` on `Item` in the workspace gets an arm:
    `overlays()`, `sprites()`, the renderers, the snapshot, and the
    tests in `crates/ui/src/grid/sprite/` (`ground.rs` and `tests.rs`).
- **The snapshot** (`snapshot.rs`), in the overlays section, one line
  per shape, its colour named as a rectangle's is:
  - `over  line 3,4 20,9  accent`, then ` w=3` when the width drawn
    isn't 1;
  - `over  circle 10,10 r=5  accent`, then ` outline` when outlined;
  - `over  polygon 0,0 4,0 4,4  accent`, then ` outline` when
    outlined;
  - last, ` clip=x,y wxh` when `clip` isn't `bounds()`.

  Update the module's doc comment, which is the format's spec.
- **The renderers** paint the runs in whole console pixels:
  - `frame-png`'s `Painter::draw_items` fills each run as a rectangle
    one console pixel high with `Image::fill` (in `shuyi_ui::frame::raster`, 2332);
  - `shuyi-app`'s `Renderer::draw_items` draws each run with one
    `draw_rectangle`, as it draws an `Overlay` (macroquad batches
    them);
  - both 1-bit renderers' `draw_items` fill each run, as `at.screen` of
    a one-row rectangle, with `Target::fill` and
    `Pen::Paint(look.shades.shape(color))`, as they fill a rectangle.
    A colour whose shade `flips` draws in its pattern: only a
    rectangle flips.
- **Either order with 2315:** if 2315 is done, `frame::shape` already
  holds the width-1 line and circle-outline rules (2315 wrote them as
  this scope says, for its gizmos). Build the rest of the module around
  them, and add what this scope asks that they lack (such as starting
  a long line's walk inside `clip`), with 2315's snapshots and pictures
  unchanged. If not, write the rules here; 2315's gizmos then draw
  through them (its clause on this ticket says so).
- **1-bit (3006 is done, ADR-0026):** a shape's colour goes through the
  one-bit renderer's `Shades` (the game's inks, or a `PatternMap`) as a
  rectangle's does, in `shuyi_ui::frame::onebit`'s fast renderer and its
  reference, and the fast-equals-reference property tests cover shapes.
- **Either order with 2389** (the inks removed): if 2389 is done, the
  renderers' only `Shades` is a `PatternMap`: shapes draw through
  `Shades::shape` as above, and the tests use a pattern map (2389's
  test helper). If not, draw the same way, through `Shades::shape`,
  never `ink_of` or an `Ink`, so 2389 has no shape code to move; it
  moves this ticket's tests that use `InkShades` with the rest.
- **Either order with 2018:** if 2018 is done, add shapes of each kind
  to its render card (`shuyi_ui::frame::raster::card`); its `match` on
  `Item` has no `_` arm, so it doesn't compile until the card shows
  them. If not, compare a captured window frame of the test scene with
  `frame-png` by hand, note it in the PR, and 2018's card takes shapes
  when it is written.
- **Public types (ADR-0039):** `Shape`, `ShapeKind` and `Fill` follow
  it (`#[non_exhaustive]` where they may grow, constructors).
- **Either order with 2122:** if 2122 is done, the module docs of
  `shuyi_core::curve` say a skin draws a curve as lines over
  `pieces()`: name `Shape::line` there. If not, nothing to do (2122
  names it).
- **Either order with 2802:** if 2802 is done, chapter 4 of the book
  (the frame and skins) gets a short section on shapes, its code in
  `site/snippets/` as 2802 says. If not, 2802 writes the chapter from
  the code.
- **Either order with 2738** (parked): if 2738 is done,
  `examples/bricks` draws its ball as a square `Overlay`: make it a
  filled circle, and redo its `frame-png` pictures. If not, nothing to
  do (2738 draws a filled circle).
- **Either order with 2740** (parked): if 2740 is done, its hex path
  arrow (`crates/ui/src/grid/path.rs`) draws the steps between rows
  with a line helper shared with `threat.rs`: draw them with
  `Shape::line(from, to, LINE_W, color, Layer::Under)` through
  `GlyphBuffer::add_shape`, clipped to `Grid::bounds`, and review its
  hex snapshots. If not, nothing to do (2740 draws them so).
- **Either order with 2744:** if 2744 is done, nothing to do (its
  automap may move onto these shapes later); if not, nothing to do
  (2744 draws its automap with these shapes when they exist).
- **What changes for the game:** `Item` gains a variant, so an
  exhaustive `match` on `Item` breaks. The game has a few, in its map
  drawing and its tests, so its PR that moves the pin adds the arm. Say
  so in this PR. The completion notes carry 2010's
  `Breaking:` line (`Breaking: shuyi-ui: Item: a new variant, Shape;
  add an arm to an exhaustive match`). No game snapshot changes.

**Out (do not do):**
- anti-aliased edges and points between pixels;
- outlines wider than one pixel: a thick polygon outline is its edges
  drawn as wide lines; a thick ring is a follow-up if a game asks;
- textured polygons and triangles (2217 adds textured triangles, on
  this rule, if it needs them);
- curves, arcs, ellipses and rounded rectangles (a curve is drawn as
  lines);
- line joins and caps other than the round ends, and dashed lines;
- opacity and blend modes on shapes (rectangles have neither; 2337 adds
  blend modes to sprites and rectangles);
- more than 16 points (a skin splits a bigger polygon);
- rotated sprites (2335);
- shapes on world layers: there are none until 1120 (parked), which can
  carry shapes there beside its sprites;
- strips (2703) and the debug view's gizmos (2315).

## Implementation steps

1. Read 2335's ADR, `glyph_buffer.rs` (`Item`, `add_overlay`,
   `add_sprite`, `cut_items`, `fade`, `blit`), `snapshot.rs`,
   `frame_png.rs` (`Painter::draw_items`, `Image::fill`), `onebit.rs`
   and `onebit/reference.rs` (`draw_items`, `Place::screen`,
   `Target::fill`), and `crates/app/src/render.rs`
   (`Renderer::draw_items`). If 2315 is done, read its gizmo drawing.
2. Write `frame/shape.rs`: the types, the constructors, `bounds`, and
   `for_each_run` for each kind. In `frame/shape/tests.rs`, write a
   plain per-pixel version of each rule (the full Bresenham walk, the
   distance test, the full midpoint walk, a crossing count) for the
   property tests to compare the runs with.
3. Add `Item::Shape` and the `GlyphBuffer` changes, with unit tests.
4. Add the snapshot lines and update the format's doc comment.
5. Draw shapes in `frame-png`, both 1-bit renderers and `shuyi-app`.
6. Build the test scene in `frame/shape/tests.rs` and commit its insta
   snapshot (as `frame/tests.rs` does): lines of width 1, 3 and 7 in all
   eight directions; circles of radius 0, 1, 5 and 20, filled and
   outlined; a five-pointed star (the even-odd hole in its middle) and a
   concave polygon, filled and outlined; one shape cut by a
   `fill_rect`. Render the same shapes with the `Painter` and look at
   the picture. In `raster`'s tests, check the `Painter`'s pixels against
   `for_each_run`. Add shapes to the 1-bit `Thing` generator
   (`onebit/speed_tests.rs`).
7. Settle the either-order clauses above (2315's rules, 2018's card,
   2802's chapter, 2738's ball and 2740's path arrow).
8. Run the gates, including the `no_std` build of `shuyi-ui`. If the PR
   grows well past about 600 changed lines (fixtures and snapshots
   aside), leave polygons out (`ShapeKind::Polygon`, `Shape::polygon`,
   their snapshot line and tests), finish the rest, and write a ticket
   for polygons (the `write-ticket` skill), blocked by this one. In the
   same PR, add it to the `blocked_by` of the tickets that build on the
   polygon rule (2703's strips, 2217's textured triangles and 2743's
   outlined quads), and point the polygons in 2335's, 2315's and
   2740's Out lines at it.

## Acceptance criteria

- [ ] A filled polygon with the corners of a rectangle covers exactly
      the pixels of the `Overlay` of that rectangle (property test).
- [ ] A convex four-point polygon cut along a diagonal into two
      triangles: every pixel of the polygon is covered by exactly one
      triangle (property test).
- [ ] Each kind's runs inside a random `clip` equal its plain per-pixel
      version cut to that `clip` (property test).
- [ ] A width-1 line draws both ends and one pixel per step along its
      longer axis, in all eight octants; its runs inside a small `clip`
      equal the full walk cut to that `clip` (property test over lines
      up to 2 000 pixels long).
- [ ] A line from `(i32::MIN, 0)` to `(i32::MAX, 1)` in a 400 × 240
      `clip` yields only that `clip`'s pixels, without walking the
      rest (a unit test that ends in milliseconds).
- [ ] A horizontal or vertical line of width `2k + 1` is `2k + 1`
      pixels across, and a width of 4 draws as 5.
- [ ] A circle of radius 0 is one pixel; every circle is symmetric
      across both axes and both diagonals, and its fill contains its
      outline (radii 0 to 64).
- [ ] `for_each_run` never yields a pixel outside `clip`, and never
      panics or overflows, for any `i32` points (property test).
- [ ] `frame-png`'s picture of each kind, at scales 1 and 3, has the
      shape's colour exactly where `for_each_run` covers.
- [ ] The fast 1-bit renderer equals the reference with shapes in its
      frames.
- [ ] The test scene's snapshot is committed; the window draws it as
      `frame-png` does (2018's card if done, else by hand, noted in the
      PR).
- [ ] Every existing snapshot and `frame-png` test passes unchanged.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] If step 8 splits polygons off, their criteria move to the new
      ticket, and 2703, 2217 and 2743 are blocked by it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each kind's runs on small hand-checked cases; widths 0 and 4
  drawn as 1 and 5; `Shape::polygon` refusing 2 and 17 points; each
  snapshot line; `add_shape` dropping a shape off the buffer;
  `cut_items`, `blit` and `fade` on a shape; the line across the whole
  `i32` range.
- Property: the rectangle, shared-edge, per-pixel, symmetry,
  clipped-walk and in-`clip` properties above; the same runs twice.
- Snapshot / integration: the test scene's snapshot; the `Painter`'s
  pixels against `for_each_run`; the fast 1-bit renderer against the
  reference.

## Completion notes

*(Filled in by the session that completes the ticket.)*
