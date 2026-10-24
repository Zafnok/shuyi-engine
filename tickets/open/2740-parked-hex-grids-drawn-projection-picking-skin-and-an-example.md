---
id: "2740"
title: "Parked: Hex grids drawn: a hex projection with picking, the glyph skin in offset rows, and a small hex example with its guide"
type: feature
milestone: "Engine: hex grids"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2121", "2511"]
nick_input: none
completed:
---

# 2740 — Hex grids drawn: projection, picking, skin and an example

> **Parked** (Nick, 2026-10-20, choosing hex grids as a kind of game the engine serves; Visions of Shuyi is "the first game I want to be launched with the engine"): like real-time strategy, visual novels and detective games, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Each engine that has hex maps (2121 lists them) turns a
tile into a place on the screen and a place back into a tile through
its hex layout: Godot's `TileMapLayer.map_to_local` and
`local_to_map`, Unity's `Grid.CellToWorld` and `WorldToCell`
(https://docs.unity3d.com/ScriptReference/GridLayout.WorldToCell.html).

What Shuyi has today:

- `crates/ui/src/grid/layout.rs`: `Projection` with one variant,
  `TopDown`, not `#[non_exhaustive]`, and the `const fn`
  `Projection::place(corner, tile, dx, dy)`; `Grid` (`origin`,
  `corner`, `tile`, `tiles`, `scale`, `projection`) with `top_down`,
  `offset`, `tile_px`, `rect_at`, `rect`, `at_art_size` and `bounds`.
  Its docs say "today top-down squares only (ADR-0003 §6; the
  isometric one is ticket 1126)".
