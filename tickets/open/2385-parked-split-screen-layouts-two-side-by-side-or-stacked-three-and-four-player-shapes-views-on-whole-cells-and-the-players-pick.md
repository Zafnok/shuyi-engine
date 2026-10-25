---
id: "2385"
title: "Parked: Split-screen layouts: two side by side or stacked, the three- and four-player shapes, views on whole cells, dividers, and the player's pick"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1120", "2314", "2351"]
nick_input: none
completed:
---

# 2385 — Split-screen layouts for two, three and four players

> **Parked** (Nick, 2026-10-24: "These will be parked since they're not needed for Visions of Shuyi"): local split screen waits until Nick unparks it. Visions of Shuyi is turn-based and plays hotseat on one shared view, so nothing here is for the game. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-24 ([`docs/decisions.md`](../../docs/decisions.md)):
"add parked tickets for adding local split screen multiplayer, both
vertical and horizontal, and lets say even 3 or 4 player split screen".

What is planned already:

- **1120** gives each world layer an optional `viewport: Option<PxRect>`
  in console pixels, a camera measured against it, and
  `split_viewports(frame, n)`: for n = 1 to 4, the frame; two side by
  side; one wide on top and two below; two by two. One fixed shape per
  player count, with no choice.
- **2406** gives each local player a seat and their own devices.
- The grid kit's `GridSkin::paint` draws a `MapScene` into any
  `area: Rect` of cells (`crates/ui/src/grid/skin.rs`), so grid games
  can already draw two maps.

What is missing is the choice the benchmarks give. Two players split
**vertically** (side by side, a divider running top to bottom) or
**horizontally** (stacked, a divider running left to right). Racing
games stack them, since a wide view sees further down the road. Top-down
games often put them side by side. Three players have several shapes:
one wide view and two below, two above and one wide below, one tall view
beside two, or two by two with the fourth quarter left spare (blank,
or a map of the whole level). Four players use two by two, or
four strips. Which one is best depends on the game and on the screen,
and many games let the player pick in the options.

This ticket is the layouts as pure maths, with the views' rectangles in
console pixels and in cells, the dividers between them, and the
player's pick saved per profile. Which seat gets which view, cameras
that follow, and the check per target are 2386's.

Kinds of game that use it: couch racing, top-down arena and co-op
shooters, a two-player platformer, a versus puzzle game showing two
boards (cells only, through the grid kit), and a strategy game with two
local players each scrolling their own map.

## Nick input

None. Unparking is Nick's call, as for every parked ticket.

## Scope

**In:**

