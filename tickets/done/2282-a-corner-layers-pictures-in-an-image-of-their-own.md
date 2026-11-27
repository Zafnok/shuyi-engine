---
id: "2282"
title: "A corner layer's pictures in an image of their own: a made corner sheet drawn from its own PNG, not pasted into the tileset's picture"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-27
---

# 2282 — A corner layer's pictures in an image of their own

## Context

2259 (done) makes a terrain's 15 corner pictures into a PNG of their own
(`shuyi_assets::corners`, `shuyi-cli`'s `corners` command, ADR-0160).
But a tileset has one picture: `TerrainTiles::image`, and every rect of
its looks, a `CornerLayer`'s included, is a cell of it
(`crates/assets/src/tileset.rs`). So a game that makes its shore today
has to paste the made sheet into its tileset's picture by hand, or with
a tool of its own, before `corners::layer` (or the 15 rects it writes in
its `terrain-tiles` file) can name the pictures. 2259's look snapshot
does that paste in its test (`crates/cli/src/corners/tests.rs`,
`tiles_picture`).

**From 2259 (done):** the gap was found there, and left out of it: its
**Out** kept the tileset format (2243) and the sprite skin unchanged.

## Nick input

None.

## Scope

**In:**
- A corner layer may name a picture of its own: `CornerLayer` gains a
  field (an `Option<ImageId>`, `None` in `new`: the tileset's), and
  the sprite skin's `paint_corners` (`crates/ui/src/grid/sprite/ground.rs`)
  draws its pictures from it.
- The `terrain-tiles` format: a corner layer may write `image:
  "tiles/water-corners.png"`; `TerrainTilesDef::resolve` looks it up in
  the image table and checks each picture against it.
- `shuyi_assets::corners::layer` (or a sibling) builds such a layer.
- The toy's shore and every existing layer are unchanged (`None`).

**Out (do not do):**
- Several images for one terrain's own tiles.
- Anything in `shuyi_assets::corners`' generator.

**Estimated size:** about 350 lines (code 150, tests 130, docs 70).

## Implementation steps

1. The field on `CornerLayer`, and the skin drawing from it.
2. The format's field, resolved and checked.
3. The 2259 look snapshot without the paste: the layer names the made
   sheet's own image.
4. Docs: `docs/formats.md`'s terrain tiles, the book's paragraph on
   corner sheets (`site/content/learn/book/frame-and-skins.md`).

## Acceptance criteria

- [x] A corner layer with its own image is drawn from it, by the sprite
  skin and the raster painter; every existing snapshot is unchanged.
- [x] The format reads `image:` on a corner layer; an image not in the
  table, and a picture outside it, are errors naming the look and layer.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the format's field, resolved and refused.
- Snapshot / integration: 2259's look snapshot with no paste.

## Completion notes

- `CornerLayer` gains `image: Option<ImageId>` (`None` in `new`, the
  tileset's) and `with_image`; the sprite skin's `paint_corners` draws
  a layer's pictures from its own image when it has one. The raster
  painter needed no change: it draws each sprite from the image the
  sprite names, given that image's pixels.
- `LayerDef::Corners` gains `image: Option<String>` (`image:
  "tiles/water-corners.png"`, left out the tileset's).
  `TerrainTilesDef::resolve` looks it up in the image table (not there:
  `MISSING_FILE`, naming the look and layer) and checks each of the
  layer's pictures against that image (`OUT_OF_RANGE`), not the
  tileset's.
- `shuyi_assets::corners::sheet_layer(of, image, tile_px)`, the sibling
  of `layer`: the made sheet's 15 pictures from its top-left, in an
  image of its own.
- 2259's look snapshot (`crates/cli/src/corners/tests.rs`) now gives the
  shore as its own image to the painter, with no paste into the tiles'
  picture; its committed `shore.png` is unchanged, and so is every other
  snapshot.
- Tests: a skin test (the same sprites, only the image differs), the
  format's field resolved, refused when not in the table and when a
  picture lies outside it though inside the tileset's, written back
  and read again; `sheet_layer`.
- Docs: `docs/formats.md` (the terrain tiles entry and `resolve`'s
  list), the `tileset` module's file, the book's paragraph on corner
  sheets in `frame-and-skins.md`, and the skill `shuyi-import-art`.
- No deviation; the ticket was written today, so its names were current.

Breaking: shuyi-assets: `tileset::LayerDef::Corners`: the variant has a new field, `image`; a game that builds the variant or matches it without `..` adds `image: None` (or `..`).

```migration
since: 0.1.0
what: A corner layer as written may name an image of its own, so `LayerDef::Corners` has a new field, `image`.
old: "LayerDef::Corners { of, tiles, size, offset, skip }"
new: "LayerDef::Corners { of, tiles, size, offset, skip, image }"
before: |
  LayerDef::Corners { of, tiles, size: None, offset: (0, 0), skip: Vec::new() }
after: |
  LayerDef::Corners { of, tiles, size: None, offset: (0, 0), skip: Vec::new(), image: None }
```

Behaviour: shuyi-ui: a corner layer whose `image` is set is drawn from that image by the sprite skin; every layer today has `None` and draws as before.

Game: nothing to adopt, the game builds `CornerLayer` with `new` (which
sets `image: None`) and never builds or matches `LayerDef::Corners`
(searched `gh search code --repo Zafnok/visions-of-shuyi 'LayerDef'` and
`'CornerLayer'`: `crates/content/src/tileset/look.rs` calls
`CornerLayer::new`; game#1219, open, plans `LayerDef` and will see the
`Breaking:` line at its pin move).
