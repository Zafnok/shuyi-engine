---
id: "3228"
title: "Animated lights cost no new texture per light: Paint::Shaded drawn by a light shader in shuyi-app, and the recoloured copies bounded"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2337"]
nick_input: none
completed:
---

# 3228 — Animated lights cost no new texture per light

## Context

Found by the blind-spot audit of 2026-11-23 (rendering, "animated lights
make unbounded texture copies"). Checked against the code that day.

What `shuyi-app` does with a light today (`crates/app/src/render.rs`):

- `Renderer::draw_sprite` (lines 357-388) maps `Paint::Shaded(light)`
  to `Recolour::Shaded(light)`, and looks the texture up in
  `recoloured: HashMap<(ImageId, Recolour), Option<Texture2D>>`
  (line 72). A key it hasn't seen decodes the whole PNG again, recolours
  every pixel on the CPU with `Light::apply` and uploads a new texture
  (`fn recoloured`, lines 510-525).
- Nothing ever removes an entry, but `Renderer::remake` when an image's
  file changes (line 140). So each distinct light a sprite is drawn in
  costs one decode, one CPU pass and one texture, kept for the run.
- macroquad 0.4.16 frees a texture when its last `Texture2D` drops
  (`TextureSlotGuarded`'s `Drop`, `src/texture.rs`), so removing an
  entry does free it.

Today a map is in one light (`MapLook::light`, painted by
`crates/ui/src/grid/sprite/ground.rs:46`), so the map's images get one
copy each. A light that moves breaks this:

- **2369** (parked: a day and night cycle) paints the ground
  `Paint::Shaded(from.mix(to, permille))`. `permille` runs 0 to 1000
  between two stops, so one day makes up to about a thousand distinct
  lights per pair of stops, and each one is a new copy of every ground
  image: a decode, a CPU pass and an upload in the frame the light
  moves, and memory that only grows.
- **2337** (tint and blend modes, the blocker) gives a colour that
  changes every frame `Paint::Tinted`, drawn as the draw colour with no
  copy. But a light is more than a multiply: `Light::apply`
  (`crates/assets/src/lighting.rs:107`) first moves each pixel
  `grey` of the way to its own grey, so `Tinted` can't stand in for it.
  2337's **Out** says "freeing or bounding `render.rs`'s
  recoloured-texture cache for `Shaded` ... a follow-up if a game
  animates lights". No ticket was written for it; this is that ticket.

The fix: draw `Paint::Shaded` from the image's own texture through a
shader that does `Light::apply`'s sum, the light in a uniform, so a new
light costs nothing but a uniform. The recoloured copies that remain
(`Silhouette`, `Dimmed`, and `Shaded` where the shader didn't load) go
in a cache with a cap.

Whichever way a light is drawn, in the shader or as a capped copy, its
multiply is on sRGB-encoded values, as 3227's ADR (the frame's colour
space, blending and alpha) decides. 3227 leaves the cost of animated
lights to this ticket.

Graphics are a skin (ADR-0002 §4, `AGENTS.md` rule 9): nothing here
reaches a scene, a view or the rules. The reference stays the raster
(`shuyi_ui::frame::raster`, `Paint::apply`): the window must match it
within the render card's tolerance, 2 per channel and no pixel past it
(`Tolerance::default`, `crates/ui/src/frame/raster.rs:445`). The card
already draws a `Shaded` sprite (`raster/card.rs:335`), so CI's `render`
and `web-render` jobs check the shader on desktop GL and WebGL.

**The wgpu renderer.** PR 532 (research 3034, open on 2026-11-23)
writes ADR-0136, whose §2 says "Recolouring stays on the CPU" in the
planned `shuyi-gpu` renderer (parked 3325), to keep the integer rules
exact, and that "a shader can take them over later without a change to
the API". This ticket is that later, for `Paint::Shaded`, on today's
backend. Its ADR says why a light may be within 2 per channel rather
than exact.

## Nick input

None.

## Scope

**In:**
- **A light material** in `shuyi-app`, made once in `Renderer::new`
  with `macroquad::material::load_material` beside 2337's two blend
  materials, and built the way they are (2337, *Scope*, `shuyi-app`):
  - the vertex shader is 2337's copy of macroquad's GLSL 100 one;
  - the fragment shader samples the texel `t`, computes
    `g = dot(t.rgb, vec3(0.299, 0.587, 0.114))` and
    `rgb = mix(t.rgb, vec3(g), grey) * tint`, then multiplies by the
    draw colour as macroquad's own fragment shader does. Those are
    `Light::apply`'s weights and order. Its floats are `mediump`, not
    `lowp`: `lowp` may keep only about 8 bits, and a mix then a
    multiply would round past 2 per channel. GLSL ES 1.00 guarantees
    `mediump` in a fragment shader;
  - two uniforms, `grey: Float1` and `tint: Float3`, the light's
    thousandths over 1000;
  - the light material keeps 2337's alpha blending. A `Shaded` sprite
    with `Blend::Add` or `Blend::Multiply` takes a recoloured copy, as
    today (it is rare; a material per blend and light would be
    three more pipelines).
- **`draw_sprite` draws `Paint::Shaded` through it.** It takes the
  image's own texture (`self.images.get`), sets the uniforms, and draws
  with the brightness and opacity as the draw colour, as today.
  - `draw_items` switches material only when the item's material or
    light differs from the last item's, and returns to the default
    material after the last one, so items stay in order. A ground in one
    light is one run.
  - The choice is a pure function the tests call:
    `fn shading(paint: Paint, blend: Blend, light_material: bool) ->
    Shading`, where `Shading` is `Own`, `Lit(Light)` or
    `Copy(Recolour)`. It lives in a module of its own
    (`crates/app/src/shading.rs`) so `render.rs` doesn't grow.
- **The recoloured copies are bounded.** `recoloured` becomes a
  `RecolourCache<T>` (generic over the texture, as `ImageTextures<T>`
  and `PaintedTextures<T>` are, so it is tested without a window), in
  `shading.rs`:
  - at most `RECOLOURED_MAX = 512` copies; a new one past that drops
    the copy drawn longest ago (each entry keeps the frame it was last
    drawn in);
  - `forget(image)` does what `Renderer::remake`'s `retain` does today;
  - a `None` (a file that doesn't decode) counts as an entry too, so a
    bad file isn't decoded every frame;
  - the module doc says why the cap exists and what it costs when a
    frame needs more than 512 (copies made again, never a wrong
    picture).
- **The light material fails to load:** logged once, and `Shaded`
  falls back to recoloured copies in the bounded cache, as 2337 falls
  back to `Alpha`. It never panics.
- **Docs:**
  - `Paint::Shaded`'s doc comment (`crates/ui/src/frame/glyph_buffer.rs`):
    2337 adds "each light makes a copy of the image in `shuyi-app`";
    replace it with: a light that changes every frame costs a uniform,
    not a texture, and `Tinted` is still cheaper when there is no grey;
  - `render.rs`'s module doc: a `Shaded` sprite is drawn from its
    image's own texture through the light material;
  - `docs/platform-backends.md`, *`present`*: a backend draws
    `Paint::Shaded` within the frame's tolerance of `Paint::apply`, and
    may do it in a shader;
  - an ADR (the `write-adr` skill, the next free number): `Shaded` is
    drawn in a shader within 2 per channel of `Light::apply`, and
    `shuyi-app`'s recoloured copies are capped. If
    `docs/adr/0136-replacing-macroquad-winit-wgpu-and-wasm-bindgen.md`
    is in `main` by then, the ADR amends its §2, and the PR adds a line
    to 3325's **Scope**: `shuyi-gpu` draws `Paint::Shaded` through the
    same light shader, and its recoloured copies keep this cap.
- **The render card** (`shuyi_ui::frame::raster::card`, 2392): beside
  the `dusk` sprite, a sprite in each bound of a light: `grey` 0,
  `grey` 1000, a `tint` of 0 in one channel and 1000 in the others. The
  `render` and `web-render` jobs then check each bound against the
  raster.
- **Either order with 2335** (turned sprites): if 2335 is done, its
  texture made on the CPU for a turned sprite leaves a `Paint::Shaded`
  light out of its pixels and its cache key, and is drawn through the
  light material, as it does for 2337's `Tinted`. If not, nothing to do
  (2335 does it; its clause with 3228 says how).
- **Either order with 3341:** if 3341 is done, lower the `lights`
  frame's upload budget in `crates/app/examples/render_stress.rs` to 0
  (no recoloured copy per light once the plain images are up), and check
  that 3341's draw counter counts the light material's pipeline as a
  reason a draw call starts; if not, nothing to do (3341 builds the
  `lights` frame on the light material).
- **For the game:** no API change. A `Shaded` sprite's window pixels may
  move by up to 2 per channel; the raster and every snapshot are
  unchanged. The completion notes get a `Behaviour:` line:
  `Behaviour: shuyi-app: a Paint::Shaded sprite is drawn by a shader, within 2 per channel of Paint::apply, and recoloured copies are capped at 512`.
  The game's PR that moves the pin follows this one.
- **Public types (ADR-0039):** none new in a public API; `Shading`,
  `RecolourCache` and `RECOLOURED_MAX` are `pub(crate)`.

**Out (do not do):**
- Changing `Light`, `Light::apply`, `Paint::apply` or the raster: the
  CPU sum stays the reference.
- `Paint::Dimmed` and the silhouette in a shader: neither changes over
  time, so one copy per image is enough.
- The light cycle itself, `Light::mix` and `LightCycle` (2369 builds
  them; it waits on this ticket).
- Point lights, normal maps and lit world layers (parked 2328, on the
  wgpu renderer), and a game's own shaders and materials (parked 2327).
- The wgpu renderer (parked 3325): only the ADR line and its **Scope**
  line above, and only if ADR-0136 is merged.
- How `recoloured` decodes a picture (2234 moves it off PNG only).
- The one-bit renderer: it computes each pixel already and makes no
  copy.

**Estimated size:** about 700 lines (code 300, tests 250, docs and the
ADR 150).

## Implementation steps

1. Read `crates/app/src/render.rs` (`Recolour`, `draw_sprite`,
   `draw_items`, `recoloured`, `remake`), `image_textures.rs` and
   `painted.rs` (the generic caches), 2337's materials as merged, and
   macroquad 0.4.16's `src/material.rs` (`load_material`,
   `MaterialParams::uniforms`, `Material::set_uniform`) and `src/quad_gl.rs`
   (its fragment shader, and when a uniform change starts a new draw
   call).
2. Write `crates/app/src/shading.rs`: `Shading`, `fn shading`,
   `RecolourCache<T>` with `RECOLOURED_MAX`, and their unit tests.
3. Write the light fragment shader and load the material in
   `Renderer::new`, logging a failure once.
4. Change `draw_sprite` and `draw_items` to draw `Shading::Lit` through
   the light material, switching material and uniforms only on change,
   and the copies through `RecolourCache`. `remake` calls `forget`.
5. Add the light's bounds to the render card, and run the `render_card`
   example locally against the raster (`--features capture`).
6. Write the doc comments, `docs/platform-backends.md`'s line and the
   ADR, then run the gates.

## Acceptance criteria

- [ ] With the light material loaded, `shading(Paint::Shaded(l),
      Blend::Alpha, true)` is `Lit(l)` for any light `l`, so no copy is
      made; with `false`, or with `Add` or `Multiply`, it is
      `Copy(Recolour::Shaded(l))`.
- [ ] Drawing one image in 2,000 different lights, one after another,
      leaves `RecolourCache` at no more than 512 entries, through the
      fallback path (a unit test over `RecolourCache<u32>`).
- [ ] The entry dropped past the cap is the one drawn longest ago; one
      drawn in the last frame is never dropped while an older one is
      there.
- [ ] `forget(image)` removes every copy of that image and no other.
- [ ] The render card, with the four light bounds, matches the raster
      within `Tolerance::default()` (2 per channel, no pixel past it) in
      CI's `render` and `web-render` jobs.
- [ ] No existing snapshot or raster test changes.
- [ ] A light material that fails to load is logged once and the frame
      still draws, through copies.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not (no API changes; likely the latter).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `shading` for every paint, each blend and both material states;
  `RecolourCache` at the cap, past it, the order of eviction, `forget`,
  and a `None` entry kept.
- Property: for any sequence of draws, `RecolourCache` never holds more
  than `RECOLOURED_MAX` entries, and a lookup of a key just inserted
  hits.
- Snapshot / integration: the render card's four light bounds and
  `dusk`, window against raster, desktop and web (the `render` and
  `web-render` jobs).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
