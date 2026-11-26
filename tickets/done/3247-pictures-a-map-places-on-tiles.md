---
id: "3247"
title: "Pictures a map places by hand on its tiles, standing taller than a tile"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-26
---

# 3247 — Pictures a map places by hand on its tiles, standing taller than a tile

## Context

- **The game ticket:** game#1378
  ([`tickets/open/1378-lamps-at-harls-gate-and-the-road-bridge.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1378-lamps-at-harls-gate-and-the-road-bridge.md)),
  split from game#1360. It waits on this ticket; this one waits on
  nothing of the game's.
- **The job:** put a lamp on a chosen tile of one map, two tiles tall,
  its flame moving: lamps at a fort's gate and beside one bridge, where
  no terrain says a lamp goes (the river and the grass beside them have
  none).
- **Friction numbers:** no friction review: a missing feature. Today a
  map file names only terrain (`shuyi_assets::grid_map`: `name`,
  `legend`, `features`, `look`), and a sprite skin's ground
  (`crates/ui/src/grid/sprite/ground.rs`) draws only what a tileset's
  look derives from terrain. A game's `features` are its rules' data
  (`MapFeature`), never look; and the only part of a map's file that
  reaches a skin is `MapScene::look` (`MapLook { tiles, light }`).
- **The generic shape:** a map's look may place named pictures on
  tiles (`look: (tiles: "…", props: { (19, 5): "lamp" })`), and a
  tileset's look says what each name is drawn as: a picture of its
  image, at its own size, standing on its tile (its bottom on the tile's
  bottom, centred across), so it may reach up over the tiles above.
- **An example that isn't tactics:** a farming game's town map with a
  signpost, a mailbox and street lamps placed by its designer; a
  top-down dungeon's torches on chosen walls and a statue in one hall.

Searched the game for a build of `MapLook` it would have to change
(`gh search code --repo Zafnok/visions-of-shuyi 'MapLook {'`, game pin
`96eb097c`): the game reads `MapLook` from its maps and clones it; check
the hits again before the PR, as the type gains a field.

## Nick input

`None.` (Nick agreed to an engine ticket for game#1360's lamps,
2026-11-26.)

## Scope

**In:**
- `shuyi_assets::grid_map::MapLook` gains `props: BTreeMap<(i32, i32),
  String>`: a tile, map coordinates, to a name. Left out of a file it is
  empty, and `print` writes it only when it isn't (as `light`). `parse`
  reports a place off the map with its line and column; a name is the
  game's, checked against its tilesets by the game (as `tiles` is).
  `MapLook` becomes `#[non_exhaustive]` with a constructor (ADR-0039;
  ADR-0151: no two neighbouring numbers): a **breaking** change for a
  game that builds one with a struct literal (CI's `api` job;
  a `Breaking:` line in the completion notes).
- `shuyi_assets::tileset::Look` gains `props: BTreeMap<String,
  ImageRect>`: what each name a map places is drawn as. Empty by
  default; a field `Look` already has `Default`, so additive if `Look`
  is `#[non_exhaustive]` (make it so, with `Look::new(tiles, layers)`,
  if it isn't: breaking, as above).
- The sprite skin's ground paints each placed picture whose tile is in
  the view (or whose picture reaches into it from the row below), after
  the look's layers and before the tints, at its own size times the
  zoom, its bottom on its tile's bottom and centred across it, in the
  map's light; clipped to the view and to tiles on the map. A name the
  look lacks draws nothing.
- `MapScene` carries it already, through `look`; the glyph skin ignores
  it (as it ignores `tiles`).

**Out (do not do):**
- Moving pictures: a game swaps a picture's frames by its clock as it
  does for its tiles today (engine#2359 makes animation data in the grid
  kit; either order works: if 2359 lands first, its frames apply to a
  placed picture's `ImageRect` as to any other).
- Sorting placed pictures with the units by their bottom edge: they are
  ground, under every unit, as a corner layer's tall pictures are.
- A game's tileset file format and its checks (game#1378 writes those in
  the game).

**Estimated size:** about 700 lines (code 250, tests 350, docs 100).

## Implementation steps

1. `MapLook::props` in `crates/assets/src/grid_map.rs`: serde with a
   default and `skip_serializing_if` empty; `look_in_full` keeps both
   old fields and adds this one; the check of places against the map's
   size in `parse`, its problem at the place's line and column
   (`header_position`). `#[non_exhaustive]` and a constructor.
2. `Look::props` in `crates/assets/src/tileset.rs`.
3. `paint_props` in `crates/ui/src/grid/sprite/ground.rs`, after the
   layers: for each place in `scene.look.props` whose picture's
   rectangle meets the view, `Rect` from the tile's bottom-left, the
   picture's own size times `grid.zoom()`; clipped by `parts` (no skip).
4. The book's chapter on maps and the sprite skin; the skill pack's
   map format (`skills/`), with an example.

## Acceptance criteria

- [x] A map file with `look: (props: { (2, 1): "lamp" })` reads, prints
      back the same, and a place off the map is a problem at its line.
- [x] A map file without `props` reads and prints exactly as before.
- [x] The sprite skin draws a 16 × 32 picture placed on tile (2, 1) on
      that tile and the one above, after every layer, at every zoom; a
      place whose tile is just below the view still shows its top; a
      name the look lacks draws nothing.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `parse` and `print` of `props` (none, some, off the map); a
  placed picture's rectangle at zoom 1 and 2, at the view's edges.
- Property: a map with props round-trips through `print` and `parse`.
- Snapshot / integration: the sprite skin's ground of a small scene
  with one placed picture, drawn after a layer.

## Completion notes

What was done:

- `shuyi_assets::grid_map::MapLook` gains `props: BTreeMap<(i32, i32),
  String>`: left out of a file it is empty, and `print` writes it only
  when it isn't. `parse` reports a place off the map as
  `look: prop at (x, y): outside the map`, `OUT_OF_RANGE`, at the
  place's line and column: the first `(x, y)` (or `(x,y)`) written after
  the word `props`, so not a feature's at the same tile. `MapLook` is
  `#[non_exhaustive]`, built with `MapLook::new(tiles, light)` or
  `Default`. A baked map (`look_in_full`) carries all three fields.
- `shuyi_assets::tileset::Look` gains `props: BTreeMap<String,
  ImageRect>`, and is `#[non_exhaustive]` with `Look::new(tiles,
  layers)`.
- The sprite skin's ground (`paint_props`) draws each placed picture
  after the layers and before the tints, in the map's light: its own
  size times the zoom, its bottom on its tile's bottom, centred across
  (half a pixel left when it can't be exactly), wherever its picture
  meets the view, even from a tile outside it, cut to drawn tiles of the
  map. A name the look lacks draws nothing.
- Tests: the format's (none, some, every field in any order, off the map
  with line and column, the position's fallbacks, the round-trip
  property with props); the ground's (after every layer and before the
  wash, in the light, a name the look lacks, two in row order, a tile
  just below the view, the view's edges and origin, tiles off the map,
  odd and empty sizes, zoom 1 and 2); and the ground's property that
  nothing spills off the map's drawn tiles now places lamps too.
- Docs: the book's *Frame and skins* (a paragraph after the two skins),
  the `shuyi-stage-grid` skill's map format (`look` and its `props`,
  with an example), the modules' and types' rustdoc.

