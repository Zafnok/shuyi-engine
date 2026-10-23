---
id: "2405"
title: "Mouse and touch for menus and grid maps, split out of parked 1121: hover and click on the engine's menus, the grid cursor follows the pointer, and never required"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2405 — Mouse and touch for menus and grid maps

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and with GameMaker, Defold and Construct 3.

**What the benchmarks have.** In every one, a mouse click or a tap
reaches every UI control, and none ties that to a physics or fixed-step
core:

- Bevy's picking gives one pointer for mouse, touch and pen, hit tests
  for sprites and UI, and pointers a test can fake
  (https://docs.rs/bevy_picking/latest/bevy_picking/);
- Godot hands pointer events to the GUI first, and every `Control`
  takes hover and click
  (https://docs.godotengine.org/en/stable/tutorials/inputs/inputevent.html);
- Unity's UI input module handles the pointer and navigation together;
  Unreal's widgets take hover and click;
- GameMaker turns taps and drags into gesture events with the mouse as
  one finger; Construct's and Defold's GUI nodes take the pointer first.

**What Shuyi has today.** No pointer at all:

- `RawInputEvent` (`crates/ui/src/game.rs`, line 28) has keys, typed
  text and pad buttons only. `shuyi-app` reads keys
  (`crates/app/src/keys.rs`) and pads (`crates/app/src/pads.rs`), and
  `Platform::poll_input` (`crates/platform/src/platform.rs`, line 198)
  speaks of "keyboard and controller events".
- Targets already say whether a pointer exists: `Input.pointer` and
  `Input.touch` (`crates/assets/src/target.rs`, lines 170 to 175).
  `desktop.ron` and `web.ron` set `pointer: true`; the Deck's touch
  screen is "nothing may need it".
- Where a menu's rows are is known only inside its glyph painter
  (`crates/ui/src/widgets/menu/glyph.rs`, `paint`). `MenuView` holds no
  geometry, and nothing records what was painted where.
- `Grid::rect` (`crates/ui/src/grid/layout.rs`) and `GridSkin::tile_px`
  (`crates/ui/src/grid/skin.rs`, line 291) go from a tile to pixels.
  Nothing goes back.
- `ListCursor`'s window (`crates/ui/src/widgets/list.rs`) moves only
  with the focus. Done 2307 left "Pointer or wheel scrolling (1121)" out.

**What is planned.** Parked 1121 bundles two kinds of work: raw mouse
and touch events with "a Harness test clicks a menu row", which need no
real time, and stick axes, a dead zone and the pointer in world pixels
in `TickInput`, which need 1119. Because of the bundle, all pointer
input waits for real time. Nick split it on 2026-10-20
([`docs/decisions.md`](../../docs/decisions.md)): the features most
games expect come after the MVP, not parked, and "some take the
turn-game half of a parked ticket (the mouse in menus and on grid maps
from 1121 …); the real-time half stays parked". This ticket is that
half. Related: 2333 (scale modes: the placement whose inverse maps the
mouse), 2355 (focus between widgets), 2354 (a grid cursor), 3042 (held
input let go on focus loss), 2518 (session records), 2407 (parked:
on-screen controls and gestures on phones' browsers, blocked by this),
3053 (platform facts at run time), 3044 (the system cursor).

**Why it matters.**

- **The game:** nothing changes. Visions of Shuyi uses no mouse (1121's
  Nick input), and its screens don't call the new methods, so its
  menus ignore the pointer unless it opts in. Its pin bump follows the
  API notes below.
- **Other 2D games:** turn games are what Shuyi makes today, and on PC
  and the web most of them (card battles, match-3, puzzles, tactics,
  point-and-click mysteries) are played with a mouse, and on phones
  with taps. Today the web build and the website's playable examples
  (2801, 2803) can't be clicked.
- **The end goal of 2026-10-19:** a game an agent makes should run on
  the web and on the Playdate alike. Until real time is unparked the
  agent check measures turn games, and their web builds should take a
  click. Because the pointer is never required, the Playdate build of
  the same game loses nothing.

## Nick input

None. Nick decided on 2026-10-20 that this half of 1121 comes after the
MVP, not parked.

## Scope

**In:**

1. **Pointer events at the platform boundary** (`crates/ui/src/game.rs`):
   - `RawInputEvent` gains:
     - `PointerMoved { x: i32, y: i32 }`: the pointer's place in console
       pixels of the frame last shown, (0, 0) its top-left;
     - `PointerLeft`: it left the frame, or a finger lifted;
     - `PointerDown` and `PointerUp`: the primary button, which is the
       left mouse button or a touch;
     - `Wheel(i32)`: whole notches, positive away from the player.
   - Integers only, so a record of them replays exactly. The enum's doc
     comment names the pointer too.
   - `Platform::poll_input`'s doc comment says pointer events come too,
     and that a backend without a pointer sends none.
     `HeadlessPlatform::push_input`
     (`crates/platform/src/headless.rs`, line 297) already takes any
     `RawInputEvent`.
2. **`shuyi-app` reads the mouse** in a new `crates/app/src/pointer.rs`,
   called from `Macroquad::poll_input` (`backend.rs`, line 265) after
   the keys and pads:
   - **Where:** `mouse_position()` is in logical pixels (macroquad
     0.4.16 divides by the DPI factor, `src/input.rs`, line 51). Times
     `screen_dpi_scale()` it is in the physical pixels `Renderer::draw`
     places the frame in (`render.rs`, line 122). The inverse of that
     placement (item 3) gives console pixels.
     - The backend keeps the `ConsoleSpec` of the last frame it drew
       (`draw_game`). Before the first game frame there are no pointer
       events.
     - `PointerMoved` only when the console pixel changes, and
       `PointerLeft` once when the pointer goes into the letterbox.
       macroquad reports no pointer leaving the window (miniquad's
       Windows backend has its mouse-leave code commented out), so off
       the window the pointer keeps its last place. That is accepted:
       hover follows motion only (item 5).
   - **Buttons:** `is_mouse_button_pressed` and
     `is_mouse_button_released` for `MouseButton::Left`. A press outside
     the frame is dropped. Every press sent gets its release, wherever
     the release happens, so nothing stays held.
   - **Order within one poll:** `PointerMoved` or `PointerLeft` for the
     place, then `PointerDown`, then `PointerUp`, then `Wheel`. A press
     and a release in one frame give `PointerDown`, then `PointerUp`.
   - **Touches:** macroquad turns touches into left-button events by
     default (`simulate_mouse_with_touch`, `src/lib.rs`, lines 323 and
     655 to 666), on native and the web, and a button event moves
     `mouse_position` to the touch. So a tap arrives as a press and a
     release at the touch's place. A finger has no hover: after a
     release that came from a touch (`touches()` was not empty at its
     press), the reader sends `PointerLeft`, last in that poll.
   - **The wheel:** `mouse_wheel().1` is a raw delta in a unit each
     platform picks:
     - Windows: `WM_MOUSEWHEEL`'s delta, 120 a notch (miniquad 0.4.11,
       `src/native/windows.rs`, line 567);
     - Linux: X11, miniquad's default backend (`LinuxBackend::X11Only`,
       which `shuyi-app` keeps), 1 a notch (`src/native/linux_x11.rs`,
       lines 100 to 103);
     - macOS: a wheel's line delta times 10, so about 10 a notch, and a
       trackpad's pixel deltas (`src/native/macos.rs`, lines 597 to
       600);
     - the web: `-deltaY` (`crates/cli/web/mq_js_bundle.js`), about 100
       a notch for a mouse and small steps for a trackpad.

     An accumulator turns the deltas into whole notches by the
     platform's unit (`cfg` on the target), keeping the remainder, so a
     trackpad scrolls too. Measure the web in Chrome and Firefox on the
     day and write the numbers in a comment. macroquad keeps only the
     last wheel event of a frame (`src/lib.rs`, line 565); that is
     accepted.
   - The mapping, the accumulator and the touch rule are pure functions
     in `pointer.rs`, tested without a window. The macroquad calls are
     a thin shell around them.
