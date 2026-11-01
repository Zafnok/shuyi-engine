---
id: "1124"
title: "Parked: Tiled and LDtk maps, as formats that reload with the rest through 2220"
type: feature
milestone: "Engine: real time"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["1119", "2231"]
nick_input: none
completed:
---

# 1124 — Tiled and LDtk maps

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): after Visions of Shuyi launches, not only after the MVP (`docs/decisions.md`, 2026-10-20). Unparked early only if the game needs it: its ticket 0060 says the army walks around freely (then 1119–1125 become game work), or it asks for a piece of this through its `engine-change` skill, which is then split out and unparked alone. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0004 §1–§2. Developers draw maps in Tiled or LDtk. The `tiled` crate
(MIT) reads TMX/TSX in pure Rust, but only with default features off,
because its `zstd` feature builds C code. LDtk's crates are stale, but
LDtk publishes a JSON schema and generated serde types (MIT). Both
importers produce one `TileMap` per file:

- tile layers: a `Grid` of tile ids per tileset;
- object layers: named boxes, points, polylines and polygons with
  properties;
- the map's own properties.

What each object keeps is enough for 2232 to read it into a game's own
types, and an LDtk file keeps all its levels (the 2026-10-20 edit).

Folders on disk are bundle layers since 2207 (`shuyi_app::read_folder`;
a game's mods first: game#1012 and game#1014). Hot reload of every asset kind is 2220's
(after MVP, not parked); the maps here reload through it with no extra
code, because they are formats.

## Nick input

None.

## Scope

**In:**
- `shuyi_assets::tilemap::TileMap`, an asset kind (`AssetKind`,
  ADR-0019 §1), and the two importers as `Format`s of it (ADR-0007,
  ADR-0019; `docs/formats.md` says how to write one), registered with
  `Formats::register_standard`, each with its id, extension (`.tmx`,
  `.ldtk`), limit and a property test that random bytes never panic it:
  - Tiled: `tiled` with `default-features = false` (its TSX tilesets are
    a `Format` of the tileset kind, or files the TMX format reads through
    its `DecodeCx`);
  - LDtk: the vendored generated types with their MIT notice in
    `THIRD_PARTY_ASSETS.md`.
  Both have fixtures made with each tool's free editor.
- **Objects keep what a game needs to type them** (2232 reads them): the
  editor's id (Tiled's object id as text, LDtk's entity `iid`), the
  name, the class (Tiled's object class, which TMX writes as `type`;
  LDtk's entity identifier), the place as a point or a box in `Fx`
  pixels of its level (Tiled's ellipses, polygons and polylines as their
  bounding box and their shape kind), and the custom properties with
  their editor types: `bool`, `int` (`i64`), `float` (as `Fx`),
  `string`, `color`, `file`, `object` (the referenced object's id;
  Tiled's 0 is none), `enum` (its type's name and the value), a Tiled
  class-typed property (its members, nested), and LDtk's `EntityRef`,
  arrays and points. A tile object and a template instance keep the
  class and properties they inherit, their own winning (Tiled's rule).
- **No float in a `TileMap`.** A number with a fraction (a place, a
  point of a polyline, a `float` property, a layer's offset or
  parallax) becomes an `Fx`, rounded to the nearest 1/256, a tie toward
  negative infinity. The readers hand numbers over as floats (the
  `tiled` crate as `f32`, LDtk's JSON as `f64`), so the importer rounds
  the float's exact value with integer operations on its bits, never
  with float arithmetic, and gets the same `Fx` on every platform. A
  value outside `Fx` is an error naming the file, the layer and the
  object.
- **Polylines and polygons.** Object layers keep polylines and polygons
  (Tiled `polyline` and `polygon` objects; an LDtk entity's field of
  points that the editor shows as a path, `PointPath`, or as a loop,
  `PointPathLoop`, which stays a property too), each as its points in
  `Fx` pixels of its level, beside boxes and points. Update ADR-0004
  §2's table row (object layers: named boxes and points with
  properties) in the same PR.
- **Either order with 2122:** if 2122 is done, add one function beside
  `TileMap` that turns a polyline object into
  `shuyi_core::curve::Curve::lines(points, false)` and a polygon into
  `Curve::lines(points, true)`, tested on a fixture made in Tiled. If
  2122 isn't done, 2122 adds it.
- **Levels and worlds:** a `.ldtk` file keeps every level, each with its
  identifier, `iid`, world position (`worldX`, `worldY`) and size, with
  its layers under it; a Tiled map is one level at (0, 0). A small kind,
  `MapWorld` (each level's file or identifier, position and size), has
  two formats: Tiled's `.world` file (its `maps` list; its
  regular-expression `patterns` are out) and the `.ldtk` file (its
  levels). 2318 and 2362 read level rectangles from it. If this is over
  a PR's worth when unparked, `MapWorld` and the `.world` format go to a
  follow-up ticket, parked like this one.
- **Nothing a painter needs is dropped (2360 draws it).** `TileMap`
  keeps, and its text dump prints:
  - each tile's flips (Tiled: the GID's horizontal, vertical and
    diagonal bits, the diagonal being a transpose done first; LDtk: the
    tile's `f` bits for x and y);
  - each tile layer's opacity (0 to 255), offset in pixels, parallax
    factor per axis, and whether it is shown. The parallax factor is
    Tiled's (1 moves with the camera); LDtk's, where 0 does, is
    converted to it;
  - each tileset's image, tile size, columns, margin and spacing;
  - the map's orientation (orthogonal, isometric, staggered or
    hexagonal; LDtk is always orthogonal), and for a staggered or
    hexagonal map Tiled's stagger axis (`x` or `y`) and stagger index
    (`odd` or `even`), and for a hexagonal one its hex side length.
    2121 and 2740 leave Tiled hex maps here: these give 2121's
    `HexLayout` (*Hex maps* below). A small hexagonal fixture made in
    Tiled shows them in the dump.
- **Tile animations.** Tiled's per-tile `<animation>` in a TSX tileset
  (`<frame tileid duration>`) is kept, one `AnimationClip<u16>` of tile
  ids (2231) per animated tile, in the tileset the map reads; the
  importer never drops it (2360 draws it). A frame's `duration` is its
  `ms`, and a frame's tile id past 65,535 is an error naming the tileset
  and the tile. LDtk has no tile animations; `docs/formats.md` says so.
- **Tile properties.** Each tileset keeps its tiles' custom properties
  with their editor types, as an object's are kept (Tiled's per-tile
  properties; LDtk's per-tile custom data, a `string`), and the text
  dump prints them. 2709 reads an integer terrain class from one.
- **Terrain for RTS paths. Either order with 2709:** if 2709 is done,
  add `TerrainGrid::from_layer(&map, layer, property)` to
  `shuyi_rts::paths`, which reads a tile layer whose tiles carry an
  integer custom property (for example `terrain`) into a `TerrainGrid`,
  tested on a fixture map made in Tiled. If 2709 isn't done, nothing to
  do: 2709 adds it.
- **Rooms for the camera. Either order with 2362:** if 2362 is done,
  add `Rooms::from_map(&TileMap, layer: &str)`, one room per box in that
  object layer, and `Rooms::from_world(&MapWorld)`, one room per level
  rectangle (an LDtk file's levels, or a Tiled `.world`'s maps). If 2362
  isn't done, 2362 adds both. If `MapWorld` went to a follow-up,
  `from_world` goes with it.
- **Hex maps. Either order with 2121:** if 2121 is done, add one
  function beside `TileMap` that gives a hexagonal map's
  `shuyi_core::hex::HexLayout`: the stagger axis gives its orientation
  (`y` is `Pointy`, `x` is `Flat`) and the stagger index its offset
  (`odd` is `Odd`, `even` is `Even`); any other map gives `None`. Test
  it on a hexagonal fixture made in Tiled. If 2121 isn't done, 2121
  adds it.
- **Either order with 1122:** if 1122 is done, add a function beside
  `TileMap` that builds 1122's `TileCollision` from a named tile layer
  and a rule per tile id, tested on the Tiled fixture. If 1122 isn't
  done, 1122 adds it.
- **From 2220 (done):** a test shows a changed `.tmx` reloading through
  `Hot<TileMap>`, including a change to a TSX tileset it reads, through
  the headless backend's script (`Harness::change_file`).
- `shuyi-cli import <map> --print` prints the map as our text.
  **From 1129 (done):** `import` tries only the kinds that
  `Importers::standard()` lists (`crates/cli/src/contract/import.rs`),
  so registering the formats isn't enough. Add
  `kind_with_text::<TileMap>` there (and `MapWorld`, and the tileset
  kind if it is one), summed up in a line, with `--print` writing the
  map's text dump, as 2231 does for `AnimationSet`. That file's module
  doc says `import` needs no change for these importers: correct it.
- **The skill pack. From 1134 (done):** drop 1124 from the
  `waits_for` of `skills/shuyi-import-art/SKILL.md` (if the LDtk
  importer moved to a follow-up, that ticket's id takes its place), and
  make the skill's part on Tiled and LDtk say what this ticket built:
  the map formats and `shuyi import <map> --print`. If that leaves
  `waits_for` empty, drop it and write the skill's worked example,
  `skills/shuyi-import-art/example.ron`, which CI's `skills` job runs
  on a fresh `shuyi new` game (`check-skills` asks for one).
- **Either order with 2719:** if 2719 is done, its RTS example's map
  is made in code and stays so: nothing to do. If not, nothing to do
  either: 2719 draws that map in Tiled or LDtk.
- **Size.** If the PR passes about 600 changed lines (not counting
  fixtures, snapshots and the vendored LDtk types), the parts for 2122,
  2362 and 2709 are left to those tickets (their clauses then add
  them), then `MapWorld` moves out as *Levels and worlds* says, then the
  LDtk importer goes to a follow-up ticket, parked like this one, with
  its acceptance lines. A part moves whole: nothing the importers keep
  is cut to fit. The PR that moves `MapWorld` or the LDtk importer out
  adds its follow-up to the `blocked_by` of each open ticket that needs
  that part (2232, 2318 and 2360 read LDtk maps; 2318 reads `MapWorld`).

**Out (do not do):**
- disk layers (done in 2207: `shuyi-app` reads a folder into a layer; only
  `shuyi-app` touches files, game ADR-0004);
- mods' folder and load order (1014); the web build's disk layers (none
  on the web);
- drawing the map: 2360;
- streaming a world's levels around the camera (loading and freeing a
  `MapWorld`'s levels as it moves): every level loads whole here.
  2906's ADR notes it; a ticket, parked like this one, when a game
  asks;
- typed map objects, the game's types written to the editors, and
  checking objects against them (2232);
- the `Curve` type and its follower (2122), the camera's rooms and
  their rule (2362), and RTS terrain and paths (2709).

## Implementation steps

1. Write `TileMap` (levels, objects with their properties, polylines and
   polygons, and what a painter needs) and the Tiled importer with the
   tile animations, with fixtures.
2. Add the LDtk types and importer, with fixtures.
3. Add `MapWorld` and its two formats, or write its follow-up ticket as
   *Levels and worlds* says.
4. Change ADR-0004 §2's row for object layers, and say in
   `docs/formats.md` that LDtk has no tile animations.
5. If 2121, 2122, 2362 or 2709 is done, add the hex layout, the curve
   function, the rooms or `TerrainGrid::from_layer`.
6. Add the map reload test through its headless
   script.
7. Update the `shuyi-import-art` skill as *The skill pack* says.

## Acceptance criteria

- [ ] Each fixture imports to its expected `TileMap` (a snapshot of its
      text dump), and the layers both editors drew alike dump alike. The
      fixtures include flipped tiles, a layer with opacity, offset and
      parallax, and (Tiled only) an animated tile, which the dump shows.
- [ ] Both fixtures keep an object of every property type, with an
      object reference, and the LDtk fixture's two levels keep their
      world positions; a `.world` fixture loads as a `MapWorld`.
- [ ] Both fixtures keep a polyline and a polygon with their points, and
      ADR-0004 §2's row says so. If 2122 is done, the Tiled fixture's
      turn into `Curve`s.
- [ ] A staggered or hexagonal Tiled fixture's dump shows its stagger
      axis and index. If 2121 is done, a hexagonal one gives its
      `HexLayout`.
- [ ] A fraction at a tie rounds toward negative infinity, and a value
      outside `Fx` is an error naming the file, the layer and the
      object.
- [ ] If 2362 is done, `Rooms::from_map` and `Rooms::from_world` read
      the fixtures.
- [ ] The Tiled fixture's tileset has a tile with an `int` property,
      which the dump shows. If 2709 is done, `TerrainGrid::from_layer`
      reads a layer of that fixture into the expected terrain grid.
- [ ] A broken fixture names the file and the place in it.
- [ ] `shuyi-cli import` reads the Tiled fixture as a `TileMap`, and
      `--print` prints its text dump with no limit saying the kind has
      no writer.
- [ ] A changed map and a changed tileset reload.
- [ ] `shuyi-import-art`'s `waits_for` no longer lists 1124, the skill
      has its worked example once that list is empty, and
      `check-skills` passes.
- [ ] `cargo deny check` passes, and the WASM build passes with both
      importers.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Fixture and snapshot tests as above.
- Unit: each property type; the class and properties a tile object and
  a template instance inherit; the rounding at a tie and past `Fx`; each
  flip bit; a tile animation's clip.
- Integration: map reload through 2220's script.

## Completion notes
