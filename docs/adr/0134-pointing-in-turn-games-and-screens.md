# ADR-0134: Pointing in turn games and screens: UI input from hit areas the painter records, never in a command, never required

- **Status:** Accepted
- **Date:** 2026-11-21
- **Related tickets:** 2405, 2413, 2409, 1121, 2407, 2355, 2322
- **Amends:** ADR-0003 §8 (only the pointer in `TickInput` and stick axes
  stay real time)

## Context

Every engine we compare ourselves with (Bevy, Godot, Unity, Unreal,
GameMaker, Defold, Construct) lets a click or a tap reach any UI
control, with no tie to a fixed-step core. Shuyi had no pointer, and
where a menu's rows are was known only inside its glyph painter.
ADR-0003 §8 put the pointer with stick axes as real-time input (1121,
parked). Nick split 1121 on 2026-10-20 (`docs/decisions.md`): the mouse
in menus and on grid maps comes after the MVP; the real-time half stays
parked. Turn games played with a mouse or a finger are most of what
Shuyi makes today, and the Playdate and the Switch have no pointer.

Rules replay exactly from their commands (ADR-0002), so the pointer
must leave the same record as the keys. Graphics are a skin (ADR-0002
§4), so where a row was drawn is the skin's fact, not the view's.

## Decision

1. **The pointer is UI input.** It moves a focus or a cursor, and a
   click on a widget gives what Confirm gives there. It never enters a
   command: only the command a key would send reaches the rules, so a
   run played with the pointer gives the record a run with keys gives.
   No action is bound to it.
2. **Events at the platform boundary, in integers.** `RawInputEvent`'s
   `PointerMoved { x, y }` (console pixels of the frame last shown),
   `PointerLeft` (into the letterbox, or a finger lifted), `PointerDown`
   and `PointerUp` (the left mouse button or a touch) and `Wheel(i32)`
   (whole notches, positive away from the player). Integers, so a
   session record replays exactly. In one poll: the place, the press,
   the release, then the wheel. The backend maps its window to console
   pixels (2413 for `shuyi-app`); one without a pointer sends none.
3. **Hit areas come from what was painted.** A painter records each
   area as it paints it (`GlyphBuffer::hit`, `hit_cells`; `Hit::AREA`
   for a widget's whole box; `crates/ui/src/frame/hit.rs`), so a swapped
   skin records its own geometry and views stay plain data. The last
   area recorded that holds a point is the one under it. Hits never show
   in a snapshot or a frame's equality; clearing the whole frame clears
   them.
4. **Only the top screen has hits.** `ScreenStack::draw` clears them
   before the top screen draws, so a click outside an overlay never
   reaches the screen below. `WidgetKey` tells a screen's widgets apart
   (`Menu::keyed`).
5. **One `PointerInput` per frame** (`crates/ui/src/input/pointer.rs`),
   folded by `Game` against the frame last shown, so a click lands on
   what the player saw, and read beside the actions
   (`FrameInput::pointer`, `None` outside the frame).
   - Hover follows motion only: a widget moves its focus to what is
     under the pointer on a frame it `moved`, so a still pointer never
     takes the focus back from the keys.
   - A click is a press and a release on the same widget's same item.
   - A touch is a pointer with no hover: its release comes with
     `PointerLeft`, and that frame keeps its click.
   - It never opens the first-key screen or changes the device the help
     names; a press ends a wait for any key; a lost focus forgets a held
     press and the place.
6. **The tile under the pointer** inverts the projection
   (`Projection::tile_at`, `Grid::tile_at_px`, `GridSkin::tile_at`,
   which a skin laying tiles its own way overrides beside `tile_px`).
7. **It is never required.** A widget that takes the pointer takes its
   keys as before, so `controls_fit` and `shuyi check --target` don't
   change, and a target without a pointer loses nothing.

## Consequences

- A turn game's web build can be played by mouse or finger, and its
  Playdate build loses nothing: the same rules from the same commands.
- Each painter records its own hits: the menu's glyph look now, the
  other widgets in 2409. A widget whose painter records none ignores
  the pointer, which is safe. The same list can give an accessibility
  tree its bounds (2322).
- macroquad reports no pointer leaving a window on Windows; hover
  following motion only makes the stale place harmless.
- Drag and drop, gestures and several touches are left out (2407).

## Alternatives considered

- **Pointer buttons as keymap actions** — a widget could not know which
  row was clicked, and the pointer would enter `controls_fit` and the
  targets' button tables. 1121 keeps that for real-time games.
- **Geometry in the views** — the logic would know the skin's layout,
  against ADR-0002 §4, and a swapped skin would be clicked where the old
  one drew.
- **Hit tests in each screen** — every screen repeats its painter's sums,
  and they drift apart.
- **Hover on every frame over a row** — a still mouse would pull the
  focus back after each key press.
- **Floats for the place** — a record would not replay bit for bit
  across native and the web.
