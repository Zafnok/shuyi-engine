---
id: "2409"
title: "Mouse and touch on the engine's other widgets and screens, and wheel scrolling: questions, the options rows, the key bindings table, the name grid, the profiles screen, the layout picker and the grid template's stage"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2405"]
nick_input: none
completed:
---

# 2409 — Mouse and touch on the engine's other widgets, and the wheel

## Context

2405 brings the pointer to turn games and to the engine's menus:

- pointer events at the platform boundary, read by `shuyi-app`;
- hit areas a painter records in the frame
  (`crates/ui/src/frame/hit.rs`: `WidgetKey`, `Hit`, `Hit::AREA`, and
  `GlyphBuffer::hit`, `hit_cells` and `hit_at`), the top screen's only;
- `PointerInput` on `FrameInput` (`at`, `moved`, `pressed`, `released`,
  `down_at`, `over`, `clicked`, `wheel`);
- the menu: `Menu::keyed`, `Menu::point` and `point_with_sound`, and
  the hits `menu::glyph::paint` records;
- grid maps: `GridSkin::tile_at`;
- the Harness's `point`, `click`, `click_text`, `wheel` and
  `leave_pointer`.

Its ADR sets the rules this ticket follows: hover follows motion only;
a click is a press and a release on the same hit and does what Confirm
does; a touch is a pointer with no hover; the pointer never enters a
command; and it is never required.

2405 left the engine's other widgets and the wheel out, and named this
ticket in its Out. Once 2405 lands, these still take no pointer:

- `Ask` (`crates/ui/src/widgets/ask.rs`), the question over a screen.
  Its painter (`ask/glyph.rs`, `paint`) draws its answers in a line and
  records nothing. The options, profiles and key bindings screens open
  one.
- The options screen's rows (`crates/ui/src/screens/options.rs`,
  painted by `options/glyph.rs`, `paint_row`), with the `◄` and `►`
  round a focused value the cursor changes.
- The profiles screen (`crates/ui/src/screens/profiles.rs`): its list
  (`profiles/glyph.rs`, `paint_row`), and its actions menu, which
  `paint_actions` paints from the `MenuView` itself, not through
  `menu::glyph::paint`, so 2405's menu hits don't reach it.
- The name grid, `NameEntry` (`crates/ui/src/widgets/name_entry.rs`,
  painted by `name_entry/glyph.rs`, `paint_grid`).
- The key bindings table (`crates/ui/src/screens/key_bindings.rs`,
  painted by `key_bindings/glyph.rs`): its slots, Restore defaults, the
  keyboard / controller switch and the controller side's choices.
- The layout picker (`crates/ui/src/screens/layout_picker.rs`). Its
  painter (`layout_picker/glyph.rs`) draws one panel per layout and no
  `MenuView`, so the screen's `Menu`, which is never drawn, gets no
  hits.
- The grid template's stage (`templates/grid/crates/game/src/stage.rs`):
  each direction key sends `Command::Step`, and nothing reads the
  pointer.
- The wheel: 2405 reads it into `PointerInput::wheel`, and nothing uses
  it. `ListCursor` (`crates/ui/src/widgets/list.rs`) moves its window
  only with the focus; done 2307 left "Pointer or wheel scrolling
  (1121)" out.

No engine list scrolls today. The options and profiles panels grow to
fit their rows (`panel` in `options/glyph.rs` and in
`profiles/glyph.rs`; at most `MAX_PROFILES` profiles), the key bindings
panel reaches down to the message, and no engine menu calls
`Menu::showing`. So the wheel reaches a `Menu` a game shows with fewer
rows than it has, and a game's own list through the new
`ListCursor::scroll_by`.

**Who uses it.** Every game played with a mouse on a PC or the web, and
with taps on a phone's browser (2407, parked): a card game's "Discard
this card? Keep · Discard", a farming game's shop list scrolled with
the wheel, a puzzle game's options, a player's name spelled by taps.
The hits each painter records also give the accessibility tree its
bounds (2322, parked) when it lands.

