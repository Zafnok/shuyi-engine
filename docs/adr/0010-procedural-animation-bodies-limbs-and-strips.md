# ADR-0010: Procedural animation: bodies of points in fixed-point, limbs that find their footing, and strips to draw them

- **Status:** Accepted; its tickets are parked
- **Date:** 2026-10-16
- **Related tickets:** 2701–2704; they build on 1119 (tick simulation),
  1120 (world layers), 1121 (pointer input), 1122 (collision), 1134
  (skills), 2003 (modules)
- **Extends:** ADR-0003 §2–§5 (tick simulation, `Fx`, the frame's world
  layers), ADR-0004 §3–§5 (animation clips, the collision kit, physics
  optional), ADR-0009 (modules)

## Context

Nick, 2026-10-16 (`docs/decisions.md`): one of the engine's modules
"should be procedural animation like rain world has". It is a late
addition, because Visions of Shuyi doesn't need it.

**What Rain World does** (Videocult, 2017). This is from memory: Joar
Jakobsson's talks on its animation describe it, and 2701 re-checks it on
the day.

- A creature's body is a few round **chunks** joined by distance rules.
  Its AI pushes the chunks, and collision keeps them out of walls.
- **Limbs, tails, antennae and tentacles** are worked out every step
  from the body and the level. A foot looks for something to grip near
  where it should be, steps to it when it falls too far behind, and
  holds it meanwhile. Nothing is drawn frame by frame.
- Bodies are **drawn as meshes** stretched over the chunks and limb
  joints, with a few sprites (eyes, a head) placed on top.
- So a creature moves believably over any terrain, including terrain
  the artist never saw, and every creature moves a little differently.

**What the engine has, or plans:**

- frame-based clips, drawn from the time alone (ADR-0004 §3, 1123);
- a fixed-step tick simulation in `Fx` (ADR-0003 §2–§3, 1119);
- tile collision and movers (1122);
- world layers with cameras and sort keys (1120), but no rotation and no
  shapes other than rectangles and sprite items;
- optional rigid bodies through rapier, in floats (1128).

Rapier could join bodies with joints, but it solves a different problem.
It gives ragdolls and stacking boxes in floats, and it has no idea of a
foot looking for a grip. Procedural animation is a handful of points
and simple rules. Written in `Fx`, it is small, exact under replay, and
can be read as text.

## Decision

### 1. A module, `shuyi-procanim`

- A crate `crates/procanim` (package `shuyi-procanim`), following
  ADR-0009: a README, an example, a skill, a catalog row, and the
  `procanim` feature of the `shuyi` crate.
- Its place among the directions (ADR-0009 §2):
  - concern: bodies and motion; answer: procedural animation;
  - alternatives: the default movers with drawn clips (ADR-0004 §3–§4),
    and `shuyi-physics` (1128);
  - serves: side view first, where gravity pulls creatures onto floors,
    walls and ceilings. Top-down free also works, with gravity set to
    zero (snakes, tentacles, swarms).
- **Alternatives side by side.** A body is a procedural body or a rapier
  body, never both. Procedural creatures can share a stage with
  rapier crates, or with movers using drawn clips. The README shows how:
  the game reads one and pushes the other.
- It depends on `shuyi-core`. Its drawing part (§5) depends on
  `shuyi-ui`, behind the crate's `view` feature, which is on by default.
- Square roots, vector lengths, and angles from vectors (`atan2`,
  `acos`) go in `shuyi_core::fx`, because the collision kit and other
  modules will want them too (2701).

### 2. Bodies: points, rules between them, stepped in fixed-point

A `Body` holds:

- **points**: a position and the previous position in `FxVec2` (Verlet
  integration, so a velocity is the difference), a radius, and an
  inverse mass (0 means pinned);
- **rules** between points:
  - `Distance { a, b, length, stiffness }`, and its rope form, which
    only stops the points from getting farther apart;
  - `Bend { a, b, c, min, max }`, which limits the angle at `b`, in the
    256-step angles of `Fx`;
  - `Pin { point, at }`.

One step of a body:

1. **Move each point**: add the difference from its previous position
   times a damping factor, then gravity, then the forces the game asked
   for this step (`push(point, force)`).
2. **Satisfy the rules** a fixed number of times (the body's
   `iterations`), always in the order they were added.
3. **Collide each point** with the `TileCollision` from 1122 as a
   circle, pushed out along the shortest way. Optionally collide with
   the boxes of the broad phase.

Everything is `Fx`, in a fixed order, with no floats and no hash-map
order, so the same inputs give the same points on every platform
(ADR-0003 §3). The body's `hash()` covers the bits of every point.

1/256 of a pixel may be coarse enough to make a resting body jitter.
2701 measures this. If it does, the fix is a rest threshold, under
which a point stops moving; `Fx` doesn't change.

### 3. Two tiers: what the rules read, and what only looks

- **Body tier.** Points the game's rules read: where a creature is, what
  can be hit or grabbed. These are in the `TickSim` state and its hash,
  and they replay and roll back like everything else.
