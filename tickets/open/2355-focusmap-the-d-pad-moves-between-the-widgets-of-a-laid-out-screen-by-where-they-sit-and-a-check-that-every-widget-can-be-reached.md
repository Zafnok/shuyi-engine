---
id: "2355"
title: "FocusMap: the d-pad moves between the widgets of a laid-out screen by where they sit, with neighbours set by hand, and a check that every widget can be reached"
type: feature
milestone: After MVP
model: reasoning
effort: high
status: todo
blocked_by: ["2329"]
nick_input: none
completed:
---

# 2355 — Focus between laid-out widgets

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8.

**What the benchmarks have.**
- Bevy 0.18 added automatic directional navigation: it works out which
  node is up, down, left and right of each from where they sit on
  screen, and links set by hand win
  (https://bevy.org/news/bevy-0-18/).
- Godot's `Control` has `focus_neighbor_left`, `_top`, `_right` and
  `_bottom`; with none set, it picks the nearest control that way.
- Unity UI's `Selectable` has a Navigation mode: Automatic by default,
  or Explicit with one selectable per direction.
- Unreal's Common UI moves a pad's focus between widgets and gives it
  back when a panel closes.

None of them checks that every widget can be reached.

**What Shuyi has today.**
- Focus inside one widget only: `ListCursor` (2307) under `Menu`
  (`crates/ui/src/widgets/menu.rs`), the question widget's answers
  (2309), and 2354's `GridCursor` (planned).
- `Menu::step` always wraps (`self.cursor.step(down, true, …)`, line
  310), so a menu never gives Up or Down away. `Menu::handle` reads only
  Up, Down, Confirm and Cancel (lines 260 to 270), so Left and Right
  leave its focus where it is.
- Each engine screen has one focused widget. Where a screen has more, it
  hand-codes the moves: the key bindings screen's switch row above its
  table (`move_row` in `crates/ui/src/screens/key_bindings.rs`). The
  options screen keeps Left and Right for its values
  (`crates/ui/src/screens/options.rs`, lines 898 and 899).
- `shuyi_core::Dir` (`crates/core/src/grid.rs`, line 52): the four
  directions, with `Dir::ALL` in a fixed order.
- A covered screen keeps its state on `ScreenStack`
  (`crates/ui/src/screen.rs`), focus included: Hello, Grid's controls
  test says so (`examples/hello-grid/tests/it/controls.rs`, line 49).
  Giving the focus back after an overlay needs nothing new.
- **Planned:** 2329 solves a tree of `Node`s into a `Layout`, a map from
  each `NodeKey` to its rectangle, in whole cells or pixels, `no_std`;
  it says nothing about focus. 2330 builds a `DocScreen` from a
  `.ui.ron` that "hands focus between menus with the UI actions", with
  no rule for which direction goes where.

**Why it matters.** On a pad, the Steam Deck and the Playdate there is
no pointer to fall back on, and ADR-0008 §3 wants the default controls
to reach all content. Once 2329 and 2330 put widgets side by side (a
shop's list and its buy panel, an inventory beside a
character's stats, tabs over a page), each direction must pick the next
widget, and without this each screen hand-codes it. A list beside a
panel is common in 2D games, Visions of Shuyi's included. For an
agent writing `.ui.ron` screens (the end goal of 2026-10-19), moves
worked out from the layout make a screen work on every device, and the
check catches a screen a pad player can't finish.

## Nick input

None.

## Scope

**In:**
1. **`shuyi_ui::focus`** (`crates/ui/src/focus.rs`), `no_std`, integers
   only:
   - **`FocusNode`**: a node that takes the focus. It holds its
     `NodeKey`, the directions its widget **keeps** (uses itself and
     never gives away: Up and Down for a menu that wraps, Left and
     Right for a row of option values), and at most one neighbour per
     direction set by hand. Built with `FocusNode::new(key)`,
     `.keeps(&[Dir])` and `.neighbour(Dir, key)`.
   - **`FocusMap::new(layout: &Layout, nodes: Vec<FocusNode>) ->
     Result<FocusMap, FocusError>`.** The order of `nodes` breaks ties
     (a screen gives them in tree order). Each `FocusError` names the
     keys:
     - a key the layout doesn't hold;
     - a key listed twice;
     - a neighbour that isn't one of `nodes`, or is the node itself;
     - two nodes whose rectangles overlap (such as a panel and a menu
       inside it: only the menu takes the focus).
   - `.wrapping()`: a move past the last node of a line comes round to
     the first.
   - `FocusMap::first() -> Option<NodeKey>`: the first node with a
     non-empty rectangle.
   - **`FocusMap::next(from, dir) -> Option<NodeKey>`**, by the first
     rule that gives one:
     1. the neighbour set by hand for `dir`;
     2. of the nodes with a non-empty rectangle wholly beyond `from`'s
        edge in `dir` (for Right: a left edge at or past `from`'s right
        edge), the best by, in order: overlapping `from` across `dir`
        (aligned ones first); the smallest distance (the gap along `dir`
        plus the gap across it, which is 0 when they overlap); the
        largest overlap across; the earliest in `nodes`;
     3. with `.wrapping()`, of the nodes aligned with `from` across
        `dir`, the one furthest the other way (for Right: the smallest
        left edge), the earliest on a tie, never `from` itself;
     4. `None`.

     The sums are in `i64`, as `Rect::contains` does
     (`crates/ui/src/frame/glyph_buffer.rs`), so no edge overflows.
   - **`FocusMap::unreachable() -> Vec<NodeKey>`**: the nodes that no
     sequence of directions reaches from `first()`, in `nodes` order,
     following from each node only the directions it doesn't keep. A
     node with an empty rectangle (cut off by 2329's overflow) is never
     reached, so it is listed.
   - **`Focus`**: which widget of a screen has the focus, over a
     `FocusMap`:
     - `Focus::new(map)` on `first()`; `at() -> Option<NodeKey>`;
       `map() -> &FocusMap`; `set(key) -> bool` (false for a key that
       isn't in the map or has an empty rectangle);
     - `pass(dir) -> bool`: the screen calls it when the focused widget
       left `dir` unused. It moves to `next` and says whether it moved,
       so the screen plays `MenuSound::Move`. A direction the focused
       node keeps never moves it (a wrapping menu of one item leaves
       Down unused, and the focus still stays), so play and
       `unreachable()` agree;
     - **the way back:** after a move from A to B by `dir`, the opposite
       direction from B goes back to A when A is beyond B that way,
       before rule 2. So Down from the second tab to the page, then Up,
       comes back to the second tab, not the first. A neighbour set by
       hand still wins. Any other move, and `set`, forget it;
     - `rebuild(map)`: after a new layout (a resize, a reload), the
       focus stays on the same key if the new map has it, else goes to
       `first()`, and the way back is forgotten.
   - The module docs give the rule every screen follows: **the focused
     widget gets each direction first, and only a direction it leaves
     unused goes to `Focus::pass`.** For a `Menu`, unused means its
     `focus()` is the same after `handle` as before. The screen sets
     `MenuView::active` (item 3) from `Focus::at()`. A doc test shows the
     `update` and the views of a screen with two menus side by side.
2. **`Menu::without_wrap()`** (`crates/ui/src/widgets/menu.rs`): Up on
   the first stop and Down on the last do nothing, so a menu above or
   below another widget can give them away. Menus still wrap by default,
   so no snapshot changes.
3. **How a menu without the focus looks:** `MenuView`
   (`crates/ui/src/widgets/menu/view.rs`) gains `active: bool`, true
   from `Menu::view` and `MenuView::new`. A screen sets it false on the
   menus without the focus. The glyph skin (`menu/glyph.rs`, `paint`)
   paints an inactive menu's focused item as an ordinary row, with no
   bar and no note, and `menu::glyph::size` doesn't change with it, so
   nothing moves when the focus does. **Either order with 2334:** if its
   `menu::text` look is done, it paints an inactive menu the same way,
   and its `size` doesn't change with it either; if not, 2334 does it.
   **Either order with 2371:** if 2371 is done, `menu::glyph::paint`
   takes a `WidgetLook`: an inactive menu leaves out the look's focus
   bar and note in `WidgetLook::GLYPH` and `WidgetLook::ONE_BIT` alike,
   with a snapshot of each; if not, nothing to do (2371 does it for its
   one-bit look, its clause says so).
4. **Asking a screen**, for tests and later tools:
   `Screen::widget_focus(&self) -> Option<&Focus>`, defaulted to `None`
   (a defaulted method breaks no game); `ScreenStack::widget_focus()`
   and `Game::widget_focus()` ask the top screen; `Harness::widget_focus()
   -> Option<NodeKey>` and `Harness::unreachable_widgets() ->
   Vec<NodeKey>` (`crates/platform/src/harness.rs`) read it.
   **Either order with 2322 (parked):** if 2322 is done, its
   accessibility tree reads `Screen::widget_focus`: on a screen with a
   `Focus`, the tree's `focus` is the focused item inside the widget
   `Focus::at()` names, and a menu with `MenuView::active` false has
   none, with a test; if not, nothing to do (2322 does it, its clause
   says so).
5. **`UiAction::dir`.** **Either order with 2354:** whichever lands
   first adds `UiAction::dir(self) -> Option<Dir>` to
   `crates/assets/src/keymap/action_set.rs`, and the other uses it.
6. **Either order with 2405** (mouse and touch for menus): if 2405 is
   done, `Focus::point(&mut self, pointer: Option<&PointerInput>, node:
   impl Fn(WidgetKey) -> Option<NodeKey>) -> bool` sets the focus from
   the pointer. On a frame the pointer `moved` over another widget's hit
   (`PointerInput::over`), or on a click on one (`clicked`), it calls
   `set` with the node `node` gives for that hit's `WidgetKey`, and says
   whether the focus moved. A screen with a `Focus` gives each menu its
   own key (`Menu::keyed`) and calls `point` before its widgets take the
   pointer (`Menu::point`). The doc test's two menus show it. If 2405 is
   not done, 2405 does it.
7. **An ADR** (`write-adr` skill): focus between widgets comes from the
   layout; the widget first, then a neighbour set by hand, the way back,
   the geometry and wrap; kept directions; the reachability check. It
   names 2330 as the `.ui.ron` side and 2405 as the pointer's. The new
   public types follow ADR-0039, with private fields and constructors
   where they keep an invariant, as `ListCursor` has.
8. If the PR would pass about 600 changed lines (fixtures and snapshots
   not counted), part 4, and part 6 if 2405 is done, go to a follow-up
   ticket (`write-ticket`, blocked by 2355). The Harness test then reads
   its test screen through `Harness::screen` (the screen opts in with
   `Screen::as_any`), and the PR says so.
   The same PR adds the follow-up's id to the `blocked_by` of each
   open ticket that uses `Screen::widget_focus` (2330 implements it;
   2322 reads it), and names the follow-up where they mention it.

**For the game:** `MenuView` gains a field, so code that builds one with
a struct literal adds `active: true` (`MenuView::new` sets it). The PR
says so, with a `Breaking:` line (2010), and the game's PR that
moves its pin follows it. The rest is additive.

**Out (do not do):**
- The `.ui.ron` fields for kept directions, neighbours and wrap,
  `DocScreen` moving its focus with `Focus`, and `shuyi check` running
  `FocusMap::unreachable` over every `.ui.ron`: 2330's, which waits for
  this ticket.
- Giving the focus back when the screen above pops: the stack already
  keeps each screen's state.
- Keeping a menu's focus by value after a reload: 2330's (`Focus::rebuild`
  keeps the widget).
- The pointer events, hit areas and `Menu::point` themselves (2405).
- Moves inside a grid widget (2354's `GridCursor`): a grid at its edge
  without wrap leaves the direction unused, as a menu at its end does.
- The accessibility tree (2322, parked), beyond the either-order part
  in item 4.
- Shoulder buttons that jump between tabs or groups, and focus across
  screens: no `UiAction` for them (the `input-actions` skill); a
  follow-up if a game asks.
- Drag and drop.
- Moving engine screens onto `Focus`: each takes the focus in one
  widget at a time (the profiles screen's actions menu takes every
  action while it is open, and 2354 puts the key bindings screen's
  switch row and table on one `GridCursor`). 2329 moves two screens
  onto layouts with their moves unchanged.

## Implementation steps

1. Read 2329's `layout` module as it landed (`Node`, `NodeKey`,
   `Layout`, its rectangle type), `widgets/menu.rs`, `menu/view.rs` and
   `menu/glyph.rs`, `screen.rs`, `game.rs`,
   `crates/platform/src/harness.rs` and `crates/ui/tests/it/main.rs`.
   Check whether 2354, 2405 and 2334 are done.
2. Write `FocusNode`, `FocusMap` and `FocusError` in
   `crates/ui/src/focus.rs`, declare the module in `lib.rs`, with unit
   and property tests.
3. Write `Focus`: `pass`, the way back, `set` and `rebuild`.
4. Add `UiAction::dir` (unless 2354 added it), `Menu::without_wrap`,
   `MenuView::active` and the inactive look, with a snapshot.
5. Add `Screen::widget_focus`, the stack's, the game's and the Harness's
   methods. Write the integration test in `crates/ui/tests/it/focus.rs`
   and add `mod focus;` to `main.rs`.
6. If 2405 is done, write `Focus::point` and its tests.
7. Write the ADR, check the size against part 8, and run the gates.

## Acceptance criteria

- [ ] On a 3 by 3 layout of equal boxes with gaps, each direction moves
      one box, or nowhere at an edge, or round the line with
      `.wrapping()` (unit tests).
- [ ] A tall list on the left, a short panel top right and a footer
      under both: `next` gives the panel for Right from the list, the
      list for Left from the panel, the footer for Down from the panel,
      and the list for Up from the footer (unit test). With `Focus`,
      Down from the panel to the footer, then Up, comes back to the
      panel (the way back).
- [ ] A neighbour set by hand wins over the geometry; the way back wins
      over the geometry and loses to a neighbour, and is forgotten after
      another move or `set` (unit tests).
- [ ] Each `FocusError` names its keys (unit tests).
- [ ] `unreachable()` lists a widget under a menu that keeps Up and Down,
      lists a widget with an empty rectangle, and is empty once the menu
      keeps nothing. `pass` on a direction the focused node keeps
      returns false and doesn't move (unit tests).
- [ ] Property: on random layouts of boxes that don't overlap (one box
      per cell of a coarse grid, each a random size inside its cell, some
      cells empty), `next` never returns `from`, gives the same answer
      twice, and with no neighbours and no wrap returns only a box
      wholly beyond `from` that way.
- [ ] Property: on a full grid of equal boxes where no node keeps a
      direction, `unreachable()` is empty.
- [ ] A snapshot of an inactive menu is committed and reviewed, and
      every existing menu snapshot is unchanged.
- [ ] A Harness test (`crates/ui/tests/it/focus.rs`) drives a test
      screen of three menus: a list (`Menu::without_wrap`), a wrapping
      menu beside it that keeps Up and Down, and a one-item footer
      without wrap under both. Right goes to the menu beside the list
      and Left comes back; Down moves inside the list, then at its end
      goes to the footer; Up comes back to the list. Each move between
      widgets plays the move sound once, the view shows only the focused
      menu as active, and `Harness::unreachable_widgets()` is empty.
- [ ] If 2405 is done: in the same test, a pointer moved over the menu
      beside the list (`Harness::point`) gives it the focus, and a click
      on the footer's item (`Harness::click_text`) gives the footer the
      focus and chooses the item.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each rule of `next`, in its order; wrap; the way back and when
  it is forgotten; `rebuild` keeping and losing the key; each error;
  `unreachable` with kept directions and an empty rectangle; `pass` on a
  kept direction; `Menu::without_wrap`; `UiAction::dir` if added here;
  `Focus::point` (a hover, a still pointer, a click, a hit of the
  focused widget) if 2405 is done.
- Property: the two above.
- Snapshot / integration: the inactive menu; the Harness test.

## Completion notes

*(Filled in by the session that completes the ticket.)*
