---
id: "2274"
title: "Parked: Tile layers from Tiled and LDtk read into a GridMap with objects at whole tiles, and Aseprite sheets into the atlas"
type: feature
milestone: "Parked: map import"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: blocked
blocked_by: ["3525", "2222"]
nick_input: none
completed:
---

# 2274 — Tiled and LDtk tile layers into a GridMap, and Aseprite sheets into the atlas

> **Parked** (the blind-spot audit, 2026-11-23): it may start when Nick unparks it in decision ticket 3525 (what 1.0 is for, among them "map import": whether Tiled and LDtk maps and Aseprite sheets should be read soon, as the level editor until the engine has its own). Until then nobody starts it. Whoever unparks it sets `status: todo`, drops `Parked:` from the title, and sets the milestone 3525's answer gives. If Nick says no, it stays parked with 1124 and 1123.

## Context

A blind-spot audit of the engine (2026-11-23) found that there is no
visual way to build a level. The engine's own `.map` format has no
writer, the editor (2506, 2507) is parked, and Tiled and LDtk import,
the cheapest stand-in for a level editor, is parked too (1124, with
Aseprite in 1123).

Why 1124 and 1123 are parked:

- 1124 is blocked on 1119 and 2231. Its `TileMap` keeps every object's
  place, every polyline and every `float` property as `Fx` (1119's
  fixed point), and its tile animations as 2231's clips. Both are real
  time work, parked until Visions of Shuyi launches (`docs/decisions.md`,
  2026-10-20).
- 1123 is blocked on 2231: its importers fill clips from Aseprite's
  frame tags.

A turn-based or grid game needs neither. What it needs is the grid kit's
own map, painted in an editor:

