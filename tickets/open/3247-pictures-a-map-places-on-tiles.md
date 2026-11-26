---
id: "3247"
title: "Pictures a map places by hand on its tiles, standing taller than a tile"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] A map file with `look: (props: { (2, 1): "lamp" })` reads, prints
      back the same, and a place off the map is a problem at its line.
- [ ] A map file without `props` reads and prints exactly as before.
- [ ] The sprite skin draws a 16 × 32 picture placed on tile (2, 1) on
      that tile and the one above, after every layer, at every zoom; a
      place whose tile is just below the view still shows its top; a
      name the look lacks draws nothing.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `parse` and `print` of `props` (none, some, off the map); a
  placed picture's rectangle at zoom 1 and 2, at the view's edges.
- Property: a map with props round-trips through `print` and `parse`.
- Snapshot / integration: the sprite skin's ground of a small scene
  with one placed picture, drawn after a layer.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
