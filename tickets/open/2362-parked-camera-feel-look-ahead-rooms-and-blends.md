---
id: "2362"
title: "Parked: Camera feel: look-ahead, rooms that hold the camera, blends between camera goals, and the camera's numbers in a file"
type: feature
milestone: "Engine: real time"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: blocked
blocked_by: ["1120", "2109"]
nick_input: none
completed:
---

# 2362 — Camera feel: look-ahead, rooms, blends

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): this is real-time work, so it waits until Visions of Shuyi launches, unless the game needs it first (its ticket 0060 says the army walks around freely, which unparks 1120, and that free-moving stage wants look-ahead, rooms or blends; this ticket is then 1120's direct follow-up). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. A 2D camera in the benchmarks does more than follow:

- Godot's `Camera2D` has position smoothing, drag margins and limits.
  Its limits are one rectangle, so a game with rooms sets them from a
  script.
- Unity's Cinemachine 3 has damping and lookahead in its Position
  Composer, a Confiner 2D that keeps the camera inside a shape, and
  blends between cameras
  (`docs.unity3d.com/Packages/com.unity.cinemachine@3.1`).
- Unreal 5.8 has its Gameplay Camera System. Bevy has `smooth_nudge`
  for smoothing that doesn't depend on the frame rate.

Shuyi has today:

- `shuyi_ui::grid::Camera` (`crates/ui/src/grid/camera.rs`): the grid
  kit's tile camera. `follow` scrolls in whole tiles to keep a margin,
  clamped to the map. It doesn't smooth, and turn-based maps don't need
  it to.
- `SceneCameraScreen` (`crates/ui/src/debug/camera.rs`): a debug screen
  that pans a picture in `f32` at `PAN_SPEED`.
- `shuyi_ui::cinematic::ease_in_out` (`crates/ui/src/cinematic.rs`):
  `f32` in and out, so it is for drawing only. No simulation step can
  use it.

Planned:

- 1120 (parked): world layers and a `Camera` with an `Fx` position, a
  whole zoom 1–4, follow with a dead zone, a clamp to bounds and a
  shake. This sweep asks 1120, in an edit to its scope, for damped
  follow (a whole-number fraction of the gap each step) and a shake that
  decays over steps, scaled by the player's settings from 2340.
- 2109: integer ease curves in `shuyi_core::ease`.
- 2318 (parked): draws the camera's dead zone, bounds and target, and
  each LDtk level's rectangle.
- 2361 (parked): draws world layers between the last two steps, with a
  `snap` on the camera for cuts.
- 3106 (parked): 3D cameras.

No ticket plans look-ahead, rooms or blends. Why they matter:

- **The game first.** Visions of Shuyi's battle uses the tile camera
  and needs none of this. If its ticket 0060 says the army walks around
  freely, its free-moving stage gets 1120's camera, and this ticket
  gives that stage rooms and blends from a data file instead of code in
  the game.
- **Other 2D games.** A platformer (1127) needs look-ahead, so the
  player sees where they run, and a vertical rule, so a jump doesn't
  bob the screen. An adventure game or a Metroidvania keeps the camera
  inside the room the player is in and slides to the next one. A boss
  door or a short scene moves the camera to a point and back.
- **The end goal** (2026-10-19: a game "like snake or mario" in under
  an hour): an agent gets a camera that feels right from a `camera.ron`,
  in integers that step the same on the web, the Playdate and the GBA.
  It doesn't write its own float smoothing, which would break replays.

The camera stays view state, as 1120 says: it is stepped once per
simulation step in `Fx`, read by the frame and the debug view, and kept
out of `TickSim::hash` unless a game puts it there.

## Nick input

None.

## Scope

**In:**
- **`CameraTarget`**, in the module where 1120 put `Camera`
  (`shuyi_ui::frame`): `{ at: FxVec2, velocity: FxVec2, grounded: bool }`,
  with `CameraTarget::still(at)` (no velocity, grounded) and
  `From<FxVec2> for CameraTarget` through `still`. 1120's per-step
  follow call takes `impl Into<CameraTarget>`, so a caller that passes a
  point (1120's test scene, 1125's town if it is done, a game) compiles
  unchanged and gets no look-ahead.
- **Look-ahead:** each step the camera aims at a point ahead of the
  target, `velocity × lead_steps`, clamped on each axis to `±max` world
  pixels. The offset moves toward that aim by its own fraction each
  step, with 1120's damping rule (a move that rounds to zero moves the
  rest). So turning around slides the view instead of jumping it, and a
  still target brings the offset back to zero.