- **Look tier.** Limbs, tails, antennae and hair: motion that only
  looks. A `Rig` holds it beside the simulation, and the stage steps it
  right after each simulation step, reading the body and the level. The
  game's rules never read it, and it isn't in the hash.
- **Both tiers use the same types and the same `Fx` code.** A game puts
  a body in whichever tier it needs. A creature whose legs really hold
  it up puts them in the body tier.
- **The look tier is deterministic too.** It is stepped from the same
  steps in replays, so `frame-png` of step N gives the same picture
  everywhere. After a rollback (1147) a game may skip re-stepping it,
  and the look catches up within a few steps.

### 4. Limbs find their footing

A `Limb` is a chain of joints from an anchor point on a body to an end
(a foot, a hand, a tentacle tip).

- **Reaching:**
  - two-joint limbs (legs, arms) use the exact two-bone solution, with
    the bend side given as data;
  - longer chains (tails, tentacles) use FABRIK with a fixed number of
    passes.

  Both are in `Fx`.
- **A gait** moves feet:
  - each foot has a rest place relative to its anchor;
  - when the foot is more than a stride from that place, it looks for a
    grip: a solid tile edge with open space beside it, within reach,
    searched in a fixed order;
  - it moves there along an arc over a set number of steps, and holds
    still while it is planted;
  - feet in a group take turns (left and right, front and back), so a
    creature never lifts every foot at once;
  - with no grip in reach, the foot hangs from the limb under gravity.
- **Variety comes from the seed.** Any randomness (stride length, the
  order of grips with equal scores) comes from `shuyi-core`'s seeded RNG,
  seeded by the body's seed and the step. Nothing else.

### 5. Drawing: the scene names the shape, the skin paints it

- **The scene carries a view.** Logic builds a `BodyView` for each body
  (ADR-0002 §4):
  - its chains as points with a half-width each and a palette role;
  - its pieces (a head, eyes) as an anchor point, an angle and a sprite
    id.

  Tests of what happened read this view or the body. Only tests of a
  look read frame items.
- **The frame gains two things on world layers** (1120):
  - a `Strip` item: a triangle strip built from a line of points with a
    half-width each, filled with a palette color, or with an image
    stretched along it;
  - a rotation on sprite items: an angle in 256 steps, with
    nearest-neighbour sampling.
- **One rule decides which pixels a shape covers:** pixel centers
  inside the shape. It is written once, so `shuyi-app` and `frame-png`'s
  software renderer draw the same pixels.
- **Snapshots** list a strip as one line, with its points rounded to
  pixels.
- **On 1-bit targets** a strip fills with its role's pattern (3006).
- **A default skin** draws chains as strips and pieces as rotated
  sprites. A game's skin can draw them any other way.

### 6. A creature is data

A creature's points, rules, limbs, gait and drawing widths are a RON
file. It is checked with file and line like any content (game ADR-0005),
and it is the source of truth (ADR-0007). Tuning a creature is editing
text, with hot reload (ADR-0004 §1).

### 7. Not in this module

- **Skeletal animation from authored bones** (Spine, DragonBones): an
  artist animates bones in a tool and the game plays it back. That is a
  different module, written if a kind of game needs it.
- **Rigid bodies, stacking and ragdolls**: `shuyi-physics` (1128).
- **Deciding where a creature goes**: the game's AI, or the grid kit's
  paths. This module moves the body the AI pushes.

## Consequences

- A game can have creatures that climb walls and ceilings, tails that
  trail, and tentacles that reach, over any tile map, without drawn
  frames. Bots, replays and `frame-png` all work as they do for any tick
  stage.
- The frame grows two concepts: strips and rotation. Every skin and
  renderer must draw them, and the 1-bit path must fill them.
- `Fx` gains square roots and inverse trigonometry, written once and
  tested against a table on native and the web.
- The module is parked: none of this is built until Nick unparks
  2701–2704, after the MVP and after 1119–1122.
- A creature's look depends on the number of iterations and on gait
  numbers. They are data, so the example documents good starting
  values.

## Alternatives considered

- **Build it on rapier joints (1128)** — floats, a larger build, and
  rapier's joints don't look for footholds. The hard part, the gait,
  would still be ours.
- **Floats in the look tier, since it is only drawing** — simpler
  arithmetic, but native and web pictures would drift apart, so
  `frame-png` snapshots couldn't be checked on both. The look tier
  would also need a second copy of every rule. `Fx` is used for both.
- **Only the look tier (bodies are always the game's own movers)** —
  Rain World's creatures are their body chunks; gameplay hits them.
  Games need both, and the same types serve both.
- **Draw each segment as a sprite, with no strips** — fine for chains of
  beads, but it leaves gaps and overlaps on bends, and soft shapes like
  bodies and tails are the point. Strips are a small addition to the
  frame.
- **Extend animation clips (1123) with code-driven frames** — clips are
  a function of time alone (ADR-0004 §3). Procedural animation has state
  and reads the level; mixing them would break the clip rule.
