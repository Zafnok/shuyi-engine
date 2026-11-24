---
id: "3237"
title: "Parked: Screen effect passes and a palette-swap material are assets that draw on today's backend, with CPU references in the raster"
type: feature
milestone: "Engine: 2D effects"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3525", "2337", "2340"]
nick_input: none
completed:
---

# 3237 — Screen effect passes and a palette-swap material on today's backend

> **Parked** (written 2026-11-23 from the engine blind-spot audit): it may start only when Nick unparks it, through decision ticket 3525 (its question on game shaders and screen effects before the renderer rewrite). If he answers that they wait for the rewrite, this ticket is closed as not needed and 2326 and 2327 stay the plan. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

The blind-spot audit of 2026-11-23 found that a 2D game gets no screen
effect and no shader of any kind before the parked renderer rewrite:

- 2326 (screen effects: CRT, bloom, palette remap, vignette) is blocked
  on 3034, 2018 and 2340, and PR 532 adds `shuyi-gpu` (3325) to it;
- 2327 (a game's own WGSL passes and sprite materials, with a built-in
  `palette_swap`) is blocked on 2326;
- 3325 (`shuyi-gpu`, ADR-0136 in PR 532) is parked until the MVP and
  until quad-snd is removed (3079).

Godot, Defold, GameMaker, LÖVE and macroquad itself give a game shaders
and render targets from the start. A CRT look, a day-to-night palette
remap and team colours on a shared sprite are among the first things a
2D developer reaches for.

Shaders on today's backend are already accepted: 2337 (open, After MVP)
puts two GLSL 100 materials on macroquad with
`macroquad::material::load_material`, a safe function. macroquad 0.4.16
(the version in `Cargo.lock`) also has a safe
`macroquad::texture::render_target(width, height)`
(`src/texture.rs:405`) and `Material::set_uniform`
(`src/material.rs:32`). `shuyi-app` denies `unsafe_code`; neither needs
it.

So this ticket lands the half of 2326 and 2327 that does not depend on
the GPU backend, and draws it on macroquad:

- the data contract: an `EffectChain` asset, a `MaterialSet` asset with
  a built-in palette swap, and the frame fields that name them;
- the CPU references in the raster (ADR-0052: the raster is the
  reference picture), so `frame-png` shows the effect without a GPU;
- a first GPU implementation in `shuyi-app` on macroquad, compared with
  the raster in CI as 2337's blends are.

When the wgpu renderer comes, its work is a port of passes already
specified and tested against the same raster, not a new feature.

Graphics are a skin (ADR-0002 §4, `AGENTS.md` rule 9). Effects and
materials are look data a skin sets on the frame. No scene, view,
simulation or record holds them, and tests of what happened never read
them.

Checked on 2026-11-23: no type named `EffectChain`, `MaterialRef`,
`MaterialSet` or `palette_swap` exists in `crates/`; `EffectChain`,
`MaterialRef` and `palette_swap` appear only in 2326, 2327, 2337 (a
line about 2327) and 3017 (its clauses with 2326 and 2327).
`crates/app/src/render.rs` draws straight to the screen and uses
`FilterMode::Nearest` (lines 432, 501, 524); it has no render target.

## Nick input

