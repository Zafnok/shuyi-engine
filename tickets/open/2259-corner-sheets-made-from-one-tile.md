---
id: "2259"
title: "The shuyi-corners module: a terrain's 15 corner pictures made from its one tile (or turned from 5 drawn pieces), checked in and tested equal"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2259 — The shuyi-corners module: corner sheets made from one tile

## Context

The grid kit draws a terrain's edges with a corner layer:

- `shuyi_assets::tileset::CornerLayer` (`crates/assets/src/tileset.rs`)
  is a "dual grid": a picture one tile big centred on each point where
  four tiles meet, chosen from the 16 [`mix`]es of those four being in
  the layer. The sprite skin paints it over the terrains' own tiles
  (`crates/ui/src/grid/sprite/ground.rs`, `paint_corners`).
- So someone draws 15 pictures for each terrain that has an edge (mix 0
  draws nothing). The toy draws its shore by hand in rows 4 and 5 of its
  tiles, mix `m` at column `m % 8`, row `4 + m / 8`
  (`crates/ui/src/toy/grid.rs`, `mix_rect`).

This ticket adds a way to make those 15 pictures from the terrain's one
tile, at tool time, in integers. The result is an ordinary PNG, so every
painter draws it as it draws a hand-drawn sheet: macroquad, the raster
painter, the one-bit bake (3007). As with `preview.png` (2811), a recipe
in text makes the file, the file is checked in, and a check fails when
it is stale. A second recipe serves hand-drawn art: a dual grid needs
only 5 pieces drawn (one corner in, two side by side, two across, three
in, all four), and the other 10 are their quarter turns.

**It is a module, opt-in** ([ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)):
the crate `shuyi-corners`, turned on with `shuyi add corners`.

- Hand-drawn corner layers stay the grid kit's way, and nothing in the
  default engine changes: no default crate depends on the module
  (ADR-0009 §4), and a game that doesn't turn it on doesn't build it.
- A game can use it for one terrain's edges and draw the rest by hand.
  A generated sheet is a plain PNG, so the game can also generate once,
  touch the sheet up by hand, and drop the recipe.

### Why it looks right: the field

A corner picture's four corners are the centres of the four tiles round
its point, so the picture is one bilinear patch. For a pixel at `(u, v)`
in `0..1` across the picture (`u = (2x + 1) / 2w`, `v = (2y + 1) / 2h`)
and the four tiles' in-values `tl, tr, bl, br` (0 or 1, from the mix's
bits 8, 4, 2, 1):

```text
field = tl (1−u)(1−v) + tr u(1−v) + bl (1−u) v + br u v
```

A pixel is in where `field + wobble > 1/2`. In integers, multiply by
`4wh`: `F = tl (2w−2x−1)(2h−2y−1) + tr (2x+1)(2h−2y−1) + bl
(2w−2x−1)(2y+1) + br (2x+1)(2y+1)`, and the test is `2F + W > 4wh`,
with `W` the wobble in the same units (below). Three things follow:

- **Corners round themselves.** One tile in gives a quarter bounded by a
  hyperbola, not a square; three in gives the hole the other way.
- **The pictures meet without a seam.** On the border two pictures
  share, the field depends only on the two corners they share, so it is
  the same on both sides. The wobble keeps this when it repeats with the
  picture's size.
- **Two across (`#..#`, `.##.`) stays two pieces.** The centre's field is
  exactly 1/2, and "in" needs more than 1/2.

## Nick input

None.

## Scope

