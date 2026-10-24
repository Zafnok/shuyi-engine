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
targets. 3104 skins meshes on the GPU from one clip at a time (the 3D
view's `MeshDraw.pose`, 3103). 3103's software rasterizer draws skinned
meshes in their bind pose. What is missing is playing more than one
clip on a skeleton at once, and morph targets.

ADR-0011 §4 samples clips in drawing only, in floats, and gameplay
never reads a bone. 3102 keeps a clip's values as `f32`, and 3104
samples them in `f32` on the CPU. So blending is a weighted sum of
poses in `f32`, in drawing, where 3104 samples clips. It never reaches
the rules, which stay in fixed point.

## Nick input

None.

## Scope

**In:**
- **Blending:** an `AnimationMix` on a skeleton: a list of (clip, time,
  weight); the pose is the weights' sum of each clip's pose, with
  rotations blended by normalized quaternion sums, in `f32` in 3104's
  clip sampler. Cross-fades over a time, and layers with a bone mask (an
  upper body that waves while the legs walk).
- **Morph targets:** 3102's reader keeps glTF morph targets (position
  and normal deltas, and their default weights) and clips that animate
  weights. The GPU painter (3104) applies them before skinning. The
  software rasterizer (3103) doesn't skin (it draws the bind pose), so
  it applies them to the bind pose.
- **The 3D view carries them.** 3103's `MeshDraw.pose` becomes an
  `AnimationMix` (one clip at weight 1 is today's `Pose`), and
  `MeshDraw` gains `morph_weights` (empty: the mesh's defaults; the
  weights the mix's clips animate are sampled in drawing, in `f32`,
  where 3104 samples clips). The draw's snapshot line prints each
  (clip, time, weight, mask) and the morph weights; one clip at weight
  1 and no weights print as before, so 3103's golden text holds.
  3104's skinning blends the mix with this ticket's `f32` blend, in its
  clip sampler.
- Fixtures: a small glTF of our own with two clips and one morph target.

**Out (do not do):**
- animation state machines for 3D (a follow-up if a game asks, built
  like 2104's 2D ones);
- inverse kinematics.

## Implementation steps

1. Read 3102's types and ADR-0011's numbers.
2. Write the mix and its `f32` blend in 3104's clip sampler, with
   tests.
3. Carry the mix and the morph weights in the 3D view. Add morph
   targets to the reader and both painters.
4. Run the gates.

## Acceptance criteria

- [ ] A mix of one clip at weight 1 equals the clip's pose exactly.
- [ ] A cross-fade's pose moves from one clip to the other with no jump
      (property test on the step between frames).
- [ ] A morph target at weight 1 moves its vertices by its deltas
      exactly in the software rasterizer (its pixel hash), and in the
      GPU painter within the tolerance of 3104's look test in the
      `render` job.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: blend weights; bone masks; quaternion normalization.
- Property: blending is the same on two runs and in the web build.
- Snapshot: a morphed mesh's 3D view text and its raster pixel hash.
- Look test: the same mesh in the GPU painter, in the `render` job at
  3104's tolerance.

## Completion notes

*(Filled in by the session that completes the ticket.)*
