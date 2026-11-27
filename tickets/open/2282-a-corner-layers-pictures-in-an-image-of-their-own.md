---
id: "2282"
title: "A corner layer's pictures in an image of their own: a made corner sheet drawn from its own PNG, not pasted into the tileset's picture"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2282 — A corner layer's pictures in an image of their own

## Context

2259 (done) makes a terrain's 15 corner pictures into a PNG of their own
(`shuyi_assets::corners`, `shuyi-cli`'s `corners` command, ADR-0159).
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

- [ ] A corner layer with its own image is drawn from it, by the sprite
  skin and the raster painter; every existing snapshot is unchanged.
- [ ] The format reads `image:` on a corner layer; an image not in the
  table, and a picture outside it, are errors naming the look and layer.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the format's field, resolved and refused.
- Snapshot / integration: 2259's look snapshot with no paste.

## Completion notes
