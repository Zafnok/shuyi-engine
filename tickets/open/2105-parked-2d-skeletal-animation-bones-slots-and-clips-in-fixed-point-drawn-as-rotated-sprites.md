---
id: "2105"
title: "Parked: 2D skeletal animation: bones, slots and clips in fixed point, mixing two poses, and a skeleton drawn as rotated sprites"
type: feature
milestone: "Engine: animation"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1119", "1120", "1123", "2109", "2335", "2337"]
nick_input: none
completed:
---

# 2105 — 2D skeletal animation: bones, slots and clips

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): the game doesn't need it, so this waits until after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after 1119, 1120, 1123, 2109 (ease curves), 2335 (rotated sprites) and 2337 (tints). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

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
- poses are computed in `Fx` with 2335's `Angle` and sine table
  (`shuyi_core::angle`), so a replay and `frame-png` give the same
  picture on every machine, and a game may read a bone's position for
  a hitbox;
- the skin draws each slot as a rotated sprite. 2335 adds rotation to
  sprite items, and 1120 carries them onto world layers.

Spine import is 2217: the official runtime in an opt-in module whose
users bring their own Spine license, or a reader of our own into this
module's `Rig`, which Nick picks when it is unparked (2026-10-20,
`docs/decisions.md`).

## Nick input

None.

## Scope

**In:**
- **A module crate `shuyi-bones`** in `crates/bones` (ADR-0009 §4:
  a crate of its own, `no_std` with `alloc`).
  - **The catalog** (2003, ADR-0041): register it in
    `docs/modules.ron` under the `bodies` concern ("Bodies and motion"),
    as an answer named "drawn bones" beside "movers and drawn clips",
    `by: "shuyi-bones"`, with its row in `docs/modules.md` and its
    `[package.metadata.shuyi]`.
  - **An ADR** (the `write-adr` skill): bones in `Fx`, no scale or
    shear, our RON as the source of truth, a pose as a pure function of
    the clip and the time.
- **The rig** (`Rig`, an asset kind, read by `RigRon`, ADR-0019):
  - `bones`: name, parent (parents listed before children, checked),
    `x`, `y` (`Fx`), `rotation` (`Angle`), `length` (for the debug
    view);
  - `slots`, in draw order: name, bone, the attachment shown at rest,
    and a palette role for tint (optional), drawn as `Paint::Tinted`
    (2337);
  - `attachments`: an atlas region (2222's `Atlas`, which 1123 imports sheets into) with an offset and
    a rotation from its bone;
  - `skins`: named sets of attachments per slot (a sword and an axe on
    the same hand slot);
  - `clips`: a duration in ms, a repeat rule like 2231's clips
    (`Repeat`), and timelines: bone rotation (`ms`, angle, ease), bone
    translation (`ms`, x, y, ease), slot attachment (`ms`, name or
    none), events (`ms`, name). A key's ease is a
    `shuyi_core::ease::Ease` (2109): `Linear`, `Steps(1)` for a stepped
    key, or any other curve, so this module has no curve type of its
    own.
- **The maths**, pure:
  - `pose_at(&Rig, clip, elapsed_ms) -> Pose`: local transforms per
    bone, and the attachment per slot;
  - `Pose::mix(&a, &b, weight: u16 /* 0–256 */) -> Pose`, rotation by
    the shorter way round: a cross-fade is mixing two clips' poses
    over a few steps;
  - `world(&Rig, &Pose) -> Vec<BoneWorld>`: each bone's position and
    angle in the rig's space, composed from the root;
  - `events_between(clip, from_ms, to_ms)`, following the rule of
    `shuyi_core::timer::crossed` (2364): an event at `t` fires when
    `from_ms < t <= to_ms`, nothing fires on a backward jump, and a
    range longer than one loop fires each event once.
    - **Either order with 2364:** if 2364 is done, build
      `events_between` on `crossed`. If not, write `crossed` in
      `shuyi_core::timer` with the rule in 2364's Scope (and its either
      order with 2115), and 2364 uses it.
- **The view and skin** (behind a `view` feature, as 2703 does):
  `RigView::from(&Rig, &Pose, skin, origin, facing)` → one sprite item
  per slot with its angle (2335), on a world layer (1120), in slot
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

1. Read ADR-0010 §7, 2335's rotated sprites and `shuyi_core::angle`,
   2222's `Atlas` and 1123's importers, 2109's `shuyi_core::ease`, and 1119's `Fx`. Check
   whether 2364 is done (`crossed`).
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

- Unit: stepped (`Steps(1)`) and linear keys; a slot switching
  attachment; a skin swap; events at the edges of a range, across a
  loop and after a backward jump.
- Property: pure and periodic `pose_at`; `mix` bounds; a parent listed
  after its child is always an error.
- Snapshot / integration: the arm's sprites at three times, and one
  mixed pose.

## Completion notes

*(Filled in by the session that completes the ticket.)*
