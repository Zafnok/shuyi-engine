---
id: "2159"
title: "Parked: Layers and blend spaces over bone clips: a bone mask per layer, additive poses, and clips blended by a one- or two-axis parameter"
type: feature
milestone: "Engine: animation"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2105", "2134"]
nick_input: none
completed:
---

# 2159 — Layers and blend spaces over bone clips

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): a follow-up of the parked skeletal module (2105) and its state machine (2134), and a follow-up of parked work is parked, so it starts after both are unparked and done. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2105 gives poses of a bone rig and mixes two of them; 2134 picks clips
with a state machine and cross-fades on a transition. 2134's **Out**
leaves two things as "a follow-up if a game needs one", with no ticket:

- **layers**: an upper body on one graph over legs on another, and
  additive poses (a flinch, breathing or an aim offset on top of
  whatever plays);
- **blends of more than two clips by a parameter**: walk, jog and run
  by speed, or eight-way strafing by a direction.

The Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found both in Bevy's animation graph: masks and additive
blends (`crates/bevy_animation/src/graph.rs`: `add_clip_with_mask`,
`add_additive_blend`, `AnimationMask`; `examples/animation/animation_masks.rs`)
and weighted blend nodes (`examples/animation/blend_graph.rs`). Skeletal
2D action games, where a character shoots while the legs run, need
both.

The rule of ADR-0004 §3 holds: the pose shown is a pure function of the
data, the stored state and the step.

## Nick input

None.

## Scope

**In:**
- **Masks:** named bone sets in the rig RON
  (`masks: { upper: [spine, arm_l, arm_r, head] }`), checked against
  the rig's bones by `shuyi check`.
- **Layers:** `Layer { graph, mask, weight: Permille, mode: Override |
  Additive }` in a list over a rig. The pose is the base layer, then
  each layer mixed per masked bone with 2105's `mix` in fixed point; an
  additive layer adds its pose minus its clip's first frame. Angles mix
  the shortest way round through 2335's `Angle`.
- **Blend spaces:** `BlendSpace1 { points: [(Fx, clip)] }` and
  `BlendSpace2 { points: [(FxVec2, clip)] }` in the rig RON;
  `weights_at(param)` gives weights in permille that sum to exactly
  1,000 (largest remainder), the 2D case by barycentric weights over a
  triangulation fixed at load. Clips in a blend are time-normalised, so
  feet stay in phase. A state of 2134's graph may be a blend space.
- New types follow ADR-0039.

**Out (do not do):**
- inverse kinematics on top of a pose (2702, parked);
- 3D blending (3110, parked);
- a blend tree editor or panel.

**Estimated size:** about 1,300 lines (code 600, tests 550, docs 150).

**Split:** past 1,000 lines, split off blend spaces into a new ticket.

## Implementation steps

1. Read 2105's and 2134's code and the rig format.
2. Add masks to the rig format and the check.
3. Add layers and the additive mode to pose evaluation.
4. Add the two blend spaces, the fixed triangulation and the weights.
5. Let a 2134 state be a blend space.
6. Write the docs page: a character that waves while it walks.

## Acceptance criteria

- [ ] A layer whose mask holds no bones changes nothing; weights 0 and
  1,000 equal the base pose and the override.
- [ ] An additive layer of a rest clip changes nothing.
- [ ] `weights_at` sums to 1,000 for any parameter, and a parameter on
  a point gives that clip alone.
- [ ] An unknown bone in a mask fails `shuyi check` with its file and
  line.
- [ ] Poses of a layered fixture are equal on wasm32 (golden row).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: masks, each mode, each blend space.
- Property: the weight sum; a layer's weight is monotone in its effect.
- Snapshot / integration: a golden row of layered poses.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
