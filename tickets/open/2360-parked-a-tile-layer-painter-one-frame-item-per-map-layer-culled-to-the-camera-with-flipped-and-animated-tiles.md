---
id: "2360"
title: "Parked: A tile-layer painter: one frame item per Tiled or LDtk map layer, culled to the camera, with flipped and animated tiles"
type: feature
milestone: "Engine: real time"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["1120", "1124", "2231", "2335"]
nick_input: none
completed:
---

# 2360 — A tile-layer painter

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): this is real-time work, so it waits until Visions of Shuyi launches, unless the game needs it first (its ticket 0060 says the army walks around freely, which unparks 1119–1125 and this ticket with them, or the game asks through its `engine-change` skill to draw a map made in Tiled or LDtk). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engine GameMaker. Each of them draws tile
maps as a built-in:

- Bevy 0.17 added `TilemapChunk`, a chunk of tiles drawn as one mesh
  (https://bevy.org/news/bevy-0-17/); `bevy_ecs_tilemap` adds animated
  tiles.
- Godot's `TileMapLayer` batches tiles in quadrants, flips and
  transposes them through alternative tiles, and animates atlas tiles
  (https://docs.godotengine.org/en/stable/classes/class_tilemaplayer.html).
- Unity has Tilemap, and Animated Tile in its 2D Tilemap Extras.
- Unreal has Paper2D tile maps. GameMaker has tile layers and animated
  tile sets.

Shuyi has nothing that draws a map made in Tiled or LDtk:

- **1124** imports a `TileMap` into `shuyi-assets` (tile layers as a
  `Grid` of tile ids per tileset, object layers, the map's properties)
  and draws nothing. It keeps what this ticket draws: flips, layer
  opacity, offset and parallax, and Tiled's tile animations as 2231's
  clips. Out says what to do if it landed without them.
- **1120** gives world layers a camera, parallax and sorted sprite
  items. It has no tile-layer item and no culling, and its snapshot
  prints one line per item.
- **The frame today** (`crates/ui/src/frame/glyph_buffer.rs`): an
  `Item` is `Item::Rect` (an `Overlay`) or `Item::Sprite`, and `Sprite`
  has `flip_x` only (line 210). Tiled's vertical and diagonal flips
  can't be drawn.
- **Culling exists only in the grid kit.** `MapScene` holds only the
  visible tiles (`crates/ui/src/grid/scene.rs`, lines 18 to 27), and
  the sprite skin paints `TerrainTiles` as one sprite item per visible
  tile (`crates/ui/src/grid/sprite/ground.rs`). `MapPicture`
  (`crates/ui/src/grid/picture.rs`) paints a whole grid map once, to
  pan over. Both are tied to the grid kit's terrain.
- **Tickets assume it.** 1125 ("a Tiled town map", line 43), 1127
  (line 44) and 2704 (line 42) take the drawing for granted. 3021 and
  3024 put world layers on the GBA's and the DS's hardware backgrounds
  and count "its map's tile layers" (1124) against
  `capabilities.tile_layers`, but no frame item says what a tile layer
  is.

Why it matters:

- **The game first.** Visions of Shuyi draws its grid maps with the grid
  kit and doesn't need this. It needs it if its 0060 lets the army walk
  around freely on a map drawn in Tiled, which unparks 1119–1125.
- **Other 2D games.** The free-movement examples (1125, 1127, and 2704
  once 1124 is done) draw their levels in Tiled or LDtk. Faking a layer
  with 1120's sprite items makes 20,000 items for a 200 by 100 map:
  20,000 snapshot lines, and likely over the web and Playdate budgets
  that 3008 measures. Dropping flips and tile animations loses what the
  artist drew, without a word.
- **The end goal** (Nick, 2026-10-19): "snake or mario" in under an
  hour. A Mario-like needs a drawn level. Under the 2026-10-20 rule it
  waits for the launch with the rest of real time.
- **Graphics are a skin** (`AGENTS.md` rule 9). One plain-data item per
  layer lets each backend draw it its own way: macroquad and the
  reference raster here, the GBA's and the DS's tile backgrounds later
  (3021, 3024).

The painter reads the `TileMap` kind, never a format (rule 10). It adds
no dependency and no file format.

## Nick input

None.

## Scope

**In:**
- **The data**, in a new module `shuyi_ui::frame::tiles`
  (`crates/ui/src/frame/tiles.rs`), `no_std` with `alloc` like the rest
  of `shuyi-ui`, and integers only:
  - `TileFlip { x: bool, y: bool, diagonal: bool }`, in Tiled's order:
    the diagonal flip (a transpose, swapping x and y) first, then x,
    then y;
  - `TileRef { sheet: u8, tile: u16, flip: TileFlip }`: one cell's
    picture;
  - `TileSheet { image: ImageId, tile_px: (u16, u16), columns: u16,
    margin: u16, spacing: u16, anims: BTreeMap<u16,
    AnimationClip<u16>> }`, with `src(tile) -> ImageRect` (`ImageId`
    and `ImageRect` are in `crates/assets/src/image.rs`). `anims` holds
    a tile's clip of tile ids of the same sheet, as Tiled writes them
    (2231's `AnimationClip<F>` is generic over its frame; its default,
    `ImageRect`, would need 1124 to turn ids into rectangles);
  - `MapTiles`: one map tile layer, built once and shared by every frame
    through `Rc`. It holds the layer's name, its size in tiles, the
    map's tile size, a `Vec<Option<TileRef>>` row by row, its sheets,
    its offset in pixels, its opacity and its parallax per axis;
  - `MapTiles::from_map(map: &TileMap, layer: &str, images: &ImageTable)`
    and `MapTiles::all(map, images)` (every tile layer the file doesn't
    mark hidden, in map order), both returning
    `Result<_, MapTilesError>`. The errors name the layer, and the cell
    where there is one: no such layer; an orientation other than
    orthogonal; a diagonal flip on a tile that isn't square; an image
    the table doesn't hold (`ImageTable::id` gives `None`); more than
    256 sheets; a tile id past 65,535 in its sheet;
  - the frame item `TileLayerItem { tiles: Rc<MapTiles>, z: i32,
    opacity: u8, brightness: u8, clock_ms: u64, visible: Rect }`. It is
    not called `TileLayer`, because `shuyi_assets::tileset::TileLayer`
    is already the grid kit's terrain layer and `ground.rs` imports it.
- **In 1120's world layers.** A `WorldLayer` holds tile-layer items
  beside its sprite items, added by `WorldLayer::add_tiles(tiles:
  Rc<MapTiles>, z: i32, clock_ms: u64)`, with `opacity` from the map
  layer (a game may lower it, for a roof the player walks under). A
  tile-layer item sorts by `z`, before every sprite item of the same
  `z`: it has no `y`. Its world layer's camera, zoom and parallax place
  it, through the one function that places sprite items.
  `GlyphBuffer::fade` lowers its `brightness`, as it does a sprite's.
- **Smoothed layers, either order with 2361:** if 2361 is done, the
  cells of a tile-layer item on a layer whose `Smoothing` isn't `Off`
  are placed in window pixels by 2361's `WorldLayer::window_px`, in
  both painters, as that layer's sprite items are, so the tiles don't
  step by whole world pixels under a smooth camera; if not, nothing to
  do (2361 does it).
- **Parallax as the map says.** `tiles::world_layers(layers:
  Vec<MapTiles>, camera: &Camera, clock_ms: u64)` (1120's `Camera`,
  with its perspective parameter if 1120 gave it one) puts
  `MapTiles::all`'s layers into world layers: consecutive layers with
  the same parallax share one, and a change of parallax starts the
  next, so the map draws in its own order. Each item's `z` is its
  layer's index in the map, so a game's sprite items sort among them by
  `z` (for example, the index of the object layer they came from). It
  returns the world layers and, by map layer name, the index of the
  world layer that holds each. Tiled gives a parallax per axis: if
  1120's parallax factor is one fraction for both, a layer whose two
  differ is a `MapTilesError` naming it.
- **Culling.** `visible_tiles(tiles: &MapTiles, view: PxRect) -> Rect`,
  pure. `view` is the part of the world, in world pixels, that the
  layer shows: 1120's placement function run backwards over the console
  rectangle the layer covers (camera, zoom and parallax). The result is
  the tiles that touch `view`, plus one tile all round, plus enough
  rows below and columns to the left to catch a tile bigger than the
  map's grid (Tiled draws those from their cell's bottom-left corner),
  clamped to the map. It is worked out once per frame, where 1120 sorts
  the items, and kept in the item's `visible`. Every painter and the
  snapshot read it; none walks the whole map.
- **Animated tiles.** A tile with a clip in its sheet's `anims` shows
  the tile id that 2231's `frame_at(clip, clock_ms)` gives (the tile
  itself if the clip has no frame). The clips come from Tiled's
  `<animation>`: 1124, which waits for 2231, keeps them as
  `AnimationClip<u16>`s of tile ids. Every tile of that id is in phase, as in
  Tiled. `clock_ms` is the stage's to give: a tick stage's step count
  times 1000 / 60 (1119's fixed step), in integers; a turn stage's
  screen clock, like `MapScene::clock_ms`
  (`crates/ui/src/grid/scene.rs`). LDtk has no tile animations.
- **Snapshots.** One line per tile-layer item, never one per tile, for
  example `tiles z=0 ground 256x256 of 16x16  visible 10,4 22x14
  (301)`. The count leaves out empty cells. ` offset=x,y`,
  ` opacity=N`, ` bright=N` and ` t=MS` appear only when they aren't
  the default, and `t` only on a layer with animated tiles.
- **The reference raster draws it.** For each cell of `visible`: its
  sheet's `src(tile)`, or `src(frame_at(clip, clock_ms))` for an
  animated tile, flipped, at its place times the zoom, by nearest
  sampling, with the item's opacity and brightness, as `blit` draws a
  sprite.
  - This goes in `shuyi_ui::frame::raster` (2332).
  - **The y flip is 2335's** (it comes before 1120 and is in
    `blocked_by`): `blit` already flips top to bottom, so only the
    diagonal flip is new.
- **`shuyi-app` draws it** in `crates/app/src/render.rs`: one
  `draw_texture_ex` per non-empty cell of `visible`, with `source`,
  `dest_size`, `flip_x` and `flip_y`, and a diagonal flip as a quarter
  turn and a flip (the flip test below shows which). A missing texture
  draws `MISSING_COLOR`, logged once, as for sprites.
- **Either order with 2392:** if 2392 is done, add a small tile-layer
  item to its render card (`raster::card`), as ADR-0052 asks of every
  new frame item, and 2018's `render` job compares the window with the
  raster; if not, compare a captured frame by hand, as 1120 does, note
  it in the PR, and 2392 adds it (its clause says so).
- **A test scene**, built in code in the raster's tests
  (beside the raster), so it is no asset and needs no credit: a `TileMap` value of
  256 by 256 cells of 16 by 16 pixels with two sheets, every flip
  combination, an animated tile, a layer at half opacity, a layer with
  an offset and a layer at parallax ½. The sheets are made in code from
  the test card (`raster::test_card`): 16 by 16, with four coloured quadrants, so each of
  the eight flip combinations gives a different picture.
- **The debug view, either order with 2318:** if 2318 is done, its
  `layout` channel (`shuyi_ui::debug::layout`) outlines each tile-layer
  item's map bounds and its `visible` rectangle (`Bounds`); if not, the
  PR adds that line to 2318's Scope.
- **A bench scenario, either order with 3008:** if 3008 is done, add a
  `.bench.ron` scenario that scrolls the camera across the test map, so
  its numbers say whether chunk caching is needed; if not, 3008 adds it
  (its clause says so).
- **A short ADR** (the `write-adr` skill) extending ADR-0003 §5: a world
  layer holds tile-layer items, one per map layer, culled before any
  painter reads them, and each backend draws them its own way (ADR-0012
  §2's hardware backgrounds read them on the GBA and the DS).
- **Size.** If the PR passes about 600 changed lines (not counting
  fixtures and snapshots), animated tiles go to a follow-up ticket,
  parked like this one, and tiles draw their first frame meanwhile.

**Out (do not do):**
- keeping flips, layer properties (opacity, offset, parallax, hidden),
  the orientation and Tiled's tile animations in `TileMap`: 1124's job
  (the importer must not drop data). If 1124 landed without them, stop:
  write that ticket (the `write-ticket` skill), add it to `blocked_by`,
  and work it first;
- the clip type and `frame_at`: 2231;
- chunk caching (pictures of chunks rebuilt on change, as `MapPicture`
  paints a grid map once): a follow-up, only if 3008's numbers show
  that culling alone misses the web or Playdate budget;
- isometric, staggered and hexagonal maps: isometric tile layers need
  1126's projection, and 1126 lands after this ticket (it is blocked
  by 1125, and 1125 by this one). 1126's Out leaves them to a
  follow-up ticket it writes when it lands, so add nothing to 1126. A
  hexagonal map is refused here, and neither 2121 nor 2740 draws its
  tile layers (2740's Out says so): a follow-up when a hex game asks,
  over this ticket's tile-layer item and 2740's `HexProjection`;
- the one-bit renderer and the Playdate: 1120 draws world layers in
  `frame-png` and `shuyi-app` only, not in `frame::onebit`
  (`crates/ui/src/frame/onebit.rs`). No ticket brings world layers
  there yet; the one that does draws tile-layer items too;
- the GBA's and the DS's backgrounds: 3021 and 3024 map tile-layer
  items to them (or the follow-up they write if they land first);
- autotiling at run time, editing maps (Tiled and LDtk are the editors,
  ADR-0004), typed map objects (2232), 3D terrain from a map (3107);
- interpolation between steps (2361), beyond the smoothed layers' part
  above; collision from tile layers (1122's `TileCollision`).

## Implementation steps

1. Read 1120's `WorldLayer`, `Camera`, sort and placement function;
   1124's `TileMap` (its flips, layer properties, tilesets and
   orientation; if they are missing, see Out); 2231's `AnimationClip`
   and `frame_at`; and `Painter` and `blit` (in
   `shuyi_ui::frame::raster`).
2. Write `frame/tiles.rs` and `pub mod tiles;` in
   `crates/ui/src/frame.rs`: the types, `MapTiles::from_map` and `all`,
   `world_layers` and `visible_tiles`, with unit and property tests.
3. Add `TileLayerItem` to `WorldLayer`: `add_tiles`, the sort,
   `visible` once per frame, `fade` and the snapshot line.
4. Draw it in the raster: flips, zoom, opacity, brightness and animated
   frames.
5. Draw it in `shuyi-app` (`crates/app/src/render.rs`), and compare the
   test scene with the raster.
6. Add the test scene, its text snapshot and its pictures, and the
   2392, 2318 and 3008 parts as their clauses say.
7. Write the ADR, add the line to 2318 if its clause asks for it, and
   run the gates.

## Acceptance criteria

- [ ] On the 256 by 256 map, at three camera positions (a corner, the
      middle, and past an edge) and at zoom 1 and 3, `visible` holds
      every tile whose picture touches the view, and is at most the
      view's size in tiles plus 3 each way (its tiles are the grid's
      size).
- [ ] A counting test shows the raster draws exactly the non-empty
      cells of `visible`, and no others.
- [ ] The test card drawn as a tile with each of the eight flip
      combinations matches a reference written in the test from
      Tiled's order (diagonal, then x, then y).
- [ ] An animated tile shows its first frame at `clock_ms` 0, its
      second at the first frame's duration, and wraps.
- [ ] The test scene's snapshot has one line per tile-layer item and
      none per tile.
- [ ] An isometric map, a diagonal flip on a tile that isn't square and
      a tile id past 65,535 each fail with an error naming the layer
      (and the cell).
- [ ] Raster pictures of the test scene at three camera positions, and
      of 1124's Tiled and LDtk fixtures, are committed as binary
      snapshots (`insta::assert_binary_snapshot!`, as
      `examples/hello-grid/tests/it/snapshot.rs` does) and reviewed.
- [ ] `shuyi-app` and the raster draw the same pixels for the test
      scene. If 2392 is done, a small tile-layer item goes on its
      render card (ADR-0052's rule for a new frame item) and 2018's
      `render` job passes; if not, a captured frame is compared by hand and noted in
      the PR.
- [ ] No snapshot of the game or of the examples changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `TileSheet::src` with margin and spacing; `MapTiles::from_map`
  on 1124's fixtures; each `MapTilesError`; the flip order; animated
  frames through `frame_at`; tile-layer items sorted among sprite
  items; `world_layers` splitting at a change of parallax; the snapshot
  line.
- Property: for any camera position, zoom 1 to 4 and parallax,
  `visible` holds every tile that touches the view, stays inside the
  map, and is bounded by the view's size in tiles plus the margin.
- Snapshot / integration: the test scene's text snapshot, its binary
  picture snapshots, and the counting test.

## Completion notes

*(Filled in by the session that completes the ticket.)*
