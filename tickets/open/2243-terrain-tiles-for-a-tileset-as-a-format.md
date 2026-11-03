---
id: "2243"
title: "Terrain tiles as a format: a tileset's looks and layers read from RON, checked alone, then resolved against the image table and the game's terrains"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2243 — Terrain tiles for a tileset as a format

## Context

A request from the game.

1. **The game ticket:** game#1219, tileset terrain looks on the engine's
   format
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1219-tileset-terrain-looks-on-the-engines-format.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** write a tileset's ground (a tile per
   terrain, the corner and tile layers over them, the other looks a map
   may ask for) in a text file, and get every mistake in it reported.
3. **The friction numbers:** no friction review. The game/engine
   boundary audit of 2026-11-03 found that the game's
   `crates/content/src/tileset/look.rs` (191 production lines) and the
   terrain half of its `tileset.rs` are generic: their only ties to the
   game are its looks list, its terrain lookup and the words "not in
   terrain.ron". `tileset.rs` here (lines 69–70) says the format is the
   game's, and the toy grid builds its `TerrainTiles` in code
   (`crates/ui/src/toy/grid.rs:173`) because there is none. The game
   kept it because "the terrain names are this game's"; `MapText::new(format,
   terrain)` (game 1116) has since shown that the terrain names can be
   the caller's while the format is the engine's.
4. **The generic shape:** a kind `TerrainTilesDef`, the file as written
   (image path, tile size, the own look's `terrain` and `layers`, other
   `looks` by name), checked alone by its format against `TileRules`
   (the look names a map may give, the tile side range), then resolved
   into the existing `TerrainTiles` against an `ImageTable` and the
   game's terrains by name, as `docs/formats.md`'s *Checks after
   loading* do. The terrains are a name-to-`TerrainId` map, not a
   closure as in `MapText`, because "every terrain has a tile" needs the
   whole list.
5. **An example from a game that isn't tactics:** a farming game's
   ground (soil, tilled soil, water, a path) with a shore of corner
   pictures round the pond and a "winter" look; a roguelike's dungeon
   and cave looks.

## Nick input

None.

## Scope

**In:**
- `crates/assets/src/tileset.rs`, all additive (no `Breaking:` line):
  - `TerrainTilesDef { image: String, tile_px: (u32, u32), terrain:
    BTreeMap<String, (u32, u32)>, layers: Vec<LayerDef>, looks:
    BTreeMap<String, LookDef> }`, `LookDef { terrain, layers }`,
    `LayerDef::{Corners { of, tiles: Vec<CornersAt> }, Tiles { of, at,
    beside, sides: Vec<SidesAt> }}`, `CornersAt { corners, at }`,
    `SidesAt { sides, at }`: `Deserialize`, `deny_unknown_fields`, the
    spelling of the game's tileset files (its `assets/tilesets/README.md`),
    `(column, row)` cells of `tile_px`. Public, so a game whose file holds
    more than the ground uses the same fields. `TerrainTilesDef` and
    `LookDef` are `#[non_exhaustive]` with `new` (ADR-0039).
  - `TileRules<'a> { looks: &'a [&'a str], side_px: RangeInclusive<u32> }`,
    `#[non_exhaustive]`, `const fn new`.
  - `TerrainTilesDef::problems(&self, rules) -> Vec<String>`, the file
    alone: each side of `tile_px` in `side_px`; each `looks` name one of
    `rules.looks` and not `grid_map::DEFAULT_TILES` (the own look is
    written at the top); a layer's `of` empty; a mix not four of
    `MIX_IN`/`MIX_OUT`, or listed twice; a corners layer with a picture
    for mix 0, or with none at all. Each problem names its look and
    layer (`look "indoor": layer 2: …`), as the game's do.
  - `TerrainTilesDef::resolve(&self, file, images: &ImageTable,
    terrains: &BTreeMap<String, TerrainId>) -> Result<TerrainTiles,
    Vec<ContentError>>`: the image in the table; every rectangle inside
    it; every terrain named (`terrain`, `of`, `beside`) in `terrains`;
    every terrain of `terrains` with a tile in each look. Every problem.
  - `TerrainTilesRon::new(rules: TileRules<'static>)`, format id
    `terrain-tiles`, extension `ron`, `TEXT_MAX_BYTES`, decoding
    `parse_ron` then `problems`; not in `register_standard` (it takes the
    game's looks, as `MapText` does). `AssetKind` for `TerrainTilesDef`,
    `NAME` `"terrain tiles"`. `TerrainTiles`' kind doc points to it.
- `crates/assets/src/image.rs`: `ImageRect::cell((column, row), (w, h),
  (ox, oy))` (saturating, as the game's `grid_rect`) and
  `ImageInfo::holds(rect) -> bool` (in `u64`), used by `resolve`.
- Docs: a row in `docs/formats.md`'s built-in formats, a bullet in
  *Checks after loading*, the module docs with a short file.

**Out (do not do):**
- Changing `TerrainTiles`, `Look`, the layers or the sprite skin.
- Unit pictures, walking sheets, anything keyed by a game's ids.
- The toy grid's tiles stay built in code.
- The game's use of it (game#1219).

## Implementation steps

1. **Read** `tileset.rs`, `grid_map.rs` (`MapText`, `DEFAULT_TILES`),
   `palette.rs` (`PaletteRon::requiring`), `format/builtin_tests.rs`,
   `format/golden_tests.rs`, and the game's `crates/content/src/tileset.rs`
   and `tileset/look.rs` at the pin (the checks to port, tests included).
2. `ImageRect::cell`, `ImageInfo::holds` with tests.
3. The `*Def` types, `TileRules`, `problems`, then `resolve`.
4. `TerrainTilesRon`, a fixture in `format/builtin_tests.rs`, and a row
   `terrain tiles: terrain-tiles` in `tests/golden/formats.ron`
   (`cargo xtask golden --bless`).
5. Docs. Expected size: about 220 production lines (the game's 191 plus
   the two helpers), 300 of tests.

## Acceptance criteria

- [ ] A fixture with two looks, a corners and a tiles layer resolves to
      the `TerrainTiles` built by hand in the test.
- [ ] Each problem listed above has a test giving that one problem.
- [ ] Random bytes into `TerrainTilesRon` never panic; its golden row
      passes.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass, `api` reporting additions
      only.

## Tests required

- Unit: `cell`, `holds`, each `problems` and `resolve` check.
- Property: random bytes never panic the format.
- Fixture and determinism: the golden row.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
