---
id: "2243"
title: "Terrain tiles as a format: a tileset's looks and layers read from RON, checked alone, then resolved against the image table and the game's terrains"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-26
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

**2026-11-26, from game#1361 (game ADR-0133):** the game's corner
layers now write engine#3246's fields: `Corners` takes `size: (w, h)`
(pixels from the top-left of its `at` cell; left out, `tile_px`),
`offset: (x, y)` and `skip: [terrains]`, set on `CornerLayer` after
`new`. Chapter 1's cliff face and its shadow use them. The `LayerDef`
below carries them, or the game loses them when 1219 moves its loader.

## Nick input

None.

## Scope

**In:**
- `crates/assets/src/tileset.rs`, all additive (no `Breaking:` line):
  - `TerrainTilesDef { image: String, tile_px: (u32, u32), terrain:
    BTreeMap<String, (u32, u32)>, layers: Vec<LayerDef>, looks:
    BTreeMap<String, LookDef> }`, `LookDef { terrain, layers }`,
    `LayerDef::{Corners { of, tiles: Vec<CornersAt>, size: Option<(u32,
    u32)>, offset: (i32, i32), skip: Vec<String> }, Tiles { of, at,
    beside, sides: Vec<SidesAt> }}` (`size`, `offset` and `skip` may be
    left out: `tile_px`, `(0, 0)`, none), `CornersAt { corners, at }`,
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
    for mix 0, or with none at all, or a `size` side outside `side_px`. Each problem names its look and
    layer (`look "indoor": layer 2: …`), as the game's do.
  - `TerrainTilesDef::resolve(&self, file, images: &ImageTable,
    terrains: &BTreeMap<String, TerrainId>) -> Result<TerrainTiles,
    Vec<ContentError>>`: the image in the table; every rectangle inside
    it (a corner layer's at its `size`); every terrain named (`terrain`,
    `of`, `beside`, `skip`) in `terrains`; every terrain of `terrains`
    with a tile in each look. A corner layer's `size`, `offset` and
    `skip` go onto its `CornerLayer`. Every problem.
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

- [x] A fixture with two looks, a corners and a tiles layer resolves to
      the `TerrainTiles` built by hand in the test; a corners layer with
      a `size`, an `offset` and a `skip` resolves to them.
- [x] Each problem listed above has a test giving that one problem.
- [x] Random bytes into `TerrainTilesRon` never panic; its golden row
      passes.
- [x] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass, `api` reporting additions
      only.

## Tests required

- Unit: `cell`, `holds`, each `problems` and `resolve` check.
- Property: random bytes never panic the format.
- Fixture and determinism: the golden row.

## Completion notes

**What was done.** `shuyi_assets::tileset` has the file's types
(`TerrainTilesDef`, `LookDef`, `LayerDef`, `CornersAt`, `SidesAt`, in
the private module `tileset/def.rs`, re-exported), `TileRules`,
`TerrainTilesDef::problems` (the file alone) and
`TerrainTilesDef::resolve` (against an `ImageTable` and the game's
terrains by name), and the format `TerrainTilesRon` (`terrain-tiles`,
`.ron`, 4 MiB), not in `Formats::standard`. `image.rs` has
`ImageRect::cell` and `ImageInfo::holds`. The checks and their wording
are the game's `tileset/look.rs` and its open PR for game#1361
(Zafnok/visions-of-shuyi#852), which adds `size`, `offset` and `skip`,
with "not in terrain.ron" made "is not one of the game's terrains".
The fixture (`TERRAIN_TILES`, a farm: soil, tilled soil, water, a
"winter" look) is in `format/builtin_tests.rs`, with its golden row
`terrain tiles: terrain-tiles` and a property test that random and
edited bytes never panic the format, nor `resolve` on what it passes.

**Deviations.**

- The ticket is 23 days old; checked against `main`: 3246 (done) built
  the `CornerLayer` fields the ticket's 2026-11-26 note names, as it
  says, and nothing built this. The golden table is
  `crates/assets/tests/golden/formats.ron` (the ticket's path is the
  crate's). No change to the ticket.
- The `*Def` types derive `Serialize` too: the golden row hashes what
  the format reads (`state_hash` takes a `Serialize`). A corner layer's
  `size` is written bare (`size: (16, 24)`), so a file written back
  with `ron::to_string` reads as itself (tested).
- `problems` takes `&TileRules`, and reports a corner layer's `size`
  only when written (the game's checks `tile_px` again when it is left
  out, so a bad `tile_px` was reported twice).
- `resolve` leaves out a mix that `problems` refused instead of
  reporting it again: it is run on a file the format passed.
- Problem codes on `resolve`'s errors: `missing-file` for the image,
  `out-of-range` for a picture outside it, `unknown-name` for a
  terrain; none for a terrain with no tile. `problems` gives plain
  strings, as the ticket says.

**Size.** 1,290 counted lines (`cargo xtask pr-size`), most of them
tests. It would split cleanly between `ImageRect::cell`,
`ImageInfo::holds`, the `*Def` types and `problems`, and then
`resolve` with the format, its fixture and its golden row.

**Docs.** `docs/formats.md` (a row in the game-registered formats, what
the file is checked for, the `Tileset` kind's row, a bullet in *Checks
after loading*), the `tileset` module's docs with a short file, the
book's *Frame and skins* chapter (a paragraph after the two skins'
shots), and the skill pack's `shuyi-import-art`.

**Follow-up ticket.** 3540: the local `site` gate runs `zola check`
without `site/copy-shots.sh` and `site/copy-previews.sh`, which CI's
`site` job runs first, so it failed here on a shot of another chapter;
the gates were run after `bash site/copy-shots.sh` by hand.

Game: game#1219 moves the game's tileset loader onto this format and
deletes its `tileset/look.rs`, `grid_rect` and `inside`.

No break: every item is new (`cargo xtask api`). Searched the game for
the kind's name and `TerrainTiles` (`gh search code --repo
Zafnok/visions-of-shuyi 'TerrainTiles'`): it builds `TerrainTiles`
itself, which doesn't change, so no `Behaviour:` line.