- **A vertical rule** for side views: `Vertical::Always` (the default)
  or `Vertical::OnGround`. With `OnGround`, the height the camera aims
  at changes only on steps where the target is grounded, or when the
  target leaves the dead zone up or down. A jump that stays inside the
  dead zone and lands at the same height leaves the camera still, and a
  long fall is still followed.
- **Rooms:** `Rooms`, a list of rectangles in world pixels (the rect
  type 1120's bounds use).
  - The target's room is the room it was in, while it is still inside
    it. Otherwise it is the first room in list order that holds the
    target's point. A target in no room keeps the last one. So
    overlapping rooms don't flicker. With no rooms, or before the
    target is first in one, `room()` is `None` and 1120's bounds apply.
  - While following, the camera is clamped to its room instead of
    1120's bounds. On an axis where the room is smaller than the view,
    the camera centres on the room, the way `clamp_axis` in
    `crates/ui/src/grid/camera.rs` centres a small map.
  - `RoomMove::Snap`: on a room change, the camera is in the new room on
    the same step. `RoomMove::Slide { steps, ease }`: it blends there
    over `steps` (below), and `Camera::sliding()` is true on those
    steps. A stage that holds its simulation during the slide (rooms in
    the Zelda way) still steps the camera. One that keeps running (the
    Metroid way) does nothing more.
  - `Camera::room() -> Option<usize>`.
- **Goals and blends:**
  - `CameraGoal::Follow` (the target, with everything above) or
    `CameraGoal::Hold { at: FxVec2, zoom: u8 }` (a fixed point, clamped
    to 1120's bounds but not to rooms);
  - `Camera::blend_to(goal, steps, ease)`: the shown position goes from
    where it is to the goal over `steps`, eased by an `Ease` from
    `shuyi_core::ease` (2109) at `step / steps`, mixed with 2109's
    `Lerp`. A `Follow` goal stays live during the blend, so the blend
    lands on a moving target;
  - a new `blend_to` during a blend starts from the shown position, so
    the view never jumps. `Camera::cut_to(goal)` jumps at once;
  - the zoom is whole (1120), so a blend changes it in one step, at
    step `steps / 2`;
  - `Camera::set_def(def)` changes the numbers from the next step. A
    game with several cameras (the player's, a boss arena's, an
    overview) keeps one `Camera` and moves it between goals and defs, so
    the frame and the debug view read one camera.
- **The order within a step**, documented on the step function and
  tested: look-ahead, the aim point and the vertical rule; then 1120's
  dead zone and damping; then the room clamp and any room move; then the
  blend; then 1120's shake, which only moves the drawn view.
- **`CameraDef`**, a new asset kind (ADR-0007, ADR-0019):
  `shuyi_assets::camera::CameraDef` (`crates/assets/src/camera.rs`,
  `no_std` like the rest of the crate) with `impl AssetKind`
  (`NAME = "camera"`), and its format `CameraRon` (id `camera`,
  extensions `["ron"]`, `max_bytes` `TEXT_MAX_BYTES`), registered in
  `Formats::register_standard` (`crates/assets/src/format.rs`), since it
  needs nothing of the game (ADR-0019 §7). It holds every number of the
  camera: 1120's (zoom, dead zone, follow fraction, shake), under the
  names 1120 gave them, and this ticket's:

  ```ron
  (
      // 1120's fields, as 1120 names them, then:
      look_ahead: Some((lead_steps: 16, max: (48, 16), follow: (1, 16))),
      vertical: OnGround,
      room_move: Slide(steps: 30, ease: InOut(Sine)),
      blend: (steps: 40, ease: Smooth),
  )
  ```

  - every number is whole: pixels, steps, and fractions as
    `(num, den)`. Curves are 2109's `Ease` values, written as 2109
    writes them in RON;
  - the format reports each of these with its file and place: a
    fraction with `den` 0 or `num` over `den`, a zoom outside 1–4, a
    slide or blend of 0 steps, a look-ahead cap over 1024 pixels;
  - it derives `Serialize`, `Deserialize`, `PartialEq` and `Debug`, so
    the pack bakes it (`Bakeable`, `crates/assets/src/pack/bake.rs`);
  - a row in the table of built-in formats in `docs/formats.md`.
- **A text dump**, `Camera::dump()`, one line: position, zoom, room,
  look-ahead offset, goal and blend progress
  (`camera (412.50, 96.00) zoom 2 room 1 look (24.00, 0.00) follow blend 12/30`).
  Tests compare dumps.
- **Rooms from a map. Either order with 1124:** if 1124 is done, add
  `Rooms::from_map(&TileMap, layer: &str)`, one room per box in that
  object layer, and `Rooms::from_world(&MapWorld)`, one room per level
  rectangle of a world (1124's `MapWorld`: an LDtk file's levels or a
  Tiled `.world`'s maps). 1124 keeps those rectangles since its
  2026-10-20 edit. If 1124 isn't done, 1124 adds both. If 1124 moved
  `MapWorld` to a follow-up ticket, `from_world` goes with it.
- **Cuts between two steps. Either order with 2361:** if 2361 is done,
  `cut_to` and `RoomMove::Snap` set the camera's `snap` on the step they
  jump, so the view isn't drawn between the old place and the new one.
  If 2361 isn't done, 2361 does it (its scope says so).
- **The debug view. Either order with 2318:** if 2318 is done, its
  `layout` channel (`shuyi_ui::debug::layout`) also draws each room as a
  `Rect` with the role `Bounds` and its index as a `Label`, the
  look-ahead point as a `Cross` with the role `Note`, and during a blend
  its goal as a `Cross` with the role `Note` (2315's gizmos). If 2318
  isn't done, 2318 draws them.
- **The diorama. Either order with 3107:** 3107's camera reads the 2D
  camera's position and zoom, so it gets look-ahead, rooms and blends
  with no code of its own (its clause says so). If 3107 is done, its
  fixture stage's snapshots pass without edits, as 1120's do; if not,
  nothing to do.
- **The book. From 2802 (done):** its chapter 4
  (`site/content/learn/book/frame-and-skins.md`) gets a section on the
  camera file (`camera.ron`).
- **What changes for a game:** additive. 1120's follow call takes a
  point as before, and `CameraDef` is a new kind. The PR says so.

**Out (do not do):**
- damped follow, the decaying shake, and scaling that shake by the
  player's settings (1120's, from this sweep's edit to 1120);
- the player's shake strength and reduced-motion settings and their
  option rows (2340's);
- a spring that overshoots (one damping model is enough in 2D),
  non-whole zoom, and rotation;
- split screen and several views (2386, through 1120's views). Each
  view has its own `Camera`, so it gets all of this with nothing more
  here;
- rails and paths the camera rides (2122's `Curve` and its follower);
- keyed tracks in the cinematic file (2364). A cinematic that moves the
  camera reads its own track and calls `blend_to` or `cut_to`;
- drawing the camera between two steps (2361), beyond `snap` above;
- what the camera steps on under game speed and hit-stop (2123). This
  ticket steps it once per simulation step, as 1120 does;
- the platformer example's camera (1127, which waits for this ticket
  and uses its look-ahead and rooms);
- the walkable town's camera (1125): it uses look-ahead and rooms if
  this ticket is done first, and is left as it is if not;
- 3D cameras (3106);
- the grid kit's tile camera, which stays as it is.

## Implementation steps

1. Read 1120's `Camera` (its follow, damping, bounds, shake and rect
   type), 2109's `shuyi_core::ease` (`Ease`, `Lerp`), 2361's camera
   `snap` if 2361 is done, and 1124's `TileMap` and `MapWorld` if 1124
   is done. If 1120 is done without damped follow, a shake that decays
   over steps, or that shake scaled by 2340's settings, write a ticket
   for what is missing (the `write-ticket` skill), add it to this
   ticket's `blocked_by`, and stop.
2. Write `CameraDef` and `CameraRon` with their checks. Register the
   format and add its row to `docs/formats.md`. In
   `crates/assets/src/format/builtin_tests.rs`, add a `named` line to
   `every_built_in_has_its_id_extensions_and_limit`, a test of each bad
   value, and the format to `no_bytes_panic_a_ron_sidecar` with an
   `edited` fixture. In `crates/assets/src/pack/tests.rs`, add a camera
   file to `sources()`, the kind to `every_kind()`, and a `form!` line
   to `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`;
   as that test's comment says, make `pack::VERSION`
   (`crates/assets/src/pack.rs`) the next number and accept the
   snapshot.
3. Add `CameraTarget`, look-ahead and the vertical rule.
4. Add `Rooms`, the room rule and the room moves.
5. Add goals, `blend_to`, `cut_to`, `set_def`, and the order within a
   step.
6. Write the dump, the property tests, the golden table and the
   snapshots.
7. Do the either-order parts for 1124, 2361, 2318, 2802 and 3107.
8. Run the gates. If the PR passes about 600 changed lines, not
   counting fixtures and snapshots, move goals and blends (step 5) and
   their acceptance lines to a follow-up ticket (the `write-ticket`
   skill, parked like this one), and say so in the Completion notes.

## Acceptance criteria

- [ ] With rooms and no blend running, the view stays inside the
      target's room on each axis where the room is at least as big as
      the view, and is centred on the room where it isn't (property
      test).
- [ ] For a still target, the camera reaches its resting point exactly
      and stays there, and the look-ahead offset returns to zero, within
      a number of steps the docs give from the def's fractions (property
      test).
- [ ] A blend to a held point ends exactly on it at its last step. With
      an `Ease` that doesn't overshoot (not `Back` or `Elastic`), it
      stays on each axis between where it started and the point
      (property test).
- [ ] A target walking from room 0 into room 1: with `Snap` the camera
      is in room 1 on that step; with `Slide { steps: 30, .. }` it is
      there after exactly 30 steps, and `sliding()` is true on those
      steps only.
- [ ] With `OnGround`, a jump that stays inside the dead zone and lands
      at its start height moves the camera vertically by 0 pixels, and a
      fall past the dead zone is followed.
- [ ] A caller that passes a point to 1120's follow call is unchanged:
      1120's own camera tests and its test scene's snapshots pass
      without edits.
- [ ] `CameraRon` has its line in
      `every_built_in_has_its_id_extensions_and_limit`; random bytes and
      edited fixtures never make it panic
      (`no_bytes_panic_a_ron_sidecar`); each bad value listed above is
      reported with its file and place; a baked camera file reads back
      equal to its source.
- [ ] A checked-in table of dumps for a 600-step scripted target path
      matches on native and on `wasm32`, where 1119 checks its `Fx`
      table. The table is a golden table (2116's
      `shuyi_assets::golden`), checked by a test with `golden` in its
      name.
- [ ] 1120's test scene, given two rooms, has `frame-png` snapshots
      before, during and after a slide.
- [ ] A tick record's replay hashes are the same with these camera
      features on and off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the look-ahead aim and its clamp; the vertical rule; the room
  rule with overlapping rooms and a target in no room; `Snap` and
  `Slide`; `blend_to` during a blend; the zoom change at `steps / 2`;
  `set_def`; a point converted to a `CameraTarget`; each `CameraRon`
  error; if 1124 is done, `Rooms::from_map` and `Rooms::from_world` on
  1124's Tiled and LDtk fixtures.
- Property: room containment, convergence and blend bounds, as above;
  the same dumps twice from the same target path.
- Snapshot / integration: the test scene at three moments; the golden
  table; the pack round trip and the pinned binary form; the built-in
  format tests.

## Completion notes

*(Filled in by the session that completes the ticket.)*
