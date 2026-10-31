---
id: "3106"
title: "Parked: 3D cameras: a first-person look from actions, a third-person orbit that pulls in at walls, and an overhead follow camera, each built for its marker"
type: feature
milestone: "Parked: 3D"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3105", "1121", "2510"]
nick_input: none
completed:
---

# 3106 — 3D cameras: first person, third person, overhead

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
§2 and §6. Each 3D direction has its own camera:

- **first person**: look from actions, a stick, or the pointer turned
  into an axis;
- **third person**: an orbit around the player that pulls in when a
  wall is in the way;
- **top-down 3D**: an overhead camera that follows with a dead zone.

Cameras are part of the simulation. A first-person game aims where the
camera looks, so camera state is in `Fx12`, stepped in the tick, and in
the hash. Each rig is built for one marker (ADR-0009 §3), so a
third-person orbit in a first-person stage doesn't compile.

A look never reads a key or a mouse (`AGENTS.md` rule 8). It reads
actions from `TickInput`, where sticks and the pointer are quantized
(1121), so records replay exactly. Read the `input-actions` skill first.

It builds on 3105 (shape casts for the orbit), 1121 (axis actions,
pointer input, the Harness's stick and pointer scripts) and 2510
(`cargo xtask compile-messages`, which checks what the compiler says).

3103 (the 3D view) and this ticket can land in either order. If 3103
has landed, add `From<CameraPose> for Camera3` here. If not, 3103 adds
it.

## Nick input

None.

## Scope

**In:**
- **`trait CameraRig<P: Perspective3>`** in `shuyi_3d::camera`, with
  `step(&mut self, look: &LookInput, target: FxVec3, world: &World3)`
  and `pose(&self) -> CameraPose`:
  - `CameraPose { transform: FxTransform, fov_y: Angle16 }` lives in
    `shuyi_3d::camera`, outside `draw`, so a headless server steps
    cameras without the drawing part;
  - `trait Perspective3: Perspective`, implemented by the three markers
    of 3101;
  - `#[diagnostic::on_unimplemented]` on `CameraRig`, so misuse says
    "`OrbitCamera` is built for `ThirdPerson`, not `FirstPerson`",
    naming both markers;
  - a stage holds `Box<dyn CameraRig<P>>` or a concrete rig for its own
    `P`.
- **`LookInput`**, built from `TickInput`:
  - a two-axis action `look` (a stick), as `i16` per axis;
  - the pointer's motion as a two-axis source. If 1121 didn't add
    relative pointer motion, add `AxisSource::PointerMotion` in
    `shuyi-ui::input` here, quantized to `i16` per step in `TickInput`,
    with pointer lock in `shuyi-app` (macroquad's cursor grab, on
    desktop and the web);
    **Either order with 3034:** if 3034 is done, lock the pointer the
    way its ADR says a game asks for the grab (macroquad's
    `set_cursor_grab` until the winit backend replaces macroquad); if
    not, use macroquad's cursor grab, and 3034 plans keeping it.
  - **Either order with 3044** (the idle cursor): a locked pointer is
    never shown. If 3044 is done, give its `IdleCursor`
    (`crates/app/src/window.rs`) a `locked: bool`, set while the lock
    holds, that hides the cursor and wins over mouse motion and 1121's
    `keep_shown`, with a unit test; if not, hide the cursor while the
    lock holds (`show_mouse(false)`), and 3044 adds `locked` (its scope
    says so).
  - sensitivity and invert-y are the player's settings in their keymap
    file, never constants in a game.
- **`FirstPersonLook: CameraRig<FirstPerson>`**: yaw and pitch as
  `Angle16`, pitch clamped (default ±85°), turn rate at full deflection
  in settings, eye height above the target.
- **`OrbitCamera: CameraRig<ThirdPerson>`**: yaw and pitch from the
  look, a distance, and a sphere cast (3105, radius 0.2 m) from the
  target toward the wanted place. On a hit it pulls in to the hit less a
  margin at once, and eases back out at a set speed when the way clears.
  A minimum distance.
- **`OverheadFollow: CameraRig<Overhead3D>`**: a fixed tilt and height
  over the target, a dead zone box in metres, clamped to bounds. An
  optional turn in 45° steps from two actions, eased.
- **Settings as data**: one RON file per rig, through the format
  registry as asset kinds `first_person_look`, `orbit_camera` and
  `overhead_follow`, each with a fuzz-style test (ADR-0007 §5).
- **A text dump** of each rig's state, one line.

**Out (do not do):**
- a free-fly debug camera (a later ticket if wanted);
- camera shake and cinematic paths in 3D;
- head bob and weapon sway (a game's job);
- drawing the camera between two steps on displays faster than 60 Hz
  (2361 does it for 2D world layers; 3112 for 3D views);
- the examples and templates (3108, 3109).

## Implementation steps

1. Add `Perspective3`, `CameraRig`, its diagnostic, and `LookInput`.
   Add the pointer motion source if 1121 lacks it, with `check-keys`
   passing.
2. Write `FirstPersonLook` with unit tests: clamp, wrap, rate.
3. Write `OrbitCamera` with a fixture corridor from 3105's kit.
4. Write `OverheadFollow`: dead zone, bounds, eased turn.
5. Write the RON settings and their formats.
6. Write a `compile_fail` doc test per rig, using it with a wrong
   marker, and a `compile-messages` fixture per rig (2510), with
   `crates/3d` added to the path dependencies of the package it writes.
7. Write the Harness tests.

## Acceptance criteria

- [ ] Each rig in a stage of another marker fails to compile
      (`compile_fail` doc tests, three), and the message names both
      markers (three fixtures in 2510's `cargo xtask compile-messages`;
      a `compile_fail` doc test checks only that the code fails, not
      what the compiler says).
- [ ] A scripted stick at half deflection turns the first-person look at
      half the rate (Harness test).
- [ ] Pitch never passes its clamp, over random input (property test).
- [ ] In the fixture corridor, the segment from the target to the orbit
      camera never passes through a wall, over random yaw and pitch
      (property test, 10 000 cases).
- [ ] The overhead camera doesn't move while the target stays in the
      dead zone, and never leaves its bounds (property test).
- [ ] The same record replays to the same rig dumps.
- [ ] No rig names a key or a button. `check-keys` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each rig's rules.
- Property: clamp, the corridor, dead zone and bounds, replay.
- Doc tests: the three `compile_fail` cases.
- Fixtures: the three message checks in `cargo xtask compile-messages`
  (2510).
- Integration: the Harness stick and pointer tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
