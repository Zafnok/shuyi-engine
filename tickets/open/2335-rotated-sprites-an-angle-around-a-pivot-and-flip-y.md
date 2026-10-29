---
id: "2335"
title: "Rotated sprites: an angle around a pivot and flip_y on sprite items, with an integer Angle and its sine table in shuyi-core"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2335 — Rotated sprites: an angle around a pivot, and flip_y

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3.
Every one of them turns a sprite by an angle and flips it both ways:

- Godot: `Node2D.rotation`, and `flip_h` and `flip_v` on `Sprite2D`
  (https://docs.godotengine.org/en/stable/classes/class_sprite2d.html);
- Bevy: a sprite's `Transform` rotation, and `flip_x` and `flip_y` on
  `Sprite` (https://docs.rs/bevy/latest/bevy/sprite/struct.Sprite.html);
- Unity and Unreal: a transform's rotation, and Unity's
  `SpriteRenderer.flipY`;
- GameMaker: `image_angle`; Construct 3 and Defold: an angle or a
  rotation on every object.

Shuyi today (`crates/ui/src/frame/glyph_buffer.rs`):

- `Sprite` (line 195) has `flip_x` (line 210) and no angle and no
  vertical flip;
- `Item` (line 305) is a `Rect` or a `Sprite`, both axis-aligned;
- four renderers draw sprites: `shuyi-app`'s `Renderer::draw_sprite`
  (`crates/app/src/render.rs`, line 248), `frame-png`'s
  `Painter::draw_sprite` (`crates/cli/src/frame_png.rs`, line 562), and
  the 1-bit renderer's `draw_sprite` in `crates/ui/src/frame/onebit.rs`
  (line 989) and in `onebit/reference.rs` (line 317);
- the snapshot prints a sprite in `sprite_line`
  (`crates/ui/src/frame/snapshot.rs`, line 146), with ` flip` for
  `flip_x`.

Rotation was planned only inside parked work: 2703 (parked with
procedural animation, which Nick called "a very late addition") held
"an angle on sprite items, 0–255", on 1120's world layers, as ADR-0010
§5 does, and 1119 (parked real time) planned "a 256-step sine/cosine
table" in `shuyi_core::fx`. The tickets now point here instead: 1119,
1120, 2103, 2104, 2105, 2216, 2217, 2701, 2703, 3021 and 3101 name
2335's `Angle` or its rotated sprites. Three places still don't:
ADR-0010 §5, ADR-0012's "rotated ones (2703)" for the GBA's affine
sprites, and `docs/ROADMAP.md`'s "skeletal animation (2105) after
2703".

`docs/decisions.md`, 2026-10-20: Nick put the features most 2D games
expect after the MVP, not parked. Some of them "take the turn-game half
of a parked ticket", among them "rotated sprites and drawn shapes from
2703". This ticket is the rotated sprites. 2336 is the shapes.

Why it matters:

- **The game first.** Visions of Shuyi asks for none of this (no open
  game ticket turns a picture or flips one upside down). It costs the
  game nothing: a sprite built with `Sprite::new` draws as it does
  today.
- **Other 2D games:** an Asteroids-style ship, a shooter's turrets and
  bullets, an aim arrow, a top-down racer, a coin that spins, a card
  turned upside down. The parked skeletal animation (2105) and
  particles (2103) need it too.
- **The end goal** (2026-10-19): an agent turns a sprite with one field,
  and the desktop, web and Playdate builds draw the same pixels.

Rotation here is integer and decided per console pixel. Each renderer
asks the same function in `shuyi-ui` which picture pixel a console
pixel shows, so they are equal by construction. `shuyi-app` does not
use macroquad's `rotation`, which samples screen pixels in floats and
would not match `frame-png` or the Playdate. A sprite at angle 0 keeps
today's drawing in every renderer.

**From 2707 (open):** Parked: `shuyi_core::angle::Angle` (256 steps) is the facing of a unit in 2707's `UnitTable`. If 2707 starts first, 1119 writes `Angle`; this ticket then uses it.

## Nick input

None. Nick decided on 2026-10-20 that rotated sprites come out of 2703
and follow the MVP.

## Scope

**In:**
- **`shuyi_core::angle`** (`crates/core/src/angle.rs`), a new module,
  `no_std`, no floats:
  - `Angle(u8)`: 256 steps a turn. A larger angle turns further
    clockwise on the screen (y points down), as Godot's rotation does.
    `Angle::new(steps: u8)`, `steps()`, the constants `ZERO`,
    `QUARTER` (64) and `HALF` (128), and wrapping `Add`, `Sub` and
    `Neg`. It derives `Copy`, `Eq`, `Ord`, `Hash`, `Debug` and
    `Default`, and serde as a plain number (`#[serde(transparent)]`).
  - `Angle::UNIT: i32 = 65_536`, the sine of a quarter turn. `sin()`
    and `cos()` return an `i32` scaled by `UNIT`. The units match
    3101's 3D table (1/65 536).
  - The table: a quarter wave of 65 `i32` values,
    `round(sin(i × 2π / 256) × 65 536)` for `i` in `0..=64`, written out
    in `angle.rs`. The other three quarters come from symmetry.
    `sin(QUARTER) == UNIT` and `cos(HALF) == -UNIT` exactly.
  - `Angle::rotate(self, x: i64, y: i64) -> (i64, i64)`:
    `(x·cos − y·sin, x·sin + y·cos)`, still scaled by `UNIT`. The
    caller divides and picks its rounding.
  - Re-exported as `shuyi_core::Angle`. 1119's `Fx` sine and cosine,
    2701's `atan2` and `acos`, 2105's bones and 2103's particles use
    this type and this table, not a second one.
  - **Either order with 2109:** if 2109 is done, it has a private
    quarter-wave table in `crates/core/src/ease/tables.rs`. If that
    table has the same 65 entries in the same units, move it here and
    make `ease` use `Angle`'s. If not, both stay, and the Completion
    notes say why. If 2109 isn't done, it decides whether to use this
    table.
  - **Either order with 1119 (parked):** if 1119 is done, it wrote this
    module's `Angle` type and its table as this list says. Keep them and
    their tests, check them against this list, and add only what is
    missing, such as `rotate` and the `shuyi_core::Angle` re-export. If
    not, write the module here, and 1119 takes `Fx`'s sine and cosine
    from it.
- **The sprite** (`crates/ui/src/frame/glyph_buffer.rs`):
  - three fields on `Sprite`:
    - `angle: Angle`, `Angle::ZERO` by default;
    - `pivot: Option<(i32, i32)>`: a point on the pixel grid (a pixel's
      corner) relative to `dest`'s top-left. `None` is `dest`'s centre,
      which may fall on half a pixel;
    - `flip_y: bool`: mirrored top to bottom within `dest`.

    `Sprite::new` sets the defaults, so every sprite built today draws
    as it does now.
  - The order: the picture is flipped within `dest` first, then turned
    around the pivot.
  - `Sprite::bounds() -> PxRect`: the console pixels the turned `dest`
    can cover. At angle 0 it is `dest`. At another angle it is the
    floor and ceiling of `dest`'s four corners turned around the pivot
    with `Angle::rotate`, plus one pixel on each side for the table's
    rounding.
  - `Sprite::dest_pixel(x: i32, y: i32) -> Option<(i32, i32)>`: the
    pixel of `dest` (before the flips, `0..w`, `0..h`) that console
    pixel `(x, y)` shows, or `None`. The rule, written in its doc
    comment:
    - work in doubled console pixels, in `i64`, so pixel centres
      (`2x + 1`) and `dest`'s centre (`2·dest.x + dest.w`) are whole;
    - turn the pixel's centre back by the angle around the pivot with
      `Angle::rotate`;
    - floor to a pixel of `dest` with `div_euclid`, and keep it if it
      lies inside `dest` (left and top edges in, right and bottom out).

    At angle 0 it is `(x − dest.x, y − dest.y)` inside `dest`.
  - `Sprite::source_pixel(x: i32, y: i32) -> Option<(i64, i64)>`: the
    image pixel shown, after the flips, by the nearest rule
    `src.x + (2u + 1)·src.w / (2·dest.w)` (and the same for `y`). It is
    what `onebit::Image::sample` computes for a picture at its own
    size.
  - `add_sprite` clips `clip` to `bounds()` instead of `dest` (the same
    at angle 0). `clip`'s doc becomes "the part of `bounds()` that is
    drawn".
  - `clipped_src()` handles `flip_y` as it handles `flip_x`, and its
    doc says it is for sprites at angle 0 only.
  - `blit` needs no change for the pivot, because it is relative to
    `dest`. Check `cut_items`, `fade` and `overhang` keep working with
    a `clip` inside `bounds()`, and add a test each.
- **The snapshot** (`crates/ui/src/frame/snapshot.rs`): after
  ` flip`, the line gets ` flip_y`, ` angle=N` and ` pivot=x,y`, each
  only when it isn't the default. ` clip=` is printed when `clip`
  differs from `bounds()`, which is `dest` at angle 0, so no line
  changes today. Update the module's doc comment, which is the format's
  spec.
- **The renderers:**
  - **The raster's `Painter`** (`shuyi_ui::frame::raster`, 2332):
    - `flip_y` in `blit`. Pass the two flips as one `(bool, bool)`, so
      `blit` stays at seven arguments (clippy's `too_many_arguments`);
    - a sprite at another angle is drawn per console pixel of its
      `clip`: the colour of `source_pixel`'s texel, painted, darkened
      and blended as `blit` does, filled `scale × scale`. So
      `draw_sprite` takes the corner and the scale, not only `clip` in
      output pixels.
  - **The 1-bit renderers** (`onebit.rs` and `onebit/reference.rs`):
    `flip_y` mirrors the row as `flip_x` mirrors the column. At another
    angle, each console pixel goes through `dest_pixel` and the flips,
    then through `Image::sample` as today (a 1-bit picture may be
    bigger than its source image: `shuyi_assets::bitmap` enlarges it
    before dithering). At angle 0 the fast renderer keeps today's
    loop, for the device's speed (game ticket 1055).
  - **From 3056 (done):** "today's loop" is `draw_sprite`'s byte at a
    time, from its `Columns` (`Run` pixel for pixel, `Table` otherwise):
    `flip_y` goes into both, where each row's picture row is sampled
    once (from the bottom, `dest.h − 1 − dy`, as `flip_x` does for the
    columns); a sprite at another angle skips both for the
    `dest_pixel` path; and flipped and turned sprites join
    `sprites_draw_as_the_reference_does` (`onebit/speed_tests.rs`)
    against `reference`.
  - **`shuyi-app`** (`crates/app/src/render.rs`): `flip_y` goes to
    `DrawTextureParams::flip_y`. A sprite at another angle is drawn from
    a texture made on the CPU:
    - its pixels come from `source_pixel` over `bounds()`, read from the
      image decoded once and kept, coloured as the paint says (as
      `recoloured` does today), with nearest filtering;
    - it is cached by image, `src`, `dest`'s size, angle, pivot, flips
      and paint, all relative to `dest`, so a moving sprite reuses it.
      The cache is cleared when it passes 1 024 textures;
    - the part of it under `clip` is drawn at `clip`'s place, tinted for
      brightness and opacity as today.
  - **Either order with 2337** (tint and blend modes): if 2337 is
    done, turned sprites take `Paint::Tinted` and `blend` too:
    `frame-png`'s per-pixel fill goes through `Blend::mix` as `blit`
    does; the 1-bit path draws `Tinted` as `Image`, `Add` as its white
    pixels only and `Multiply` as its black pixels only; `shuyi-app`
    makes its texture without the tint (a `Tinted` paint stays out of
    the cache key and is the draw colour) and draws it with the blend's
    material. If not, nothing to do (2337 does it).
- **An ADR** (the `write-adr` skill), the next free number:
  - rotation and flips are part of the core frame for every game, not
    of procedural animation;
  - one `Angle` and one sine table, in `shuyi_core::angle`;
  - the pixel rule: every renderer decides each console pixel of a
    turned item from its centre, in integers, through functions written
    once in `shuyi-ui` (`Sprite::dest_pixel` here, and 2336's shape
    runs). A covered console pixel is filled whole at any scale;
  - the window does not use the GPU's rotation;
  - it supersedes ADR-0010 §5's sprite rotation and the rotated-sprite
    half of its coverage rule (2703 keeps strips, on this rule). Change
    only ADR-0010's `Status:` line and its row in `docs/adr/README.md`;
  - ADR-0012's "rotated ones (2703)" now means this ticket's sprites.
- **What still sends rotation elsewhere** (the `write-ticket` skill,
  "look the other way"): the tickets already point here (see Context).
  Left to do:
  - `docs/ROADMAP.md`: "skeletal animation (2105) after 2703" becomes
    "after 2335";
  - search `tickets/open/` once more for a sprite angle sent to 2703 or
    a sine table sent to 1119, and point any new one here.
- **What changes for the game:** `Sprite` gains three fields. Only code
  that builds a `Sprite` with a struct literal and no `..` breaks. Say
  so in the PR; the game's PR that moves the pin follows it. No game
  snapshot changes.
- **Public types (ADR-0039):** the new public types follow it
  (`#[non_exhaustive]`, constructors).
- **Either order with 2392:** if 2392 is done, add rotated and `flip_y`
  sprites from this ticket's test scene to its render card
  (`raster::card`), which 2018's `render` job compares with the window.
  If not, compare a captured window frame with `frame-png` by hand,
  note it in the PR, and 2392 adds them.

**Out (do not do):**
- drawn shapes, and rotated rectangles as polygons (2336);
- scale and shear beyond `dest` (`dest` already stretches a picture);
- smooth or filtered rotation, and RotSprite-style upscaling;
- world layers and cameras (1120 carries rotated sprites onto them);
- the GBA's affine sprites and their 32 matrices (3021), and the DS's
  (3024). **Either order with 3021 and 3024** (both parked): if one
  is done first, it wrote a follow-up ticket blocked by this one for
  its affine sprites, which this PR unblocks: nothing to do. If not,
  nothing to do (each maps rotated sprites itself);
- `Fx`, `FxVec2` and their trigonometry (1119), and `atan2` and `acos`
  (2701);
- strips (2703);
- an example game that turns sprites: 2738's shmup (parked until after
  the launch) turns its foes if this ticket is done, and keeps one
  picture if not. Nothing to do here either way.

## Implementation steps

1. Read `glyph_buffer.rs` (`Sprite`, `add_sprite`, `clipped_src`,
   `blit`, `cut_items`, `fade`), `snapshot.rs`, `frame_png.rs`
   (`Painter::draw_items`, `Painter::draw_sprite`, `blit`), `onebit.rs`
   and `onebit/reference.rs` (`draw_sprite`, `Image::sample`),
   `crates/app/src/render.rs` (`draw_sprite`, `recoloured`), and
   ADR-0010 §5.
2. **Size.** This ticket is near the limit of about 600 changed lines
   (fixtures and snapshots not counted). If it would pass it, write
   `shuyi_core::angle` as a ticket of its own first (the
   `write-ticket` skill, block 21xx), add it to this ticket's
   `blocked_by`, work it as its own PR, and point 2109, 2122 and
   2225's "2335's `Angle`" at it.
3. Write the ADR.
4. Write `shuyi_core::angle` and its tests.
5. Add the fields, `bounds`, `dest_pixel`, `source_pixel` and the
   `add_sprite` change, with unit tests. Fix the tests that build a
   whole `Sprite` literal (`glyph_buffer/sprite_tests.rs`).
6. Add the snapshot fields and update the format's doc comment.
7. Draw `flip_y` and turned sprites in `frame-png`, both 1-bit
   renderers and `shuyi-app`.
8. Build a test scene: the test card (16 × 16;
   `shuyi_ui::frame::raster::test_card`) at eight angles,
   flipped each way, and turned around a corner pivot, in `dest`s of
   16 × 16 and 32 × 32.
   - In `crates/cli/src/frame_png/tests.rs`, check hand-worked pixels
     at angles 64 and 128 at scales 1 and 2, as
     `matches_a_hand_checked_frame` does. Look at the picture once
     (save it from a scratch test), then keep only the checks.
   - In `shuyi-ui`, commit the scene's `insta` snapshot.
   - Add angles, pivots and `flip_y` to the 1-bit `Thing` generator
     (`onebit/speed_tests.rs`).
9. Update `docs/ROADMAP.md`, and any ticket the search in the scope
   finds.
10. Run the gates, including the `no_std` builds of `shuyi-core` and
    `shuyi-ui`.

## Acceptance criteria

- [ ] An ADR records the design, and ADR-0010's status line points to
      it.
- [ ] `Angle`: `sin(ZERO) == 0`, `sin(QUARTER) == UNIT`,
      `cos(HALF) == -UNIT`; `sin(-a) == -sin(a)` and
      `sin(HALF - a) == sin(a)` for every `a`; each table value equals
      `round(sin × 65 536)` computed in `f64` in the test.
- [ ] A sprite at angle 0, with no pivot and no `flip_y`, draws exactly
      as before: every existing snapshot and `frame-png` test passes
      unchanged.
- [ ] At angle 128 around `dest`'s centre, a sprite whose `dest` is
      `src`'s size or a whole multiple of it draws exactly as the same
      sprite with `flip_x` and `flip_y`, in `frame-png` and in both
      1-bit renderers.
- [ ] At angle 64 on a square `dest` (the test card at 16 × 16 and
      32 × 32), the picture is turned a quarter clockwise exactly (its
      top row becomes its right column).
- [ ] Every console pixel that `dest_pixel` maps inside `dest` lies in
      `bounds()` (property test over random sprites, angles and
      pivots).
- [ ] The fast 1-bit renderer equals the reference with turned and
      flipped sprites in its frames.
- [ ] The window and `frame-png` draw the test scene the same (2392's
      card in 2018's `render` job if 2392 is done, else by hand, noted
      in the PR).
- [ ] No ticket in `tickets/open/`, and not `docs/ROADMAP.md`, still
      sends sprite rotation to 2703 or the sine table to 1119.
- [ ] `shuyi-core` and `shuyi-ui` still build for
      `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the `Angle` table and its symmetries; wrapping arithmetic;
  `rotate` at the four quarter turns; `dest_pixel` at angle 0, 64 and
  128 with and without a pivot; `flip_y` alone and with `flip_x`;
  `bounds` of a turned odd-sized `dest`; the snapshot fields shown only
  when not default; `cut_items`, `fade` and `blit` on a turned sprite.
- Property: `dest_pixel` inside `bounds`; angle 0 is the identity;
  `Angle` addition wraps; no panic or overflow for any `i32` `dest`
  and pivot.
- Snapshot / integration: the test scene's snapshot and its
  hand-checked `frame-png` pixels; the fast 1-bit renderer against the
  reference.

## Completion notes

*(Filled in by the session that completes the ticket.)*
