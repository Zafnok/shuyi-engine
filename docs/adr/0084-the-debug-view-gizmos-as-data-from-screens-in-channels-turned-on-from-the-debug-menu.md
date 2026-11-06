# ADR-0084: The debug view: gizmos as data from screens, in channels turned on from the debug menu

- **Status:** Accepted
- **Date:** 2026-11-05
- **Related tickets:** 2315 (this), 2316 (pause and step), 2317–2320
  (collision, world layers, skeletons, 3D), 2375 (the log), 1140 (the
  `net` channel), 2122 and 2106 (the `curve` and `nav` channels), 2336
  (drawn shapes, which build on this ADR's two pixel rules), 2506–2508
  (the editor's live view)

## Context

Nick, 2026-10-19 (`docs/decisions.md`), asked to see "level layouts and
collisions bounds and skeletons in realtime (debug mode)". Godot's
"Visible Collision Shapes" and Unreal's `show collision` draw what the
game's logic sees over the running game; both draw it from lists of
shapes the engine collects each frame, not from the game's drawing.

Before this ADR the debug tools were screens of their own on the debug
menu (`shuyi_ui::debug`: the menu, the glyph sampler, the scene
camera). Nothing drew *over* a game while it ran.

Four rules from `AGENTS.md` shape the answer:

- **Graphics are a skin** (rule 9, ADR-0002 §4): what is shown is plain
  data, a skin picks the look, and tests read the data.
- **The core stays pure** (rule 5): looking must not change what is
  looked at. A replay with the view on has the hash it has with it off.
- **No key is hard-coded** (rule 8): the debug menu is the one way in,
  and the Debug action already opens it.
- **Debug tools only** (`Ctx::debug_tools`, `shuyi_ui::DEBUG_TOOLS`): a
  shipped build pays for none of it.

The frame's items (`Item`) are `Copy` and `Hash`, and every renderer
(`shuyi-app`'s, the raster `frame-png` paints with, the one-bit
renderer and its reference) draws them. ADR-0078's painted pictures
showed how a new item kind joins them.

## Decision

1. **A gizmo is data.** `shuyi_ui::debug::gizmo::Gizmo` is a shape in
   console pixels (the frame's own space): `Rect` (an outline one pixel
   wide), `Line`, `Cross`, `Circle` (an outline) and `Label` (ASCII
   text, since debug tools are English only). Each is handed in with a
   **channel** (`Channel`: a stable `snake_case` id and a label for the
   menu) and a **role** (`GizmoRole`: what it shows: `Bounds`, `Solid`,
   `Trigger`, `Path`, `Bone`, `Note`; `#[non_exhaustive]`, so a later
   channel adds roles). A gizmo carries no colour.
2. **Screens hand gizmos in, read-only.** `Screen::debug_draw(&self,
   ctx: &Ctx<G>, out: &mut Gizmos)` has a default that does nothing, so
   adding it breaks no game (ADR-0039 §7). `ScreenStack::debug_draw`
   asks every screen it draws, bottom to top, so an overlay's gizmos
   come after those of the screen under it. A screen builds them from
   `&self` and `&Ctx`, never `&mut`: turning the view on can't change
   the game's state.
3. **`Gizmos` is one frame's list.** `push(channel, role, gizmo)` keeps
   the order it was filled in and drops a gizmo whose channel is off;
   `channel_on(channel)` tells a screen to skip the work for a channel
   that is off. Both record the channel as **offered**.
4. **The switch is `Ctx::debug_view: DebugView`:** the channels that are
   on (none at first) and those the running screens have offered, kept
   from frame to frame in the order first offered, so the debug menu,
   which hides the screens under it, can still list them.
   `Ctx::set_debug_channel` turns one on or off and saves the set under
   the key `debug_view` in the computer's storage (not the profile's: it
   is the developer's choice, not a player's). It is read in
   `Ctx::with_storage` and written only with debug tools on, so a
   shipped build never reads or writes it. A key of its own, beside the
   engine's settings rather than inside them, keeps it out of every
   game's `GameSettings::to_ron`.
5. **The menu:** `DebugMenuScreen::new` adds a "Debug view" entry beside
   "Save session" (the engine's entries, after the game's and the
   modules'). It opens `DebugViewScreen`, a menu of every offered
   channel with on or off, toggled with Confirm.
6. **The game asks every frame, while debug tools are on.**
   `Game::redraw` asks the stack for gizmos after the modules'
   `draw_over` (ADR-0040) and the crank hint, with the channels that
   are on, keeps what was offered, and paints the list when it holds
   anything. It asks even when no channel is on: that is how a channel
   is offered before anyone can turn it on. A screen that tests
   `channel_on` first pays a call and a branch for each channel; with
   debug tools off, nothing is asked.
7. **The debug skin paints them** (`debug::glyph::paint_gizmos`) as a
   new item kind, `Item::Gizmo(GizmoItem)`, always on the `Over` layer
   and after everything else. The colour comes from the role through
   `EngineColor` (the debug tools' look), never from the game's
   palette: `Bounds` text_dim, `Solid` warning, `Trigger` accent,
   `Path` focus, `Bone` bright, `Note` highlight. A label prints its
   glyphs into the cells under its position, in the frame's own font,
   and its item (whose text is kept, up to 24 characters, so `Item`
   stays `Copy`) draws no pixels.
8. **One integer rule draws the pixels**, in
   `shuyi_ui::frame::shape`: Bresenham's line (the all-octant form whose
   error term starts at `dx + dy`, in closed form so a line far longer
   than the frame costs no more than the frame) and the midpoint circle
   (its decision value starting at `1 - r`). `GizmoItem::for_each_run`
   gives each item's pixels as runs of whole console pixels inside its
   clip, and every renderer fills those runs: `shuyi-app` with one
   `draw_rectangle` per run, not macroquad's float lines; the raster and
   the one-bit renderers with their rectangle fill. So the window equals
   `frame-png` by construction. 2336's drawn shapes build on these two
   rules.
9. **Tests read the list, not pixels.** The snapshot lists each gizmo in
   its overlays section (`over  gizmo grid bounds rect 0,0 8x8`), and a
   channel's helper is tested by the `Gizmos` it fills.
10. **A later channel plugs in** with a `Channel` constant and a helper
    in its own module that a game's screen calls from `debug_draw`, the
    way `shuyi_ui::grid::debug::layout_gizmos` gives the grid kit's
    `grid` channel: the cell lines, the map's bounds, the cursor's cell,
    each unit's cell and index, and the edges the map goes on past.
11. **Tools turn channels on through the script.** The input script
    (ADR-0082) gets a `debug` step (`debug("grid,collision")`, or
    `debug("all")` for every channel, offered or not) that turns debug
    tools and those channels on in the Harness and draws the frame
    again. `frame-png --debug <channels>` puts it first, so every frame
    it captures shows them. Over the live link and in a capture plan the
    step is refused for now.

## Consequences

- Every later channel (2317–2320, 2375, 1140's `net`, 2122's `curve`,
  2106's `nav`) is a constant and a helper; none touches the frame, the
  renderers or the menu.
- A test of what a channel shows is a test of a list, in any skin, on
  any target.
- `Item` gains a variant; it is `#[non_exhaustive]`, so other crates'
  matches already have a `_` arm. `Screen` gains a defaulted method;
  `Step` gains a variant (it is `#[non_exhaustive]` too), and the
  contract's schema lists it.
- Debug builds ask every drawn screen for gizmos every frame, even with
  no channel on: one call per screen. A release build without the
  `debug-tools` feature asks nothing.
- Labels sit in glyph cells, so one never lands between two cells; a
  label over the game's text replaces those glyphs while the view is on.
- Gizmos are in console pixels only. World coordinates wait for world
  layers (1120) and 2318's camera transform.

## Alternatives considered

- **Screens draw gizmos straight into the buffer.** Fewer types, but
  the look would be each screen's, tests would read pixels, and the
  editor (2506) could not show the same channels in its own view.
- **A `&mut` hook, or gizmos made in `update`.** A screen could then
  change its state while drawing them, and a replay with the view on
  could differ. `&self` and `&Ctx` make that impossible.
- **The channels in the engine's `Settings`.** Every game's
  `GameSettings::to_ron` would have to carry them (a break), and a
  player's settings file would hold a developer's switch.
- **Ask for gizmos only when a channel is on** (as 2315's ticket first
  said). Then no channel is ever offered before one is on, and the menu
  would have nothing to list.
- **Macroquad's `draw_line` and `draw_circle_lines`** in `shuyi-app`.
  They work in floats at screen pixels, so the window would not match
  `frame-png` at every scale.
- **A `Label` with a `String` in `Item`.** `Item` would lose `Copy`,
  which every renderer and the buffer's cutting and blitting lean on.
  The text is short, so a fixed array of 24 bytes keeps it.
- **A `--debug` parameter on `GameTools::frame`.** A required change in
  every game's tools; the script step reaches every game's Harness
  through the script it already plays.