3. **The placement's inverse** (`crates/ui/src/frame/console.rs`):
   `Layout::frame_px(self, spec, window: (f32, f32)) -> Option<(i32,
   i32)>`, the console pixel at a window pixel (rounded down), `None`
   in the letterbox. **Either order with 2333:** if 2333 is done, the
   inverse is `Placement`'s, its `Fit` fraction included, and
   `pointer.rs` uses the mode the frame was drawn in; if not, this
   inverts today's `layout`, and 2333 moves it onto `Placement`.
4. **Hit areas in the frame** (a new `crates/ui/src/frame/hit.rs`,
   `no_std`):
   - `WidgetKey(pub u16)`, 0 by default, and `Hit { rect: PxRect,
     widget: WidgetKey, item: u32 }`, with `Hit::new(rect, widget,
     item)` and the const `Hit::AREA` (`u32::MAX`), an item that stands
     for the widget's whole box. `Hit` is `#[non_exhaustive]`, as
     `shuyi_platform::Frame` is.
   - On `GlyphBuffer` (`crates/ui/src/frame/glyph_buffer.rs`):
     - `hit(rect: PxRect, widget, item)`;
     - `hit_cells(cells: Rect, widget, item)`, in the buffer's own cell
       size (`ConsoleSpec::cell_px`);
     - `hits() -> &[Hit]`;
     - `hit_at(x, y) -> Option<Hit>`: the last one recorded that holds
       the point, since what was painted last is on top;
     - `clear_hits()`. Filling the whole buffer with a solid cell (how
       a frame is cleared) clears them too, as it clears the backdrop
       (`fill_rect`).
   - Hits never show in snapshots.
   - `ScreenStack::draw` (`crates/ui/src/screen.rs`, line 360) clears
     the hits before it draws the top screen. So only the screen that
     gets the input has hit areas, and a click outside an overlay never
     reaches the screen below it, even when both menus have key 0.
   - The painter records the hits, not the view. A swapped skin records
     its own geometry, and the views stay plain data with no geometry
     (rule 9). One list serves every reader: the pointer here, and
     later the accessibility tree's bounds (2322).