Deviations:

- Placed pictures are drawn row by row from the top (then left to
  right), not in the map's key order (left to right, then down): a lower
  one is then drawn over a taller one reaching down to it, as things
  nearer the bottom of a top-down view are nearer the viewer. The ticket
  named no order.
- A picture on a tile off the map still draws its part over the tile
  above when that one is on the map: "clipped to tiles on the map" as
  the scope says, for each tile it covers. A map file can't place one
  there (`parse` refuses it); only a game's own scene could.
- The baked form of a map grew by its `props`, so the pack format is
  version 24 and the golden rows of decoded maps moved (the Breaking
  lines below); the ticket didn't name these.

No ADR: `props` follow the look data of game ADR-0052 and ADR-0058 (a
map's file names look data beside its terrain, which the rules never
see) and the painting of ADR-0133 (a picture at its own size, cut to
drawn tiles of the map); nothing new is decided.

Breaking: shuyi-assets: `grid_map::MapLook`: it is `#[non_exhaustive]`, so a game builds it with `MapLook::new(tiles, light)` (or `Default`) in place of a struct literal, and a `match` or `let` on its fields adds `..`.

```migration
since: 0.1.0
what: MapLook is non-exhaustive and gains props; build it with MapLook::new.
old: "MapLook { tiles, light }"
new: "MapLook::new(tiles, light)"
before: |
  let look = MapLook { tiles: look.to_owned(), light: light.to_owned() };
after: |
  let look = MapLook::new(look, light);
find: ["MapLook { tiles"]
```

Breaking: shuyi-assets: `tileset::Look`: it is `#[non_exhaustive]`, so a game's tileset reader builds it with `Look::new(tiles, layers)` (or `Default`) and sets `props` after, in place of a struct literal.

```migration
since: 0.1.0
what: A tileset's Look is non-exhaustive and gains props; build it with Look::new.
old: "Look { tiles, layers }"
new: "Look::new(tiles, layers)"
before: |
  Look { tiles, layers }
after: |
  Look::new(tiles, layers)
find: ["Look { tiles, layers }"]
```

Breaking: shuyi-assets: `pack::VERSION` and `formats.ron` "map: map": a map's binary form holds its look's `props`, so a baked pack is of version 24 and a decoded map's state hash moves; a pack baked before is refused, so a game bakes again with `shuyi bake`, and blesses again its own golden row of a decoded map.

```migration
since: 0.1.0
what: The pack format is version 24, its maps with the pictures placed on their tiles; a pack of version 23 is refused, and a golden row of a decoded map moves.
old: "pack::VERSION == 23"
new: "pack::VERSION == 24"
before: |
  cargo shuyi bake   # with the engine before 3247
after: |
  cargo shuyi bake   # again, with this engine
  cargo xtask golden --bless   # or the game's own way, for a row of a decoded map
```

Searched the game (`gh search code --repo Zafnok/visions-of-shuyi
'MapLook'` and `'Look {'`, 2026-11-26): `crates/ui/src/map_view/glyph/tests.rs`
builds `trpg_content::MapLook { tiles, light }` and
`crates/content/src/tileset/look.rs` builds `Look { tiles, layers }`;
both are the struct literals the first two Breaking lines name. The
game's other uses read `MapLook` and clone it (`MapLook::default()`,
`.light`, `.tiles`), which still compile.

Book: the chapter *Frame and skins* says what a map's `props` are.

Game: game#1378 moves the pin to this change, places its lamps in its
maps' `look` and gives its tileset format a picture for each name.
