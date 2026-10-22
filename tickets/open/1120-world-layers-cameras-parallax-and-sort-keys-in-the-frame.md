---
id: "1120"
title: "Parked: World layers in the frame: cameras that follow, parallax, and sort keys for y-sorting"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1119", "2335", "2336"]
nick_input: none
completed:
---

# 1120 — World layers: cameras, parallax, sort keys

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): after Visions of Shuyi launches, not only after the MVP (`docs/decisions.md`, 2026-10-20). Unparked early only if the game needs it: its ticket 0060 says the army walks around freely (then 1119–1125 become game work), or it asks for a piece of this through its `engine-change` skill, which is then split out and unparked alone. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0003 §5. Today's frame places items on integer console pixels in two
layers (Under, Over) with no sort key, cuts items where cells are
replaced, and has one backdrop with whole zoom 1–4
(the game repository's [boundary map](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/engine/boundary.md), *What other game styles would break*). A
free-moving world needs layers placed in world pixels, a camera, and
depth sorting.

The 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6 and Unreal
5.8 found one more thing a camera needs: a rectangle of the screen to
draw into. Godot draws a camera into a `SubViewport`
(docs.godotengine.org/en/stable/tutorials/rendering/viewports.html),
Bevy has `Camera::viewport`, and Unity has `Camera.rect`. That is how
split screen for local co-op and versus is made, and an inset such as a
boss camera or a security monitor. As written, a world layer covers the
whole frame and nothing clips it to a rectangle. Adding that after this
ticket lands would change the frame's API and every snapshot, so it is
part of this ticket: each world layer gets an optional viewport, and a
layer without one draws exactly as before. Grid games already have
this: the grid kit's `GridSkin::paint` draws a `MapScene` into any
`area: Rect` of cells (`crates/ui/src/grid/skin.rs`), so two map views
or a map inset need nothing here. Which player or device a viewport
belongs to is 2406's.

Leave room for 2361 (parked: world layers drawn between the last two
steps on displays faster than 60 Hz). Keep the camera's and the items'
positions in `Fx` world pixels in the frame, floored to whole world
pixels when drawn. Build items, the camera and world layers through
constructors, so 2361 can add an item's `key` and `snap`, the camera's
`snap` and a layer's `Smoothing` without breaking the API that
templates and games use. Put the placement of a world item in window
pixels in one function in `shuyi-ui` that both painters call, as
`Backdrop::scene_offset` (`crates/ui/src/frame/glyph_buffer/backdrop.rs`)
does for the backdrop.

## Nick input

None. Our game's frames don't change.

## Scope

**In:**
- `WorldLayer` in `shuyi-ui::frame` has:
  - a `Camera`: a position in `Fx`, a whole zoom 1–4, follow with a dead
    zone, and clamp to bounds;
    - **damped follow:** each step the camera moves a whole-number
      fraction `(num, den)` of the gap to where the dead zone wants it
      (`(1, 1)` is a hard follow), with `Fx`'s rounding. A move that
      rounds to zero moves the rest, so a still target is reached
      exactly and never overshot;
    - **a shake that decays**, which moves only the drawn view: 2340's
      `FrameShake` maths (trauma in thousandths, seeded, whole pixels,
      pure in time), advanced to each step's time in whole ms, so it
      falls to zero over a set number of steps. The player's
      `shake_strength` scales it (0 turns it off), and there is none
      under `Motion::Reduced` or with idle motion off
      (`Ctx::shows_idle_motion`). **Either order with 2340:** if 2340
      isn't done, write `shuyi_ui::frame::shake` as 2340 describes it,
      and 2340 reuses it. The camera then takes a shake strength the
      game sets (0 to 100), and the PR adds a line to 2340's Scope that
      ties it to the player's settings;
  - a parallax factor per axis (two fractions, x and y): Tiled and LDtk
    give one per axis, and a side-view background often scrolls only
    sideways (2360 reads them);
  - items placed in world pixels with `sort: (z, y, order)`: the frame's
    `Sprite`, with 2335's angle, pivot and flips, and 2336's `Shape`,
    drawn by the same pixel rules as on the console. **Either order
    with 2337:** if 2337 is done, they carry its `blend` and draw
    `Paint::Tinted` as `Sprite` does, through the same code in every
    painter; if not, 2337 adds them;
  - a `viewport: Option<PxRect>` in console pixels (`PxRect` is the
    alias in `crates/ui/src/frame/glyph_buffer.rs`, re-exported from
    `shuyi_ui::frame`). `None` is the whole frame. The layer's items are
    clipped to it, and its camera's position, dead zone and clamp to
    bounds are measured against the viewport's place and size, not the
    frame's.
- `split_viewports(frame: PxRect, n: u8) -> Vec<PxRect>` in
  `shuyi_ui::frame`, pure: for n = 1 to 4, the frame; two side by side;
  one wide on top and two below; two by two. The rects are whole
  pixels, don't overlap and cover the frame exactly; an odd pixel goes
  to the first column or row. Any other n gives an empty `Vec`. A game
  may place a viewport anywhere else, such as a small inset over
  another layer. Split screen repeats a stage's layers once per
  viewport, each with its own camera.
- The camera is view state. It is stepped once per simulation step
  (`FrameInput::ticks()`, `crates/ui/src/input/frame.rs`) in `Fx`, and
  kept out of `TickSim::hash` unless a game puts it in its simulation,
  as ADR-0015 (§2, §9) keeps the RTS camera out.
- World layers are drawn between the backdrop and the cells. Their items
  are never cut by cells.
- Snapshots list world layers and their items, one line each. A layer's
  line names its viewport, or `frame` when it has none. `frame-png` and
  the renderer in `shuyi-app` draw them, each layer clipped to its
  viewport, with the same pixels. Both already clip the backdrop to a
  rectangle: `Renderer::draw_backdrop` in `crates/app/src/render.rs`
  with a `Camera2D` viewport (macroquad's scissor needs `unsafe`, which
  the workspace forbids), and `Painter::draw_backdrop` in
  `crates/cli/src/frame_png.rs` through a picture the size of the
  window. Viewports clip the same way.
- Whole-frame effects (fades, game ADR-0060's brightness) include world
  layers, in every viewport. So do 2339's transition looks (when
  `compose` takes cells from one frame, it takes the world-layer items
  over those cells with them), 2339's veil, drawn over them, and
  2340's shake offset, which moves them with the rest of the frame in
  every painter. **Either order with 2339 and 2340:** if one isn't
  done, the PR adds a line to its Scope saying world layers take part.
- **Either order with 2363:** if 2363 is done, add
  `Camera::listener(&self) -> shuyi_ui::audio::spatial::Listener` for
  the world camera: the centre of the view in whole world pixels and
  half the view's width (its viewport's, or the frame's) at the
  camera's zoom, with a test that sounds at the left and right edges of
  the view get pans of the same size and opposite sign. If 2363 isn't
  done, 2363 adds it.
- **Either order with 2406:** if 2406 is done, the test scene adds two
  seats, each with the stage's layers in its own viewport from
  `split_viewports` and a camera following that seat, redone when one
  joins or leaves. If not, 2406 adds that scene.
- Perspective markers (ADR-0009 §3):
  - `shuyi_core::perspective` has the trait `Perspective` and the
    markers `TopDownGrid` and `TopDown`;
  - `WorldLayer`, `Camera` and a world scene are generic over
    `P: Perspective`, and the drawing side reads `P`'s projection;
  - keep the 2D projection off `Perspective` itself (a trait of its
    own for 2D markers), because the parked 3D markers (3101) have no
    2D projection;
  - 1126 and 1127 add `Isometric` and `SideView`.

**Out (do not do):**
- a light blended over the time of day on world layers: 2369 gives the
  grid scene `look.blend`, and a world layer takes the same field in
  the ticket that first needs it;
- isometric projection (1126);
- drawing between the last two steps (2361);
- look-ahead, rooms that hold the camera, blends between camera goals,
  and the camera's numbers as a `camera.ron` (2362);
- ease curves for camera moves, such as a blend: they come from
  `shuyi_core::ease` (2109) and 2362, never from a curve of this
  ticket's own. The shake dies down by 2340's trauma rule, not a curve;
- tile layers from Tiled and LDtk maps (2360 adds them as one item per
  map layer, never one sprite item per tile);
- which player, seat or device a viewport belongs to, and split screen
  for each player count (2406);
- a HUD per viewport as a system of its own: a game draws cells or
  items into the viewport's rectangle with what the frame already has;
- Voronoi or other splits that move with the players;
- drawing a layer into a texture of its own and running shaders on it
  (the wgpu work: 2326's render targets, 2327's shaders);
- a minimap through a second camera: 2716 draws its minimap from coarse
  grids;
- more than one screen: a surface (3023) is a physical screen, and a
  viewport is a rectangle inside one surface. Neither builds a second
  layout system for the other;
- grid games: the grid kit already paints a `MapScene` into any
  `area: Rect`.

## Implementation steps

1. Add the types, the viewport and `split_viewports`, and their
   snapshot lines.
2. Sort items once per frame.
3. Draw them in `shuyi-app` and in the raster (`shuyi_ui::frame::raster`
   if 2332 is done; if not, `frame-png`'s `Painter` in
   `crates/cli/src/frame_png.rs`, and 2332 moves it), each layer
   clipped to its viewport, with the same results.
4. A test scene with two parallax layers and twenty sprites, y-sorted,
   with the camera following one of them. Then the same scene twice
   more: split side by side by `split_viewports(frame, 2)`, its layers
   repeated per viewport, each camera following a different sprite; and
   with a small inset viewport over the full-frame layers.
5. The either-order parts for 2337, 2339, 2340, 2363 and 2406.

## Acceptance criteria

- [ ] The test scene's snapshot and `frame-png` picture match at three
      camera positions.
- [ ] The side-by-side and inset cases' snapshots and `frame-png`
      pictures match their checked-in files, and no item is drawn
      outside its layer's viewport.
- [ ] For n = 1 to 4 and any frame size, `split_viewports` gives rects
      that don't overlap and cover the frame exactly (property test).
- [ ] `frame-png` and the window draw the same pixels. **Either order
      with 2018:** if 2018 is done, the test scene goes on its render
      card (`shuyi_ui::frame::raster::card`) or in a plan, and its
      `render` CI job passes; if not, compare a captured frame by hand,
      note it in the PR, and 2018 adds the scene.
- [ ] No snapshot of the game changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: camera follow, clamp, dead zone, sort order; follow, clamp and
  dead zone again in a viewport that isn't at (0, 0); `split_viewports`
  for n = 0 to 5; damped follow reaches a still target exactly without
  overshoot; the shake is zero after its last step and at a shake
  strength of 0.
- Property: `split_viewports` covers any frame exactly with no overlap.
- Snapshot: the test scene full frame, side by side, and with an inset.

## Completion notes