5. **The pointer per frame** (a new `crates/ui/src/input/pointer.rs`,
   `no_std`):
   - `PointerInput`, plain data and `#[non_exhaustive]`:
     - `at: (i32, i32)`;
     - `moved: bool`: the pointer moved this frame;
     - `pressed: bool` and `released: bool`: the primary button went
       down or up this frame;
     - `down_at: Option<(i32, i32)>`: where the press held now, or
       released this frame, began;
     - `over: Option<Hit>`: the hit area under `at` in the frame last
       shown;
     - `clicked: Option<Hit>`: on a release, the hit the press began
       on, if the release is over the same widget and item. A press on
       one row and a release on another is no click;
     - `wheel: i32`: the notches this frame.
   - `Pointer`, the tracker: it folds a frame's pointer events into one
     `PointerInput`, or `None` while the pointer is outside the frame
     or there never was one. A frame with a press or a release whose
     events end in `PointerLeft` (a tap) still gives its `PointerInput`
     with `over` `None`, so the tap's click is kept; the next frame
     gives `None`. The hit lookup comes in as a closure, so the tracker
     is tested without a buffer.
   - `FrameInput::with_pointer(Option<PointerInput>)` and
     `FrameInput::pointer() -> Option<&PointerInput>`
     (`crates/ui/src/input/frame.rs`). `declared` keeps the pointer, as
     it keeps keys and text.
   - `Game::step` (`crates/ui/src/game.rs`, line 293) feeds the frame's
     pointer events to its `Pointer`, looking hits up in `self.buffer`
     before `redraw`. So a click lands on what the player saw.
     - A pointer press ends `KeyPrompt::Waiting`, as a key does.
     - The pointer never opens the first-key screen, and never changes
       `ctx.device` (help text keeps naming the keys or buttons used
       last).
   - **Either order with 3042:** if 3042 is done,
     `Game::forget_held_input` also forgets a held pointer press (its
     release is then no click) and the pointer's place. If not, 3042
     does it.
