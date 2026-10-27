---
id: "2315"
title: "The debug view: gizmos drawn over the frame, channels turned on from the debug menu, and the grid kit's layout as the first channel"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2315 — The debug view: gizmos, channels, the grid kit's layout

## Context

Nick, 2026-10-19 (`docs/decisions.md`): "seeing level layouts and
collisions bounds and skeletons in realtime (debug mode)", next to a
visual editor like Godot's or Unreal's.

Today the debug tools are screens on the debug menu (`shuyi_ui::debug`:
`DebugMenuScreen`, the glyph sampler, the scene camera). Nothing draws
*over* a running game to show what its logic sees: where the map's
cells are, which part of the map is in view, where a box collides.
Godot calls this "Visible Collision Shapes" and Unreal "show
collision"; in both the game keeps running underneath.

This ticket is the part every later channel shares:

- the shapes (gizmos) as plain data;
- a way for any screen to hand them in;
- the switch per channel;
- the painting, the snapshot lines and `frame-png`.

Its first channel is the grid kit's layout, because the grid kit is the
only world the engine has today (`shuyi_ui::grid`, from game#1115).
Later channels add themselves:

- collision bounds: 2317;
- level layouts in world layers: 2318;
- skeletons of procedural bodies: 2319;
- online play: a `net` channel that 1140 brings (round trip, loss,
  bytes per channel, the lockstep turn, hash agreement);
- curves: a `curve` channel that 2122 (parked) brings.
  **Either order with 2122:** if 2122 is done, nothing to do (it wrote
  a follow-up ticket for its helper); if not, nothing to do (2122 adds
  the helper when it lands);
- the log's newest lines: 2375 (the follow-up 3040 promised);
- 3D: 2320.

Pausing and stepping one frame are 2316. The editor (2506–2508) shows
the same channels in its live view.

The rules this keeps (`AGENTS.md`):

- **Graphics are a skin** (rule 9). A gizmo is plain data: a shape, a
  channel and a *role* (what it shows: bounds, solid, trigger, path,
  bone, note). The debug skin picks the colors from the role. Tests of
  what a channel shows read the gizmo list, never pixels.
- **The core stays pure** (rule 5). Gizmos are built from `&self` and
  `&Ctx`, never `&mut`. Turning the view on changes no state, so a
  replay with the view on has the same hash as one with it off.
- **No key is hard-coded** (rule 8). Channels are turned on from the
  debug menu, which the Debug action already opens.
- **Debug tools only.** Nothing is built or drawn unless
  `Ctx::debug_tools` is on (`shuyi_ui::DEBUG_TOOLS`: debug builds or
  the `debug-tools` feature). A shipped build pays one branch per frame.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill) for the debug view: gizmos as
  data, channels, roles, the read-only rule and how later channels
  plug in. Link it from `shuyi_ui::debug`'s module docs.
- **`shuyi_ui::debug::gizmo`**, the data:
  - `Gizmo`, one of:
    - `Rect { rect: PxRect }`, an outline one pixel wide;
    - `Line { from: (i32, i32), to: (i32, i32) }`;
    - `Cross { at: (i32, i32), size: u8 }`;
    - `Circle { center: (i32, i32), radius: u16 }`, an outline;
    - `Label { at: (i32, i32), text: String }`, ASCII only (debug
      tools are English only).

    All in console pixels, like the frame's items.
  - `GizmoRole`: `Bounds`, `Solid`, `Trigger`, `Path`, `Bone`, `Note`.
    It is `#[non_exhaustive]`, so later channels can add roles.
  - `Channel`: a stable `&'static str` id in `snake_case` (`grid`,
    `collision`, …) and a label for the menu.
  - `Gizmos`: what one frame hands in. It has
    `push(channel, role, gizmo)` and a `channel_on(id) -> bool` that
    skips the work for a channel that is off. It keeps the order it
    was filled in.
- **`Screen::debug_draw(&self, ctx: &Ctx<G>, out: &mut Gizmos)`**, with
  a default that does nothing. Adding a defaulted method breaks no
  game, so the game's PR that moves the pin needs no change.
  `ScreenStack` calls it on every screen it draws, bottom to top, so an
  overlay's gizmos come after those of the screen under it.
- **The switch:** `Ctx::debug_view: DebugView`:
  - the set of channels that are on (none at first);
  - the list of channels the running screens have offered, kept from
    one frame to the next so the menu can list them.

  It is saved with the engine's settings when debug tools are on. A
  shipped build never reads or writes it.
- **The menu:** a "Debug view" entry on `DebugMenuScreen`. It opens a
  small screen listing every offered channel with on or off, toggled
  with Confirm. It uses the menu widget and the `ListCursor` from 2307.
  **Either order with 2519:** if 2519 is done, the entry goes beside
  its "Save session" entry, the engine's two together; if not, nothing
  to do (2519 puts its entry beside this one).
- **Painting:** `Game::frame` asks the stack for gizmos after it draws,
  when debug tools are on and any channel is on. It hands them to
  `debug::glyph::paint_gizmos`, which adds them to the `GlyphBuffer`
  as a new item kind `Item::Gizmo` on the `Over` layer, drawn last.
  Colors come from the role through `EngineColor` (the debug tools'
  look), never from the game's palette. A label prints in glyph cells
  at its position, in the frame's own font.
  The gizmos are painted after the modules' `draw_over` loop in
  `Game::redraw` (2358, ADR-0040), so they stay over the modules'
  drawing; the snapshot test checks it with a module that draws.
