---
id: "2281"
title: "The terrain-tiles format reads the pictures a map may place on its tiles"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2281 — The terrain-tiles format reads the pictures a map may place on its tiles

## Context

- **What came before it:**
  - 3247 (done) gave a map's look `props`: named pictures placed by hand
    on tiles (`look: (props: { (19, 5): "lamp" })`).
  - It gave a tileset's look `Look::props: BTreeMap<String, ImageRect>`,
    what each name is drawn as.
  - The sprite skin's ground draws them (`paint_props`,
    `crates/ui/src/grid/sprite/ground.rs`).
  - 3247 was written before 2243 (done) gave the engine its own tileset
    file, the `terrain-tiles` format (`tileset::TerrainTilesDef`, read by
    `TerrainTilesRon`, resolved by `TerrainTilesDef::resolve`,
    `crates/assets/src/tileset/def.rs`). So 3247 left "a game's tileset
    file format" out, for the game to write.
- **The gap:** 2243's file has no field for `props`, and `resolve`
  builds each `Look` with none. A game on the engine's format can place
  lamps in its maps but can't say what a lamp looks like without
  building `Look::props` in code after `resolve`.
- **The game:**
  - game#1219
    ([`tickets/open/1219-tileset-terrain-looks-on-the-engines-format.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1219-tileset-terrain-looks-on-the-engines-format.md))
    moves its tileset loader onto this format.
  - game#1378
    ([`tickets/open/1378-lamps-at-harls-gate-and-the-road-bridge.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1378-lamps-at-harls-gate-and-the-road-bridge.md))
    places its lamps with 3247.
  - Once 1219 lands, 1378 writes its lamp's picture in this file.
  - Neither is a blocker: either order works for the engine.
- **Friction numbers:** no friction review: a gap between two tickets
  that landed apart.
- **The generic shape:** a look in a `terrain-tiles` file names the
  pictures a map may place, each a cell of the image and a size in
  pixels: `props: { "lamp": (at: (4, 0), size: (16, 32)) }`.
- **An example that isn't tactics:** a farming game's tileset with its
  signpost, mailbox and street lamp, which its town map places by name.

## Nick input

`None.`

## Scope

**In:**
- `TerrainTilesDef` and `LookDef` each gain `props: BTreeMap<String,
  PropAt>`: a name to its picture. Each is left out as empty
  (`#[serde(default)]`). Both types are `#[non_exhaustive]`, so the new
  field is additive.
- A new `PropAt`, `#[non_exhaustive]`, built with `PropAt::new(at)`:
  - `at: (u32, u32)`, the picture's `(column, row)` cell of `tile_px`
    cells;
  - `size: Option<(u32, u32)>`, in pixels from the cell's top-left, left
    out as `tile_px`;
  - `with_size(size)`, since `new` takes no two neighbouring numbers
    (ADR-0151).
- `TerrainTilesDef::problems` reports:
  - a name that is empty;
  - a `size` with a side of 0, or outside the rules' `side_px`, as a
    corner layer's `size` is reported.

  Each problem names its look (`look "winter": prop "lamp": …`).
- `TerrainTilesDef::resolve` fills each `Look::props` with the picture's
  `ImageRect`, and reports a picture outside the image as it reports a
  layer's.
- Docs:
  - `docs/formats.md`'s *Terrain tiles* paragraph;
  - the `tileset` module's example file;
  - the book's paragraph on placed pictures in *Frame and skins*;
  - the `shuyi-import-art` skill, where it shows the file.

**Out (do not do):**
- Checking a map's prop names against a tileset. That is the game's to
  do (3247), as a map's tile look is.
- Changing `Look`, `MapLook` or the sprite skin (3247 did them).
- Moving pictures (2359, parked).
- The game's use of it (game#1219, game#1378).

**Estimated size:** about 500 lines (code 180, tests 250, docs 70).

## Implementation steps

1. `PropAt` and the `props` fields in `crates/assets/src/tileset/def.rs`,
   with serde (`deny_unknown_fields`, as the others).
2. `problems`: the checks above, in the order the file is read (the own
   look, then each other look by name).
3. `resolve`: in `look(…)`, build `Look::new(tiles, layers)` and insert
   each prop's `ImageRect` (`ImageRect::cell` and the size, as a corner
   layer's), checked against the image (`ImageInfo::holds`).
4. Tests in `crates/assets/src/tileset/def/tests.rs`. Add a prop to the
   golden fixture (`format/builtin_tests.rs`), which moves the
   `terrain tiles: terrain-tiles` row of `tests/golden/formats.ron`:
   bless it, and write a `Breaking:` line for the golden value.
5. The docs above.

## Acceptance criteria

- [ ] A `terrain-tiles` file with `props: { "lamp": (at: (4, 0), size:
      (16, 32)) }` resolves to a look whose `props` has "lamp" as the
      16 × 32 rectangle at pixel (4 × tile_px.0, 0).
- [ ] A prop with no `size` is `tile_px`. A file without `props` reads
      and resolves exactly as before.
- [ ] An empty name, a side of 0, and a size outside `side_px` are each
      reported, naming the look and the prop. A picture outside the image
      is reported by `resolve`.
- [ ] Another look's `props` are its own: a look without any places
      nothing, even when the own look has some.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: reading and resolving `props` (none, some, with and without
  `size`, in another look), and each problem above.
- Property: a `TerrainTilesDef` with props round-trips through RON.
- Snapshot / integration: the golden row of the fixture with a prop.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