6. **The menu takes the pointer** (`crates/ui/src/widgets/menu.rs`,
   `menu/view.rs` and `menu/glyph.rs`):
   - `menu::glyph::paint` records the box as `Hit::AREA`, then each
     shown row (`Rect::new(x + 1, row, w - 2, 1)`) as its item's index,
     under the view's widget key. `MenuView` gains `widget: WidgetKey`,
     which `MenuView::new` and `Menu::view` set. `Menu::keyed(WidgetKey)`
     gives a menu its key, for a screen with more than one.
   - `Menu::point(&mut self, pointer: Option<&PointerInput>) ->
     Option<MenuEvent<T>>` and `point_with_sound`, by these rules:
     - **hover:** on a frame the pointer `moved`, over a row of this
       menu the focus may stop on, the focus goes there. A pointer that
       stays still never takes the focus back from the keys;
     - **click:** `clicked` on a row of this menu moves the focus there
       if it may stop there, then does what Confirm does on that row:
       `Chosen` for an enabled row, nothing for a disabled one;
     - the sounds of `handle_with_sound`: Move when the focus moves,
       Select when a row is chosen, Denied for a click on a disabled
       row.
   - The screens whose menus `menu::glyph::paint` draws call it beside
     their actions:
     - the debug menu (`crates/ui/src/debug.rs`, line 191, painted by
       `debug/glyph.rs`);
     - the toy's lobby menu (`crates/ui/src/toy.rs`, line 769);
     - Hello, Grid's title and end menus
       (`examples/hello-grid/src/screens.rs`, lines 174 and 424, painted
       by `centred_menu` in `skin.rs`). They rebuild their menu each
       frame from `self.focus`, so they store `menu.focus()` after
       `point` as they do after `handle_with_sound`.
   - **Either order with 2355:** if 2355 is done, add
     `Focus::point(&mut self, pointer: Option<&PointerInput>, node: impl
     Fn(WidgetKey) -> Option<NodeKey>) -> bool` as 2355's item 6 says:
     on a frame the pointer `moved` over another widget's hit
     (`PointerInput::over`), or on a click on one (`clicked`), it calls
     `Focus::set` with that widget's node and says whether the focus
     moved. A screen with a `Focus` gives each menu its own key
     (`Menu::keyed`) and calls `Focus::point` before its widgets take
     the pointer. If not, 2355 does it.
