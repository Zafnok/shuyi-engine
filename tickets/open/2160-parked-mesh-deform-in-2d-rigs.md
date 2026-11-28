---
id: "2160"
title: "Parked: Mesh attachments and deform keys in 2D rigs: vertices weighted to bones, and per-vertex offsets keyed in clips, drawn as textured triangles"
type: feature
milestone: "Engine: animation"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2105", "3262"]
nick_input: none
completed:
---

# 2160 — Mesh attachments and deform keys in 2D rigs

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): a follow-up of the parked skeletal module (2105), and a follow-up of parked work is parked, so it starts after 2105 is unparked and done. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2105 draws a bone rig as rotated sprites, one per slot. Its **Out**
leaves "mesh attachments and deformation", and 2217 (Spine) and 2216
(DragonBones) refuse rigs with meshes for that reason. The Bevy
coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found the games this leaves out: faces that change shape,
cloth flaps, squash and stretch, jelly and soft-looking characters.
Bevy draws skinned and morphed meshes
(`crates/bevy_animation/src/morph.rs`, `crates/bevy_mesh`;
`examples/animation/morph_targets.rs`, `custom_skinned_mesh.rs`).

3262 adds textured triangles to the frame, drawn by one integer rule in
every renderer. This ticket builds the rig side on top of it.

## Nick input

None.

## Scope

**In:**
- **A mesh attachment** in the rig RON: vertices in the slot's space,
  texel positions, triangles, and per-vertex bone weights in permille
  that sum to 1,000 (checked at load, with the file and line).
- **Deform keys:** a clip track keying per-vertex `Fx` offsets, eased
  through `shuyi_core::ease`.
- **The pose:** vertex positions resolved in fixed point from the bones
  and the deform keys, a pure function of the data and the step.
- **The skin:** the view gives 3262's triangle item one mesh per slot,
  so frame-png, the raster and the one-bit renderer draw it as 3262
  defines.

**Out (do not do):**
- reading Spine's or DragonBones' meshes: 2217 and 2216 (parked) lift
  their refusals and read into this format when they are unparked;
- 3D morph targets (3110, parked);
- free-form deformation lattices.

**Estimated size:** about 1,100 lines (code 500, tests 450, docs 150).

**Split:** past 900 lines, split off deform keys into a new ticket.

## Implementation steps

1. Read 2105's code and 3262's triangle item.
2. Add the attachment to the rig format and its load checks.
3. Resolve weighted vertices in the pose; add the deform track.
4. Hand the triangles to the skin.
5. Write the docs page: a face that blinks by a deform key.

## Acceptance criteria

- [ ] A mesh whose vertices are all weighted to one bone moves exactly
  as that bone's sprite would.
- [ ] Weights that don't sum to 1,000 fail to load with the file and
  line.
- [ ] A deform key halfway through a clip gives half the offset under a
  linear ease.
- [ ] A golden row of resolved vertices is equal on wasm32.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the load checks, weighted vertices, the deform track.
- Property: any weights summing to 1,000 keep vertices inside the hull
  of their bones' positions.
- Snapshot / integration: frame-png of a deformed fixture.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