Related ADRs:
[ADR-0025](../../docs/adr/0025-controls-fit-screens-declare-actions-targets-have-button-tables-and-the-harness-holds-back-the-rest.md)
(controls fit),
[ADR-0038](../../docs/adr/0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md)
(the `Breaking:` line) and
[ADR-0039](../../docs/adr/0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)
(public types that can grow).

## Nick input

None.

## Scope

**In:**

1. **Grid items** (`crates/ui/src/frame/hit.rs`, `no_std`):
   - `grid_item(row: usize, col: usize) -> u32`: the item of a cell in
     a widget of rows and columns, the row in the high 16 bits and the
     column in the low 16;
   - `grid_cell(item: u32) -> Option<(usize, usize)>`: its inverse,
     `None` for `Hit::AREA`.

   The name grid, the key bindings table and the options rows use
   them, and so can a game's grid of cells.
2. **The wheel scrolls** (`crates/ui/src/widgets/list.rs` and
   `menu.rs`):
   - `ListCursor::scroll_by(&mut self, rows: i32, rows_shown: usize,
     enabled: impl Fn(usize) -> bool) -> bool`. A positive `rows`
     shows later rows, a negative one earlier rows. It does nothing and
     gives `false` for 0 rows, or for a list that fits in `rows_shown`.
     - Under `Scroll::KeepInView` the window's first row moves by
       `rows`, clamped as `window` clamps it. If the focus is now
       outside the window, it moves to the nearest row inside it that
       `enabled` accepts: the window's first such row when the window
       moved down, its last when it moved up. If the window holds no
       such row, nothing moves. It gives whether the window or the
       focus moved. The next `window(rows_shown)` gives the moved
       window.
     - Under `Scroll::Centred` the focus steps `rows` rows that way,
       one `step(forward, false, enabled)` at a time, stopping at the
       ends, and the window follows it as `window` places it. It gives
       whether the focus moved.
   - `Menu::point` (2405) takes the wheel. On a frame with `wheel` not
     0 and the pointer over this menu (`over` holds its key), a menu
     `showing` fewer rows than it has calls `scroll_by(-wheel, rows,
     ..)` with the rows its focus may stop on. A notch away from the
     player (a positive `wheel`) shows earlier rows, as in a browser.
     `point_with_sound` plays Move when the focus moved. A menu that
     shows all its rows ignores the wheel.
3. **`Ask` takes the pointer** (`widgets/ask.rs`, `ask/view.rs` and
   `ask/glyph.rs`):
   - `Ask::keyed(self, widget: WidgetKey) -> Self`, and `AskView`
     gains `widget: WidgetKey`, which `view` sets. A screen gives each
     question a key other than its rows'.
   - `glyph::paint` records the box as `Hit::AREA`, then each answer
     (its name and the space either side) as its index, under
     `view.widget`.
   - `Ask::point<V>(&mut self, pointer: Option<&PointerInput>, audio:
     &mut AudioQueue<V>) -> Option<T>`:
     - on a frame the pointer `moved` over an answer of this question,
       the focus goes there, with Move when it moved;
     - a click on an answer moves the focus there and gives that
       answer, as Confirm does. Its sound stays the screen's, as for
       `handle`;
     - a click anywhere else gives nothing: a question is closed by
       its answers or by Cancel.
   - A yes/no (`Ask::yes_no`) shows no answers, only the line naming
     its keys. It records only its box and ignores the pointer.
4. **`NameEntry` takes the pointer** (`widgets/name_entry.rs` and
   `name_entry/glyph.rs`):
   - `NameEntry::keyed(self, widget: WidgetKey) -> Self`, and
     `GridView` gains `widget: WidgetKey`, which `view` sets.
   - `paint_grid` records its box as `Hit::AREA`, then each cell (its
     label and the space either side) as `grid_item(row, col)`, under
     `grid.widget`.
   - `NameEntry::point<G: GameData>(&mut self, pointer:
     Option<&PointerInput>, ctx: &mut Ctx<G>) -> NameEvent`: hover
     moves the focus to the cell, with Move when it moved; a click on a
     cell moves the focus there and does what Confirm does on it
     (`confirm`: a letter, a space, Delete or Done, with their sounds).
   - The typing box (`NameBox`) takes no pointer: it is typed.
   - **Either order with 2354:** if 2354 is done, `NameEntry` keeps its
     focus in a `GridCursor`, and a hit sets it with
     `GridCursor::set_focus`; if not, a hit sets `NameEntry`'s own `at`,
     and 2354 carries the pointer onto `GridCursor` when it moves
     `NameEntry` (its Out says so).
   - **Either order with 2346:** if 2346 is done, the grid shows one
     page at a time, with a Page cell. Each page's cells and its Page
     cell record their hits as the other cells do, and a click on Page
     turns the page as Confirm does, with a unit test in the toy's
     debug language (two pages). If not, nothing to do: 2346's new
     cells take the pointer as the other cells do (its Out says so).
