---
id: "3122"
title: "Parked: The pointer and the camera in 3D: a ray through the pointer cast against colliders and stored in the step's input, a pan-orbit and a pan camera, and a free-fly debug camera"
type: feature
milestone: "Parked: 3D"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3105", "3106"]
nick_input: none
completed:
---

# 3122 — 3D picking and camera rigs

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

3105 casts rays in Fx12, and 3106's rigs follow a target in first
person, third person or from overhead. Nothing builds a ray from the
camera through the pointer (3107's picking walks the diorama's tile
grid only), no rig orbits and pans freely around a point, and 3106 and
2320 leave "a free-fly debug camera" to a later ticket. The Bevy
coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found the 3D games this leaves out: 3D city builders, RTS,
board and tactics games and model viewers, all driven by a pan-orbit
camera and by clicking things in the world. Bevy has viewport rays,
mesh picking and camera controllers
(`crates/bevy_camera/src/camera.rs` `viewport_to_world`,
`crates/bevy_picking`, `crates/bevy_camera_controller/src/`;
`examples/3d/3d_viewport_to_world.rs`, `mesh_ray_cast.rs`,
`examples/camera/pan_orbit_camera_*.rs`, `free_camera_controller.rs`).

## Nick input

None.

## Scope

**In:**
- **The pointer ray:** `CameraPose::ray_through(pointer, viewport) ->
  Ray3` in Fx12 (no floats in the rules), outside the drawing code.
- **Picking:** a `WorldPick` for 3D views casting it against 3105's
  world, or a mesh's triangles within its bounds; the hit (entity and
  point) is stored in `TickInput` or the turn command, so a record
  replays without the view.
- **Rigs** on 3106's `CameraRig` trait: `PanOrbit` (a focus point, yaw
  and pitch as `Angle16`, a distance with zoom limits, pan in the view
  plane, an optional orthographic zoom) and `Pan`, from actions and the
  pointer's drag; their settings as RON asset kinds; sensitivity and
  invert from the keymap settings, never constants.
- **A free-fly debug camera** for the debug view only (2320), stepped by
  the debug view's input and never in a record.

**Out (do not do):**
- picking 2D items (2417 shapes 2D hits);
- gamepad aim assist;
- a camera editor.

**Estimated size:** about 1,200 lines (code 550, tests 450, docs 200).

**Split:** past 1,000 lines, split off the rigs (third and fourth
bullets) into a new ticket.

## Implementation steps

1. Read 3105, 3106, 3107's picking and 2320.
2. Write `ray_through` and its tests.
3. Write the 3D `WorldPick` and store hits in the input.
4. Write `PanOrbit`, `Pan` and the free-fly camera.
5. Write the docs page: clicking a piece on a 3D board.

## Acceptance criteria

- [ ] A click at a known pixel picks the known object on every target
  (golden fixture, equal on wasm32).
- [ ] A record with picks replays to the same hash without a view.
- [ ] `PanOrbit` keeps its focus at the centre of the view through any
  yaw and pitch within limits.
- [ ] The free-fly camera is absent from release builds and from
  records.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the ray, each rig's limits.
- Property: a ray through the centre pixel is the camera's forward
  direction within one unit.
- Snapshot / integration: a pick fixture; a replay.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