7. **Grid maps: the tile under the pointer**
   (`crates/ui/src/grid/layout.rs` and `skin.rs`):
   - `Projection::tile_at(self, corner, tile, px) -> (i32, i32)`, the
     inverse of `place` (with `div_euclid`): the tile's offset from the
     view's first. `Grid::tile_at_px(&self, px: (i32, i32)) ->
     Option<Pos>`, `None` outside the drawn tiles;
   - `GridSkin::tile_at(&self, scene, area, px) -> Option<Pos>`,
     defaulted through `grid`, beside `tile_px`. A skin that lays its
     tiles its own way overrides both.
   - **Hello, Grid's stage** (`examples/hello-grid/src/screens/stage.rs`)
     uses it with the skin of the player's look (`with_skin`), after the
     walk check in `update`, where the actions are read:
     - on a frame the pointer moved over a drawn tile on the map, the
       cursor goes there (`Cursor::jump`), ticks as `step` does, and
       steers the path as `step` does (`steer` already takes a jump);
     - a release whose press began on the same tile (`down_at` and `at`
       give the same tile) moves the cursor there and does what Confirm
       does (`confirm`);
     - nothing while a walk is shown, as for the keys.
   - **The rule** (the ADR below): the pointer never enters a command.
     Only the command a key would also send (`Command::Move`) reaches
     the rules, so a run played with the pointer and one played with
     keys give the same record.
8. **The Harness** (`crates/platform/src/harness.rs`), each step a frame
   of `FRAME_DT` as `keys` does:
   - `point(x, y)`: a frame with `PointerMoved`;
   - `click(x, y)`: `point` when the pointer is elsewhere, then a frame
     with `PointerDown` and one with `PointerUp`;
   - `wheel(notches)`: a frame with `Wheel`;
   - `leave_pointer()`: a frame with `PointerLeft`;
   - `click_text(text)`: clicks the centre pixel of the first cell of
     the first run of `text` in the frame (`Game::buffer`), so a test
     clicks a menu row by its words. It panics when the text isn't
     shown.
9. **Never required.** Every widget and screen that takes the pointer
   takes the same keys as before. No action is bound to the pointer, so
   `controls_fit` (`crates/ui/src/input/fit.rs`) and `shuyi check
   --target` don't change, and a target without a pointer (the
   Playdate, the Switch) or one where nothing may need it (the Deck's
   touch screen) loses nothing.
10. **Docs:**
    - an ADR (the `write-adr` skill) on pointing in turn games and
      screens:
      - the pointer is UI input: it moves a focus or a cursor and gives
        the click a widget turns into what Confirm gives. It is never in
        a command or a turn record;
      - hit areas come from what was painted, the top screen's only;
      - the backend maps the window to console pixels, in integers;
      - hover follows motion only, a click is a press and a release on
        the same hit, and a touch is a pointer with no hover;
      - it is never required.

      It amends ADR-0003 §8 ("Input for real time"): only the pointer in
      `TickInput` and stick axes stay real time (1121). ADR-0003's
      `Status:` line gets a note naming it, as it has for ADR-0011;
    - the `input-actions` skill's rule 8: pointing (hover, and a click
      or a tap on a widget or a tile) is not a key, so nothing binds it.
      It gives what the keys give, through the widget, and it is never
      the only way. Pointer buttons as game actions, axes and the
      pointer in `TickInput` are 1121's;
    - `docs/platform-backends.md`: the pointer in the `poll_input`
      section, and pointer tests in the checklist for a new backend.
11. **Either order with 2518:** if 2518 is done, the new variants carry
    its serde derives (they are on the enum) and a pointer session plays
    back with the same hash per frame (a test). If not, 2518 records
    them as it records any `RawInputEvent`.
12. The new public types follow ADR-0039.
13. No new dependency.

**For the game:** `RawInputEvent` gains variants, which breaks a
`match` on it with no `_` arm, and `MenuView` gains a field, which
breaks building one with a struct literal. The PR says so (a
`Breaking:` line if 2010 is done), and the game's PR that moves its pin
follows it. The rest is additive.

**Out (do not do):**
- The pointer on the engine's other widgets and screens, and wheel
  scrolling. This is a follow-up ticket, After MVP like this one; if
  none exists when this ticket starts, write it first with the
  `write-ticket` skill (a 24xx id, blocked by 2405), and split it if it
  passes one PR. It covers:
  - `Ask` (`crates/ui/src/widgets/ask.rs`), the options rows, the key
    bindings table and `NameEntry` (2354's grid cursor for the last
    two, if it is done);
  - the profiles screen: its list, and its actions menu, which
    `paint_actions` in `crates/ui/src/screens/profiles/glyph.rs` paints
    from the `MenuView` itself;
  - the layout picker, whose painter
    (`crates/ui/src/screens/layout_picker/glyph.rs`) draws one panel per
    layout and no `MenuView`;
  - the grid template's stage (`templates/grid/crates/game/src/stage.rs`);
  - `ListCursor::scroll_by(rows, rows_shown, enabled) -> bool` and the
    wheel scrolling a list or a menu: the window moves under
    `Scroll::KeepInView` and the focus follows into it, and under
    `Scroll::Centred` the focus steps. The options and profiles lists
    are what scroll today; no engine menu calls `Menu::showing`.

  Their painters record hits as item 4 says. This ticket reads the
  wheel into `PointerInput::wheel` so `RawInputEvent` breaks once.
- Mouse buttons other than the primary, the pointer as a game action,
  stick axes, and the pointer in world pixels in `TickInput`: 1121
  (parked).
- Gestures (swipe, pinch, long press), several touches at once,
  on-screen controls and a web-touch target: 2407 (parked).
- Drag and drop, and dragging a scroll bar.
- Moving the focus between widgets: 2355 (either order above).
- Laying widgets out: 2329. A painter records hits in whatever
  rectangle it paints, so either order works, and its `Layout` keeps
  no second list of them.
- The accessibility tree's bounds: 2322 (parked) reads these hits when
  it lands.
- Picking isometric tiles: 1126 (parked) gives its projection a
  `tile_at`.
- Hiding the system cursor: 3044. Telling a running game whether it has
  a pointer: 3053.
- Pointer steps in the tool contract and the MCP server: write a ticket
  when an agent needs them.
- The toy's grid screen (`crates/ui/src/toy/grid.rs`): Hello, Grid
  shows the grid part.

## Implementation steps

1. Read ADR-0003 §8, ADR-0024 and ADR-0025, the `input-actions` skill,
   `game.rs` (`RawInputEvent`, `step`, `redraw`), `screen.rs` (`draw`),
   `input/frame.rs`, the menu's three files, `debug/glyph.rs`,
   `grid/layout.rs`, `grid/skin.rs`, `grid/cursor.rs`,
   `frame/console.rs`, `frame/glyph_buffer.rs`,
   `crates/app/src/backend.rs`, `keys.rs` and `render.rs`,
   `crates/platform/src/platform.rs`, `harness.rs` and `headless.rs`,
   and Hello, Grid's `screens.rs`, `screens/stage.rs`, `skin.rs` and
   `tests/it/main.rs`. Check whether 2333, 2355, 3042, 2518 and
   2010 are done. If the follow-up for the other widgets doesn't exist,
   write it now (Out).
2. Write the hit areas and the stack's rule, with their tests.
3. Add the events, `Pointer`, `PointerInput`, `FrameInput::pointer` and
   the wiring in `Game::step`, with their tests.
4. Give the menu its hits, `keyed`, `point` and `point_with_sound`, and
   call them from the screens listed in item 6.
5. Add the Harness helpers and the Harness tests of the toy's lobby.
6. Write `tile_at` with its property test, then Hello, Grid's stage and
   its pointer tests. A test helper in `tests/it/main.rs` clicks a tile
   at the centre of its `tile_px` in the look's skin (`map::GLYPHS` or
   the game's `sprite_skin()`).
7. Write `pointer.rs` in `shuyi-app`, the inverse in `console.rs` and
   the backend's wiring. Try the desktop build and the web build by
   hand (`cargo shuyi web` in `examples/hello-grid`): hover and click on
   the title menu and a turn on the stage. Measure the web's wheel
   deltas (item 2) with a print you remove before the PR. The PR says
   what was tried where.
8. Write the ADR, the note in ADR-0003, the skill's rule and the
   backend doc.
9. If the PR would pass about 600 changed lines (fixtures and snapshots
   not counted), Hello, Grid's stage (item 7's stage bullets and their
   tests) goes to the follow-up, the kit's `tile_at` and its property
   test stay, and the PR says so. Run the gates.

## Acceptance criteria

- [ ] A Harness test clicks the toy lobby's "Key bindings" row with
      `click_text` and gets the same screens and the same sounds (Move,
      then Select) as the keys `Down Enter`.
- [ ] Hover moves a menu's focus only on a frame the pointer moved: with
      the pointer still over row 2, a key Down puts the focus on row 3
      and it stays there.
- [ ] A press on one row and a release on another chooses nothing. A
      click on a disabled row chooses nothing and plays Denied.
- [ ] A tap (a press, then a release with `PointerLeft` in the same
      frame) still chooses the row it was on.
- [ ] Only the top screen has hit areas: with an overlay over a screen
      with a menu, a click on a row of the lower menu outside the
      overlay does nothing (a unit test of `ScreenStack::draw`).
- [ ] Hello, Grid (unless step 9 moved it): a turn played with the
      pointer (click your piece, then a tile of its reach) sends the
      same commands and leaves the stage in the same state as
      `walk_to` by keys. A whole stage played out with the pointer gives
      the same commands as one played with keys, with the glyph look
      and the tile look.
- [ ] With no pointer events, every existing test passes unchanged,
      `FrameInput::pointer()` is `None`, and the tests of `controls_fit`
      don't change.
- [ ] `shuyi-app`'s pure parts are tested: the mapping inside the frame,
      in the letterbox and at a DPI factor of 1.75; the wheel's
      accumulator for each unit; a press and a release in one frame; a
      touch's release followed by `PointerLeft`.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] The ADR, ADR-0003's note, the skill's rule 8 and
      `docs/platform-backends.md` are written, and the PR names its API
      changes for the game.
- [ ] The follow-up ticket for the other widgets and wheel scrolling
      exists in `tickets/open/`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - `hit_at` (the last recorded wins, a point outside every hit, the
    hits cleared by a full fill);
  - `ScreenStack::draw` keeping only the top screen's hits;
  - `Pointer`: `moved` only when the place changes, a click on the same
    hit, a press here and a release there, a press and a release in one
    frame, a release and `PointerLeft` in one frame still clicking,
    `PointerLeft` alone giving `None`, the wheel's notches;
  - `Menu::point`: hover, a still pointer, a click on an enabled and a
    disabled row, two menus told apart by their keys;
  - `Layout::frame_px` and `shuyi-app`'s accumulator and touch rule.
- Property:
  - `Grid::tile_at_px` gives back the tile for every pixel of every
    drawn tile, and `None` outside `Grid::bounds`, for the glyph skin
    and the sprite skin at each of its zooms;
  - `Layout::frame_px` inverts the placement for every console pixel,
    at scales 1 to 4.
- Integration (Harness): the toy's lobby clicked; Hello, Grid's turn
  and stage played with the pointer against the keys; the existing
  key-only playthroughs unchanged.

## Completion notes

*(Filled in by the session that completes the ticket.)*