None. (The decision to start it is 3525's.)

## Scope

**In:**
- **`EffectChain`**, an asset kind in `shuyi-assets`
  (`crates/assets/src/effects.rs`), read from RON (`effects.ron` in a
  skin) through the format registry (ADR-0007, ADR-0019), with the same
  passes and numbers 2326 plans, so 2326 later adds only the wgpu side:
  - `Crt { scanlines: u8, curvature: u8, mask: Aperture | Slot | None }`;
  - `Bloom { threshold: u8, strength: u8, radius: u8 }`;
  - `Remap { to: palette id }`: each colour of the frame's palette
    becomes the same-named colour of another palette (day to night), at
    most 64 colours (checked at load, naming the file and line);
  - `Vignette { strength: u8 }`;
  - each pass has two flags: `optional` (a painter that can't run it
    drops it) and `flashing` (dropped when `Settings::flashing`, from
    2340, is off).
  - `EffectChain` and the pass enum are `#[non_exhaustive]` with
    constructors (ADR-0039): 2327 adds a `Custom` pass later.
- **`MaterialSet`**, an asset kind beside it (`materials.ron` in a
  skin): named materials, of one kind for now,
  `PaletteSwap { from: [colour names], to: [colour names] }` (equal
  lengths, at most 16 pairs, names checked against the skin's palette at
  load). `#[non_exhaustive]`: 2327's shader materials join it.
- **The frame** (`crates/ui/src/frame/glyph_buffer.rs`):
  - `GlyphBuffer` gains `effects: Option<EffectsRef>` (a chain id and
    one `amount: u8` per pass, so a skin fades a pass in and out), with
    `set_effects` and `effects`;
  - `Sprite` gains `material: Option<MaterialRef>` (a material id), set
    `None` by `Sprite::new`, with a `#[must_use] pub const fn
    with_material` builder like 2337's `blended`.
  - Both are additive: `GlyphBuffer`'s fields are private, and
    `Sprite` is already `#[non_exhaustive]`, built with `new` (2082).
    No `Breaking:` line.
- **Snapshots** (`crates/ui/src/frame/snapshot.rs`): words only when not
  the default, so no existing snapshot changes:
  - one line `effects <chain id> <amount> <amount> …` after the
    backdrop line when the frame has a chain;
  - ` material=<id>` on a sprite line, after 2337's ` blend=`.
  - The module doc (the format's spec) lists them, and reserves the
    words ` (not drawn on cpu)` for a material or pass without a CPU
    reference, which 2327's game shaders will use. Every pass and
    material of this ticket has a CPU reference.
- **The CPU references** in `shuyi_ui::frame::raster` (2332), integers
  only, `no_std`:
  - a palette swap replaces a sprite texel whose colour equals a `from`
    colour with the paired `to` colour, before `paint`, tint and blend
    (2337's `frame::blend` rules) apply;
  - after the frame is painted, the chain runs in order on the image at
    the frame's logical size: `Remap` (exact colour match), `Bloom` (a
    threshold, a box blur of `radius` pixels in integers, added at
    `strength`), `Vignette` (a darkening by squared distance from the
    centre, in integers); each pass at `amount` 0 leaves the image as
    it was, and at 255 is the full pass;
  - `Crt` runs last, at the window's size, in the window path only; the
    raster at the logical size draws it as the scanlines alone (every
    other row darkened by `scanlines`), and its doc says so.
- **`shuyi-app`** (`crates/app/src/render.rs`), on macroquad:
  - with a chain, the frame is drawn into a `render_target` at its
    logical size (filter `Nearest`), then each pass is a full-screen
    quad drawn from one target into the other (two targets, ping-pong),
    with a GLSL 100 material per pass made once with `load_material`
    (2337's vertex shader; `lowp` floats as 2337 says); the last target
    is drawn to the screen with today's integer placement; `Crt` is the
    last draw, at the window's size;
  - `Remap` and `PaletteSwap` send their colour pairs as uniforms
    (`set_uniform_array`); the loop in GLSL 100 runs to a constant
    bound (64 and 16) and stops at the count;
  - a sprite with a material draws through its material, with 2337's
    blend state for its `blend`;
  - with no chain and no material the frame draws exactly as today
    (no render target is made);
  - a material that fails to load is logged once and its pass or
    material is skipped; it never panics.
- **Other painters**: the one-bit renderer and the Playdate painter
  ignore `effects` and draw a palette-swapped sprite as
  `Paint::Image` (its doc says so). **Either order with 3017:** if 3017
  is done, add `shaders` to its `Capabilities` and `screen_passes` and
  `materials` to its `Needs` as its clauses with 2326 and 2327 describe
  them, naming `EffectChain` and `MaterialSet` here; if not, nothing to
  do (3017 adds them).
- **Player settings**: `Settings::screen_effects: bool` (on by default)
  in `crates/ui/src/settings.rs`, saved, with an option row. With it
  off, only `Remap` runs (it can carry meaning, such as night). A pass
  marked `flashing` is dropped when 2340's `Settings::flashing` is off.
- **Hot reload**: `effects.ron` and `materials.ron` reload through
  2220's `Hot` values, as other assets do; a broken file keeps the last
  good one, with the error shown as 2220 shows it.
- **CI**: a capture plan per pass and one for the palette swap (2018's
  capture mode, `shuyi_platform::capture`), compared with the raster by
  `raster::compare` within a `Tolerance` the worker picks on the test
  frames and writes why, in 2018's `render` job (Xvfb, llvmpipe) and
  2019's `web-render` job.
- **`hello-grid`** (`examples/hello-grid`) gets a CRT chain, turned on
  and off by the `screen_effects` row, and one palette-swapped sprite.
- **Docs**: `docs/platform-backends.md`'s *`present`* says a backend
  runs a frame's `effects` and a sprite's `material` to the raster's
  rules or drops the optional ones; the book's chapter on the frame and
  skins (`site/content/learn/book/frame-and-skins.md`) gets a short
  section with a chain and a swap, its code in `site/snippets/`.
- **For the game**: its frames don't change (no chain, no material). The
  new fields are additive.

**Out (do not do):**
- a game's own shaders (WGSL or GLSL), `Custom` passes, naga, and
  shader materials: 2327 plans them on the wgpu renderer, where a game
  writes WGSL once;
- the wgpu side of these passes and the swap: 2326 and 2327 already
  plan it on `shuyi-gpu` (3325);
- render-graph nodes (2331 already plans moving the passes into nodes);
- 2D lights (2328) and normal maps;
- whole-frame flash, shake and transitions (2340, 2339);
- blend modes and tint (2337, a blocker);
- smooth filtering or a smooth 2D tier (3236);
- the frame's colour space and blending space (3227): passes work on
  the frame's sRGB bytes, as 2337's blends do.

**Estimated size:** about 2,400 lines (code 1,200, tests 850, docs 350).

**Split:** past 1,200 lines, split off steps 6–8 (the macroquad passes
and material in `shuyi-app`, the capture plans and `hello-grid`) into a
new parked ticket blocked by this one; this PR then ships the assets,
the frame fields, the snapshot words, the raster references and the
setting.

## Implementation steps

1. Read 2326, 2327 and 2337; ADR-0002 §4, ADR-0007, ADR-0019, ADR-0039,
   ADR-0052; `glyph_buffer.rs` (`GlyphBuffer`, `Sprite`), `snapshot.rs`,
   `raster.rs` and `raster/card.rs`, `settings.rs`,
   `crates/assets/src/cinematic.rs` (a RON kind to copy),
   `crates/app/src/render.rs`, and macroquad 0.4.16's `src/material.rs`
   and `src/texture.rs` (`render_target`, `RenderTarget`).
2. Write `crates/assets/src/effects.rs`: `EffectChain`, `MaterialSet`,
   their RON formats and checks with file and line; register both kinds;
   add golden rows `effects: ron` and `materials: ron`
   (`shuyi_assets::golden`) and the random-bytes tests.
3. Add `EffectsRef`, `MaterialRef`, the builders and the snapshot words,
   with their tests.
4. Write the CPU references in `raster` (each pass and the swap), with
   unit and property tests.
5. Add `Settings::screen_effects`, its saved form and its row, and the
   `flashing` and `screen_effects` drop rules (pure functions the tests
   call).
6. In `shuyi-app`, add the render targets, the pass and swap materials
   and the uniform packing (pure functions for the packing and the
   chain's order after the drop rules).
7. Add the capture plans and run them in `render` and `web-render`.
8. Give `hello-grid` its chain and swapped sprite, and add hot reload
   of the two files.
9. Write the docs and run the gates.

## Acceptance criteria

- [ ] No existing snapshot changes: no `.snap` file and no snapshot
      string in a test is edited.
- [ ] Each pass and the palette swap has a committed `frame-png` picture
      of its test frame, with hand-worked pixel values in its unit test.
- [ ] A pass at amount 0 leaves the frame's pixels unchanged (a
      property test in the raster).
- [ ] The desktop and web builds draw each test frame within the plan's
      tolerance of the raster, in the `render` and `web-render` jobs.
- [ ] A frame with no chain and no material draws in `shuyi-app`
      without a render target (a unit test of the pure choice).
- [ ] With `Settings::flashing` off, a `flashing` pass never runs; with
      `screen_effects` off, only `Remap` runs (tests).
- [ ] A chain with 65 remap colours, or a swap with unequal lists, an
      unknown colour name or 17 pairs, fails at load with file and line.
- [ ] Random bytes never panic either format; the golden rows pass on
      wasm32 in the `determinism` job.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each format's checks; each CPU pass on small images at amounts
  0, 128 and 255; the swap on a small sprite, before tint and blend; the
  snapshot words alone and absent at the defaults; the drop rules; the
  uniform packing; the `screen_effects` saved form.
- Property: a pass at amount 0 is the identity; `Remap` and the swap
  with `from == to` are the identity; random bytes into both formats.
- Snapshot / integration: each test frame through `frame-png`; the
  desktop and web windows against the raster through the capture plans.

## Completion notes

*(Filled in by the session that completes the ticket.)*
