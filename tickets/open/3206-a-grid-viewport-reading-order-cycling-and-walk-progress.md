---
id: "3206"
title: "A grid Viewport (cursor, camera, view and map size kept together), reading-order cycling over tiles, and walk progress beside walk::steps"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3206 — A grid viewport, reading-order cycling and walk progress

## Context

A request from the game.

1. **The game ticket:** game#1217, which moves its battle screen, AI
   phase, flight and cinematic walks onto these once this lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1217-the-battles-cursor-camera-and-walks-on-the-kits-viewport.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** move a cursor over a grid map bigger
   than the screen with the camera keeping up, jump it to the next of a
   set of tiles, and know where a piece walking a path is.
3. **The friction numbers:** no friction review; the game/engine
   boundary audit of 2026-11-03. The game wires `grid::Cursor` and
   `grid::Camera` together by hand (about 30 lines) and re-derives the
   tile reached, arrival and tiles entered from `walk::steps` at five
   sites (about 20). Here, `toy/grid.rs` pairs `Cursor::step` with
   `Camera::follow` (lines 607 to 613), tests `steps(..) + 1 >= len`
   (666) and takes `path.get(steps(..))` (555); Hello, Grid
   (`examples/hello-grid/src/screens/stage.rs`) does the last two (124,
   299 to 300). Both match `UiAction` onto `Dir` by hand (toy 678 to
   681, Hello, Grid 308 to 311).
4. **The generic shape** (grid kit, `shuyi_ui::grid`): a `Viewport`
   holding a `Cursor`, a `Camera`, the view's size in tiles and the
   map's, whose moves keep the camera following; `cycle`, the next or
   previous of a set of tiles in reading order (row by row), wrapping;
   and three free functions of walk progress beside `steps` and `gait`.
5. **Examples that aren't tactics:** a picross or crossword bigger than
   the screen (the cursor steps, the camera keeps up, "next empty cell"
   cycles in reading order); a roguelike's look mode cycling through
   what is in sight; a board game's token walking its squares, with a
   step sound per square entered.

## Nick input

None.

## Scope

**In:**
- `crates/ui/src/grid/viewport.rs` (`pub mod viewport;`, `pub use
  viewport::{Viewport, cycle};` in `grid.rs`):
  - `Viewport { pub cursor: Cursor, pub camera: Camera, .. }`, with the
    view `(i32, i32)`, the map `(u16, u16)` and a margin
    (`Camera::MARGIN` by default) private. `#[non_exhaustive]`
    (ADR-0039) with `Viewport::new(at, map, view)` (camera
    `centred_on(at)`) and `with_margin`.
  - `step(dir) -> bool` (`Cursor::step`, then `follow` if it moved);
    `jump(pos)` (`Cursor::jump`, then follow); `follow(pos)` (the
    camera alone); `followed(pos) -> Camera` (where `follow` would put
    it, nothing moved); `centred(pos) -> Camera`; `refit(view) -> bool`
    (a new view size: the camera centres on the cursor; false and
    nothing changes for the same size); `view()`, `map()`;
    `tick(dt)` (the cursor's pulse).
  - `cycle(tiles: &[Pos], from: Pos, forward: bool) -> Option<Pos>`:
    the first tile after `from` in `(y, x)` order, or the last before
    it, wrapping; `None` for no tiles. The input needn't be sorted.
- `crates/ui/src/grid/walk.rs`, beside `steps` and `gait`:
  `tile_at(path, tiles) -> Option<Pos>` (`path.get(steps(..))`),
  `arrived(len, tiles) -> bool` (`steps(len, tiles) + 1 >= len`) and
  `entered(len, from_tiles, to_tiles) -> usize` (steps between,
  saturating). Their docs say NaN and negative distances act as
  `steps` has them.
- `UiAction::dir(self) -> Option<Dir>`
  (`crates/assets/src/keymap/action_set.rs`) if no other ticket added
  it (below).
- The toy's `Field` holds a `Viewport` and uses `tile_at`, `arrived`
  and `UiAction::dir`. Hello, Grid uses `tile_at`, `arrived` and
  `UiAction::dir`; its camera stays centred on its hero (its own look).
- `skills/shuyi-stage-grid/SKILL.md`: a line on `Viewport`, `cycle` and
  the walk helpers beside its `Camera::centred_on` line; `check-skills`
  passes.
- **What changes for the game:** the API grows; no break, no
  `Breaking:` line. The game's PR that moves the pin follows (game#1217).

**Either order with 2354:** if 2354 is done, `UiAction::dir` exists:
use it and add nothing; if not, add it here as 2354's Scope describes
(`Some` for the four directions, `None` for the rest), and 2354 uses it.

**Either order with 2355:** if 2355 is done, `UiAction::dir` exists:
use it and add nothing; if not, add it here, and 2355 uses it.

**Out (do not do):**
- Camera feel: smoothing, look-ahead, rooms, blends (2362, parked).
- A walk value type that keeps time: each caller keeps its own clock.
- Unit facing on `MapUnit`, or frames as clips (2359, parked).
- The tutorial's boards (`site/tutorial/*/board.rs`), which centre on
  the player as Hello, Grid does.
- `ListCursor` in two dimensions (2354): a widget's cursor, not a map's.

## Implementation steps

1. Read `grid/cursor.rs`, `grid/camera.rs`, `grid/walk.rs`, `toy/grid.rs`
   (`Field`) and Hello, Grid's `screens/stage.rs`. Check whether 2354
   or 2355 is done.
2. Write `viewport.rs` and its tests in `grid/viewport/tests.rs`.
3. Add the three walk helpers and their tests.
4. Add `UiAction::dir` with a unit test, unless 2354 or 2355 did.
5. Move the toy and Hello, Grid; their Harness tests and snapshots pass
   unchanged.
6. The skill's line; run the gates, `thumbv7em-none-eabihf` included.
   Expect about 120 production lines added and 15 removed, about 300
   changed in all.

## Acceptance criteria

- [ ] `Viewport::step` at the map's edge returns false and moves neither
      cursor nor camera; a step past the margin scrolls the camera as
      `Camera::follow` does (unit tests).
- [ ] `refit` with a new size centres on the cursor and returns true;
      with the same size returns false and changes nothing.
- [ ] `cycle` over unsorted tiles goes forward and back in `(y, x)`
      order and wraps at both ends; `None` for no tiles (unit tests).
- [ ] Property: for random paths and distances, `tile_at` is
      `path.get(steps(..))`, `arrived` is `steps + 1 >= len`, and
      `entered(len, a, b)` is `steps(b) - steps(a)` for `a <= b`, and 0
      for `a > b`.
- [ ] The toy's `Field` holds a `Viewport`; neither the toy nor Hello,
      Grid calls `walk::steps` or matches `UiAction` onto `Dir` itself.
- [ ] Every existing test and snapshot passes unchanged.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Viewport` moves, `refit`, `followed` (nothing moves), `cycle`;
  `UiAction::dir` if added here.
- Property: the walk helpers against `steps`.
- Snapshot / integration: the toy's and Hello, Grid's existing ones.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