- `shuyi_assets::grid_map::GridMap<T, F>` (`crates/assets/src/grid_map.rs`
  line 184) is a name, a `Grid<T>` of terrain, `features:
  BTreeMap<Pos, F>` (the game's own `MapFeature` type, by tile), the
  legend it was written with and a `MapLook`. Everything in it is an
  integer or a string.
- Its one format, `MapText<T, F>` (line 233), is built by the game with
  its `MapFormat` and a function from a terrain id to its `T`
  (`MapText::new(format, terrain)`), and registered by the game:
  `examples/hello-grid/src/lib.rs` line 226 registers
  `MapText::<TerrainId, Feature>`. `grid_map::print` (line 489) writes a
  map back as `.map` text.
- The tileset (`crates/assets/src/tileset.rs`) picks each tile's picture
  from the terrain around it, so a map file only names terrain. An
  editor's tile is read for its terrain, and its picture is not needed.
- A format reads other files of its bundle through `DecodeCx::bytes`
  (`crates/assets/src/format.rs` line 246), and `Hot<T>` (2220, done)
  reloads a file and every file its decode read, so a Tiled map and its
  `.tsx` tileset reload together with no extra code.

This ticket is that slice: tile layers into a `GridMap`, objects at whole
tiles with integer pixels, and an Aseprite JSON sheet into 2222's
`Atlas` as named rectangles with no clips. No `Fx`, no clips, no `TileMap`.

The game draws its maps in `.map` and keeps doing so. Searched on
2026-11-23 at the game's pin `50472bbcd3084555a61728a5d07518f720bd6514`:
`gh search code --repo Zafnok/visions-of-shuyi 'MapText'` finds
`crates/content/src/map.rs` registering `MapText::<TerrainId,
FeatureDef>`, and game ADR-0005 turns down Tiled `.tmx` maps for its own
content. The formats here are new and behind features that are off by
default, so nothing in the game changes.

## Nick input

None. (3525 is the decision. This ticket only waits for it.)

## Scope

**In:**

- **Features of `shuyi-assets`** (`crates/assets/Cargo.toml`), each off
  by default, as 2203's formats are:
  - `tiled = ["std", "dep:tiled"]`: the `tiled` crate (MIT) with
    `default-features = false`, because its `zstd` feature builds C
    code (ADR-0004 §2). Check its tree with
    `cargo tree -p shuyi-assets -e normal --features tiled` and
    `cargo deny check`. A dependency that builds C code is refused.
  - `ldtk = ["dep:serde_json"]` and `aseprite = ["dep:serde_json"]`:
    `serde_json` (MIT OR Apache-2.0) with `default-features = false,
    features = ["alloc"]`, so both build without `std`.
  - Add `tiled` and `serde_json` to `shuyi-assets`' `outside` list in
    `tools/layers.ron` (line 56), each with its comment.
- **`shuyi_assets::grid_map::import`** (a new module beside
  `grid_map.rs`): what both map readers share.
  - `MapObject`, `#[non_exhaustive]` plain data (ADR-0039): the
    editor's id as text (Tiled's object id, LDtk's `iid`), its name, its
    class (Tiled's class, which TMX writes as `type`; LDtk's entity
    identifier), `at: Pos` (the tile it stands on), `pixel: (i32, i32)`
    and `size: (u32, u32)` in whole pixels, and `properties:
    BTreeMap<String, Property>`.
  - `Property`, `#[non_exhaustive]`: `Bool`, `Int(i64)`, `Text`,
    `Colour` (as text), `File` (the bundle path, joined to the map's
    folder; one that leaves the bundle's root is an error), `Enum { of,
    value }`, `Object` (the other object's id; Tiled's 0 is no
    property), and `Point(Pos)` (LDtk). A `float` property is an error
    naming the file, the layer, the object and the property, saying that
    fractions come with 1124.
  - `GridImport<T, F>`: built with `GridImport::new(format: MapFormat,
    terrain: impl Fn(&str) -> Option<T>)`, as `MapText::new` is, with
    `.layer(name)` (which tile or IntGrid layer is the terrain; the only
    one if not given) and `.objects(feature: impl Fn(&MapObject) ->
    Result<Option<F>, String>)` (an object a game doesn't want gives
    `None`; without it every object is skipped). The two formats below
    wrap it.
  - **The legend.** An imported map has no characters, and `print`
    needs them. Each terrain id gets its first letter if no other id has
    it, else the first free one of `A`–`Z`, `a`–`z`, `0`–`9`, in the
    ids' sorted order. More than 62 terrains is an error. So
    `grid_map::print` writes an imported map as `.map` text.
  - **The look.** The map's own string properties `tiles` and `light`
    fill `MapLook`, checked against `MapFormat::looks` as `parse` checks
    a `.map`'s. Its `name` property is the name, else the file's stem.
- **`grid_map::TiledGrid<T, F>`** (feature `tiled`), a `Format` of
  `GridMap<T, F>`: id `tiled`, extension `.tmx`, `TEXT_MAX_BYTES`.
  - The terrain layer: each tile's terrain id is its tileset tile's
    string property `terrain` if it has one, else its class. A tile with
    neither, an empty cell, or an id `terrain` doesn't know, is an error
    naming the file, the layer and the cell. The flip bits are dropped:
    terrain has no direction, and the tileset paints it.
  - `.tsx` tilesets are read through the `tiled` crate's reader over
    `DecodeCx::bytes`, never from the disk.
  - Refused with an error that says why: an infinite map (save it at a
    fixed size), a non-orthogonal map, a map past `MapFormat::max_size`.
  - Objects of every object layer: a point, a box, an ellipse or a tile
    object. Its `at` is the tile holding its centre (a tile object's `y`
    is its bottom edge, Tiled's rule). Tiled gives places as `f32`: each
    must be whole, checked with integer operations on the float's bits
    (as 1124 rounds), never with float arithmetic, so every platform
    agrees. A fraction, a polyline, a polygon, a text object or a place
    outside the map is an error naming the object. A tile object and a
    template instance keep the class and properties they inherit, their
    own winning (Tiled's rule).
- **`grid_map::LdtkGrid<T, F>`** (feature `ldtk`), a `Format` of
  `GridMap<T, F>`: id `ldtk`, extension `.ldtk`, `TEXT_MAX_BYTES`.
  - Our own small serde types for the fields read (`jsonVersion`,
    `defs.layers[].intGridValues`, `levels[].layerInstances[]` with
    `intGridCsv`, `entityInstances` with `iid`, `__identifier`,
    `__grid`, `px`, `width`, `height` and `fieldInstances`), unknown
    fields ignored. Not the vendored generated types 1124 plans: this
    reads far less. A `jsonVersion` that isn't `1.x` is an error.
  - The terrain layer is an IntGrid layer. Each cell's value gives the
    `identifier` of that value in the layer's definition. Value 0
    (empty), a value with no identifier, or an id `terrain` doesn't know
    is an error naming the file, the level, the layer and the cell.
  - The file has one level. A file of several levels is an error saying
    to save one level per file (1124's `TileMap` keeps them all).
  - Entities: `at` is `__grid`, the pixels are `px`, all integers in the
    JSON. A `Float` field is the error above.
- **The objects become features.** For each object, `feature` gives
  `Some(f)` to put `f` at `at`, `None` to skip it, or an error. Two
  features on one tile is an error naming both objects. Every problem
  of a file is reported together, each with its place, as `parse` does.
- **`atlas::AsepriteJson`** (feature `aseprite`), a `Format` of 2222's
  `atlas::Atlas`, registered by `Formats::standard()` when its feature
  is on (2203's pattern). Id `aseprite-json`, extension `.json`.
  - It is Aseprite's only if `meta.app` starts with
    `https://www.aseprite.org/`, else an error naming `meta.app`. So
    1123's TexturePacker reader can sniff the same extension later
    (ADR-0019 §2).
  - Both `json-hash` and `json-array` frames. One page, the sheet's
    image (`meta.image`, joined to the JSON's folder, read through the
    registry as 2222's loader reads a page), and each frame an entry
    named by its `filename`, with its `frame` rectangle. A frame that
    leaves the image (`meta.size`) is an error.
  - A trimmed frame (`trimmed: true`) or a rotated one is an error
    naming the frame: 2222's `Atlas` has no field for its offset, which
    1123 adds. `frameTags`, `duration` and `slices` are not read
    (1123 reads them into clips and nine-slices); `docs/formats.md`
    says so.
  - Errors name the file and the line and column `serde_json` gives.
- **`shuyi import`.** The map formats are a game's, like `MapText`: a
  game adds them to its `GameTools::importers` (`crates/cli/src/game.rs`
  line 633). If `Importers::standard()`
  (`crates/cli/src/contract/import.rs` line 187) has no `Atlas` kind,
  add `.kind::<Atlas>(..)` summed up with `count` (`1 page, 12
  entries`). Correct that file's module doc (lines 8–11), which says
  `import` needs no change for Tiled, LDtk and Aseprite.
- **Golden rows** in `shuyi_assets::golden` (2116): `map: tiled`, `map:
  ldtk` and `atlas: aseprite-json`, the `state_hash` of each fixture's
  decode, which CI's `determinism` job checks on wasm32 too.
- **Fixtures** in `crates/assets/tests/fixtures/`, made with Tiled's and
  LDtk's free editors and Aseprite's JSON export, with art of our own
  (CC0): a Tiled map with a `.tsx` tileset and objects of every kind
  read, an LDtk file of one level, a sheet in both frame layouts, and a
  broken file of each.
- **Docs:**
  - `docs/formats.md`: rows in *The built-in formats* for the three
    formats, and rows in *Formats behind a feature* for `tiled`, `ldtk`
    and `aseprite`, with the crate, its licence, its last release, C
    code (none), `no_std`, and the WASM build's size with the feature
    on, as the other rows give.
  - ADR-0004 §2's *Tile maps* and *Sprite sheets* rows: a line saying a
    Tiled or LDtk tile layer also reads into the grid kit's `GridMap`
    (2274), and an Aseprite sheet into `Atlas` without its tags (2274),
    beside what 1124 and 1123 add. An amendment as the `write-adr` skill
    says.
  - `skills/shuyi-import-art/SKILL.md`: its description and *Not built
    yet* say that a Tiled or LDtk tile layer reads into a `GridMap` and
    an Aseprite sheet into an `Atlas`, with the steps (the feature, the
    format registered, `shuyi import --print`). Its `waits_for` keeps
    `1123 1124`: they still build the rest.
- New public types are `#[non_exhaustive]` with a constructor
  (ADR-0039): `MapObject`, `Property`, `GridImport`, `TiledGrid`,
  `LdtkGrid` and `AsepriteJson`. Nothing existing changes shape, so the
  API only grows.
- **Either order with 1124:** if 1124 is done, read the tile layer and
  the objects from its `TileMap` (build the `GridMap` from it, keeping
  the rules above: whole pixels, no float), and use its Tiled and LDtk
  readers, not a second parse; `TiledGrid` and `LdtkGrid` stay the
  formats of `GridMap`. If not, write the readers here, and 1124 builds
  its `TileMap` beside them, reusing the `tiled` feature and the LDtk
  types where they fit.
- **Either order with 1123:** if 1123 is done, Aseprite's JSON already
  reads into `Atlas`: drop the `aseprite` part here (nothing left to
  do). If not, write it here as above, and 1123 adds the trimmed
  frame's fields, the frame tags as 2231's clips, the slices, the
  TexturePacker reader and the RON sidecar on top of it, and turns the
  `Atlas` row of `Importers::standard()` into `kind_with_text`.

**Out (do not do):**
- `TileMap`, `Fx` places, polylines and polygons, several levels per
  LDtk file, `MapWorld`, layer opacity, offsets and parallax, tile
  animations (1124, parked).
- Typed map objects read by serde, and the game's types written to the
  editors (2232, parked).
- Drawing an editor's tile pictures, flips and animated tiles: the
  grid kit's tileset paints terrain (2360 and 2359, parked).
- Frame tags as clips, trimmed frames, nine-slice slices, TexturePacker,
  `.aseprite` files (1123, parked; 2231 has the clips).
- The atlas itself, its packer and its drawing (2222).
- Writing `.map` back while keeping comments (2273); a Tiled or LDtk
  writer.
- Isometric, staggered and hexagonal maps (1124 and 2121).

**Estimated size:** about 2,000 lines (code 900, tests 800, docs 300),
fixtures and golden rows not counted.

**Split:** past 900 lines, split off steps 4 and 6 (the LDtk reader)
into a new ticket parked like this one; past 1,300, split off step 5
(the Aseprite atlas) the same way. A split-off ticket keeps its
acceptance lines and this ticket's clause with 1124 or 1123, and the
PR that splits it writes that clause's other side in 1124 or 1123.

## Implementation steps

1. Read `grid_map.rs`, `format.rs` (`DecodeCx`, `Formats::standard`),
   `hot.rs`, 2222's `atlas` module and `docs/formats.md`'s *Writing a
   format plugin*. If 1124 or 1123 is done, follow their clauses.
2. Add the features, the dependencies and their `tools/layers.ron`
   lines; check `cargo tree` and `cargo deny check`.
3. Write `grid_map::import`: `MapObject`, `Property`, `GridImport`, the
   legend, the look, and the objects-to-features step.
4. Write `TiledGrid` with its fixtures and errors.
5. Write `AsepriteJson` with its fixtures and errors, and the `Atlas`
   kind in `Importers::standard()`.
6. Write `LdtkGrid` with its fixtures and errors.
7. Add the golden rows, the docs, the ADR-0004 amendment and the skill's
   text. Measure each feature's WASM size for `docs/formats.md`.

## Acceptance criteria

- [ ] The Tiled fixture and the LDtk fixture, painted alike, decode to
      equal `GridMap`s, and `grid_map::print` of each parses back to it.
- [ ] A tile whose terrain is its `terrain` property and one whose
      terrain is its class both read; a tile with neither, an empty cell
      and an unknown terrain id are each an error naming the file, the
      layer and the cell.
- [ ] Each object kind read lands on the tile holding its centre,
      including a tile object (bottom-edge `y`); a fractional place, a
      polygon, a float property and two features on one tile are each an
      error naming the object.
- [ ] A tile object inherits its tile's class and properties, its own
      winning.
- [ ] A changed `.tmx` and a changed `.tsx` it reads both reload
      through `Hot<GridMap<..>>`, through the headless backend's
      `Harness::change_file`.
- [ ] An LDtk file of two levels, an infinite Tiled map and a
      non-orthogonal one are each refused with an error that says why.
- [ ] The Aseprite fixture reads to the same `Atlas` in `json-hash` and
      `json-array`; a trimmed frame and a JSON from another tool are
      each an error naming the frame or `meta.app`.
- [ ] `shuyi import` reads the Aseprite fixture as an `Atlas` with its
      summary.
- [ ] A property test: random bytes never panic any of the three
      formats.
- [ ] The three golden rows pass on x64 and wasm32 (CI's `determinism`
      job).
- [ ] `cargo deny check` passes with the features on, and the web build
      passes with them.
- [ ] `check-layers` passes with the new outside crates.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the legend's letters (a clash, more than 62); the look from the
  map's properties; each `Property` kind; the whole-pixel check on
  `f32` bits at 0.5, -0.5 and a large value; inheritance of a tile
  object.
- Property: random bytes never panic `TiledGrid`, `LdtkGrid` or
  `AsepriteJson`.
- Fixture and snapshot: each fixture's `GridMap` printed as `.map` text,
  each sheet's `Atlas`, and each broken fixture's errors.
- Integration: a map and its tileset reload through the headless
  harness.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
