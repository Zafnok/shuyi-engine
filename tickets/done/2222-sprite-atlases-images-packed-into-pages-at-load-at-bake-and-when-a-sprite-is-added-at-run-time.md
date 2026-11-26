---
id: "2222"
title: "Sprite atlases: images packed into pages at load, at bake time, and when a sprite is added while the game runs"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2219"]
nick_input: none
completed: 2026-11-26
---

# 2222 — Sprite atlases

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): every feature
on Bevy's homepage. Its 2D renderer lists "sprite sheets, dynamic
texture atlases". Sprite sheets come with 1123 (Aseprite and
TexturePacker, parked), which imports a sheet into this ticket's
`Atlas` as an atlas of one page, so there is one `Atlas` type for
sheets and for packed images. The glyph atlas exists
(`crates/cli/src/font_atlas.rs`).
What is missing is packing loose images into atlas pages, so a
backend binds one texture for many sprites, and adding one at run time
(a player's avatar, text rendered once, a generated icon).

The packer is pure and deterministic: the same images in the same order
give the same pages, so a baked atlas can be tested equal to the one
packed at load (ADR-0007).

**From 2399 (done):** a picture a skin paints at run time
(`shuyi_ui::frame::painted`, ADR-0078) is never a source of the atlas
here: `shuyi-app` keeps a texture per painted picture, uploaded when its
revision moves. Packing painted pictures into pages is left out of this
ticket too unless it says otherwise.

## Nick input

None.

## Scope

**In:**
- **`shuyi_assets::atlas`:**
  - `AtlasBuilder::new(page_size, padding)`, `.add(name, &Bitmap)`, and
    `.build() -> Atlas` with pages (`Bitmap`s) and a table of name to
    page and rectangle. `Atlas` is plain data that an importer can fill
    without the packer (1123 does, and adds the optional fields a
    trimmed frame needs);
  - a skyline packer, images sorted by height then name, so the result
    doesn't depend on the order they were added;
  - `Atlas::insert(name, &Bitmap) -> Result<Placed, Full>` for a sprite
    added at run time: placed in free space on a page, or a new page;
    `Placed` says which page changed so a backend uploads it again;
  - pages never larger than the target profile's texture limit
    (`targets/*.ron`; a field added with a default of 4096 if it has
    none).
- **At load:** an `atlas.ron` in a folder (`AtlasSpec { from: "sprites/",
  page_size, padding }`) is a format that packs every image in the
  folder through the registry (`load_each`, 2219, needed first).
- **Either order with 1123** (parked): if 1123 is done, `Atlas` is in
  `shuyi_assets::atlas` already, with the optional fields a trimmed
  frame needs and a RON sidecar format of its table. Add
  `AtlasBuilder`, the packer and `insert` to that type, keeping its
  fields, and tell `atlas.ron` apart from that sidecar as 1123's
  completion notes say (two formats of one kind can't claim one
  extension, ADR-0019 §2). If not, write `Atlas` here, and 1123 adds
  its fields and its sidecar (its Scope says so).
- **At bake:** `shuyi bake` (2204) stores packed pages, and the pack's
  equality test covers an atlas fixture.
- **In `shuyi-app`:** the sprite skin draws a sprite from its atlas page
  and rectangle; a page that changed is uploaded again before the frame.
  **From 2919 (done):** ADR-0128 §5 treats a page as a picture's
  texture: kept for the run, never freed (its trigger, textures over a
  quarter of `peak_heap_bytes`, isn't crossed), and an atlas's names are
  its images' paths, `&'static str` as every path stays (§1).
  **From 3317 (done):** a picture's texture is made the first frame a
  sprite draws it (`ImageTextures`, `crates/app/src/image_textures.rs`):
  a page is uploaded when it is first drawn, as a picture is, and again
  when it changes.
  **From 2234 (done):** `crates/app/src/render.rs` has
  `upload_rgba(width, height, rgba)`, which uploads pixels as a texture
  with the nearest filter, and every picture is uploaded through it
  (from `render::pixels`, which reads 2203's formats too): upload a page
  with it.
- **In the raster** (`shuyi_ui::frame::raster`): atlas pages and inserted
  sprites are drawn as the window draws them, so `frame-png` keeps
  showing what players see. Add an atlas sprite and an inserted one to
  the render card (`raster::card`, 2392).
- Fixtures: images of our own, made for the tests.
- **Docs** (`docs/formats.md`): a row of *The built-in formats* table
  for `atlas.ron`. In *The kinds of ADR-0007 §1*, split the row that
  gives `Atlas` and `AnimationSet` to 1123: `Atlas` is `atlas::Atlas`
  (2222), with Aseprite and TexturePacker as its formats in 1123, and
  `AnimationSet` is 2231's, whose Docs item names its type.
- **Either order with 3229:** if 3229 is done, add the atlas pages to
  `Renderer::forget_textures` (`crates/app/src/render.rs`), so a lost GPU
  context drops them and they are packed again from the kept sources on
  the next frame; if not, nothing to do (3229 adds the pages to its
  forget path when it lands after this one).

**Out (do not do):**
- rotation or trimming of sprites in the packer;
- the wgpu renderer (3325, parked). It refuses a texture past the
  device's `max_texture_dimension_2d`, 2048 on WebGL 2 (ADR-0136 §3), so
  the web's target profile gives a texture limit of 2048, not the
  default 4096.

## Implementation steps

1. Write the packer and its tests.
2. Add `insert`, the spec format and the bake support.
3. Draw from atlases in `shuyi-app` and in the raster.
4. Run the gates.

## Acceptance criteria

- [x] No two packed rectangles overlap, and each is inside its page with
      its padding (property test over random sizes).
- [x] The same images in any order give the same atlas (property test).
- [x] An atlas packed at load equals the baked one (the pack's test).
- [x] An inserted sprite is drawn on the next frame (a `frame-png`
      snapshot).
- [x] The render card holds an atlas sprite and an inserted one, and
      2018's `render` job passes.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Property: no overlap; order independence; `insert` never moves a
  placed sprite.
- Snapshot: a frame drawn from an atlas.

## Completion notes

**Done:**
- `shuyi_assets::atlas` (`crates/assets/src/atlas.rs`): `Atlas` (the kind,
  `atlas`), `Page`, `Placed { page, rect }`, `AtlasError`, `AtlasBuilder`,
  `AtlasSpec` and `AtlasRon`. A bottom-left skyline packer, the tallest
  first then by name; each page keeps its skyline, so `Atlas::insert` puts
  a sprite in free space or on a new page and moves none. `AtlasRon` is a
  standard format (`atlas`, `.ron`): an `atlas.ron` packs every image
  directly inside `from` through a registry of the picture formats whose
  features are on (`Formats::load_each`).
- `target::Display::max_texture_side` (4096 unless said, 2048 in
  `targets/web.ron`), given to `AtlasBuilder::with_max_side` and
  `AtlasRon::with_max_side`.
- Bake: `Atlas` is `Bakeable`; the pack's test bakes `sprites/atlas.ron`
  and checks the atlas read from the pack equals the one packed at load.
  `pack::VERSION` is 24 (the target profile's binary form grew; 23 went to 2280 on `main` first), and the
  binary form of `Atlas` is pinned beside the others.
- `shuyi_ui::frame::atlas::{show, insert, sprite, page_id}`: a page is a
  painted picture of the frame (ADR-0078) named by the page, so
  `shuyi-app` uploads it the first frame a sprite draws it and again when
  `insert` moves its revision (through `upload_rgba` now), and the raster
  paints it. ADR-0155 records why.
- The render card holds an atlas sprite (`Over`) and one inserted after
  its page was shown (`Under`); a raster snapshot shows a sprite inserted
  between two frames drawn on the next one.
- Docs: `docs/formats.md` (a built-in formats row, *Sprite atlases*, the
  kinds row split), the book's *Frame and skins*, ADR-0155. Handoffs
  written in 1123 and 3229 as **From 2222 (done)**.

**Deviations:**
- The ticket's `.add(name, &Bitmap)`: `bitmap::Bitmap` is the one-bit
  file reader; RGBA pixels are `image::Image`, which `add` and `insert`
  take.
- `AtlasBuilder::new(id, page_size).with_padding(padding)`, not
  `new(page_size, padding)`: ADR-0151 (no two neighbouring numbers), and
  an id names the pages (`id#0`), which a frame needs as `&'static str`.
- `build()` gives `Result<Atlas, AtlasError>` and `insert` gives
  `Result<Placed, AtlasError>`, not `Full`: an atlas is never full (a new
  page is made), so the errors are a sprite too big for a page, a name
  taken, and a picture with no pixels.
- "The sprite skin draws a sprite from its atlas page": an atlas sprite is
  a `PaintedSprite` of the page's picture, not a `Sprite` (ADR-0155 §5),
  so it has a painted sprite's brightness and clip, not a sprite's paints,
  opacity or mirroring. The window and the raster needed no new path.
- 1123 isn't done, so `Atlas` is written here and 1123 adds its import
  fields (its Scope says how, from this ticket).
- 3229 isn't done: nothing to do; its ticket now says the painted
  textures' forget path covers the pages.
- The `render` job (2018) is CI's: the render card example compares the
  window with the raster there; it wasn't run locally.

Breaking: shuyi-assets: `pack::VERSION` and `formats.ron` "target profile: target-ron": a target profile's binary form has `max_texture_side`, and a baked pack is of version 24; a pack baked before is refused, so a game bakes again with `shuyi bake`.

```migration
since: 0.1.0
what: The pack format is version 24, its target profiles with a texture limit; a pack of version 23 is refused.
old: "pack::VERSION == 23"
new: "pack::VERSION == 24"
before: |
  cargo shuyi bake   # with the engine before 2222
after: |
  cargo shuyi bake   # again, with this engine
```

Behaviour: shuyi-assets: `Formats::standard()` has an `atlas` format of the new `Atlas` kind, claiming `.ron`: `Formats::load_under::<Atlas>` reads every `.ron` file of a folder as an `atlas.ron`.
Behaviour: shuyi-assets: `TargetProfile`'s `display` has `max_texture_side` (4096 unless said; the engine's `web` profile says 2048).
Behaviour: shuyi-app: a painted picture's texture is made with `upload_rgba` (the same nearest filter as before).

Skills: none, the skill pack's worked examples draw no atlas and name no pack version.

Game: nothing to adopt, the game bakes no packs, builds no `Display` literal and keeps no golden row of a target profile (searched `gh search code --repo Zafnok/visions-of-shuyi` for `TargetProfile`, `pack::VERSION` and `max_texture_side`: `TargetProfile::from_source` of its Playdate profile reads the new field's default).

Size: 1,554 counted lines, past 1,500 only by the skyline's direct tests that CI's mutation testing asked for; the frame half (`shuyi_ui::frame::atlas`, the render card) draws the assets half's `Atlas`, and the ticket's acceptance needs both, so they landed together.

Nick input: none.