5. **The options screen** (`screens/options.rs` and `options/glyph.rs`):
   - `paint_row` records, under the default key, the row (the panel's
     inner width) as `grid_item(i, 0)`, `i` its place in
     `OptionsView::rows`, then the `◄` and `►`, when they are shown, as
     `grid_item(i, 1)` and `grid_item(i, 2)`.
   - `update` reads the pointer before the frame's actions, while no
     box or question is open:
     - hover moves the focus to the row, as `move_row` does (the
       message goes, Move);
     - a click on a row focuses it and does what Confirm does there
       (`confirm`, as from the keys, not a controller);
     - a click on `◄` or `►` does what Left or Right does (`change`).
   - The question asked on leaving (`LeaveChoice::ask`) is keyed and
     takes the pointer through `Ask::point`. Its answer is handled as
     `answer_leave` handles one from the keys. The yes/no questions and
     the volume box take none.
   - A click that opens a box, a question or a screen ends the frame's
     input, as a Confirm key does.
6. **The profiles screen** (`screens/profiles.rs` and
   `profiles/glyph.rs`):
   - `paint_row` records each row (the panel's inner width) as its
     place in `ProfilesView::rows`, under the default key.
     `paint_actions` records its box as `Hit::AREA`, then each item's
     row as the item's index, under `actions.menu.widget`.
   - `update`, with nothing open: hover moves the focus to the row (as
     `move_row`), and a click does what Confirm does on it (`confirm`).
   - The open box takes the pointer instead, each under its own key:
     - the actions menu is `Menu::keyed` and takes it through
       `point_with_sound`; its event is handled as `act` handles one
       from the keys;
     - the name grid is keyed and takes it through `NameEntry::point`;
       its event is handled as `type_name` handles one;
     - the two questions (`DeleteChoice::ask`, and the one on backing
       out of a name box) are keyed and take it through `Ask::point`.

     A click outside the open box does nothing, and never reaches a
     row under it.
7. **The key bindings table** (`screens/key_bindings.rs` and
   `key_bindings/glyph.rs`):
   - The painter records, under the default key:
     - each slot's bar as `grid_item(i, j)`, `i` the row's place in
       `KeyBindingsView::rows` (the screen's `row`) and `j` the slot;
     - Restore defaults as `grid_item(n, 0)`, and the switch's
       keyboard and controller sides as `grid_item(n + 1, 0)` and
       `grid_item(n + 1, 1)`, `n` the number of action rows: the
       screen's `restore_row` and `switch_row`.

     `paint_choices` records its box as `Hit::AREA`, then each line as
     its index, under a key of its own.
   - `update`, after the `releasing` check, while nothing is captured
     and no question is open:
     - hover moves the focus to the slot, to Restore defaults or to the
       switch (the message goes, Move);
     - a click on a slot or on Restore defaults focuses it and does
       what Confirm does (`confirm`: a capture, the choices, or the
       question);
     - a click on a side of the switch focuses the switch and shows
       that side (`show`);
     - with the choices open, hover moves their focus and a click takes
       a line, as Confirm on it does (`answer_choice`).
   - While capturing, and while the yes/no "restore the defaults?" is
     open, the pointer does nothing: the slot waits for a key or a
     button, and the question for its keys.
   - **Either order with 2354:** if 2354 is done, the screen keeps its
     focus in a `GridCursor`, and a hit sets it with
     `GridCursor::set_focus` (the switch is one cell there, so a side's
     hit focuses that cell, then shows the side); if not, a hit sets
     `row` and `slot`, and 2354 carries the pointer onto `GridCursor`
     when it moves the screen (its Out says so).
8. **The layout picker** (`screens/layout_picker.rs` and
   `layout_picker/glyph.rs`): `paint` records each layout's panel as
   its index, under the default key, which is the key of the screen's
   `Menu`. `update` calls `self.menu.point_with_sound(input.pointer(),
   &mut ctx.audio)` before the keys and handles its event as theirs:
   hover focuses a panel, and a click chooses its layout.
9. **The grid template's stage**
   (`templates/grid/crates/game/src/stage.rs`), by 2405's rule that the
   pointer sends only the command a key would also send:
   - a release whose press began on the same tile (`down_at` and `at`
     give the same tile through `GLYPHS.tile_at(&scene, AREA, px)`,
     with the scene `draw` shows) on one of the four tiles beside your
     piece sends `Command::Step` that way, as that direction's key
     does. A click on any other tile does nothing;
   - once the stage is over, a click on the map does what Confirm does
     (the next stage);
   - no hover: the stage has no cursor;
   - the module's doc comment says so.
10. **Hello, Grid's stage, if 2405 moved it here.** If 2405's step 9
    left Hello, Grid's stage out (its completion notes say so), 2405's
    item 7 stage bullets, their tests and its Hello, Grid acceptance
    criterion are this ticket's.
11. **Either order with 2759** (parked, the inventory grid of shapes):
    if 2759 is done and its grid widget takes no pointer yet (it was
    done before 2405), its painter records each cell as
    `grid_item(row, col)` under its key, and a click picks an item up
    and a click drops it, through the code its Confirm runs, no drag,
    with a test; if not, nothing to do (2759 is then worked after 2405
    and takes the pointer itself, as its clause with 2405 says).
12. **Never required.** Every widget and screen here takes the same keys
    as before. No action is bound to the pointer, so `controls_fit`
    (`crates/ui/src/input/fit.rs`) and every target's fit test don't
    change.
13. **Public types** (ADR-0039): the new functions and methods are
    additive. `AskView` and `GridView` are views a skin reads, and they
    gain a field, so they move under the ADR here (its *When*):
    `#[non_exhaustive]`, with `AskView::new(text)` and
    `GridView::new(title)`, the rest assigned.
14. No new dependency.

**For the game:** `AskView` and `GridView` gain a field and become
`#[non_exhaustive]`, which breaks a struct literal of either. The
completion notes say so with a `Breaking:` line (ADR-0038), and the
game's PR that moves its pin follows this one. The rest is additive,
and the game's screens behave as before with no pointer.

**Out (do not do):**
- The pointer on the typing box (`NameBox`), on the options' volume box
  and on a yes/no question: keys answer them.
- The wheel on a list that shows every row: it does nothing. The
  options, profiles and key bindings screens don't scroll, so the
  wheel does nothing there.
- Scroll bars, a click on a scroll mark (`▲`, `▼`), dragging, and drag
  and drop: no ticket plans them; write one when a game asks.
- Mouse buttons other than the primary, the pointer as a game action,
  and the pointer in `TickInput`: 1121 (parked).
- Swipes and several touches at once: 2407 (parked).
- Hello, Grid's title and end menus, the debug menu and the toy's
  lobby: 2405 does them.
- The toy's grid screen (`crates/ui/src/toy/grid.rs`), which 2405 also
  leaves out: Hello, Grid shows the grid part.
- Moving the focus between widgets of one screen: on each screen here
  one widget takes the input at a time.
- A new ADR: these are 2405's rules applied to more widgets.

## Implementation steps

1. Read 2405 as it landed: its ADR, `frame/hit.rs`, `input/pointer.rs`,
   `Menu::point`, the Harness helpers, and its completion notes (did
   its step 9 move Hello, Grid's stage here?). Read `widgets/list.rs`,
   `ask.rs` and `name_entry.rs` with their painters, the four screens
   with their painters and tests, `crates/ui/tests/it/screens.rs`, and
   the grid template's `stage.rs` and `tests/it/main.rs`. Check
   whether 2354, 2346 and 2759 are done.
2. Write `grid_item` and `grid_cell`, with their test.
3. Write `ListCursor::scroll_by` with its unit and property tests, then
   the wheel in `Menu::point`.
4. `Ask`: `keyed`, `AskView::widget` and `AskView::new`, the painter's
   hits and `point`, with their tests. Then `NameEntry` the same way.
5. The options screen, the profiles screen, the key bindings table and
   the layout picker, one at a time: the painter's hits with a test
   that `hit_at` on a pixel gives the item, then `update`, then the
   Harness test.
6. The grid template's stage and its tests. Make a game from the
   template and run its checks, as the `run-gates` skill's "A new game
   from each template" says (`templates/` isn't in the workspace).
7. The docs (the `document` skill): the module docs of `ask`,
   `name_entry` and `list`, and of the four screens, say which hits
   their glyph painters record and under which key, so a game's own
   painter (`painted_by`) records the same.
8. Write the `Breaking:` line and run the gates.

## Acceptance criteria

- [ ] `ListCursor::scroll_by`: under `KeepInView` one notch moves the
      window one row and the focus follows into it when the window
      leaves it; under `Centred` the focus steps; a list that fits
      gives `false`.
- [ ] A `Menu` showing 3 of its 10 rows scrolls one row a notch with
      the pointer over it, and not with the pointer over another widget
      or with no pointer.
- [ ] In the toy, through the Harness, from a click on the lobby's
      Options row: a click on the Colors row changes it as `Enter` on
      it does; a click on `►` of the focused music volume raises it by
      `VOLUME_STEP`; leaving with a change and clicking Keep keeps it.
      The settings and the sounds are those the keys give.
- [ ] A click on the options' layout row, then on the second layout's
      panel, leaves the second layout in use, as `Down Enter` in the
      picker does.
- [ ] On the key bindings screen, a click on an action's second slot,
      then a key, puts the key in that slot, as moving there and
      `Enter` then the key do. A click on the switch's controller side
      shows it.
- [ ] On the profiles screen (a Harness started on it), a click on a
      profile opens its actions, a click on Delete opens the question,
      and a click on Yes deletes the profile, as the keys do.
- [ ] A still pointer never takes a focus back from the keys: with the
      pointer still over an answer, a row or a cell, a key moves the
      focus and it stays there (for `Ask`, `NameEntry` and the options
      rows).
- [ ] With a box open over the profiles list, a click outside the box
      does nothing, and a click on a row under it doesn't reach the
      row.
- [ ] "Al" spelled by clicks on A, l and Done gives `NameEvent::Done`
      with the name "Al".
- [ ] The grid template: a stage played by clicks on the tiles beside
      your piece gives the same commands and the same state as one
      played by keys, and a click on the map once it is over starts the
      next stage.
- [ ] With no pointer events, every existing test and snapshot passes
      unchanged, and the targets' fit tests don't change.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] The completion notes have a `Breaking:` line for `AskView` and
      `GridView`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - `grid_item` and `grid_cell` give each other back, and
    `grid_cell(Hit::AREA)` is `None`;
  - `scroll_by` under `KeepInView`: the window moves and stops at both
    ends, the focus follows in from either edge, it skips disabled
    rows, and a window of disabled rows moves nothing. Under
    `Centred`: steps, and stops at the ends. A list that fits, and 0
    rows, give `false`;
  - the wheel in `Menu::point`;
  - `Ask::point`: hover, a still pointer, a click on an answer, a click
    outside the box, a yes/no ignoring the pointer, and two questions
    told apart by their keys;
  - `NameEntry::point`: hover, a letter, Delete and Done clicked;
  - each painter's hits: `hit_at` on a pixel of an answer, a cell, an
    options row and its arrows, a profiles row, an actions item, a
    slot, the switch's sides, a choices line and a layout panel gives
    its key and item;
  - the profiles screen with a box open, clicked outside it.
- Property: after any `scroll_by` and then `window`, the focus is in the
  window and the window is in the list, for any length, scroll rule,
  rows shown, notches and disabled rows.
- Integration (Harness): the toy's options, layout picker, key bindings
  and profiles screens clicked against the keys; the grid template's
  stage played by clicks against keys; the existing key-only tests
  unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
