---
id: "3112"
title: "Parked: 3D views drawn between the last two steps: the camera, mesh draws and billboards blended in the view only, so 3D stages move smoothly on displays faster than 60 Hz"
type: feature
milestone: "Parked: 3D"
model: reasoning
effort: high
status: blocked
blocked_by: ["2361", "3101", "3103", "3106"]
nick_input: none
completed:
---

# 3112 — 3D views drawn between the last two steps

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"; and 2026-10-20 for 2361, as its note quotes, `docs/decisions.md`): a follow-up of the parked 3D view (3103) and cameras (3106), and of the parked drawing between steps (2361), and a follow-up of parked work is parked. So it starts after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`), after the 2D work already planned, and after all three are unparked and done. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2361 draws 2D world layers between the last two steps, so a tick stage
moves smoothly on a display faster than its 60 steps a second. Its Out
list leaves 3D views and cameras (3103, 3106) to this ticket. 3106's
Out list leaves drawing its cameras between two steps to it too.

3D needs it more than 2D. At 144 or 165 Hz a step shows for 2 frames,
then 3, in turn (2361's Context). In a 2D stage a scrolling camera
judders. In a first-person stage every turn of the head moves the whole
picture, so the whole picture judders. Godot's physics interpolation
works in 3D as in 2D, and Unity interpolates a `Rigidbody`.

What it builds on:

- **2361** (parked): `Overstep` (thousandths of a step) in
  `shuyi_ui::tick`, the overstep handed to screens
  (`FrameInput::overstep()`, `None` on a 1-bit display or a backend
  that shows frames at a fixed rate), `BetweenSteps` on the headless
  platform and the Harness, `ItemKey` and `snap` on sprite items,
  `StepViews` for world layers, and its ADR: a view may be drawn
  between steps, in the view only.
- **3103** (parked): `View3d { camera, draws, billboards, lights, fog,
  effects }` in `shuyi_3d::view`, plain data in `Fx12`, put in the
  frame as a module layer; its snapshot lines and golden text; and the
  software rasterizer (`shuyi_3d::raster::render`).
- **3106** (parked): the camera rigs (`CameraRig`), stepped in the tick
  in `Fx12`, and `CameraPose`, which becomes a `Camera3` through
  `From<CameraPose> for Camera3`. Its `OrbitCamera` pulls in to a wall
  at once and eases back out.
- **3101** (parked): `FxTransform { rot: FxMat3, pos: FxVec3 }`,
  `FxQuat` with `nlerp`, `renormalize` and `to_mat3`, `Fixed<F>`, and
  `isqrt_u64`.

It keys draws with 2361's `ItemKey`. If 2361 left item keys to a
follow-up (its *Size* line), that follow-up is a blocker here too, and
2361's PR puts it in `blocked_by`.

2361's rules hold here:

- the simulation, its records and its hash don't change;
- the stage keeps the views it built after each of the last two steps,
  and a pure function blends them, in integers;
- the painters draw what they get and keep nothing between frames;
- it is off by default, and a smoothed view is drawn up to one step
  (16.7 ms) behind the newest state;
- no extrapolation.

What is new in 3D is rotation. A 2D sprite's angle shows the newest
step (2361's Out). A 3D camera turns, and a turn that judders is what a
first-person player sees most. So rotations are blended too: a
normalized blend (`nlerp`) of 3101's quaternions, for transforms that
are pure rotations.

## Nick input

None.

## Scope

**In:**

- **The overstep in `Fx12`.** 2361's `Overstep::lerp` takes `Fx` (and
  `FxVec2`). Make it generic over 3101's `Fixed<F>`, with the same
  rounding, so `Fx`'s results don't change. Add
  `Overstep::thousandths(self) -> u16` if 2361 gave it no getter.
- **Rotations from matrices.**
  `FxQuat::from_rotation(m: &FxMat3) -> Option<FxQuat>` in
  `shuyi_3d::space`, in integers. Use Shepperd's method: start from the
  largest of the trace and the three diagonal entries, so no root is
  taken of a small number. The root is 3101's `isqrt_u64`.
  - It gives `None` when `m` isn't a pure rotation: a column whose
    squared length is further from 1 than the bound 3101 documents for
    `to_mat3`, or a negative determinant (a mirror).
  - The quaternion it gives has `w >= 0`.
  - It is outside `draw`, so `clippy::float_arithmetic` still holds.
- **New fields on 3103's view types**, in `shuyi_3d::view`:
  - `View3d` gains `smoothed: bool`, false by default;
  - `Camera3` gains `snap: bool` (a cut: draw it where it is now);
  - `MeshDraw` and `Billboard` gain `key: Option<ItemKey>` (2361's)
    and `snap: bool` (it teleported this step);
  - they are views under ADR-0039 (2013), `#[non_exhaustive]` with
    constructors, so the new fields are additive. The constructors
    leave them off.
- **`blend(prev: &View3d, now: &View3d, at: Option<Overstep>) ->
  View3d`**, pure, in `shuyi_3d::view`. It returns `now` with:
  - the camera's transform blended (below), unless `now`'s camera is
    snapped;
  - each keyed draw's transform, and each keyed billboard's `pos`,
    blended between where it is in `prev` and in `now`. Draws are
    matched with draws, and billboards with billboards;
  - an unkeyed, new or snapped draw or billboard where it is in `now`;
    a key missing from `now` is gone.

  `None`, or `now.smoothed` false, gives `now` unchanged. Everything
  else is `now`'s: the projection, near and far, the mesh, material,
  pose and order, the billboard's sprite, size and mode, the lights,
  fog and effects. A key that repeats among the draws, or among the
  billboards, panics in a debug build and draws at `now` in a release
  build. Keys are matched through a sorted list of `prev`'s keys, with
  no hashing, so it builds without `std`.
- **A transform's blend:**
  - the position: `Overstep::lerp` per axis, in `Fx12`;
  - the rotation: equal `rot`s are copied as they are, and overstep 0
    copies `prev`'s, so a still or new thing never wobbles from
    rounding;
  - otherwise both `rot`s go through `from_rotation`. If both are
    rotations, the result is `nlerp(a, b, t).to_mat3()`, with `t` the
    overstep as `Fx12` (`Fx12::from_ratio(n, 1000)`). It takes the
    shorter way: if 3101's `nlerp` doesn't, negate `b` when the dot
    product is negative;
  - if either isn't a rotation (a scaled or mirrored draw), the
    rotation is `now`'s and only the position blends.
- **`StepViews3`** in `shuyi_3d::view`. It keeps the `View3d` a stage
  built after each of the last two steps, as 2361's `StepViews` keeps
  world layers:
  - `push(view)` after each step;
  - `set_overstep(..)` in the stage's update, from
    `FrameInput::overstep()`;
  - `drawn(&self) -> View3d` in its draw, which the stage puts in the
    frame as its module layer, where it put the view before. With one
    view so far, `drawn` gives it as it is;
  - a stage that isn't updated (under a pause overlay, say) keeps the
    overstep it last read, so the picture holds still (2361's *Holding
    still*).
- **Cameras that move at once.** 3106's `CameraRig::step` returns a
  `CameraMove`, a new `#[non_exhaustive]` enum in `shuyi_3d::camera`:
  `Eased` or `AtOnce`.
  - `OrbitCamera` returns `AtOnce` on the step it pulls in to a wall
    at once. Easing back out is `Eased`.
  - `FirstPersonLook` and `OverheadFollow` always return `Eased`.
  - The stage sets that step's `Camera3::snap` from it. A doc example
    on `CameraRig` shows it:
    `camera.snap = moved == CameraMove::AtOnce`.
  - It is a return value, not a field, so the rigs' state, their dump
    lines and the hash don't change. It isn't `#[must_use]`: a stage
    with smoothing off ignores it.
  - A stage that cuts on its own (it switches rigs, or teleports the
    player) sets `snap` itself, as in 2D.
- **Snapshot lines.** A view with `smoothed` false writes the same
  lines as before, so 3103's golden text holds. A smoothed view's
  camera line ends with `smoothed`, and `snap` when set. Its draw and
  billboard lines end with `key=N` when keyed, and `snap` when set.
  The lines show the blended values, as exact `Fx12` decimals (3103's
  rule).
- **ADR-0011 §5** gains a paragraph: a 3D view may be drawn between
  the last two steps, as 2361's ADR allows for world layers. It gives
  this ticket's rules for rotations (blended only when both are pure
  rotations, copied when equal) and for `CameraMove`, and points at
  2361's ADR by its number.
- **Docs.** `View3d::smoothed`'s doc says what 2361's `Smoothing` doc
  says: 120 and 240 Hz are already even, 60 Hz with VRR needs nothing,
  and a smoothed view is drawn up to one step behind. It also says that
  a scaled draw turns at the step. `crates/3d/README.md`'s how-to
  section gets a short paragraph with the stage's three calls.
- **No API break for the game.** 3D is parked, and the game uses none
  of it. `Overstep::lerp` becoming generic keeps every call on `Fx`
  compiling. If CI's `api` job reports a break, write a `Breaking:`
  line in the Completion notes (ADR-0038).

**Out (do not do):**

- 2D world layers (2361 does them);
- lights, fog and effects: they show the newest step, so a point light
  carried by a smoothed draw is up to one step ahead of it;
- a draw's pose, animation mix and morph weights (3110), a billboard's
  size and frame, and a camera's field of view, near and far: they show
  the newest step, as 2D animation frames and zoom do in 2361;
- painting: 3103's raster and 3104's GPU painter draw the blended view
  as they draw any view, and keep nothing between frames;
- the diorama (3107): it builds its view each frame from the 2D world
  layers the stage draws, which 2361's `StepViews` blends, so it needs
  nothing here;
- 3D gizmos (2320): they are made from the newest state and show it,
  up to one step ahead of a smoothed draw; a developer turns smoothing
  off to line them up;
- a scaled or mirrored draw's rotation: it turns at the step;
- slerp: `nlerp` is enough between two steps a sixtieth of a second
  apart;
- extrapolation (drawing past the newest step);
- the simulation, collision and the rigs' state: nothing there changes.

## Implementation steps

1. Read 2361's ADR, `Overstep`, `StepViews` and `blend`
   (`shuyi_ui::tick`, `shuyi_ui::frame`); 3103's `shuyi_3d::view`, its
   snapshot lines and golden test; 3106's `CameraRig`, `CameraPose` and
   `OrbitCamera`, and its fixture corridor; 3101's `FxQuat`, `nlerp`
   and the bound it documents for `to_mat3`.
2. Make `Overstep::lerp` generic over `Fixed<F>`. 2361's tests pass
   unchanged.
3. Write `FxQuat::from_rotation`, with its tests.
4. Add the new fields. Write `blend` for the camera first: it needs no
   keys, and a turning camera is where the judder shows. Then the keyed
   draws and billboards.
5. Write `StepViews3`.
6. Add `CameraMove` and the rigs' return values. 3106's tests and rig
   dumps pass unchanged.
7. Write the snapshot lines. 3103's golden test passes unchanged.
8. Write the stepped fixture, in `crates/3d/tests/`: a tick stage over
   3103's fixture scene. Each step its `TickSim` moves the cube 1/8 m
   along x and turns it 1/128 of a turn (512 steps of `Angle16`) about
   the vertical axis, moves a billboard 1/16 m along z, and moves the
   camera 1/8 m along x while turning it 1/256 of a turn. After each
   step the stage builds a `View3d`, with the cube and the billboard
   keyed, and pushes it into `StepViews3`. Then write the tests below.
9. Write ADR-0011's paragraph and the docs.
10. Run the gates.

## Acceptance criteria

- [ ] Off by default: 3103's golden snapshot and fixture pixel hash,
      and 3106's tests and rig dumps, are unchanged.
- [ ] Determinism: the stepped fixture played by the Harness for 600
      frames of 1/144 s with `BetweenSteps::Clock` gives the same hash
      list per step, and the same steps per frame, with `smoothed`
      false and true.
- [ ] The judder, measured in that run from the drawn camera's
      position: with `smoothed` false, it changes only on frames that
      ran a step; with it true, it changes on every frame once two
      steps exist, and the largest and smallest change differ by at
      most 4 raw units of `Fx12`.
- [ ] `blend`: overstep 0 gives `prev`'s camera and draws, their
      transforms equal to `prev`'s bit for bit; `None`, or `smoothed`
      false, gives `now`; a snapped camera, and snapped, unkeyed and
      new draws and billboards, are at `now`; a key gone from `now`
      isn't drawn.
- [ ] Rotation: at overstep 500, between a yaw of 0 and a quarter
      turn, the drawn camera's yaw (`Angle16::atan2` of its forward
      vector) is an eighth of a turn, within 16 steps of `Angle16`.
      Equal rotations are copied bit for bit at every overstep. A draw
      scaled by 2 keeps `now`'s matrix and blends its position.