- The glyph skin (`crates/ui/src/grid/glyph.rs`): a tile is two 8 × 16
  cells. `Layout` (private fields `origin`, `cell`, `tiles`) and
  `tile_to_cell` ("the only place the 'two cells per tile' rule
  lives"); but `GlyphStyle::draw_tiles`
  (`crates/ui/src/grid/glyph/style.rs`) works out a tile's cells as
  `layout.cell.0 + TILE_W_CELLS * dx` itself, and
  `GlyphStyle::view_tiles` and `GlyphStyle::layout` count
  `TILE_W_CELLS` cells to a tile. `GlyphSkin` has public fields and a
  `const fn new`; the toy and Hello, Grid build it in `const` items.
- The path arrow (`crates/ui/src/grid/path.rs`) draws straight legs
  between four-way neighbours: `segment` and `arrowhead` match on
  `(1, _)`, `(-1, _)`, `(_, 1)` and the rest. The threat arrows
  (`crates/ui/src/grid/threat.rs`) already draw a line at any angle
  between tile middles found with `Grid::tile_px`, as one-pixel runs
  from the private `line`.
- The sprite skin (`crates/ui/src/grid/sprite.rs`) paints rectangles:
  each tile's picture, corner layers between tiles
  (`crates/ui/src/grid/corners.rs`) and tints. Hex tiles drawn from
  real hexagon art overlap their neighbours' rectangles, so its tints,
  layers and cursor don't carry over.
- `Cursor::step(dir: Dir, map_w, map_h)` moves by offset steps, and
  `steer` appends a tile when `Pos::manhattan(end, to) == 1`.
- Hello, Grid (`examples/hello-grid`) is the one example. CI's `wasm`
  job (`.github/workflows/ci.yml`), the `run-gates` skill and
  `.github/workflows/pages.yml` name it by its folder.

What it builds on:

- 2121 (parked, before this): `shuyi_core::hex`, `HexLayout`,
  `Orientation`, `Offset`, `Topology::Hex`, and the ADR that amends
  ADR-0003 §6. It proves that `(±1, 0)` and `(0, ±1)` are neighbours in
  every offset layout, so the cursor and `steer` work on hex maps as
  they are;
- before 2121: 2117's `find_path`, `PathOptions` and `reachable_on`,
  and 2013's ADR on public types that can grow, which `Projection` and
  `GlyphSkin` follow here;
- 2511 (MVP): the short way to start a game (a prelude, `run_game` and
  engine defaults), which keeps the example small;
- 2405 (after the MVP): the tile under the pointer
  (`Projection::tile_at`, `Grid::tile_at_px`, `GridSkin::tile_at`);
- 2336 (after the MVP): lines at any angle as frame items
  (`Shape::line`, `GlyphBuffer::add_shape`);
- 1126 (parked): `shuyi_ui::world::Projection` with `TopDown`,
  `Isometric` and `SideView`.

On a glyph console a hex map is offset rows of two-cell tiles, each
shifted row moved one cell right: the brick pattern text-console hex
games use. Every tile then shares an edge with exactly its six
neighbours, and no two tiles share a cell. So the glyph skin takes
pointy-top layouts only: flat top would need half-cell shifts, which a
cell console can't draw.

Why it matters:

- **The game first:** Visions of Shuyi needs none of this, so it waits
  for the launch with 2121.
- **Other 2D games:** wargames, hex strategy and hex board games see
  their map. ADR-0009 §4 has a direction claimed only when its example
  passes CI. Hex grids are served by the default kits (2121's ADR), and
  this ticket's example is that proof.
- **The end goal** (2026-10-19): an agent asked for "a hex strategy
  game" starts from this example and its guide.

## Nick input

None.

## Scope

**In:**
- **`Projection::Hex(HexProjection)`** in
  `crates/ui/src/grid/layout.rs`:
  - `Projection` becomes `#[non_exhaustive]` if it isn't yet (ADR-0039:
    a set that will grow; 1126 adds two more variants).
    `HexProjection { layout: HexLayout, step: i32 }` is a plain value
    under that ADR: public fields, exhaustive, deriving `Debug`,
    `Clone`, `Copy`, `PartialEq` and `Eq`;
  - `step` is the pixels from one row to the next (pointy top) or one
    column to the next (flat top), in console pixels like `Grid::tile`
    (it counts the zoom): the tile's height for the glyph skin's
    bricks, about three quarters of it for regular hexagon art.
    `Grid::hex` clamps it to `1..=` the tile's height (its width for
    flat top), so rows never leave a gap;
  - placing, pointy top: a tile's top-left pixel is `corner.x + dx ×
    tile.w`, plus `tile.w / 2` on a shifted row, and `corner.y + dy ×
    step`. Flat top swaps the axes: `corner.x + dx × step`, and
    `corner.y + dy × tile.h`, plus `tile.h / 2` on a shifted column.
    Whether a row (or column) is shifted comes from the tile's own row
    (`origin.y + dy`), not `dy`, so a camera on an odd row draws the
    same map. Work its parity as `(origin.y ^ dy) & 1`, which can't
    overflow. Add the `const fn` `Projection::place_at(self, corner,
    tile, origin: Pos, dx, dy)` and make `Grid::tile_px` and
    `Grid::rect_at` call it with the grid's origin; `place` keeps its
    signature and calls it with the origin `(0, 0)`;
  - `Grid::hex(origin, corner, tile, tiles, scale, projection:
    HexProjection)`, a `const fn` beside `Grid::top_down`.
    `Grid::bounds`, pointy top: `tiles.0 × tile.w + tile.w / 2` across
    and `(tiles.1 − 1) × step + tile.h` down, empty for no tiles; flat
    top swaps them. `Grid::at_art_size` divides `step` by the zoom as
    it does the tile;
  - **picking:** among the tiles whose rectangles hold a console pixel
    (one in each row, or column, whose band holds it), the one whose
    middle is nearest by squared distance in `i64`, ties to the smaller
    `(y, x)`. With `step` equal to the tile's height that is the one
    rectangle under the pixel. `Grid::tile_at_px` counts only drawn
    tiles, and gives `None` when no drawn tile's rectangle holds the
    pixel. **Either order with 2405:** if 2405 is done, add
    `Projection::tile_at_from(self, corner, tile, origin: Pos, px)`,
    which its `tile_at` calls with the origin `(0, 0)` as `place` calls
    `place_at`, give `Grid::tile_at_px` the hex case, and test the
    pointer on a hex map. If not, add `Grid::tile_at_px` with 2405's
    name and signature for both projections, and add to 2405's scope in
    this PR: "`Grid::tile_at_px` exists (2740); use it";
  - **either order with 1126:** if 1126 is done, `Projection` lives in
    `shuyi_ui::world`: add `Hex` there, with tile to world, world to
    tile (the picking above) and a depth key: the row for pointy top;
    for flat top, the row, then 1 on a shifted column and 0 on the
    others, so a lower tile draws later. If not, add it to the grid
    kit's `Projection` as above, and 1126 carries it over.
- **The glyph skin on a hex map** (`crates/ui/src/grid/glyph.rs`,
  `crates/ui/src/grid/glyph/style.rs`):
  - `GlyphSkin` gets the field `hex_rows: Option<Offset>`, `None`
    (square rows) in `GlyphSkin::new`, and `#[must_use] pub const fn
    with_hex_rows(self, hex_rows: Option<Offset>) -> Self`. With
    `Some(offset)` the skin lays a pointy-top map with that offset. A
    new field breaks a struct literal, so `GlyphSkin` follows 2013's
    ADR as config a game fills in: `#[non_exhaustive]` if it isn't yet.
    Literals inside `shuyi-ui` (`glyph/style/tests.rs`) may stay;
  - `Layout` keeps the offset in a private field, set by `#[must_use]
    pub const fn Layout::with_hex_rows(self, hex_rows: Option<Offset>)
    -> Self`. On a hex map, `GlyphSkin`'s `view_tiles`, `grid` and
    `paint` hand `GlyphStyle::view_tiles` and `GlyphStyle::layout` an
    area one cell narrower, so the shifted rows' last cell stays inside
    it, then set the layout's rows. `Layout::cells` is one cell wider
    on a hex map;
  - `tile_to_cell` moves a tile on a shifted row one cell right, judged
    by the tile's own row as `place_at` does, and stays the one place
    the rule lives: `GlyphStyle::draw_tiles` takes its cells from it
    instead of `layout.cell.0 + TILE_W_CELLS * dx`;
  - `Layout::grid` returns `Grid::hex` with `HexProjection { layout:
    HexLayout { orientation: Orientation::Pointy, offset }, step:
    CELL_PX.1 }` on a hex map, so `cell_of`, the units' painter, the
    threat arrows and 2405's pointer follow it;
  - both cursor looks (`CursorLook::Pulse` and `Flat`) draw on the
    shifted cells;
  - **the path arrow** (`path.rs`): a step between two rows (`dy` of
    ±1, `dx` of −1, 0 or 1) is a line `LINE_W` pixels wide between the
    two tiles' middles. The first step's line starts where it leaves
    the unit's tile, as `threat.rs`'s `Start::Edge` does. Steps within
    a row keep today's rectangles. **Either order with 2336:** if 2336
    is done, draw the line with `Shape::line(from, to, LINE_W, color,
    Layer::Under)` through `GlyphBuffer::add_shape`, clipped to
    `Grid::bounds`. If not, move `threat.rs`'s private `line` into a
    `pub(crate)` helper both files use, and grow each of its runs by
    `LINE_HALF` pixels on both sides, across the run. **Either order
    with 2117's follow-up** (a diagonal step in the path arrow, written
    when 2117 is done): if it is done, draw the steps between rows with
    its line; if not, say in that ticket that this line exists. The
    arrowhead keeps its four shapes: on a last step between rows it
    points down or up, so its `match` tests `dy` first (no change on a
    square map, where one of the two is always 0);
  - every square-map snapshot stays as it is.
