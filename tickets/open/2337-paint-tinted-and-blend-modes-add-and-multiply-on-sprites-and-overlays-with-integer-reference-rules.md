---
id: "2337"
title: "Paint::Tinted and blend modes (add and multiply) on sprites and overlays, with integer reference rules in frame-png"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2337 — Tint and blend modes on sprites and overlays

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3.
Every one of them can tint a sprite, and most can blend it:

- Godot: `CanvasItem.modulate` tints, and `CanvasItemMaterial` blends
  (mix, add, subtract, multiply, premultiplied alpha)
  (<https://docs.godotengine.org/en/stable/classes/class_canvasitemmaterial.html>);
- Bevy: `Sprite::color` tints; additive needs a material of your own;
- Unity: `SpriteRenderer.color` tints; additive needs a shader;
- GameMaker: `image_blend` tints, `gpu_set_blendmode` blends;
- Defold: every sprite has a `tint` and a blend mode (alpha, add,
  multiply, screen) (<https://defold.com/manuals/sprite/>);
- Construct 3: blend modes on every object, screen included.

What Shuyi has today (checked 2026-10-20):

- `crates/ui/src/frame/glyph_buffer.rs:160-189`: `Paint { Image,
  Solid(Rgb), Dimmed, Shaded(Light) }`. `Paint::Solid` draws a
  silhouette in one colour, and its doc comment names a hit flash, so a
  flash in any colour already works.
- `glyph_buffer.rs:191-226`: a `Sprite` has `opacity` and `brightness`
  (a grey multiply).
- `Paint::Shaded(Light { grey: 0, tint })` is a per-sprite colour
  multiply (`crates/assets/src/lighting.rs`, `Light::apply`). But
  `shuyi-app` draws it from a recoloured copy of the image, cached per
  `(ImageId, Recolour)` and never freed (`crates/app/src/render.rs:58-60`,
  `249-263`). A tint that changes every frame (a colour pulse) would
  upload and keep a new texture each frame, though the renderer already
  multiplies by a draw colour for free (`render.rs:271-285`).
- `glyph_buffer.rs:142-158`: an `Overlay` (a rectangle) has a colour
  and a layer, and no opacity.
- No blend mode exists. A search of `crates/`, `docs/` and `tickets/`
  for blend, additive, multiply and modulate finds only 3D alpha
  (3102 to 3104), animation blending (2104, 3110), and a parked
  shader ticket's materials (2327).

The parked 2D effects (2326 screen passes, 2327 a game's shaders and
materials, 2328 lights) all wait for the wgpu renderer (3034). This
ticket doesn't: macroquad already multiplies a texture by a draw colour,
and `load_material` takes a pipeline with its own blend state
(macroquad 0.4.16, miniquad 0.4.11's `PipelineParams::color_blend`).

Why it matters:

- **The game** doesn't need it: it has `Solid`, `Dimmed`, `Shaded` and
  `brightness`. Its frames must not change (the defaults draw as today).
- **Other 2D games** expect it: one white or grey sprite drawn in many
  colours (bullets, pickups, icons by state), a colour pulse, additive
  glow for fire, lasers and sparks, multiply shadows. Particles (2103,
  parked) look flat without additive blending.
- Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)):
  the features most 2D games expect come after the MVP, not parked.
  This is the cheapest of the 2D effects, and the one 2103 wants when it
  is unparked.

Graphics are a skin (ADR-0002 §4, `AGENTS.md` rule 9). Tint and blend
are look data a skin sets on frame items. No scene, view, simulation or
record holds them.

## Nick input

None.

## Scope

**In:**
- **`shuyi_ui::frame::blend`** (`crates/ui/src/frame/blend.rs`,
  `no_std`, integers only), re-exported as `shuyi_ui::frame::Blend`:
  - `#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, Default)]
    pub enum Blend { #[default] Alpha, Add, Multiply }`, with
    `pub const fn name(self) -> &'static str` (`"alpha"`, `"add"`,
    `"multiply"`), as `Layer::name` is;
  - `pub const fn mul(x: u8, y: u8) -> u8`, the one rounding rule:
    `(x·y + 127) / 255` in `u32`, which is `x·y / 255` to the nearest
    (there is never a tie);
  - `Blend::mix(self, under: Rgb, over: Rgb, alpha: u8) -> Rgb`, per
    channel, with `s` the item's colour (`over`), `a` its alpha and `d`
    what is beneath (`under`), in `u32`, each rounded once:
    - `Alpha`: `(s·a + d·(255 − a) + 127) / 255`;
    - `Add`: `min(255, d + mul(s, a))`;
    - `Multiply`: `(d·(65025 − a·(255 − s)) + 32512) / 65025`, which is
      `d` times `s` at full alpha and `d` at none.
  - The module doc says where an item's `s` and `a` come from: for a
    sprite pixel, `s = mul(paint.apply(texel), brightness)` and
    `a = mul(texel alpha, opacity)`; for an overlay, its colour and its
    opacity. It also says that a GPU draws within one step per channel
    of these rules, and gives the one-bit rule (below). These rules are
    the frame's, like `Paint`'s; no ADR.
- **`Paint::Tinted(Rgb)`** (`glyph_buffer.rs`): each pixel multiplied
  by the colour, `mul` per channel; white leaves the image as it is.
  - Its doc comment says it is Godot's `modulate` and Bevy's
    `Sprite::color`, and that it costs nothing to change every frame.
  - `Paint::Shaded`'s doc comment gains a line: each light makes a copy
    of the image in `shuyi-app`, so a colour that changes every frame
    uses `Tinted`.
  - It composes with `brightness` and `opacity` as the other paints do.
- **`Sprite.blend: Blend`** (`Sprite::new` sets `Alpha`) and
  `#[must_use] pub const fn blended(self, blend: Blend) -> Self`, like
  `painted`.
- **`Overlay.opacity: u8`** (255 = solid) and **`Overlay.blend:
  Blend`**. `Overlay::new` keeps its signature and sets 255 and `Alpha`.
  `#[must_use] pub const fn with_opacity(self, opacity: u8) -> Self`
  and `#[must_use] pub const fn blended(self, blend: Blend) -> Self`.
- **`GlyphBuffer::fade`** (`glyph_buffer.rs:725-748`): `Alpha` and
  `Add` items fade as today. A `Multiply` item is left as it is: what is
  beneath it is already darkened, and fading it too would darken twice.
  Its doc comment says so.
- **Snapshots** (`crates/ui/src/frame/snapshot.rs`): words appear only
  when not the default, so no existing snapshot changes.
  - A sprite (`sprite_line`): ` tint=colour` in the paint's place
    (beside ` solid=`, ` dimmed`, ` shade=`), then ` blend=add` or
    ` blend=multiply`, before ` clip=`.
  - An overlay: `layer  x,y wxh  colour`, then ` opacity=N` (below 255),
    then ` blend=add` or ` blend=multiply`.
  - The module doc, which is the format's spec, lists the new words.
- **`frame-png`** (`crates/cli/src/frame_png.rs`), the reference:
  - `Image::blend(&mut self, x, y, color: Rgb, alpha: u8, blend:
    Blend)` calls `Blend::mix`. `Image::fill` passes `Blend::Alpha` at
    255. Floats leave the colour arithmetic; where a texel is sampled
    stays as it is.
  - `blit` takes `tint: Rgb`, `opacity: u8` and a `Blend` instead of
    `tint: [f32; 4]`: `s = mul(paint.apply(texel), tint)` per channel
    and `a = mul(texel alpha, opacity)`. `draw_sprite` passes its
    brightness as a grey tint, its opacity and its blend. The glyph path
    passes the cell's fg, 255 and `Alpha`.
  - `draw_items` draws an overlay with its opacity and blend.
  - The module doc's "pixel for pixel" line says: to `frame::blend`'s
    rules.
- **The one-bit renderer** (`crates/ui/src/frame/onebit.rs` and
  `onebit/reference.rs`, kept equal):
  - `Paint::Tinted` draws as `Paint::Image`, as `Shaded` does today
    (the `solid` match in `draw_sprite`);
  - `Add` lightens: only the item's white pixels are drawn (a set OR);
  - `Multiply` darkens: only its black pixels are drawn (a set AND).
    Drawing them as normal would show the black surround of additive
    art and the white surround of multiply art as solid boxes;
  - an overlay's opacity dithers in the Bayer order a sprite's does
    (`shows`, `onebit.rs:980`);
  - in the fast renderer an overlay at full opacity with `Alpha` keeps
    today's whole-byte `fill`; any other overlay may be drawn pixel by
    pixel with `set`, as a sprite is (they are rare);
  - an invert rectangle (`Ink::Invert`) stays as it is: blend and
    opacity don't apply to it.
- **`shuyi-app`** (`crates/app/src/render.rs`):
  - `Paint::Tinted(c)` in `draw_sprite`: no recolour, and `c` is the
    tint (scaled by brightness, as `Solid`'s colour is). It never makes
    a recoloured copy.
  - Two materials, made once in `Renderer::new` with
    `macroquad::material::load_material` (a safe function; `shuyi-app`
    denies `unsafe_code`):
    - the vertex shader is a copy of macroquad's own GLSL 100 one
      (`shader::VERTEX` in macroquad 0.4.16's `src/quad_gl.rs`), which
      divides `color0` by 255;
    - the fragment shader computes `c = color * texture2D(Texture, uv)`
      and outputs `vec4(c.rgb * c.a, c.a)`. Every float in it is
      `lowp`, as in macroquad's own fragment shader, because GLSL ES
      1.00 has no default float precision there;
    - `MaterialParams { pipeline_params: PipelineParams { color_blend,
      ..Default::default() }, ..Default::default() }`, with
      `color_blend` from `macroquad::miniquad::{BlendState, BlendFactor,
      BlendValue, Equation}`:
      - `Add`: `BlendState::new(Equation::Add, BlendFactor::One,
        BlendFactor::One)`;
      - `Multiply`: `BlendState::new(Equation::Add,
        BlendFactor::Value(BlendValue::DestinationColor),
        BlendFactor::OneMinusValue(BlendValue::SourceAlpha))`.
  - `draw_items` calls `gl_use_material` when an item's blend differs
    from the last item's, and `gl_use_default_material` after the last
    one, so items stay in order.
  - Overlays draw with `Color::from_rgba(r, g, b, opacity)`.
  - A material that fails to load is logged once, and its items draw as
    `Alpha`; it never panics.
  - GLSL 100 runs on desktop OpenGL and the web build's WebGL 1.
    `window_conf` (`crates/app/src/lib.rs`) keeps miniquad's default on
    macOS, OpenGL; the module doc says a Metal build would need the
    shaders in MSL too.
- **Docs:**
  - `docs/platform-backends.md`, *`present`*: a backend draws a sprite's
    `paint` and `blend`, and an overlay's `opacity` and `blend`, as
    `shuyi_ui::frame::blend` says;
  - `crates/ui/src/frame.rs`'s module doc names `blend`.
- **Other tickets, edited in this PR** (`write-ticket`, *Check the
  dependency chain*):
  - 2103 (parked): its look keys may carry a `Paint::Tinted` and a
    `Blend` (additive sparks);
  - 2327 (parked): per-sprite tint and blend exist (2337), so its
    materials are for what they can't do; a material replaces a
    sprite's tint and blend; `palette_swap` stays the tool for team
    colours on full-colour art;
  - 3034 (parked): the renderer it plans draws `Paint::Tinted`, `blend`
    and an overlay's opacity to `frame::blend`'s rules;
  - 3017 (parked): per-sprite tint and blend are optional capabilities
    (one-bit draws them by the one-bit rule; the GBA blends per layer).
- **Either order with 2332** (the Painter moved into
  `shuyi_ui::frame::raster`): if 2332 is done, the reference changes go
  there instead of `frame_png.rs`; if not, they go in `frame_png.rs`,
  and 2332 moves them.
- **Either order with 2018 and 2019** (the desktop and web builds
  compared with the raster): if done, add tinted, added and multiplied
  sprites and an overlay at half opacity to their comparison. If not,
  the PR shows the same frame drawn by the desktop build, the web build
  and `frame-png`: a local change to `examples/hello-grid`'s skin, not
  committed, that draws its `tiles.png` tinted and blended (its tools
  in `examples/hello-grid/src/tools.rs` run `frame-png`).
- **Either order with 2335** (turned sprites): if 2335 is done, its
  turned-sprite paths (`frame-png`'s per-pixel fill, the one-bit
  `dest_pixel` path, `shuyi-app`'s texture made on the CPU) take
  `Tinted` and `blend` too. If not, add a line to 2335 saying its turned
  sprites take them.
- **Either order with 2339** (the veil): if 2339 is done, its veil's
  `Image::blend` call takes `Blend::Alpha`. If not, 2339 says so
  already.
- **Either order with 1120** (world layers, parked): if 1120 is done,
  its world-layer sprite items draw `paint` and `blend` through the code
  here. If not, add a line to 1120 saying its sprite items carry them.
- **Either order with 2802** (the book): if 2802 is done, its chapter
  "The frame and skins" (`site/content/learn/book/`) gets a short
  section with a tinted sprite and an additive one. If not, nothing.
- **Public types (ADR-0039):** `Blend` (a set that may grow) is
  `#[non_exhaustive]`. `Paint`, `Sprite` and `Overlay` aren't under the
  ADR yet, and neither its table nor 2024 moves them, so they move
  under it in this PR, which adds to them, as its *When* says. Then a
  game builds `Sprite` and `Overlay` with `new`, not with `..`.
- **For the game:** its frames don't change. `Overlay` and `Sprite`
  gain fields that `new` sets; the game builds them with `new` or with
  `..` (its `crates/ui/src/map_view/sprite.rs`, checked 2026-10-20). A
  search of its code found no exhaustive `match` on `Paint`; if one
  exists, it gets a `Tinted` arm in the game's PR that moves the pin.
  The new variant and fields are still breaks of `shuyi-ui` to
  cargo-semver-checks: if 2010 is done, the completion notes carry its
  `Breaking:` line, and the PR says the game's pin bump follows.

**Out (do not do):**
- **Screen**, subtract, min and max blending: a follow-up if a game
  asks (Defold and Construct have screen; it is one more material, one
  formula, and the lighten rule in one-bit);
- opacity and blend on 2336's drawn shapes;
- blending cells and glyphs (cells stay opaque; `blend_bg` stays);
- shaders, materials and per-pixel or masked blending (2327);
- palette swap and team colours on full-colour art (2327): a multiply
  tint over full-colour art looks muddy;
- screen passes and bloom (2326); full-screen flashes and fades (2340
  and 2339, on the frame's veil); lights and emissive maps (2328);
- building the wgpu renderer (3034) or target capabilities (3017): this
  PR only adds a line to each;
- freeing or bounding `render.rs`'s recoloured-texture cache for
  `Shaded` (`Tinted` makes no copy; a follow-up if a game animates
  lights);
- changing `Light::apply`, `Rgb::dimmed` or `Rgb::scale`.

## Implementation steps

1. Read `glyph_buffer.rs` (`Overlay`, `Paint`, `Sprite`, `fade`),
   `snapshot.rs`, `frame_png.rs` (`Image::blend`, `fill`, `blit`,
   `draw_items`, `draw_sprite`), `onebit.rs` and `onebit/reference.rs`
   (`draw_items`, `shows`, `draw_sprite`), `crates/app/src/render.rs`,
   and macroquad 0.4.16's `src/material.rs` and `src/quad_gl.rs`.
2. Write `frame/blend.rs`: `Blend`, `mul`, `Blend::mix`, the module doc
   and the property tests. Re-export `Blend` from `frame.rs`.
3. Add `Paint::Tinted`, `Sprite.blend`, `Overlay.opacity` and
   `Overlay.blend` with their builders and doc comments, and the `fade`
   rule. Add `blend: Blend::Alpha` to the one full `Sprite` literal in
   `glyph_buffer/sprite_tests.rs`.
4. Add the snapshot words and their tests.
5. Move `frame-png`'s pixel path onto `frame::blend` and draw the new
   fields. Run the existing tests in `crates/cli/src/frame_png/tests.rs`
   before changing any of them.
6. Add the one-bit rules to `onebit.rs` and `onebit/reference.rs`, and
   extend `speed_tests.rs`'s random `Thing` with tints, blends and
   overlay opacities.
7. Add `Tinted` and the two materials to `render.rs`, with the pure
   choices (the recolour and draw colour a paint gets; the `BlendState`
   per mode) in functions the tests can call.
8. Check the window and the web build against `frame-png` (2018 and
   2019, or by hand as above).
9. Write the docs and the lines in the other tickets, and run the
   gates.

## Acceptance criteria

- [ ] No existing snapshot changes: no `.snap` file and no snapshot
      string in a test is edited.
- [ ] The existing tests in `crates/cli/src/frame_png/tests.rs` pass
      with their expected values unchanged.
- [ ] A test frame (the test card tinted, added and multiplied over a
      known background, and an overlay at half opacity, alone and
      added) is drawn by `frame-png` with hand-worked pixel values in
      its test, and its picture is in the PR.
- [ ] The desktop and web builds draw tinted, added and multiplied
      sprites and a half-opacity overlay within one step per channel of
      `frame-png` (2018 and 2019, or by hand, shown in the PR).
- [ ] A `Paint::Tinted` sprite in `shuyi-app` never asks for a
      recoloured texture (a unit test of the pure choice).
- [ ] The one-bit fast renderer still draws as the reference does, with
      random tints, blends and overlay opacities.
- [ ] 2103, 2327, 3017 and 3034 each have their line, and 1120 and 2335
      theirs unless they are done.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - `mul` and `Blend::mix` on hand-worked values: `Add` of
    `(200, 100, 0)` at 255 over `(100, 200, 50)` is `(255, 255, 50)`;
    `Multiply` of black at 128 over 200 is 100; `Alpha` of
    `(10, 20, 200)` at 128 over 16 is `(13, 18, 108)` (as
    `frame_png/tests.rs` has it today);
  - `Paint::Tinted(white)` leaves a colour as it is; the test card's
    white border tinted is the tint;
  - `fade` leaves a `Multiply` item's colour and brightness as they are
    and darkens an `Add` one;
  - the snapshot words: each one alone, and none at the defaults;
  - one-bit: an `Add` sprite's black pixels and a `Multiply` sprite's
    white pixels leave the frame as it was; an overlay at opacity 128
    dithers as a sprite at 128 does;
  - `shuyi-app`'s pure choices: the recolour and draw colour per paint,
    the `BlendState` per mode.
- Property:
  - for every mode, `a = 0` leaves `d`; `Add` never darkens;
    `Multiply` never brightens; `Add` of black and `Multiply` of white
    leave `d`;
  - `Alpha` equals `round(s·a/255 + d·(255 − a)/255)` computed in `f64`
    for random `s`, `a` and `d`;
  - the one-bit fast renderer equals the reference (the existing
    property, with the wider `Thing`).
- Snapshot / integration: the test frame through `frame-png`; the
  desktop and web builds as above.

## Completion notes

*(Filled in by the session that completes the ticket.)*
