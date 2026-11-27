---
id: "2259"
title: "Corner sheets made, not drawn, opt-in: a terrain's 15 corner pictures made from its one tile (or turned from 5 drawn pieces), checked in and tested equal"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-27
---

# 2259 — Corner sheets made from one tile, opt-in

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

**Opt-in, and kept apart.** Hand-drawn corner layers stay the grid
kit's way. This is something a game chooses to use, never the main way:

- a game that doesn't opt in sees nothing change and builds none of it
  (or nothing beyond a few small helpers);
- the code sits in one place that could be removed whole;
- a game can use it for one terrain's edges and draw the rest by hand.
  A generated sheet is a plain PNG, so the game can also generate once,
  touch the sheet up by hand, and drop the recipe.

Where the code lives is the session's first decision, made against those
goals: its own module crate
([ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)),
a feature that is off by default, or tool-only code (Scope, *Where it
lives*). A whole crate may be more than this needs, but it isn't ruled
out.

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
- **Where it lives: decide first, and write down why.** Weigh the
  options against two goals that pull against each other: **keep the
  default engine clean** (a game that doesn't opt in builds none of it
  and sees nothing change; no new concept in the default crates' docs)
  and **keep it modular** (one place, removable whole; opt-in, never the
  grid kit's main way; not more paperwork than the feature). At least:
  - **A module crate**, `shuyi-corners`, as ADR-0009 §4 describes:
    metadata, a README with its measured size, a `tools/layers.ron`
    line, a feature of the `shuyi` crate off by default, a catalog row,
    a skill. Cleanest separation; the most paperwork for a small
    feature.
  - **A feature off by default** of the crate the code belongs beside
    (`shuyi-assets`, where `tileset` is), with the tool's command behind
    a matching feature, or always on, in `shuyi-cli`. ADR-0009 §4
    already does this for formats. Less paperwork; the code sits in a
    default crate, gated.
  - **Tool-only code.** The generator, the recipe and the command live
    where tools live (`shuyi-cli`), since a game ships only the PNG it
    makes. A game's build gains nothing; only the small layout helpers
    (`sheet_rect`, `layer`) would sit where a game can reach them.
  - Anything else the session finds that serves both goals better.

  Whichever is chosen: the generator stays pure (no I/O, no floats;
  files only in a tool, `AGENTS.md` rule 5); new engine edges or outside
  crates get their `tools/layers.ron` lines; the generator's tests run
  in CI's `determinism` job on wasm32; and the choice and the reasons
  for it go in the completion notes, with an ADR (the `write-adr` skill)
  if the choice sets a pattern others will follow (for example, the
  first feature of a default crate that isn't a format).

  If the choice is a module and it answers a concern in
  `docs/modules.ron`: **Either order with 2260:** if 2260 is done and
  added a `pictures` concern ("Pictures made from data",
  `alternatives: false`), add this answer and its catalog row there; if
  not, add that concern with its `drawn` answer (`by: "default"`,
  `ready`) and this one, or no concern at all if this isn't a module.
- **The generator**, pure (no I/O, no floats), `no_std` with `alloc` if
  its home allows it:
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
- **The tool**, where tools may touch files (in a module, behind its
  own `std` feature; in `shuyi-cli`, as a command):
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
  - a command that writes each recipe's `out` (a module's own binary,
    or `shuyi corners <recipe.corners.ron>...`), says which files it
    wrote, and with `--check` writes nothing and runs `check`. The docs
    say how a game runs it. Files are touched only here and in the
    caller's `read`: the generator stays pure (`AGENTS.md` rule 5).
- **Docs** (the `document` skill): where the chosen home's docs go, and a
  line in the grid kit's docs pointing to it for made corner sheets.

Nothing breaks, and nothing changes for a game that doesn't opt in: no
`Breaking:` line.

**Out (do not do):**
- **Making it the grid kit's main way.** Nothing that exists today
  changes how it works: hand-drawn layers, the toy's shore, the sprite
  skin. A game opts in.
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
  with `layer`. No change to 2243.
- **Finding recipes on their own** (a scan of the project). The command
  and `check` take the files they are given.
- **An example game.** The tests and the look snapshot prove it; an
  example comes when a kind of game asks (ADR-0009 §7).

**Estimated size:** about 1,400 lines (code 650, tests 550, docs 200).

**Split:** past 1,100 lines, split off `rotated` and the `Rotated`
recipe into a new ticket.

## Implementation steps

1. Choose the home (*Where it lives*): read ADR-0009, `docs/modules.md`
   and how `shuyi-assets`' format features and `shuyi-cli`'s generator
   commands (`font-atlas`, `glyph-tiles`) are set up, then set up the
   home with what it needs and nothing more.
2. A `field` module: the field `F` and the wobble `W` as two small
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
6. The recipe, then the tool's `make`, `check` and command.
7. What the home needs (features, `tools/layers.ron`, the CI step; for a
   module, its catalog row and skill too), docs, and the completion
   notes' reasons. Run `cargo xtask check-layers` (and `check-modules`
   for a module).

## Acceptance criteria

- [x] For a 24 × 24 tile, `generate(tile, None, None)`:
  - [x] mix 15's picture equals the tile shifted half a tile in both
    directions, wrapping, pixel for pixel;
  - [x] mix 12 (`##..`) has rows 0 to 11 in and rows 12 to 23 clear;
  - [x] mix 8 (`#...`): pixel (0, 0) is in, and pixel (11, 11) is out,
    though a square quarter would hold it (the corner is rounded);
  - [x] mix 9's in pixels are two regions, joined side by side (not
    corner to corner): one holds (0, 0), the other (23, 23). The same
    for mix 6 with (23, 0) and (0, 23);
  - [x] mix 0's place is clear.