- **The cursor and steering need no change** (2121's ADR). A test
  steers a path across rows on a hex map with the four directional
  actions alone, the cheapest path coming from `find_path` with
  `PathOptions::default().with_topology(Topology::Hex(layout))`.
- **The perspective marker.** **Either order with 1120:** if 1120 is
  done, add `HexGrid` to `shuyi_core::perspective` (ADR-0009 §3), with
  `Projection::Hex` as its projection in `shuyi-ui`. If not, add to
  1120's scope in this PR: "add the `HexGrid` marker for the hex
  projection of 2740".
- **`examples/hello-hex`**, a turn game written the short way 2511's
  minimal template writes a game (its prelude and `run_game`), on the
  `shuyi` crate alone, drawn
  by the glyph skin `with_hex_rows(Some(Offset::Odd))`:
  - a 9 × 7 map in `files/stage.map`. The format doesn't change: its
    rows are the offset rows of a pointy-top, odd-r layout, which the
    game's code names. Its own terrains: open ground, water that costs
    2 to enter, walls, and a goal tile;
  - a piece with 3 move points a turn. Its move range comes from
    `reachable_on` with `Topology::Hex`, shown as a range; the player
    steers its path with the cursor (`steer`, the cheapest path from
    that `Reach`'s `path_to`) and moves it with Confirm. Then two foes
    each step one tile along `find_path` toward it. Reaching the goal
    tile wins; a foe stepping onto the piece's tile loses. The map
    leaves a winning line, which the playthrough test proves;
  - `TurnRules` and `Sides`, so `bots::play` runs it;
  - `files/keymap.ron` with a keyboard layout, the pad's buttons and the
    `steam-deck` target's buttons, as Hello, Grid's has (the
    `input-actions` skill; nothing names a key); `shuyi.ron` with the
    `desktop`, `web` and `steam-deck` targets and the `notices` part for
    its font's license (3039); the font atlas copied from
    `examples/hello-grid/files/font` with its license file; a short
    `README.md` like Hello, Grid's;
  - CI's `wasm` job gets its two steps (`cargo shuyi check --target
    all` and `cargo shuyi web --release`, from its folder), the
    `run-gates` skill names it beside Hello, Grid, and Pages publishes
    it beside Hello, Grid with a link, as Hello, Grid is linked (2801's
    Examples page if 2801 is done, the index in `pages.yml` if not);
  - nothing from an existing hex game, and nothing from Visions of
    Shuyi (`AGENTS.md` rule 4);
  - tests: a Harness playthrough to the goal through actions only
    (`Harness::act`); a bot that plays 20 seeds with no rules error;
    replay determinism (the same hash list twice; if 2116 is done, add
    a golden table as its docs say a new example does); `frame-png`
    pictures of the start and of a path across rows, as binary
    snapshots like `examples/hello-grid/tests/it/snapshot.rs`;
    `cargo shuyi check`, `cargo shuyi check --target all` and
    `cargo shuyi check-keys` pass from its folder.
- **The guide and the catalog:**
  - `docs/guides/hex-grid.md`, in ADR-0009 §5's order: what starts it
    (this example, until a template part exists); the kits it uses (the
    grid kit with `Topology::Hex`, the glyph skin with `with_hex_rows`);
    what looks related but isn't for it (isometric, 1126; square maps
    drawn in bricks); the example to read first. **Either order with
    2004:** if 2004 is done, replace its planned page and its check
    passes. If not, write the file, and change 2004's scope in this PR
    so it lists `hex-grid.md` as written by 2740, not as a planned
    page;
  - **the catalog:** set the `hex-grid` answer in `docs/modules.ron`
    to ready (2003 wrote it parked).
- **What changes for the game:** `Projection` gains a variant and may
  become `#[non_exhaustive]`, so an exhaustive `match` on it breaks;
  `GlyphSkin` gains a field and may become `#[non_exhaustive]`, so a
  struct literal of it breaks. Check the game at its pin and say in the
  PR what its pin bump needs. No game snapshot changes.
- **Size:** if the PR passes about 600 changed lines without fixtures
  and snapshots, move the example and the guide, with their acceptance
  criteria, to a follow-up ticket, parked with this one (the
  `write-ticket` skill), and say so in the PR.

**Out (do not do):**
- a hex sprite skin: a picture per tile, tints and a cursor in the
  shape of a hexagon, and flat-top maps. They need 2336's polygons or a
  mask picture; a follow-up when a hex game asks;
- hex autotiling and corner layers (`corners.rs` stays square), and
  isometric hex;
- an arrowhead at an angle;
- a template part and a wizard answer (2501, 2502): a follow-up;
- Tiled and LDtk hex maps (1124) and a tile-layer painter for hex (see
  2360's *Out*);
- the maths, the searches and the ADR (2121);
- eight-way steering and diagonal steps on square maps (2117's
  follow-up).

## Implementation steps

1. Read 2121's ADR and `shuyi_core::hex`, 2013's ADR,
   `crates/ui/src/grid/layout.rs`, `glyph.rs`, `glyph/style.rs`,
   `glyph/cursor.rs`, `path.rs` and `threat.rs`, and 2405's picking if
   it is done.
2. Write `HexProjection`, `place_at`, `Grid::hex`, `bounds`,
   `at_art_size` and the picking, with unit and property tests; mark
   `Projection` `#[non_exhaustive]` if it isn't.
3. Add `GlyphSkin::with_hex_rows` and `Layout::with_hex_rows`, move
   `draw_tiles` onto `tile_to_cell`, and lay the tiles, the cursor and
   the path on hex maps, with snapshots. Check every square snapshot is
   unchanged.
4. Do the either-order items (2405, 1126, 1120, 2336, 2117's
   follow-up).
5. Write `examples/hello-hex` with its tests and pictures, and add it
   to CI's `wasm` job, the `run-gates` skill and Pages.
6. Write the guide, do the 2003 and 2004 items, and run the gates.

## Acceptance criteria

- [ ] For every layout and both kinds of `step`, each drawn tile's
      middle pixel picks that tile, and every pixel picks a drawn tile
      whose rectangle holds it, or `None` when none does (property
      test).
- [ ] A camera on an odd row and one on an even row draw each tile at
      the same place relative to its neighbours (property test).
- [ ] On a hex map, no two tiles of the glyph skin share a cell, and the
      tiles whose cells share an edge with a tile's cells are exactly
      its drawn neighbours (property test over small maps).
- [ ] A path steered across rows with the four directional actions is
      drawn from middle to middle, and its snapshot is reviewed.
- [ ] Every existing snapshot passes unchanged (the glyph and sprite
      skins, the toy, Hello, Grid).
- [ ] `Projection` and `GlyphSkin` follow 2013's ADR (both
      `#[non_exhaustive]`), and the PR says what the game's pin bump
      needs.
- [ ] The example's playthrough, bot, replay and `frame-png` pictures
      pass in CI, and CI's `wasm` job builds its web build and runs its
      `check --target all`.
- [ ] `docs/guides/hex-grid.md` exists, and 2004's guide check passes
      if 2004 is done.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `place_at` for each layout, on even and odd origins and on an
  origin at `i32::MAX`; `bounds`; `at_art_size` at a zoom of 2;
  picking at a tile's middle, at a shared edge, in the half tile at a
  row's end and outside the map; `tile_to_cell` on shifted and
  unshifted rows, and `cell_of(&layout.grid(), tile) ==
  tile_to_cell(tile, &layout)` on a hex map.
- Property: the picking, camera and shared-edge criteria above.
- Snapshot / integration: a hex map in the glyph skin with a range, the
  cursor in both looks, a path and a threat arrow; the example's
  Harness playthrough, bot, replay and pictures.

## Completion notes

*(Filled in by the session that completes the ticket.)*
