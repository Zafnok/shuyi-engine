---
id: "2286"
title: "Corner sheets from bought kits: recipes that read a Tiled terrain set, a 3×3 patch and a quarter-tile kit"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2286 — Corner sheets from bought kits

## Context

A request from the game.

1. **The game ticket:** game#1405, the tileset importer's corner sheets
   from these recipes
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1405-corner-sheets-from-the-engines-recipes.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** turn a bought tile kit into the 15
   mixes of a corner layer, whether the kit is a Tiled tileset whose
   terrain set tags each tile's corners, a 3×3 patch of drawn edges
   around a centre, or a kit of quarter tiles (the RPG Maker layout:
   each tile made of four quarter cells, picked by which corners are
   inside).
3. **The friction numbers:** the game's third engine-friction sweep
   (game#1397, 2026-11-27): the game's importer
   (`crates/tools/src/tileset_import.rs`) keeps about 355 production
   lines for these three kit shapes:
   - the Tiled reader (`tags`, `parse_tsx`, `corner_terrains`, lines
     539–680) and the pick of a terrain's 16 corner tiles
     (`Sheet::corners`, 779–803);
   - the 3×3 patch (`PATCH`, 470–491, and `patch_layer`, 1618–1660);
   - the quarter kit (`Kit::quarters`, 436–461, `quartered`, 998–1019,
     `joined`, 1067–1092, and `quarters_layer`, 1580–1616).
   With these recipes the game would keep about 50 (its mapping file's
   choice of kit per layer), so the ratio is about 7×.
4. **The generic shape:** three more ways to `Make` a corner sheet in
   `crates/assets/src/corners/recipe.rs`, beside `Generated` and
   `Rotated` (2283, done):
   - `Tiled { tsx, terrain, pick }`: a `.tsx` file of the recipe's
     folder and the name of one terrain of its terrain set. The 16
     corner combinations are found from each tile's `terrain="a,b,c,d"`
     (older Tiled) or its Wang set's corner colours (newer Tiled).
   - `Patch { picture, origin, size }`: a 3×3 block of tiles (corners,
     edges, centre) with the inner corners beside it, cut into the 15
     mixes by a fixed table.
   - `Quarters { picture, origin, layout }`: a quarter-tile kit; each
     mix is built from four half-size cells chosen by which of its
     corners are inside.
   Each one yields the same `CornerSheet` that `Generated` and `Rotated`
   do, so a tileset's layer doesn't know which kind of kit drew it.
5. **Examples from games that aren't tactics:** a farming game on a
   bought Tiled tileset of tilled soil and grass; a top-down adventure
   on an RPG Maker-style quarter-tile water and cliff kit. Both want the
   same 15 mixes from the art they bought, with no hand-cut sheet.

## Nick input

None.

## Scope

**In:**
- The three `Make` variants, their RON spelling, their checks (a
  terrain name the `.tsx` doesn't have, a corner combination no tile
  has, a patch or kit outside its picture: each an error naming the
  file and what is missing), and their docs.
- A small, dependency-free reader of the part of a `.tsx` file the
  `Tiled` recipe needs: the image, the tile size and margins, terrain
  and Wang sets, and each tile's corners. Not a general XML parser.
- Reading the picture through the recipe's folder, as `Rotated` does.
- `Make` is already an enum that grows: say in the completion notes
  whether it is `#[non_exhaustive]` (ADR-0039); add the variants without
  breaking.

**Out (do not do):**
- Tiled or LDtk maps (`.tmx`, `.ldtk`): parked 1124 and 2274. This reads
  a tileset's corner tags only, not a map.
- Tile animations in a `.tsx`: parked 2359 and 2231.
- The game's use (game#1405) and its own tileset file format.
- Edge (side) Wang sets and mixed sets: corners only, the corner layer's
  model. A side-only set is an error naming the set.

**Estimated size:** about 900 lines (code 450, tests 350, docs 100).

## Implementation steps

1. **Read** `crates/assets/src/corners.rs`, `corners/recipe.rs`
   (`Make`, `Generated`, `Rotated`, how a recipe reads its folder) and
   2283 in `tickets/done/`.
2. The `.tsx` reader: tags and attributes, entities unescaped, then the
   tileset's image and grid, terrain sets (older Tiled) and Wang sets
   with corner colours (newer Tiled). Cite the Tiled docs' field names in
   comments.
3. `Make::Tiled`: the 16 corner masks to tile indices (several tiles
   with one mask: `pick` chooses, 0 by default), then the 15 mixes cut
   from the image.
4. `Make::Patch`: the fixed table from each mix to its patch cells.
5. `Make::Quarters`: each mix's four quarter cells by the corners
   inside, joined into one tile.
6. Docs on `Make` and the corners chapter of the book; a doc example per
   variant.

## Acceptance criteria

- [ ] A recipe test per variant, on a small test picture drawn in the
      test: the 15 mixes match a hand-built expected sheet pixel for
      pixel.
- [ ] A `.tsx` with older `terrain=` tags and one with a Wang corner set
      give the same sheet for the same picture.
- [ ] A missing terrain, a missing corner combination and a kit outside
      its picture each give one error naming the file and the missing
      piece.
- [ ] Existing `Generated` and `Rotated` recipes and their tests are
      unchanged.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the `.tsx` reader on both Tiled spellings, entities and
  margins; each variant's table.
- Property: every mix a recipe makes is the tile's size; a kit of one
  flat colour gives 15 flat tiles.
- Snapshot / integration: none.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
