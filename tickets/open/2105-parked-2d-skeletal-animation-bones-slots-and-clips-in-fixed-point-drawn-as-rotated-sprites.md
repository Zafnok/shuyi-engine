---
id: "2105"
title: "Parked: 2D skeletal animation: bones, slots and clips in fixed point, mixing two poses, and a skeleton drawn as rotated sprites"
type: feature
milestone: "Engine: animation"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1119", "1120", "1123", "2703"]
nick_input: none
completed:
---

# 2105 — 2D skeletal animation: bones, slots and clips

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): the game doesn't need it, so this waits until after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after 1119, 1120, 1123 and 2703 (rotated sprites). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6. Godot
has `Skeleton2D` and `Bone2D`; many 2D studios animate characters in a
bone tool (Spine, DragonBones) instead of drawing every frame.
[ADR-0010](../../docs/adr/0010-procedural-animation-bodies-limbs-and-strips.md)
§7 left it out of procedural animation: "Skeletal animation from
authored bones (Spine, DragonBones) ... is a different module, written
if a kind of game needs it." This is that module.

An artist animates bones in a tool; the game plays it back. Here:

- the rig and its clips are our own RON (the source of truth, ADR-0007
  §4). 2216 imports DragonBones JSON into it;
- poses are computed in `Fx` with 1119's angle and sine table, so a
  replay and `frame-png` give the same picture on every machine, and a
  game may read a bone's position for a hitbox;
- the skin draws each slot as a rotated sprite. 2703 adds rotation to
  sprite items on world layers (1120).

Spine import is 2217: a reader of our own, written from Spine's public
format documentation, with the user bringing their own Spine license
(Nick, 2026-10-20, `docs/decisions.md`).

## Nick input

None.

## Scope

**In:**
- **A module crate `shuyi-bones`** in `crates/bones` (ADR-0009 §4:
  a crate of its own, `no_std` with `alloc`).
  - **Either order with 2003:** if 2003 is done, register it in
    `docs/modules.ron` under the "Bodies and motion" concern, as an
    answer named "drawn bones" beside "movers and drawn clips". If not,
    add a line to 2003's scope listing it.
  - **An ADR** (the `write-adr` skill): bones in `Fx`, no scale or
    shear, our RON as the source of truth, a pose as a pure function of
    the clip and the time.
- **The rig** (`Rig`, an asset kind, read by `RigRon`, ADR-0019):
  - `bones`: name, parent (parents listed before children, checked),
    `x`, `y` (`Fx`), `rotation` (`Angle`), `length` (for the debug
    view);
  - `slots`, in draw order: name, bone, the attachment shown at rest,
    and a palette role for tint (optional);
  - `attachments`: an atlas region (1123's `Atlas`) with an offset and
    a rotation from its bone;
  - `skins`: named sets of attachments per slot (a sword and an axe on
    the same hand slot);
  - `clips`: a duration in ms, a repeat rule like 1123's clips, and
    timelines: bone rotation (`ms`, angle, `Linear` or `Stepped`), bone
    translation (`ms`, x, y, curve), slot attachment (`ms`, name or
    none), events (`ms`, name).
- **The maths**, pure:
  - `pose_at(&Rig, clip, elapsed_ms) -> Pose`: local transforms per
    bone, and the attachment per slot;
  - `Pose::mix(&a, &b, weight: u16 /* 0–256 */) -> Pose`, rotation by
    the shorter way round: a cross-fade is mixing two clips' poses
    over a few steps;
  - `world(&Rig, &Pose) -> Vec<BoneWorld>`: each bone's position and
    angle in the rig's space, composed from the root;
  - `events_between(clip, from_ms, to_ms)`.
- **The view and skin** (behind a `view` feature, as 2703 does):
  `RigView::from(&Rig, &Pose, skin, origin, facing)` → one sprite item
  per slot with its angle (2703), on a world layer (1120), in slot
  order; `facing` flips the rig left and right.
- **The debug view:** if 2315 is done, a helper
  `shuyi_bones::debug::bone_gizmos` drawing each bone as a `Bone` line
  from its position along its length. If not, leave it and note it.
- **Fixtures:** a three-bone arm and a small walk clip (ours, CC0).

**Out (do not do):**
- bone scale and shear (a follow-up if a rig needs them; 2216 reports
  them as errors);
- mesh attachments and deformation, IK constraints, physics on bones;
- DragonBones import (2216);
- Spine import (2217);
- a state machine that picks the clip (2104 picks clips; mixing two of
  this module's clips by its states is a follow-up once both are done).

## Implementation steps

1. Read ADR-0010 §7, 2703's rotated sprites, 1123's `Atlas`, and 1119's
   `Angle` and sine table.
2. Write the ADR.
3. Write the rig types, `RigRon` and its checks, with fixtures.
4. Write `pose_at`, `mix`, `world` and `events_between`, with property
   tests.
5. Write the view and the default skin; add snapshot tests.
6. Run the gates, including the crate's `no_std` build.

## Acceptance criteria

- [ ] `pose_at` repeats with the clip's period for a looping clip.
- [ ] `mix(a, b, 0) == a` and `mix(a, b, 256) == b`; rotation mixes the
      shorter way round.
- [ ] With every bone at angle 0, a bone's world position is the sum of
      the offsets down its chain.
- [ ] The fixture arm's snapshot at three times is committed and reviewed
      with `frame-png`.
- [ ] Bad fixtures give errors with file and line; random bytes never
      panic `RigRon`.
- [ ] `cargo deny check` passes, and the new crate is in CI's `no_std`
      job.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: stepped and linear keys; a slot switching attachment; a skin
  swap; events at the edges of a range.
- Property: pure and periodic `pose_at`; `mix` bounds; a parent listed
  after its child is always an error.
- Snapshot / integration: the arm's sprites at three times, and one
  mixed pose.

## Completion notes

*(Filled in by the session that completes the ticket.)*
