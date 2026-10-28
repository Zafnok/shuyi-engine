---
id: "2374"
title: "Eight-way on square maps in the grid kit's drawing and input: a diagonal step in the path arrow, steering over diagonal actions, and an eight-way choice in the grid template"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2117"]
nick_input: none
completed:
---

# 2374 — Eight-way in the grid kit's drawing and input

## Context

2117 gives `shuyi_core` eight-way moves: `Dir8`, `Pos::step8`,
`Topology::Square8(Diagonals)` with a corner rule (`Corners`),
`Topology::steps`, `Topology::step_weight`, `reachable_on`,
`flow_field_on` and `find_path`. It leaves the grid kit's drawing and
input four-way, and its *Out* names this ticket for them. What the kit
has after 2117:

- The path arrow (`path_overlays`, `crates/ui/src/grid/path.rs`, the
  look of game ADR-0018) draws `MapScene::path` as straight legs
  between four-way neighbours. `segment` and `arrowhead` match on
  `(1, _)`, `(-1, _)`, `(_, 1)` and the rest. A diagonal step comes out
  as a filled square over the box between the two tiles' middles, and
  its arrowhead points right or left. 2117 changed only the docs of
  `MapScene::path` (`crates/ui/src/grid/scene.rs`) and `path_overlays`,
  to say they draw four-way steps.
- `steer` (`crates/ui/src/grid/steer.rs`) appends a tile only when
  `Pos::manhattan(end, to) == 1`. A diagonal cursor move falls back to
  the cheapest path.
- `Cursor::step` (`crates/ui/src/grid/cursor.rs`) takes a `Dir`.
- The threat arrows (`crates/ui/src/grid/threat.rs`) already draw a
  line at any angle between tile middles, as one-pixel runs from the
  private `line`. `Start::Edge` starts one where it leaves its tile.
- The walk (`crates/ui/src/grid/walk.rs`) already takes any step:
  `gait` glides along both axes, and `facing` turns a unit left or
  right on a step that goes across.
- The grid template (`templates/grid`) steps a piece with
  `Command::Step(Dir)` and four actions.

A diagonal is an action, never a key. The `input-actions` skill's rule
1: a game adds an action to its own `ActionSet`, and the engine adds a
`UiAction` only when every game's widgets need it. A diagonal isn't
that: a Playdate's d-pad has four directions (ADR-0008 §3). So the
diagonal actions are the game's own and optional, and four-way stays
the default.

2740 (parked) draws hex maps. Its path-arrow bullet draws a step
between two hex rows as a line between the tiles' middles, the line
this ticket draws a diagonal step with. 2501 (done) turned
`templates/grid` into the part `stage-top-down-grid` of
`templates/shuyi-default`; the clauses below say what that means
here.

Why it matters:

- **The game first:** Visions of Shuyi moves four ways. Nothing changes
  for it: new items only, and four-way stays the default.
- **Other 2D games:** a tactics or board game that moves a king's step
  steers a route round a corner on the diagonal and sees it drawn. A
  grid game started from the template moves eight ways by changing one
  line.
- **The end goal** (2026-10-19): an agent asked for an eight-way grid
  game finds the choice in the template and needs nothing else.

## Nick input

None.

## Scope

**In:**
- **The path arrow draws a diagonal step**
  (`crates/ui/src/grid/path.rs`):
  - a step whose `dx` and `dy` are both non-zero is a line `LINE_W`
    pixels wide between the two tiles' middles: the pixel `line_mid` of
    each axis, where the straight legs run. Straight steps keep today's
    rectangles;
  - the first step's line starts where it leaves the unit's tile, as
    `threat.rs`'s `Start::Edge` does: no pixel of it falls on the
    unit's tile, as `segment`'s `from_edge` keeps the straight leg off
    it;
  - **Either order with 2740:** if 2740 is done, draw the diagonal step
    with the line 2740 draws between hex rows; if not, draw it as a
    line `LINE_W` pixels wide between the two tiles' middles, made as
    2740's path-arrow bullet says, and 2740 draws its steps between
    rows with it;
  - the arrowhead keeps its four shapes. On a last diagonal step it
    points by `dy` (down or up), so its `match` tests `dy` first, as
    2740 says. Whichever of the two lands first makes that change. It
    changes nothing on a straight step, where one of `dx` and `dy` is
    always 0;
  - the line is clipped to the view and zoomed through `add`, as the
    straight legs are;
  - every existing test of `path.rs` and every four-way snapshot stays
    as it is;
  - the docs of `MapScene::path` and `path_overlays` say a step goes to
    any of the eight neighbours, and a diagonal one is drawn as a line
    between the tiles' middles.