**In:**
- **The crate** `shuyi-corners` in `crates/corners`, as ADR-0009 §4
  says:
  - `[package.metadata.shuyi]`: `answers = [{ concern = "pictures",
    answer = "corner-sheets" }]`, `serves = ["top-down-grid"]`;
  - its line in `tools/layers.ron`: engine `shuyi-core` and
    `shuyi-assets`; outside `serde` and `png` (MIT OR Apache-2.0, already
    in the workspace through `shuyi-assets`; `cargo deny` checks it);
    `devices: false`, `floats: false`;
  - `README.md` in ADR-0009 §4's order, its size measured as
    `shuyi-script`'s README measures its own;
  - the `shuyi` crate's feature `corners = ["dep:shuyi-corners"]`, off
    by default (a library, so no `Module`, like `script`);
  - its row in `docs/modules.md` and its answer in `docs/modules.ron`.
    **Either order with 2260:** if 2260 is done, the `pictures` concern
    ("Pictures made from data", `alternatives: false`) and its catalog
    section exist: add `corner-sheets` (`by: "shuyi-corners"`, `status:
    ready`, `tickets: ["2259"]`) and the row; if not, add the concern
    with its `drawn` answer (`by: "default"`, `ready`) and this one, and
    the catalog's section with this row;
  - a skill `shuyi-corners` in `skills/` (`check-skills` passes);
  - `-p shuyi-corners` in CI's `determinism` job's crate list
    (`.github/workflows/ci.yml`).
- **The library**, `#![no_std]` with `alloc`, pure (no I/O, no floats):
  - `Wobble { strength: u8, cells: u8, seed: u64 }` and `Rim { colour:
    [u8; 3], width: u8, side: RimSide }` with `RimSide { Outside,
    Inside }`. Both `#[non_exhaustive]` with a `new` (ADR-0039).
  - `pub fn generate(tile: &Image, wobble: Option<Wobble>, rim:
    Option<Rim>) -> Result<Image, CornersError>`: the sheet, 8 × 2 tiles
    of `tile`'s size, mix `m` at column `m % 8`, row `m / 8`, mix 0's
    place clear. `Image` is `shuyi_assets::image::Image`.
  - `pub fn rotated(pieces: [&Image; 5]) -> Result<Image, CornersError>`:
    the same sheet from the pieces of mixes 8 (`#...`), 12 (`##..`),
    9 (`#..#`), 14 (`###.`) and 15 (`####`), each other mix a quarter
    turn of one of them. Square pieces only.
  - `pub const fn sheet_rect(origin: (u32, u32), tile_px: (u32, u32), m:
    u8) -> ImageRect` and `pub fn layer(of: BTreeSet<TerrainId>, origin,
    tile_px) -> CornerLayer`, which fills mixes 1 to 15 of a sheet. The
    grid kit's `CornerLayer` is built from its public fields, unchanged.
  - `CornersError`, a `thiserror` enum (ADR-0002 §9): an empty tile, a
    tile side over 256, pieces of different sizes, pieces that aren't
    square, `cells` 0 or over 32, `strength` over 100.
  - The recipe's types and parse (below), through
    `shuyi_assets::ron::parse_ron`, so errors give the file and line.
- **The rules of `generate`**, as *Why it looks right* says, plus:
  - **Pixels.** An in pixel takes the tile's pixel at `((x + w/2) mod w,
    (y + h/2) mod h)`: the picture sits half a tile off the tile grid,
    so mix 15 lines up with the plain tiles round it. An out pixel is
    clear (`[0, 0, 0, 0]`), so the tile below shows.
  - **Wobble.** Value noise on a lattice of `cells × cells` points that
    repeats every picture, so the left border's lattice is the right
    border's. Each point's value is in `−128..=128`, from `SimRng::new
    (seed)` (`shuyi_core::rng`) rolled in row order. A pixel's value
    `N` is the bilinear blend of its four lattice points in integers,
    and `W = N × strength × 4wh / (128 × 100)`, rounded toward zero. A
    strength of 0, or no wobble, gives `W = 0`.
  - **Rim.** A pixel's worth of field is `2 / (w + h)` (one pixel across
    a straight edge on a square tile). `Outside`: an out pixel whose
    field is within `width` pixels' worth below the threshold is
    `colour`, opaque (foam on the water). `Inside`: an in pixel within
    `width` pixels' worth above it is `colour` instead of the tile (a
    dark edge on the grass). The rim is read from the field, never from
    neighbouring pixels, so it stays seamless too.
- **The recipe**, a RON file, `<name>.corners.ron`,
  `deny_unknown_fields` (a tool's input, not an asset kind: the game
  never loads it):
  ```ron
  (
      make: Generated(
          tile: (image: "tiles/terrain.png", at: (24, 0), size: (24, 24)),
          wobble: Some((strength: 30, cells: 4, seed: 7)),
          rim: Some((colour: "#E8F4FF", width: 2, side: Outside)),
      ),
      out: "tiles/water-corners.png",
  )
  ```
  or `make: Rotated(image: "tiles/shore.png", size: (16, 16), pieces:
  [(0, 0), (16, 0), (32, 0), (48, 0), (64, 0)])`. Paths are relative to
  the recipe's folder. A colour is `#RRGGBB`, as the palette's
  (`crates/assets/src/palette.rs`).