- **The renderers:** `shuyi-app`'s `render.rs` and `frame-png` both
  draw `Item::Gizmo` as runs of whole console pixels from one integer
  rule (`shuyi-app` with one `draw_rectangle` per run, not macroquad's
  lines), so the window equals `frame-png`. **Either order with
  2336:** if 2336 is done, gizmo lines and circles use
  `shuyi_ui::frame::shape`'s width-1 line and circle-outline rules. If
  not, write those two rules in `frame::shape` as 2336's scope says
  (Bresenham with the error term starting at `dx + dy`; the midpoint
  circle from `1 - r`), and 2336 builds on them. `frame-png`'s painter is
  `crates/ui/src/frame/raster.rs` (2332).
- **The snapshot** (`frame/snapshot.rs`): gizmos are listed in the
  overlays section, one line each, as
  `over  gizmo <channel> <role> rect x,y wxh` (and `line x,y x,y`,
  `cross x,y n`, `circle x,y r`, `label x,y "text"`). Update the
  module's doc comment, which is the format's spec.
- **The grid kit's channel** `grid`, in `shuyi_ui::grid`. A
  `MapScene`'s screen offers it, through a helper
  `grid::debug::layout_gizmos(&MapScene<U>, &Grid, &Camera, &mut
  Gizmos)` that a game's map screen calls from its `debug_draw`. It
  shows:
  - the cell lines inside the view (`Bounds`);
  - the map's bounds, where they are on screen (`Bounds`);
  - the cursor's cell (`Note`);
  - each unit's cell, with its index as a label (`Note`);
  - **From 2304 (done):** the edges where the map goes on past the
    view (`Note`), read from `MapScene::more`: `MapEdges`'s `left()`,
    `right()`, `up()` and `down()`.
- **`frame-png --debug <channel>[,<channel>…]`** draws the named
  channels. An agent then sees what a person sees with the view on.
  `all` turns every channel on.
- **Either order with 2514:** if 2514 is done, `--debug` draws the
  channels on every frame that `--every` and `--count` capture (a test
  with a two-frame sheet); if not, 2514 does that.
- **Either order with 2105 (parked):** if 2105 is done, add the
  helper it left here: `shuyi_bones::debug::bone_gizmos`, behind that
  crate's `view` feature, pushing each bone of 2105's `world(&Rig,
  &Pose)` as a `Line` (`GizmoRole::Bone`) from its position along its
  length, on a `bones` channel, placed by the same origin and facing
  as `RigView::from`, with a unit test of its gizmo list. If not,
  nothing to do (2105 adds it).
- The toy game (`toy::grid`) calls the helper, and its tests in
  `tests/it/grid.rs` use it.

**Out (do not do):**
- pausing, stepping, slow motion and a readout of state (2316);
- a frame profiler (2325);
- a game's drawn shapes (`Item::Shape`, wide lines, fills and
  polygons): 2336. Only the width-1 line and circle-outline rules may
  land here (the renderers above);
- collision, world layers, skeletons and 3D (2317–2320);
- the `nav` channel of paths for free movement (2106): 2106 adds it,
  or, if 2106 lands first, the follow-up ticket it writes for it,
  blocked by this one;
- gizmos in world coordinates. There are no world layers until 1120,
  and 2318 adds the camera transform;
- an inspector or anything that edits state (the editor, 2506–2508).
  Debug commands that change state through the game's own commands are
  2356;
- new actions or keys. The debug menu is the only way in.

## Implementation steps

1. Read `crates/ui/src/debug.rs`, `debug/glyph.rs`, `screen.rs`,
   `game.rs`, `frame/glyph_buffer.rs` and `frame/snapshot.rs`, and the
   `input-actions` skill. Read 2336's `frame/shape.rs` and 2514's
   `--every` and `--count` if they are done.
2. Write the ADR.
3. Add `debug/gizmo.rs` with the types and their unit tests.
4. Add `Item::Gizmo` to the frame, its snapshot lines, and its drawing
   as runs in `crates/app/src/render.rs` and `crates/cli/src/frame_png`,
   through `frame::shape`'s width-1 rules (2336's, or written here).
5. Add `Screen::debug_draw`, the call from `ScreenStack`, and
   `Ctx::debug_view`, with saving in the engine's settings.
6. Add the "Debug view" screen to the debug menu.
7. Write `grid::debug::layout_gizmos` and call it from the toy's grid
   game.
8. Add `--debug` to `frame-png`, on every captured frame if 2514 is
   done.
9. Run the gates.

## Acceptance criteria

- [ ] An ADR records the design.
- [ ] With debug tools off, `Screen::debug_draw` is never called (a
      test screen counts its calls).
- [ ] With the `grid` channel on, the toy's grid game shows its cell
      lines, its bounds, the cursor's cell and each unit's cell, and
      the snapshot lists them in that order.
- [ ] A Harness replay of the toy's grid game has the same state hash
      with every channel on as with none.
- [ ] Toggling a channel from the "Debug view" screen turns it on and
      off, and the choice comes back after a restart in a debug build.
- [ ] `cargo shuyi frame-png --debug grid` on the toy draws the gizmos.
      The PNG is reviewed, and its snapshot is committed.
- [ ] No key is named in code (`check-keys` passes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Gizmos::push` keeps order, and skips a channel that is off.
  Each gizmo's snapshot line. Bresenham lines in all eight octants and
  the midpoint circle, if this ticket writes them in `frame::shape`.
- Unit: `layout_gizmos` gives edge gizmos only for the sides
  `MapScene::more` sets, and none for a map that fits the view.
- Property: `layout_gizmos` never yields a gizmo outside the buffer's
  `pixel_bounds`, for any map size, view and camera.
- Snapshot / integration:
  - the toy's grid game with `grid` on, in both skins;
  - the "Debug view" screen;
  - the Harness replay-hash test above;
  - `--debug grid` on a two-frame sheet, if 2514 is done.

## Completion notes

*(Filled in by the session that completes the ticket.)*
