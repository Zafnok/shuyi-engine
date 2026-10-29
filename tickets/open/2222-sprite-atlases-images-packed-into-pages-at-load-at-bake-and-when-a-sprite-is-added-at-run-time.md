---
id: "2222"
title: "Sprite atlases: images packed into pages at load, at bake time, and when a sprite is added while the game runs"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2219"]
nick_input: none
completed:
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
  **Either order with 2234** (pictures in 2203's formats): if 2234 is
  done, `crates/app/src/render.rs` has `upload_rgba(width, height,
  rgba)`, which uploads pixels as a texture with the nearest filter:
  upload a page with it. If not, write it there and upload pages with
  it, and 2234 uploads its pictures with it.
- **In the raster** (`shuyi_ui::frame::raster`): atlas pages and inserted
  sprites are drawn as the window draws them, so `frame-png` keeps
  showing what players see. **Either order with 2392:** if 2392 is
  done, add an atlas sprite and an inserted one to its render card
  (`raster::card`); if not, nothing to do (2392 adds them).
- Fixtures: images of our own, made for the tests.
- **Docs** (`docs/formats.md`): a row of *The built-in formats* table
  for `atlas.ron`. In *The kinds of ADR-0007 §1*, split the row that
  gives `Atlas` and `AnimationSet` to 1123: `Atlas` is `atlas::Atlas`
  (2222), with Aseprite and TexturePacker as its formats in 1123, and
  `AnimationSet` is 2231's, whose Docs item names its type.

**Out (do not do):**
- rotation or trimming of sprites in the packer;
- the wgpu renderer (3034, parked).

## Implementation steps

1. Write the packer and its tests.
2. Add `insert`, the spec format and the bake support.
3. Draw from atlases in `shuyi-app` and in the raster.
4. Run the gates.

## Acceptance criteria

- [ ] No two packed rectangles overlap, and each is inside its page with
      its padding (property test over random sizes).
- [ ] The same images in any order give the same atlas (property test).
- [ ] An atlas packed at load equals the baked one (the pack's test).
- [ ] An inserted sprite is drawn on the next frame (a `frame-png`
      snapshot).
- [ ] If 2392 is done, its render card holds an atlas sprite and an
      inserted one, and 2018's `render` job passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: no overlap; order independence; `insert` never moves a
  placed sprite.
- Snapshot: a frame drawn from an atlas.

## Completion notes

*(Filled in by the session that completes the ticket.)*