- [ ] `FxQuat::from_rotation` gives back, through `to_mat3`, every
      matrix built by `FxQuat::from_axis_angle(..).to_mat3()`, within
      3101's bound, and `None` for a scaled or a mirrored matrix.
- [ ] Pinned oversteps: on the stepped fixture, after some steps,
      `BetweenSteps::Pinned` at 0, 500 and 999, each followed by one
      frame in which no time passes, give snapshot lines with the
      camera at `prev`'s position, halfway, and 999 thousandths of the
      way to `now`'s. `shuyi_3d::raster::render` of the view at 500
      differs from the view at 0.
- [ ] The orbit camera, in 3106's fixture corridor: on the step it
      pulls in to a wall, `step` returns `AtOnce`, and the drawn camera
      at overstep 500 is at `now`'s place, not between.
- [ ] Holding still: with a screen pushed over the stepped stage, 30
      frames of 1/144 s with `BetweenSteps::Clock` give the same
      snapshot each time.
- [ ] `clippy::float_arithmetic` finds nothing in `shuyi_3d::space`
      or in `blend`.
- [ ] ADR-0011 §5 has the paragraph, and it points at 2361's ADR.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `from_rotation` on hand-worked matrices (the identity, a
  quarter turn about each axis, a half turn, which has the smallest
  trace) and its `None` cases; the generic `lerp` on `Fixed<12>`,
  rounding toward negative infinity when moving down too; each `blend`
  case; `StepViews3` pushing and holding; `CameraMove` from each rig;
  the snapshot lines of a smoothed view.
- Property: for any two rigid transforms and any overstep, the blended
  position lies between `prev`'s and `now`'s on each axis, inclusive;
  the blended rotation passes `from_rotation`, and its angle from
  `prev`'s is at most the angle from `prev`'s to `now`'s plus 3101's
  bound; `blend(prev, now, None) == now`; `blend` changes nothing but
  the camera's, the draws' and the billboards' places and turns.
- Integration (Harness): the hash lists on and off; the judder test;
  the pinned oversteps and their pictures; the orbit pull-in; the
  pushed screen holding still.
- Manual: if a display faster than 60 Hz is at hand (VRR off, the cap
  at its rate), run a 3D example of the repository, if one exists,
  with smoothing on and off while turning the camera, and say in the
  PR what you saw. If none is at hand, say so.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