- [x] Seams: for every pair of mixes that meet side by side (the right
  pair of corners of one is the left pair of the other), the field with
  wobble at `X = 2w` of the first equals the field at `X = 0` of the
  second, for every `Y`, with 20 seeds and `cells` 1 to 8; the same for
  pairs that meet top to bottom.
- [x] Rim, on mix 12 of a 24 × 24 tile with no wobble: `Outside` with
  width 2 colours rows 12 and 13 and leaves rows 14 to 23 clear; `Inside`
  with width 2 colours rows 10 and 11 and leaves rows 0 to 9 the tile's.
- [x] The same tile, wobble and rim give the same sheet every run; two
  seeds give different sheets; `strength: 0` gives the sheet with no
  wobble.
- [x] ~~`rotated`: mix 4's picture is mix 8's piece turned a quarter
  clockwise, and mix 6's is mix 9's turned once; all 15 mixes are
  filled; pieces of two sizes, or not square, are an error.~~ (not
  done: split off to 2283, at the ticket's `Split:` line)
- [x] `layer(of, (0, 96), (24, 24))` equals the toy's hand-built shore
  layer's mixes 1 to 15 (`crates/ui/src/toy/grid.rs`, `layered`): the
  same layout.
- [x] The command writes a sheet; `--check` then passes; after the
  recipe's seed changes, `--check` fails, names the file and gives the
  command that remakes it; a missing `out` fails too; a bad recipe fails
  with its file and line.
- [x] A golden table, `tests/golden/corners.ron` in the chosen crate, has rows
  `generated` and `rotated` (2283 adds that one) holding the FNV hash (`shuyi_assets::fnv`)
  of a fixed recipe's sheet, and the `determinism` job runs it on
  wasm32. The sheet is an asset a tool makes, not a frame's look, so its
  hash may be golden (`shuyi_assets::golden`, *What goes in a table*).
- [x] A look snapshot: a 4 × 3 map with a generated shore, painted by the
  grid kit's sprite skin and the raster painter (dev-dependencies).
- [x] Opt-in: a game that doesn't opt in builds none of the generator
  (`cargo tree` or `cargo build` timings, recorded in the completion
  notes), and every existing snapshot, golden row and toy picture is
  unchanged. `cargo xtask check-layers` passes (and `check-modules`, if
  it is a module).
- [x] The completion notes say which home was chosen, which others were
  weighed, and why.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

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

**What was done.** `shuyi_assets::corners`, behind a new `corners`
feature of `shuyi-assets` (off by default, no dependency, `no_std`):
`generate`, `sheet_rect`, `layer`, `Wobble`, `Rim`, `RimSide`,
`CornersError`, and the recipe (`corners::recipe`: `parse`, `Recipe`,
`Make`, `Tile`, and `Recipe::sheet`, which cuts the tile out of its
picture and makes the sheet, purely). `shuyi_cli::corners`, behind
a new `corners` feature of `shuyi-cli` (off by default, turning the first
on): `make`, `check` and the command `corners [--check] <recipe>...`.
The golden table `crates/assets/tests/golden/corners.ron` (row
`generated`) runs with `shuyi-assets`' tests, so on wasm32 in
the `determinism` job; the no-std job builds the feature too. The look
snapshot is `crates/cli/tests/fixtures/corners/shore.png`.

**Where it lives, and why** (ADR-0160, which extends ADR-0009 §4). The
home is an off-by-default feature of the default crate the code belongs
beside (`shuyi-assets`, beside `tileset` and `image`), and a feature of
the same name of `shuyi-cli` for the command. Weighed:
- *A module crate, `shuyi-corners`*: the cleanest separation, but the
  metadata, README, skill, catalog row and `shuyi` feature are more than
  the code. And its concern would add a question to `shuyi new`'s wizard
  about something no game must choose. So it isn't a module, and no
  concern was added to `docs/modules.ron` (2260's clause now says so).
- *Tool-only code in `shuyi-cli`*: its tests would not run on wasm32
  (`shuyi-cli` isn't built for WASI), and `layer` would still need a home
  a game's code can reach.
- *Always on in `shuyi-cli`*: every game's tools would build it.

Opt-in, measured: `cargo tree -p hello-grid --features tools -e features
-i shuyi-assets` and `cargo tree -p shuyi-cli -e normal,features -i
shuyi-assets` show no `corners` feature: a game that doesn't opt in
builds none of it, tools included. With `--features corners`,
`shuyi-cli` turns `shuyi-assets/corners` on. No existing snapshot,
golden row or toy picture changed.

**Deviations.**
- **Split, at the ticket's `**Split:**` line:** `rotated`, `PIECES`,
  their two errors and the `Rotated` recipe went to 2283, with their
  acceptance case and the golden table's `rotated` row. With them the
  PR measured 1,816 counted lines. `Make` and `CornersError` are
  `#[non_exhaustive]`, so 2283 adds to them without a break.
- `Wobble::new(seed)` and `Rim::new(colour, side)`, with
  `with_strength`, `with_cells` and `with_width`, in place of
  constructors of every field: ADR-0151 refuses a constructor of two
  neighbouring numbers. The defaults are strength 30, 4 cells, a rim 1
  pixel wide.
- `RimSide` is `#[non_exhaustive]` too.
- An in pixel takes the tile's pixel at `((x + w − w/2) mod w, (y + h −
  h/2) mod h)`. That is the ticket's `(x + w/2) mod w` for an even side,
  and one pixel over for an odd one. The painter puts a picture's
  top-left half a tile, rounded down, up and left of its point, so this
  is what lines mix 15 up with the tiles round it
  (`an_odd_tile_lines_up_as_the_painter_centres_it`).
- `W` keeps `N` exact and rounds once, toward zero: `blend × strength /
  12800`, with `blend = N × 4wh`.
- A tile outside its picture is found when the sheet is made, not as
  the recipe parses: only then is the picture's size known. Its error
  gives the line of the recipe's `at`, found in the recipe's text, as
  RON gives no line for a value.
- `make` and `check` both hand `read` the paths as the recipe writes
  them, from its folder: the caller's `read` resolves them (the command
  joins the recipe's folder). The command a stale sheet's message ends
  with is `cargo shuyi corners <recipe>`, as a game runs it.
- The look snapshot lives in `shuyi-cli`'s tests (they have the raster,
  the toy and a PNG encoder). The toy has no tile picture, only its
  size, so the test draws its own grass and water tiles. A tileset has
  one picture, so the test pastes the made sheet into it at `(0, 96)`.
  A game must do the same today: ticket 2282 lets a corner layer name an
  image of its own. The snapshot draws the water tiles' own picture as
  grass, so the layer draws all the water, rounded. With the water's own
  tile under it, a made layer only rounds the inner corners and adds the
  rim, since a convex corner's in pixels lie on the water tile anyway.

**Follow-up tickets.** 2283: the sheet turned from 5 drawn pieces (the
split). 2282: a corner layer's pictures in an image of their own.

Breaking: none. Everything is new and behind two features that are off
by default.

Behaviour: shuyi-cli: `shuyi`'s usage and `shuyi_cli::COMMANDS` list a
new command, `corners`. Without the feature, it runs in the game's tool
program, or says how to turn the feature on.

Book: `site/content/learn/book/frame-and-skins.md` has a paragraph on
corner sheets made from a recipe.

Game: nothing to adopt. The game names `CornerLayer` (`gh search code
--repo Zafnok/visions-of-shuyi CornerLayer`: its tileset content and map
view), which only gains a doc line.