- **Steering in eight directions**, additions only:
  - `Cursor::step8(&mut self, dir: Dir8, map_w: u16, map_h: u16) ->
    bool` (`cursor.rs`): each axis clamped as `step` clamps it, so a
    diagonal against the map's edge slides along it; a move restarts
    the pulse. `step` keeps its signature and its results (it may call
    `step8(Dir8::from(dir), …)`);
  - `steer_on(topology: Topology, path, to, open: impl Fn(Pos) ->
    bool, cheapest, valid) -> Vec<Pos>` (`steer.rs`), re-exported from
    `grid` beside `steer`: `steer`'s three rules, but `to` is appended
    when `topology.step_weight(end, to, &open)` answers `Some` and the
    longer path is valid. `open` is the openness the game's search
    used (a tile is open when its cost function answers `Some`), so an
    appended diagonal never cuts a corner the search wouldn't cut. The
    docs say so;
  - `steer` keeps its signature: it is `steer_on(Topology::Square4,
    path, to, |_| true, cheapest, valid)`, whose results are today's;
  - the diagonal actions are the game's own: four actions of its
    `ActionSet`, optional (`is_required` false: a target with four
    directions has no button for them), repeatable like the straight
    moves, with no `ui()`. A screen declares them only when its map
    moves eight ways, and calls `step8` or sends a `Dir8` for them. The
    grid template below is the worked example. No `UiAction` changes.
- **The walk needs no change.** A unit test of `walk::gait` halfway
  along a diagonal step pins it: an offset of `(0.5, 0.5)`, facing
  right.
