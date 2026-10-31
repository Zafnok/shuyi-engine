---
id: "2320"
title: "Parked: 3D in the debug view: bones, colliders and the camera, drawn as lines in the 3D view"
type: feature
milestone: "Parked: 3D"
model: reasoning
effort: high
status: blocked
blocked_by: ["2315", "3102", "3103", "3105"]
nick_input: none
completed:
---

# 2320 — 3D in the debug view: bones, colliders, the camera

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): a follow-up of the parked 3D tickets 3102, 3103 and 3105, so it starts after they are unparked and done, and after the debug view (2315). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): see "collisions bounds and
skeletons in realtime (debug mode)". In 3D (ADR-0011):

- skeletons are glTF skins (3102's `Skeleton` and `Clip3`), posed in
  the 3D view (3103's `MeshDraw` with a `Pose`);
- collision is 3105's `World3`: triangles, `Sphere`, `Capsule` and
  boxes, in 20.12 fixed point.

2315's gizmos are 2D, in console pixels. A 3D gizmo has to be
projected by the 3D view's camera and depth-tested against the scene,
or drawn on top of it. So this ticket adds 3D gizmos to the 3D view as
plain data. Both painters draw them: the software rasterizer (3103)
and the GPU painter (3104, if done).

Gameplay never reads a bone (ADR-0011 §4). The bone channel only draws
the pose the view already computed for display. It must not become a
way for gameplay to reach bones.

## Nick input

None.

## Scope

**In:**
- **`shuyi_3d::view::Gizmo3`**: `Line3 { a, b }`, `Box3 { min, max,
  transform }`, `Sphere3 { centre, radius }` (three rings),
  `Capsule3 { a, b, radius }` (two rings and four lines), and
  `Cross3 { at, size }`. Each has a channel, a role (2315's
  `GizmoRole`) and `on_top: bool` (drawn over the scene, or
  depth-tested against it).
- **`View3d::gizmos: Vec<Gizmo3>`**, filled only when debug tools and
  the channel are on. The view's snapshot lists them, one line each,
  after the draws.
- **Painting:**
  - the software rasterizer draws them as one-pixel lines after the
    scene, depth-tested unless `on_top`;
  - the GPU painter (3104), if done, draws them as a line pass. If
    3104 isn't done, 3104 adds that pass.
- **Channels and helpers** in `shuyi_3d::debug`:
  - `bones`: each posed joint as a line from its parent (`Bone`), on
    top;
  - `collision3`: `World3`'s colliders (`Solid`), moving shapes
    (`Bounds`), and the character mover's last hits as crosses
    (`Note`), depth-tested;
  - `camera3`: another camera's frustum as lines (`Note`), for a
    second view of the same stage, never that camera's own.
    **Either order with 2506:** 2506 plans the editor's views. If 2506
    is done, add a line to its ticket for the editor's 3D view saying
    that view turns `camera3` on, or write that ticket, parked, if
    there is none. If not, nothing to do (2506 plans it).
- Call them from the third-person example (3108) if it is done. If it
  isn't, 3108 calls them when it lands.

**Out (do not do):**
- HD-2D's diorama skin (3107): its stage is 2D, so 2315–2318's
  channels cover it, and 3107 places their gizmos through its camera;
- the DS's hardware 3D (3025);
- a free-fly debug camera (3106 leaves it to a later ticket);
- editing anything in 3D.

## Implementation steps

1. Read ADR-0011, 2315's ADR, 3102, 3103 and 3105.
2. Add `Gizmo3` and `View3d::gizmos`, with snapshot lines.
3. Draw them in the software rasterizer, with depth.
4. Write the helpers and channels.
5. Add the GPU pass, if 3104 is done.
6. Run the gates.

## Acceptance criteria

- [ ] A two-joint arm (3102's fixture) posed at its clip's middle draws
      two bone lines whose ends match the joints' projected positions.
- [ ] A capsule behind a wall is hidden when depth-tested and shown
      when `on_top`.
- [ ] The simulation hash is the same with every 3D channel on and off.
- [ ] If 3108 is done, `frame-png --debug bones,collision3` on its
      third-person example is reviewed, and its snapshot is committed.
      If not, 3108 does it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the projection of each `Gizmo3` kind for a fixed camera; the
  ring count of spheres and capsules; one camera's `camera3` frustum,
  seen from a second camera.
- Snapshot / integration: the arm fixture with `bones`; a room with a
  capsule mover with `collision3`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