- **`shuyi_ui::frame::split`** (`crates/ui/src/frame/split.rs`), pure
  and `no_std` as the rest of `shuyi-ui`. New public types follow
  2013's ADR: `SplitLayout`, `SplitSpec`, `SplitError` and `Split` may
  grow, so they are `#[non_exhaustive]`. Only `split` builds a `Split`,
  and its parts are read through methods.
  - `SplitLayout`, plain data (`Debug`, `Clone`, `Copy`, `PartialEq`,
    `Eq`, `Hash`, serde):
    - `Columns`: n views side by side, vertical dividers ("vertical
      split"), for 1 to 4;
    - `Rows`: n views stacked, horizontal dividers ("horizontal
      split"), for 1 to 4;
    - `OneOverRest`: one wide view on top, the rest in columns below,
      for 3;
    - `RestOverOne`: the rest in columns on top, one wide view below,
      for 3;
    - `OneBesideRest`: one tall view on the left, the rest stacked on
      the right, for 3;
    - `Grid`: two by two, for 3 and 4. With 3 players the bottom-right
      quarter is the spare.
  - `SplitLayout::fits(self, n: u8) -> bool`, and
    `SplitLayout::for_players(n) -> &'static [SplitLayout]`, every
    layout that fits n, in the order above.
  - `SplitLayout::default_for(n) -> SplitLayout`: `Columns` for 1 and
    2, `OneOverRest` for 3, `Grid` for 4. These are 1120's shapes, so
    `split_viewports(frame, n)` becomes
    `split(frame, SplitLayout::default_for(n), n, &SplitSpec::default())`'s
    views, and every snapshot of 1120 stays as it is.
  - `SplitSpec { divider: u16, align: Align }` (`#[non_exhaustive]`,
    `SplitSpec::new()`, builder setters): the divider's thickness in
    console pixels (0, the default, is none), and `Align::Pixel` (the
    default) or `Align::Cells { w: u16, h: u16 }`, which puts every
    view's edge on a cell boundary of that size, so each view holds a
    whole number of cells for its HUD.
  - `split(frame: PxRect, layout, n: u8, spec: &SplitSpec) ->
    Result<Split, SplitError>`:
    - `Split::views() -> &[PxRect]`, in reading order (left to right,
      then top to bottom); view `i` is player `i`'s place in the layout
      (2386 maps seats to it);
    - `Split::spare() -> Option<PxRect>`: the empty quarter of `Grid`
      with 3 players;
    - `Split::dividers() -> &[PxRect]`: one rectangle per line between
      two views (or a view and the spare). A divider's crossing with
      another is in exactly one of them;
    - views, spare and dividers are whole pixels, never overlap, and
      cover the frame exactly. Space that can't be shared evenly goes
      to the first column or row, as 1120 rules. Under
      `Align::Cells`, the views' edges and the dividers sit on cell
      boundaries: a divider is one cell thick at least when `divider`
      is above 0, and a frame that isn't a whole number of cells gives
      its remainder to the last column and row;
    - `SplitError`: a layout that doesn't fit n, n of 0 or above 4, and
      a frame too small to give every view at least one pixel (one cell
      under `Align::Cells`) after the dividers.
  - `split_cells(cols: u16, rows: u16, layout, n, divider_cells: u16)
    -> Result<CellSplit, SplitError>`: the same layouts in whole cells,
    as `shuyi_ui::grid::Rect`s, for screens drawn only in cells (the
    grid kit's `GridSkin::paint` into each `area`). It is `split` with
    `Align::Cells { w: 1, h: 1 }` over a frame of `cols` × `rows`, so the
    two never disagree (a test says so).
- **The player's pick**, per profile:
  - `SplitPrefs` (`#[non_exhaustive]`, `SplitPrefs::new()`), the
    game's side, set once with `Ctx::set_split_prefs`: the layouts the
    game allows (every layout by default) and its default for each
    player count (`SplitLayout::default_for` until set). A default the
    game doesn't allow is a `SplitError` from the setter;
  - the player's pick for 2, 3 and 4 players is a setting of the
    current profile, saved with the other settings (`crates/ui/src/ctx.rs`;
    an older save without it reads as no pick);
  - `Ctx::split_layout(n) -> SplitLayout`: the player's pick when the
    game allows it, else the game's default;
  - option rows (2314) for the three counts, `split_layout_row(n)`,
    listing only the layouts that fit n and the game allows, so a game
    puts "Two players: side by side / stacked" in its options screen
    with one line. A count with one allowed layout gives no row;
  - names through text keys, with their English in the engine's
    `crates/ui/lang/en/ui.ron` (2351): `split_layout.columns` ("Side by
    side"), `split_layout.rows` ("Stacked"),
    `split_layout.one_over_rest` ("One on top"),
    `split_layout.rest_over_one` ("One below"),
    `split_layout.one_beside_rest` ("One on the left") and
    `split_layout.grid` ("Four squares"), and the rows' labels
    `split_layout.players_2`, `_3` and `_4` ("Two players", …).
    `check-keys` passes.
- **Snapshots**: none change. A `Split` has a `Debug` that tests read.
- **Docs**: the frame's page in `docs/` (the `document` skill says
  which) gets a section with a drawing of each layout in text, and the
  `shuyi-stage-field` skill (`skills/shuyi-stage-field/SKILL.md`) a
  line on `split` and the player's pick. `check-skills` passes.

**Out (do not do):**

- which seat gets which view, cameras per view, a view's HUD, the
  spare's contents, drawing the dividers, reflowing when a player joins
  or leaves, and the check that views fit a target (2386);
- a listener per view (2387);
- more than four views, and views of different sizes the game sets by
  hand (a game places any viewport it likes through 1120);
- splits that move with the players (Voronoi) and a shared view that
  zooms out (1120's Out already rules them out);
- a split across two physical screens (3023: a surface is a screen, a
  viewport a rectangle inside one);
- an animated change from one layout to another.

## Implementation steps

1. Read 1120's frame types as they landed (`PxRect`, `WorldLayer`,
   `split_viewports`), 2013's ADR, 2314's option rows and 2351's
   built-in English.
2. Write `split.rs`: the enum, `fits`, `for_players`, `default_for`,
   `SplitSpec`, `split` and `split_cells`. Make `split_viewports` call
   `split`.
3. Add `SplitPrefs`, the profile setting, `Ctx::split_layout` and
   `split_layout_row`, with the text keys.
4. Docs and the skill line.
5. Run the gates.

## Acceptance criteria

- [ ] `split_viewports(frame, n)` gives the same rectangles as before
      for n = 0 to 5, and no snapshot or `frame-png` picture changes.
- [ ] For every layout, every n it fits, any frame from 16 × 16 to
      1920 × 1080 pixels and any divider from 0 to 8: views, spare and
      dividers don't overlap and cover the frame exactly (property
      test).
- [ ] Under `Align::Cells`, every view's edges are on cell boundaries
      (property test).
- [ ] `Columns` with 2 gives a left and a right view; `Rows` with 2 a
      top and a bottom one; `Grid` with 3 gives three views and a spare
      in the bottom-right quarter (unit tests with exact rectangles on a
      400 × 240 frame, the Playdate's).
- [ ] `split` refuses `OneOverRest` with 2, n of 5 and a frame too small
      for its dividers, one `SplitError` each.
- [ ] `split_cells` equals `split` with one-pixel cells for every
      layout and n.
- [ ] A Harness test: the player picks "Stacked" for two players on a
      test options screen, it survives a restart, and a game that
      allows only `Columns` shows no row for two and returns `Columns`
      from `Ctx::split_layout(2)`.
- [ ] `check-keys` and `check-skills` pass.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each layout's rectangles on a 400 × 240 frame with and without
  dividers and cell alignment; `fits`, `for_players` and `default_for`;
  each `SplitError`; `SplitPrefs`' setter refusing a default it doesn't
  allow; the old save reading as no pick.
- Property: coverage with no overlap, cell alignment, and `split_cells`
  agreeing with `split`.
- Snapshot / integration: the options screen's Harness test above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
