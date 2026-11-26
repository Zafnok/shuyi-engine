---
id: "3246"
title: "A corner layer's pictures at their own size and offset, leaving out the tiles of some terrains"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-25
---

# 3246 — A corner layer's pictures at their own size and offset, leaving out the tiles of some terrains

## Context

- **The game ticket:** game#1359
  ([`tickets/open/1359-chapter-1s-hills-and-peaks-as-raised-ground.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1359-chapter-1s-hills-and-peaks-as-raised-ground.md)).
  It is not a blocker.
- **The job:** draw raised ground on a grid map: a cliff's face hanging
  from its rim over the tile below, a drop shadow beside it, and no face
  or shadow painted over a wall.
- **Friction numbers:** no friction review: a missing feature. Neither a
  corner layer (pictures one tile big, centred on the point where four
  tiles meet) nor a tile layer (one picture per tile) can hang a picture
  lower than one tile or keep it off some tiles.
- **The generic shape:** a corner layer's picture may be another size
  than a tile and sit off its point, and a corner layer may leave out the
  tiles of some terrains. Raised ground then hangs its face from the
  rim's corner pieces (a picture taller than a tile, reaching down past
  its point), draws a drop shadow as a second corner layer of the same
  pieces moved a few pixels right and down, and leaves walls bare.
- **An example that isn't tactics:** a farming game's raised field beds
  and terraces (a soil face under each bed's edge, a shadow on the path
  beside it, none on the fence tiles), or a top-down dungeon crawler's
  ledges and pits.

Today `shuyi_assets::tileset::CornerLayer` (`crates/assets/src/tileset.rs`)
is `{ of, tiles }`, and `paint_corners` in
`crates/ui/src/grid/sprite/ground.rs` draws each mix's picture on
`Rect::new(left + i*w - w/2, top + j*h - h/2, w, h)` (always one tile,
whatever the picture's size), clipped to the four tiles round the point
that are drawn and on the map.

## Nick input

`None.` (Nick approved the engine ticket and PR for game#1359.)

## Scope

**In:**
- `CornerLayer` gains `skip: BTreeSet<TerrainId>` (tiles of these
  terrains get no part of the layer's pictures) and `offset: (i32, i32)`
  (pixels at the art's size, added to where a picture's top-left goes;
  `(0, 0)` is today's place, half a tile up and left of its point).
- `CornerLayer::new(of, tiles)`, with no skip and no offset, and the type
  `#[non_exhaustive]` (ADR-0039, config a game fills in), so a later
  field is additive.
- A picture is drawn at its own `ImageRect` size times the grid's zoom,
  as a tile is; clipped to every tile its rectangle covers that is drawn,
  on the map, and not shown as a `skip` terrain (`Shown::at`, the
  terrain the mix uses). One sprite when nothing is clipped away.
- **The game:** the API breaks (the struct literal `CornerLayer { of,
  tiles }` no longer builds). The game's PR that moves the pin follows
  this one and changes its two literal sites to `CornerLayer::new`
  (`crates/content/src/tileset/look.rs` and
  `crates/ui/src/map_view/sprite.rs`, found with `grep -rn "CornerLayer {"`
  in the game at its pin). The game's corner pictures are cut at its
  `tile_px` (`look.rs`, `grid_rect(at, self.tile_px, …)`), so they paint
  as before.

**Out (do not do):**
- A file format for these fields: how a game writes its tiles is its own
  (the module docs of `tileset.rs`).
- Tile layers (`TileLayer`) gaining size or offset; projections other
  than top-down (`paint_corners` stays square, as 2740 says).
- 2259 (corner sheets made from one tile) and 2359 (animated tiles) are
  not changed by this: either order works, and neither names a field
  this adds.

**Estimated size:** about 400 lines (code 90, tests 270, docs 40).

## Implementation steps

1. `crates/assets/src/tileset.rs`: the fields, `CornerLayer::new`,
   `#[non_exhaustive]`, the module and type docs; a test that `new` is
   the literal with the defaults.
2. `crates/ui/src/grid/sprite/ground.rs`: `paint_corners` places
   `dest` at the old top-left plus `offset × zoom`, sized `rect × zoom`;
   a helper `parts` returns the clips inside each covered tile that is
   drawn, on the map and not skipped, and whether that is every covered
   tile (the fast path). Module docs.
3. `crates/ui/src/toy/grid.rs`: build the shore with `CornerLayer::new`.
4. Tests (below); docs that describe corner layers.

## Acceptance criteria

- [x] A 24 × 36 picture with offset `(0, 12)` on 24 px tiles is drawn
      from its point down 36 px: whole inside the view, cut at the view's
      edges, and cut where the map ends.
- [x] At zoom 2, a 12 × 18 picture with offset `(0, 6)` on 12 px tiles
      covers the same pixels as the case above.
- [x] A tile shown as a skipped terrain gets no part of any picture of
      the layer, while its neighbours get theirs; a tile that would
      become that terrain is skipped too.
- [x] A tile-sized picture with no offset paints exactly as before: the
      existing ground tests pass unchanged except where a test drew
      pictures of another size than its tiles (then it is given
      tile-sized ones).
- [x] The property test holds with a taller, offset, skipping layer
      added: nothing spills off the map, nothing lands on a skipped tile.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `CornerLayer::new`'s defaults; in `ground.rs`, a taller picture
  with an offset (in the view, at its edges, at the map's end, at zoom
  2), and a skipped terrain (shown, and becoming).
- Property: `every_tile_of_the_map_has_one_picture_of_its_own_and_none_spills`
  with a third layer: a 24 × 37 face at offset `(5, -7)` that skips the
  bridge.
- Snapshot / integration: none new; the existing ones unchanged.

## Completion notes

`CornerLayer` (`crates/assets/src/tileset.rs`) gains `skip:
BTreeSet<TerrainId>` and `offset: (i32, i32)`, is `#[non_exhaustive]`,
and has `CornerLayer::new(of, tiles)` (no skip, offset `(0, 0)`).
`paint_corners` (`crates/ui/src/grid/sprite/ground.rs`) draws a picture
at its own size times the zoom, its top-left at the old place plus
`offset × zoom`, and its new helper `parts` clips it to every tile its
rectangle covers that is drawn, on the map and not shown as a skipped
terrain (one sprite when none is cut away). The toy's shore uses `new`.

- Tests: `a_new_corner_layer_skips_nothing_and_sits_on_its_points`;
  `a_picture_between_tiles_may_be_taller_than_a_tile_and_sit_off_its_point`
  (in the view, its edges, the map's end, zoom 2, a picture of no
  pixels); `a_tile_of_a_skipped_terrain_gets_no_part_of_a_picture_between_tiles`
  (shown and becoming); the property test with a 24 × 37 face at
  `(5, -7)` that skips the bridge.
- Docs: the module docs of `tileset.rs` and `ground.rs`, and
  `CornerLayer`'s. No other doc describes corner layers (`grep -rni
  "corner layer" docs site skills` finds only `docs/carve.md`'s history).
- Deviations: `an_odd_tile_is_cut_where_its_pictures_meet` drew the toy's
  24 px pictures on 9 × 13 tiles, which only worked because every picture
  was squeezed to a tile; it now gives its layer 9 × 13 pictures, with
  its expected clips unchanged. `#[non_exhaustive]` is added beyond the
  request, as ADR-0039 asks of config a game fills in, so the next field
  is not a break.

Breaking: shuyi-assets: `tileset::CornerLayer`: it is `#[non_exhaustive]` and gains `skip` and `offset`; a game that built one by a literal calls `CornerLayer::new(of, tiles)` and assigns the new fields if it wants them

```migration
since: 0.1.0
what: CornerLayer is non_exhaustive and gains skip (terrains whose tiles get none of its pictures) and offset (pixels at the art's size); build it with CornerLayer::new.
old: CornerLayer { of, tiles }
new: "#[non_exhaustive] CornerLayer { of, tiles, skip, offset }, CornerLayer::new(of, tiles)"
before: |
  let layer = CornerLayer { of, tiles };
after: |
  let layer = CornerLayer::new(of, tiles);
find: ["CornerLayer {"]
```

Behaviour: shuyi-ui: a corner layer's picture is drawn at its own size (times the zoom), not squeezed to one tile; a tileset whose corner pictures are a tile big paints exactly as before.

Searched the game (pin `b69585ee`): `gh search code --repo
Zafnok/visions-of-shuyi 'CornerLayer'` finds two literals,
`crates/content/src/tileset/look.rs` (pictures cut at the tileset's
`tile_px`, `grid_rect(at, self.tile_px, …)`) and the test skin in
`crates/ui/src/map_view/sprite.rs` (24 px pictures on the `test`
tileset's 24 px tiles): both change to `CornerLayer::new`, and both
paint as before.

Game: game#1359 moves the pin and changes the two literals.

Book: none, the book has no chapter on terrain tiles or their layers (`grep -rni "tileset\|corner" site/content/learn` finds none about them).

Skills: none, no schema, template, format or problem code changes, and no skill describes corner layers.