- **The `std` feature** (off by default), for tools only:
  - `pub fn make(recipe: &Recipe, read: impl Fn(&str) -> Result<Vec<u8>,
    String>) -> Result<Image, String>`: decodes the recipe's pictures with
    `shuyi_assets::bitmap::convert::png_pixels` (`shuyi-assets`' `png`
    feature) and makes the sheet;
  - `pub fn check(recipe_file: &str, recipe_text: &str, read) ->
    Result<(), String>`: fails when `out` is missing or its decoded
    pixels differ from what the recipe makes now (pixels, not bytes, as
    `shuyi-cli`'s `preview::check` does). The message names the file and
    ends with the command that remakes it. A game calls it from a test,
    as a game's test calls `preview::check`, so a stale sheet fails the
    game's CI;
  - a binary, `shuyi-corners <recipe.corners.ron>...`
    (`required-features = ["std"]`), which writes each recipe's `out`
    (the `png` crate's encoder) and says which files it wrote, and with
    `--check` writes nothing and runs `check`. The README says how a
    game installs it (`cargo install --locked --git … shuyi-corners
    --features std` at its engine pin). Files are touched only here and
    in the caller's `read`: the library stays pure (`AGENTS.md` rule 5).
- **Docs** (the `document` skill): the README, the skill, and a line in
  the grid kit's docs pointing to the module for made corner sheets.

Nothing breaks and the default engine doesn't change: no `Breaking:`
line.

**Out (do not do):**
- **Any change to a default crate** (`shuyi-core`, `shuyi-assets`,
  `shuyi-ui`, `shuyi-cli`, …) beyond the `shuyi` crate's feature. The
  toy keeps its hand-drawn shore; `shuyi-cli` gets no command.
- **Variants of a mix.** A generated edge repeats wherever the same mix
  does, as a hand-drawn sheet's does. Several pictures per mix, picked
  by position, change `CornerLayer` and its painter; a game that asks
  gets its own ticket.
- **Drawing at run time or on the GPU.** No shader, no per-frame work:
  the 2D effects (2326, 2327) are parked, and a baked sheet draws on
  every target. Neither ticket changes.
- **Rotating sprites as they are drawn (2335).** The quarter turns are
  baked into the sheet; 2335 doesn't change.
- **The tileset format (2243).** It names each mix's picture by its
  rect; a game writes the sheet's 15 rects there, or builds the layer
  with `shuyi_corners::layer`. No change to 2243.
- **Finding recipes on their own** (a scan of the project). The binary
  and `check` take the files they are given.
- **An example game.** The module's tests and its look snapshot prove
  it; an example comes when a kind of game asks (ADR-0009 §7).

## Implementation steps

1. Create `crates/corners` with its `Cargo.toml` (metadata, `std`
   feature, the binary), `README.md`, and its `tools/layers.ron` line.
2. `src/field.rs`: the field `F` and the wobble `W` as two small
   functions over doubled coordinates `X = 2x + 1`, `Y = 2y + 1`, so a
   test can also ask for the field on a border (`X = 0`, `X = 2w`). Use
   `i64` for the products: `4wh` with a side of 256 and a strength of
   100 fits easily.
3. `generate`: for each mix 1 to 15 and each pixel, the in test, then the
   rim, then the pixel. Write each picture into the sheet at
   `sheet_rect`.
4. `rotated`: a quarter turn clockwise moves top-left to top-right,
   top-right to bottom-right, bottom-right to bottom-left and
   bottom-left to top-left, both for the pixels and for the mix's bits.
   Fill the 15 mixes from the 5 pieces and their turns; mix 15 is the
   piece as drawn, never turned.
5. `sheet_rect` and `layer`.
6. The recipe, then the `std` feature's `make`, `check` and the binary.
7. The `shuyi` feature, the catalog, `modules.ron`, the skill, the CI
   step, docs. Run `cargo xtask check-modules` and `check-layers`.

## Acceptance criteria

- [ ] For a 24 × 24 tile, `generate(tile, None, None)`:
  - [ ] mix 15's picture equals the tile shifted half a tile in both
    directions, wrapping, pixel for pixel;
  - [ ] mix 12 (`##..`) has rows 0 to 11 in and rows 12 to 23 clear;
  - [ ] mix 8 (`#...`): pixel (0, 0) is in, and pixel (11, 11) is out,
    though a square quarter would hold it (the corner is rounded);
  - [ ] mix 9's in pixels are two regions, joined side by side (not
    corner to corner): one holds (0, 0), the other (23, 23). The same
    for mix 6 with (23, 0) and (0, 23);
  - [ ] mix 0's place is clear.
- [ ] Seams: for every pair of mixes that meet side by side (the right
  pair of corners of one is the left pair of the other), the field with
  wobble at `X = 2w` of the first equals the field at `X = 0` of the
  second, for every `Y`, with 20 seeds and `cells` 1 to 8; the same for
  pairs that meet top to bottom.
- [ ] Rim, on mix 12 of a 24 × 24 tile with no wobble: `Outside` with
  width 2 colours rows 12 and 13 and leaves rows 14 to 23 clear; `Inside`
  with width 2 colours rows 10 and 11 and leaves rows 0 to 9 the tile's.
- [ ] The same tile, wobble and rim give the same sheet every run; two
  seeds give different sheets; `strength: 0` gives the sheet with no
  wobble.
- [ ] `rotated`: mix 4's picture is mix 8's piece turned a quarter
  clockwise, and mix 6's is mix 9's turned once; all 15 mixes are
  filled; pieces of two sizes, or not square, are an error.
- [ ] `layer(of, (0, 96), (24, 24))` equals the toy's hand-built shore
  layer's mixes 1 to 15 (`crates/ui/src/toy/grid.rs`, `layered`): the
  same layout.
- [ ] The binary writes a sheet; `--check` then passes; after the
  recipe's seed changes, `--check` fails, names the file and gives the
  command that remakes it; a missing `out` fails too; a bad recipe fails
  with its file and line.
- [ ] A golden table, `crates/corners/tests/golden/corners.ron`, has rows
  `generated` and `rotated` holding the FNV hash (`shuyi_assets::fnv`)
  of a fixed recipe's sheet, and the `determinism` job runs it on
  wasm32. The sheet is an asset a tool makes, not a frame's look, so its
  hash may be golden (`shuyi_assets::golden`, *What goes in a table*).
- [ ] A look snapshot: a 4 × 3 map with a generated shore, painted by the
  grid kit's sprite skin and the raster painter (dev-dependencies).
- [ ] `cargo xtask check-modules` and `check-layers` pass; `shuyi
  modules` lists the module; a game without the `corners` feature
  builds no `shuyi-corners` (`cargo tree` in a test or the README's
  measurement says so).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each acceptance case of `generate`, `rotated`, `sheet_rect` and
  `layer`; each `CornersError`; the recipe's parse errors (unknown
  field, bad colour, piece outside its image), each with its line.
- Property: the seam property over random seeds, cell counts and tile
  sizes from 2 × 2 to 64 × 48; a quarter turn done four times gives the
  piece back.
- Snapshot / integration: the binary and `--check` on a temporary
  folder, the golden table, and the look snapshot.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
