---
id: "2354"
title: "A grid cursor: ListCursor in two dimensions, for rows of different lengths, cells that can't take the focus and a scroll window; the name grid and the key bindings screen move onto it"
type: feature
milestone: After MVP
model: implementation
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2354 — A grid cursor

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8.

**What the benchmarks have.** In each, a grid of buttons gets d-pad
moves with no code per screen: Godot finds the nearest control in a
direction when no `focus_neighbor_*` is set
(https://docs.godotengine.org/en/stable/classes/class_control.html),
Unity UI's `Selectable` navigates automatically over a
`GridLayoutGroup`, Bevy 0.18 added automatic directional navigation,
and Unreal's Common UI moves a pad's focus over a grid of widgets. 2355
brings that between the widgets of a screen. This ticket is the smaller
half: moves inside one widget that is a grid.

**What Shuyi has today.**
- `ListCursor` (2307, `crates/ui/src/widgets/list.rs`): the focus along
  one line, with `step(forward, wrap, enabled)`, `settle`, `set_len`,
  and a window over the rows (`window`, `window_by_heights`,
  `first_shown`) placed by a `Scroll` rule. `Menu` keeps its focus in
  one.
- Two dimensions are written by hand, twice:
  - `NameEntry` (2310, `crates/ui/src/widgets/name_entry.rs`) keeps
    `at: (usize, usize)` (line 220) and moves it in `handle` (lines 254
    to 284): Left and Right wrap round the row; Up and Down wrap round
    the rows and take `col.min(width - 1)` on a shorter row, so from
    the 13th letter down to the last row (5 cells) and back up, the
    focus stays on the 5th.
  - `KeyBindingsScreen` (`crates/ui/src/screens/key_bindings.rs`) keeps
    `row` and `slot` (lines 236 and 238). `move_row` (lines 501 to 510)
    wraps through the actions, Restore defaults and the keyboard /
    controller switch (`restore_row` and `switch_row`, lines 390 to
    398). `move_across` (lines 515 to 538) moves the slot, stops at the
    ends, and keeps the slot while the focus is on a row without slots,
    so the way back finds it.
- Nothing shared turns a `UiAction` into a `shuyi_core::Dir`
  (`crates/core/src/grid.rs`, line 52): the toy's grid screen matches
  them by hand (`crates/ui/src/toy/grid.rs`, lines 686 to 689).

**Why it matters.** The engine's menus are played with a pad, and on
the Playdate with a d-pad alone (ADR-0008 §3); its name grid and its key
bindings table are grids today. Most 2D games have more: an inventory,
a level select of 5 by 4, a shop's shelf, a crafting table, a skill
board. Each would write the wrap and the clamp again, with the same
bugs on a shorter last row. One cursor tested once keeps them alike,
and an agent writing a new screen (the end goal of 2026-10-19) reaches
for it instead of the modulo.

## Nick input

None.

## Scope

**In:**
- **`GridCursor`** in `crates/ui/src/widgets/grid_cursor.rs`,
  re-exported from `widgets` with `GoalColumn`:
  - `GridCursor::new(widths: Vec<usize>)`: one width per row. A row of
    width 0 can't take the focus.
  - `at(row, col)`, a builder clamped as `ListCursor::at` is: the row to
    the last row, the column to that row's last cell (0 on a row of
    width 0). `set_focus(row, col)` does the same in place, for a page
    turn. `focus() -> (usize, usize)` (row, column; `(0, 0)` with no
    cells, as `ListCursor::focus` gives 0), `widths()`, and
    `set_widths(widths)`, which clamps the focus as `set_len` does.
  - `step` and `settle` below call their `enabled` predicate only on a
    cell that exists (`col < widths[row]`), so a caller can index its
    rows without a check.
  - `step(dir: Dir, wrap: bool, enabled: impl Fn(usize, usize) -> bool)
    -> bool`, which says whether the focus moved, so the caller plays
    its move sound. The caller picks `wrap` per call, so a screen can
    wrap one axis and not the other:
    - Left and Right: the next cell of the row that `enabled` accepts,
      round the row's ends if `wrap`, else stopping at them
      (`ListCursor::step`'s rule along the row);
    - Up and Down: the next row with a cell `enabled` accepts, round the
      ends if `wrap` (`ListCursor::step`'s rule over the rows). On that
      row the column is the goal column, or the row's last cell if the
      row is shorter. If that cell is disabled, the nearest enabled cell
      of the row, the left one on a tie.
    - A step that doesn't move changes nothing, the goal column
      included.
  - **The goal column**, set with `.goal_column(GoalColumn::Keep)`
    (`GoalColumn` derives `Debug, Clone, Copy, Default, PartialEq, Eq`):
    - `GoalColumn::Clamp`, the default: the column a move up or down
      lands on becomes the goal (`NameEntry` today);
    - `GoalColumn::Keep`: only Left and Right set the goal, and Up and
      Down go back to it on a row long enough (the key bindings
      screen's slot today, and a text editor's column).
  - `settle(enabled) -> bool`: off a disabled cell (or a row of width 0)
    to the nearest enabled one in its row, else by
    `ListCursor::settle`'s rule over the rows, landing on that row's
    column as a move up or down does.
  - The window over the rows: `scrolling(Scroll)`, `window(rows_shown)`,
    `window_by_heights(heights, room)` and `first_shown()`, by 2307's
    rules. The simple way: a `ListCursor` over the rows inside the
    `GridCursor`, which the moves up and down use too.
  - `no_std`, integers only, nothing allocated per step. It derives
    `Debug, Clone, Default, PartialEq, Eq` as `ListCursor` does, so
    `NameEntry` keeps its derives.
- **`UiAction::dir(self) -> Option<Dir>`** in
  `crates/assets/src/keymap/action_set.rs` (`shuyi-assets` already
  depends on `shuyi-core`): `Some` for the four directions, `None` for
  Confirm, Cancel and Debug. **Either order with 2355:** whichever
  lands first adds it, and the other uses it. The toy's grid screen
  uses it instead of its own match.
- **`NameEntry` moves onto it:** the `at` field becomes a `GridCursor`
  built from the widths of `grid(rules)`'s rows, `GoalColumn::Clamp`,
  every cell enabled, `wrap` on both ways. `NameEntry::focus()` and
  `GridView::focus` keep their `(usize, usize)`.
- **`KeyBindingsScreen` moves onto it:** `row` and `slot` become one
  `GridCursor`: a row per action, `SLOTS` wide, then Restore defaults
  and the switch, one cell each (the switch only with a keyboard), in
  the index order `restore_row` and `switch_row` give today, so up from
  the first action still lands on the switch. `GoalColumn::Keep`;
  `wrap` on for Up and Down, off for Left and Right. On the switch,
  Left and Right still pick a side before the cursor sees them.
  `focus`, `is_on_switch`, `view` and `row_view` read the cursor's
  focus, and the hand-written `Debug` and `Clone` impls show and copy
  the cursor in place of `row` and `slot`.
- **Either order with 2346** (names in any language): if 2346 is done,
  `NameEntry` has pages, rows with cells left out (so of different
  widths) and a Page cell. Turning a page calls `set_widths` with the
  new page's widths, then `set_focus` on its Page cell. If 2346 is not
  done, it puts its pages on `GridCursor` (its text says so).
- The `widgets` module docs (`crates/ui/src/widgets.rs`) name the grid
  cursor.

**For the game:** additive. `GridCursor`, `GoalColumn` and
`UiAction::dir` are new, and `NameEntry` and `KeyBindingsScreen` keep
their public API, so no `Breaking:` line (2010). The game's PR that
moves its pin needs no change. `GridCursor` and `GoalColumn` follow
ADR-0039: `GridCursor` has private fields and constructors, as
`ListCursor` has.

**Out (do not do):**
- A drawn grid widget (cells, a box, a skin): this is a cursor, as
  `ListCursor` is, and the screen draws.
- Moves between widgets (2355). A grid at its edge without wrap leaves
  the direction unused, and 2355 hands it to the next widget.
- The mouse and touch on a grid cursor's cells (`NameEntry`, the key
  bindings table): 2409, 2405's follow-up for the engine's other
  widgets, through 2405's hit areas. If 2409 is done first, this
  ticket keeps their pointer working on the grid cursor.
- A window that scrolls columns sideways: the window is over rows.
- A page action or shoulder buttons: `UiAction` has none (the
  `input-actions` skill).
- The tactics grid map's cursor (`crates/ui/src/grid/cursor.rs`): it
  walks a map, not a widget.

## Implementation steps

1. Read `widgets/list.rs` and its tests, `name_entry.rs` and its tests,
   and `key_bindings.rs` (`move_row`, `move_across`, `restore_row`,
   `switch_row`, `focus`, `is_on_switch`, `view`, `row_view`, and the
   `Debug` and `Clone` impls) and its tests. Check whether 2355 and
   2346 are done.
2. Add `UiAction::dir` with a unit test, unless 2355 added it, and use
   it in `toy/grid.rs`.
3. Write `GridCursor` and `GoalColumn` in `widgets/grid_cursor.rs`,
   with its unit and property tests (`proptest`, as `list.rs`'s tests
   use) in `widgets/grid_cursor/tests.rs`, and re-export both from
   `widgets.rs`.
4. Move `NameEntry` onto it, then `KeyBindingsScreen`, running their
   tests after each. A test that fails means the move changed
   behaviour: fix the code, not the test.
5. If 2346 is done, move its page turn onto `set_widths` and
   `set_focus`.
6. Update `widgets.rs`'s docs, and run the gates.

## Acceptance criteria

- [ ] On rows of 13, 13, 13, 13 and 5 cells with wrap, `GridCursor`
      goes Left from (0, 0) to (0, 12), Up to (4, 4) and Right to
      (4, 0), as `NameEntry` does today (unit test).
- [ ] With `GoalColumn::Keep`, a move through a row of one cell and
      back returns to the column chosen before; with `GoalColumn::Clamp`
      it stays on the clamped one (unit tests).
- [ ] A disabled cell is skipped across and down, and a row with no
      enabled cell or of width 0 is skipped (unit tests).
- [ ] Property: over random widths (0 to 8 rows, 0 to 6 cells each),
      random enabled cells and random steps after a `settle`, nothing
      panics, `enabled` is never called on a cell that doesn't exist,
      the focus is on an enabled cell whenever the grid has one, and
      `window(k)` for a random `k` contains the focus's row.
- [ ] Property: on a full grid of `h` rows of `w` cells, `w` steps Left
      or Right, or `h` steps Up or Down, with wrap come back to the
      start.
- [ ] `UiAction::dir` gives `Some` for the four directions and `None`
      for the other three (unit test), and the toy's grid screen moves
      through it.
- [ ] `NameEntry` has no `at` field and `KeyBindingsScreen` no `row` or
      `slot` field: their moves go through `GridCursor::step`.
- [ ] The tests and snapshots of the name entry, the profiles screen and
      the key bindings screen (`crates/ui/src/widgets/name_entry/`,
      `crates/ui/src/screens/profiles/`,
      `crates/ui/src/screens/key_bindings/`,
      `crates/ui/tests/it/key_bindings.rs`) pass with no change to them.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each direction with and without wrap; the two goal-column
  rules; disabled cells and rows; `settle`; `at` and `set_focus`
  clamping; `set_widths` shrinking past the focus; a grid with no
  cells; the row window; `UiAction::dir`.
- Property: the two above.
- Snapshot / integration: the existing name entry, profiles and key
  bindings tests and snapshots, unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