- **An eight-way choice in the grid template** (`templates/grid`):
  - `crates/rules/src/moves.rs`, re-exported from the rules crate's
    root: `pub const MOVES: Topology = Topology::Square4;`. Its docs
    give the other choice,
    `Topology::Square8(shuyi_core::Diagonals::KING)` (a king's step: a
    diagonal as dear as a straight step), with `Diagonals` named by its
    path so either value compiles with no unused import. That one line
    is the choice;
  - the rules (`crates/rules/src/lib.rs`): `Stage` keeps a `moves:
    Topology`, `MOVES` in `Stage::new`, with `#[must_use] pub fn
    with_moves(self, moves: Topology) -> Self` and `pub fn moves(&self)
    -> Topology`. `Command::Step(Dir8)` and `Error::Blocked(Dir8)`.
    `check` asks `self.moves.step_weight(hero, hero.step8(dir), |p|
    self.open(p))`. `legal_commands` walks `Dir8::ALL`. The wanderer
    picks from `self.moves.steps(wanderer, |p| self.open(p))`, less the
    goal. With `Square4` each list comes in today's order (`Dir::ALL`'s,
    which `Dir8::ALL` keeps for the straight steps), so the same seed
    plays the same game, and every existing test of the template passes
    unchanged;
  - the game (`crates/game/src/action.rs`, `stage.rs`): `Action` gains
    `UpLeft`, `UpRight`, `DownLeft` and `DownRight`, optional and
    repeatable, with no `ui()`, and `Action::diagonal(self) ->
    Option<Dir8>`. The stage screen's `actions()` lists them only when
    `stage.moves() != Topology::Square4`, and its `update` sends
    `Command::Step` of their `Dir8`. The straight moves send
    `Dir8::Up` and the rest;
  - `crates/game/files/keymap.ron`, in the `Arrows` layout: `"UpLeft":
    ["Kp7", "Home"]`, `"UpRight": ["Kp9", "PageUp"]`, `"DownLeft":
    ["Kp1", "End"]` and `"DownRight": ["Kp3", "PageDown"]`, the
    numpad's corners and the keys beside the arrows. None on the pad,
    whose d-pad has four; a comment says why;
  - `crates/game/files/en.ron`: `stage.help.eight`, the help line of an
    eight-way stage, naming the diagonal keys through `{UpLeft}`,
    `{UpRight}`, `{DownLeft}` and `{DownRight}` beside `{Cursor}`
    (rule 2 of `input-actions`: no key written). It fits the bottom
    line of the 48-cell console with the default keys;
  - the template's `README.md` and `AGENTS.md`: a line each on the
    choice and the file that holds it;
  - **From 2501 (done):** add the choice to the grid part of
    `templates/shuyi-default` as an answer it reads, where `templates/grid`
    is named above (its files are under
    `templates/shuyi-default/stages/top-down-grid`). As an answer: a question of the template's own,
    `moves`, with the answers `four` (the default) and `eight`, and two
    parts, each picked by one answer and the grid stage's part, that
    write `crates/rules/src/moves.rs`. The CI step below then makes its
    game with `--set moves=eight` instead of editing the line;
  - CI (`.github/workflows/ci.yml`, the `templates` job): a step "An
    eight-way grid game passes its gates", `if: matrix.template ==
    'grid'`. It makes a second game in `$RUNNER_TEMP/eight` with
    `shuyi new grid`, sets `MOVES` to the eight-way line with `sed`,
    fails if `grep` doesn't then find that line, and runs the same
    gates as "Run its gates".
  - **Either order with 2033:** if 2033 is done, its test in
    `crates/cli/src/template/tests.rs` reads every `run: |` block of the
    `templates` job with a `cargo shuyi check` line, this step's too:
    write the step's gates as the same lines as "Run its gates", in the
    same order; if not, nothing to do (2033 makes the step's lines match
    when it lands).
- **What changes for the game:** nothing. Two new functions
  (`Cursor::step8`, `steer_on`); `path_overlays` draws a step a
  four-way path never has; the arrowhead's `match` order changes
  nothing on a straight step. The template's changes reach only games
  made after it. Say so in the PR.
- **Public types (ADR-0039):** no new type.

**Out (do not do):**
- hex maps (2121 and 2740 draw them);
- an arrowhead at an angle;
- a flag of its own in `shuyi new` for the choice, and anything in the
  new-game wizard for it: a follow-up if games ask;
- a diagonal from two straight directions held at once (a d-pad
  pressed between two of them): a follow-up if a game asks;
- changing `Dir`, `Cursor::step`, `steer`, `UiAction` or any four-way
  snapshot;
- the toy's field and Hello, Grid: they stay four-way. The toy's
  actions are the sample action set (`shuyi_assets::keymap::sample`)
  that many engine tests share, and Hello, Grid is a four-way example.

## Implementation steps

1. Read 2117's completion notes and `crates/core/src/path/topology.rs`
   (`Dir8`, `Topology::steps`, `step_weight`), then
   `crates/ui/src/grid/path.rs`, `threat.rs`, `steer.rs`, `cursor.rs`
   and `walk.rs`, the `input-actions` skill, and 2740's path-arrow
   bullet (under *The glyph skin on a hex map*, in its file in
   `tickets/open/` or `tickets/done/`).
2. The line. Make it as 2740's bullet says. If `shuyi-ui`'s frame has
   no line shape by then, that is `threat.rs`'s private `line` moved
   into a `pub(crate)` helper both files use (say
   `crates/ui/src/grid/line.rs`), with each run grown by `LINE_HALF`
   pixels on both sides, across the run. `threat.rs`'s tests pass
   unchanged.
3. In `path.rs`, draw a diagonal step with it, on the art-sized grid
   as `segment` is. For the first step, cut the runs to the pixels off
   the unit's tile before growing them: a run is one pixel thick, so
   one rectangle or none is left. Make the arrowhead's `match` test
   `dy` first. Run the existing tests unchanged, then add the new ones.
4. Add `Cursor::step8` and `steer_on`, make `steer` call `steer_on`,
   and add their tests. Add the walk test.
5. Add the snapshots of a diagonal path in the glyph skin
   (`crates/ui/src/grid/glyph.rs`'s tests) and the sprite skin
   (`crates/ui/src/grid/sprite/tests.rs`), and check that every
   existing snapshot passes unchanged.
6. The template: `moves.rs`, the rules, the actions, the keymap, the
   help line, the screen and their tests; then the README and
   `AGENTS.md`. Make a game from it with `shuyi new grid`, switch the
   line by hand, and run its gates before writing the CI step.
7. The docs (the `document` skill): the docs of `MapScene::path`,
   `path_overlays`, `Cursor::step8` and `steer_on`, and the grid kit's
   module docs (`crates/ui/src/grid.rs`, which name `steer()`). Settle
   the either-order clauses and run the gates.

## Acceptance criteria

- [ ] On the glyph skin's 16 px tiles, the path `(1, 1)`, `(2, 2)` draws
      a line through the middle pixel of `(2, 2)` that starts at pixel
      `(32, 32)`, the first one off `(1, 1)`, with no pixel on `(1, 1)`,
      and its arrowhead points down (unit test with exact rectangles).
- [ ] A path mixing straight and diagonal steps passes through each
      tile's middle pixel, and has one arrowhead, by the last step's
      `dy` (unit test).
- [ ] Every existing test and snapshot of `shuyi-ui` passes unchanged:
      `path.rs`, `threat.rs`, the glyph and sprite skins, the toy's
      field; and Hello, Grid's pictures.
- [ ] `Cursor::step8(Dir8::from(d), …)` moves a cursor as `step(d, …)`
      does, on any map and from any tile (property test), and a
      diagonal at the map's edge slides along it (unit test).
- [ ] `steer` and `steer_on(Topology::Square4, …, |_| true, …)` give the
      same path for any cursor moves (property test).
- [ ] With `Topology::Square8(Diagonals::KING)` and a `Reach` from
      `reachable_on`, a path steered by any of the eight moves stays
      legal (each step has a `step_weight`, every tile is reachable, the
      cost is within the budget) and ends on the cursor whenever the
      cursor is on a reachable tile (property test).
- [ ] The grid template with `MOVES` four-way: every test of its rules
      and its game passes unchanged, and its screen doesn't declare the
      diagonal actions.
- [ ] The template's rules with
      `with_moves(Topology::Square8(Diagonals::KING))`: a diagonal step
      moves the piece, one past a wall's corner is refused, and
      `legal_commands` lists the open diagonals (unit tests).
- [ ] CI's `templates` job makes an eight-way grid game and its gates
      pass.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the diagonal step's rectangles and its first step off the
  unit's tile, on square tiles and on 24 × 20 tiles; the arrowhead on
  each of the four diagonals; a diagonal clipped at the view's edge and
  at a zoom of 2; `step8` at each edge and corner; `steer_on` appending
  a diagonal, refusing one that cuts a closed corner, and cutting back;
  `walk::gait` on a diagonal step; the template's rules on both
  topologies.
- Property: `step8` against `step`; `steer_on` against `steer` on
  `Square4`; steered eight-way paths stay legal and follow the cursor,
  over `steer.rs`'s `field()` map searched with `reachable_on`.
- Snapshot: one diagonal path in the glyph skin and one in the sprite
  skin, as an insta text snapshot of the overlays each draws (one line
  per rectangle: layer, x, y, w, h).
- Integration: a Harness test of the template's game. On an eight-way
  stage, a diagonal action's default key steps the piece diagonally and
  the screen keeps `Command::Step` of that `Dir8`; on a four-way stage
  the screen doesn't declare it (`Harness::undeclared` lists the press)
  and the piece stays. It reads `rules::MOVES` to know which to expect,
  so it passes with either choice: CI's `templates` job runs it both
  ways. Its test that plays a stage to its end presses the key of each
  legal step, diagonals included.

## Completion notes

*(Filled in by the session that completes the ticket.)*
