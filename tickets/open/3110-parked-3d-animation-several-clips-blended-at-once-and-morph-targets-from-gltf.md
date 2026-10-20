---
id: "3110"
title: "Parked: 3D animation: several clips at once with weights and smooth blends, and morph targets read from glTF"
type: feature
milestone: "Parked: 3D"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["3102", "3103", "3104"]
nick_input: none
completed:
---

# 3110 — 3D animation: blending and morph targets

> **Parked** (Nick, 2026-10-17: "add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D", parked like the rest of 3D; and 2026-10-20, Bevy features already parked stay parked, `docs/decisions.md`): it follows 3102, 3103 and 3104. Nobody starts it until Nick unparks 3D. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): every feature
on Bevy's homepage. Its animation: skeletal rigs, "multiple animations
can play at once with smooth blending", blend shapes (morph targets),
and import from glTF.

3102 reads meshes, skeletons and clips from glTF and leaves out morph
targets. 3103 and 3104 draw skinned meshes. What is missing is playing
more than one clip on a skeleton at once, and morph targets.

ADR-0011 keeps 3D in fixed point (the 20.12 space), so blending is a
weighted sum of poses in fixed point, the same on every device.

## Nick input

None.

## Scope

**In:**
- **Blending:** an `AnimationMix` on a skeleton: a list of (clip, time,
  weight); the pose is the weights' sum of each clip's pose, with
  rotations blended by normalized quaternion sums in fixed point.
  Cross-fades over a time, and layers with a bone mask (an upper body
  that waves while the legs walk).
- **Morph targets:** 3102's reader keeps glTF morph targets (position
  and normal deltas, and their default weights) and clips that animate
  weights. The software rasterizer (3103) and the GPU painter (3104)
  apply them before skinning.
- Fixtures: a small glTF of our own with two clips and one morph target.

**Out (do not do):**
- animation state machines for 3D (a follow-up after 2104's 2D ones);
- inverse kinematics.

## Implementation steps

1. Read 3102's types and ADR-0011's numbers.
2. Write the mix and its fixed-point blend, with tests.
3. Add morph targets to the reader and both painters.
4. Run the gates.

## Acceptance criteria

- [ ] A mix of one clip at weight 1 equals the clip's pose exactly.
- [ ] A cross-fade's pose moves from one clip to the other with no jump
      (property test on the step between frames).
- [ ] A morph target at weight 1 moves its vertices by its deltas
      exactly, in both painters (snapshots).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: blend weights; bone masks; quaternion normalization.
- Property: blending is the same on two runs and in the web build.
- Snapshot: a morphed mesh in both painters.

## Completion notes

*(Filled in by the session that completes the ticket.)*
