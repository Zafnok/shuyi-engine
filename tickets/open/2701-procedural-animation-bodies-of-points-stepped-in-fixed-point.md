---
id: "2701"
title: "Parked: Procedural animation: bodies of points and rules, stepped in fixed-point, in a shuyi-procanim crate"
type: feature
milestone: "Engine: procedural animation"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["1119", "1122", "2003", "2139"]
nick_input: none
completed:
---

# 2701 — Procedural animation: bodies of points and rules

> **Parked** (Nick, 2026-10-16: "this will be very late addition since it won't tie into visions of shuyi game"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after 1119 and 1122. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-16 (`docs/decisions.md`): a module for "procedural
animation like rain world has", with "different primitives" from a
normal physics and animation system.
[ADR-0010](../../docs/adr/0010-procedural-animation-bodies-limbs-and-strips.md)
sets the design. This ticket is its §1, §2 and §6: the crate, bodies
stepped in `Fx`, and creatures as RON. Limbs are 2702, drawing is 2703,
and the example is 2704. The module follows ADR-0009 §4.

ADR-0010 describes Rain World's methods from memory. Before starting,
check them against Joar Jakobsson's talks on Rain World's animation, and
write what differs in the Completion notes. Copy nothing from the game:
no names, creatures, art, or numbers taken from its files.

## Nick input

None.

## Scope

**In:**
- **From 2139 (a blocker):** `Fx::sqrt`, `FxVec2::length`,
  `length_squared` and `normalize`, and `Angle::atan2` and
  `Angle::acos`, are 2139's.
- **The crate** `crates/procanim` (`shuyi-procanim`), following
  ADR-0009 §4:
  - a `view` feature, empty here, filled by 2703;
  - the metadata from ADR-0010 §1 (concern: bodies and motion; answer:
    procedural animation; alternatives; serves);
  - the `procanim` feature in the `shuyi` crate;
  - **From 2003 (done):** the `shuyi-procanim` row in `docs/modules.md`
    and the `procedural` answer in `docs/modules.ron` exist, parked:
    set both to building. 2003's catalog has no building status
    (`shuyi_cli::catalog::Status` is ready, planned or parked). If it
    still has none, add `Status::Building`, named `building`, with its
    doc, the `status` line of `docs/modules.ron`'s header, *How to read
    it* in `docs/modules.md` and the `modules` schema. `Status` is
    `#[non_exhaustive]` since 2086 (done), so the new variant is no
    break;
  - the README's first two sections (what it is for, what it is not
    for). 2704 finishes the rest.
- **`Body`** (ADR-0010 §2):
  - points with a position, a previous position, a radius and an
    inverse mass;
  - the rules `Distance` (with its rope form), `Bend` and `Pin`;
  - `push(point, force)`;
  - `step(&mut self, level: &TileCollision)`: move, satisfy the rules
    `iterations` times in the order added, then collide each point as a
    circle;
  - `hash()` over every point's bits;
  - a text dump, one line per point;
  - a seed (`u64`), given when a body is made from its `BodyDef`, so
    two creatures of one file move a little differently (ADR-0010 §4).
    Nothing here reads it; 2702's gait seeds its RNG with it and the
    step.
- **`BodyDef` in RON**: points by name, rules naming points, damping,
  gravity and iterations.
  - It is checked with file and line: unknown point names, lengths ≤ 0,
    `min > max`.
  - It goes through the format registry (2201, ADR-0007) as asset kind
    `body`.
- **Measure the rest jitter** (ADR-0010 §2): hang a 6-point chain from a
  pin for 600 steps, then report the largest per-step movement of any
  point. If it is over 1/256 px, add a rest threshold to `BodyDef` and
  show that the jitter is gone. Write the numbers in the Completion
  notes.

**Out (do not do):**
- limbs and gaits (2702);
- drawing (2703);
- the example game (2704);
- collision with boxes (2704 adds it if the example needs it);
- code over a store: `Body` and `BodyDef` are plain values stepped by a
  function, naming neither `Arena<T>` nor `shuyi-ecs`'s `World` (2904's
  ADR), and a game keeps its bodies in whichever store it uses.

## Implementation steps

1. Create the crate, its metadata, its feature in `shuyi`, the README's
   start, and the catalog's building status.
2. Write `Body` and `step`, with unit tests on hand-made bodies: a
   pendulum, a rope, and a box of four points with diagonals.
3. Write `BodyDef`, its format plugin, and error fixtures.
4. Do the jitter measurement.

## Acceptance criteria

- [ ] A pinned point never moves, over random pushes (property test).
- [ ] After `step`, no point's circle overlaps a solid tile, starting
      from any position outside solids (property test, 10,000 cases).
- [ ] With no gravity or pushes, `Distance` errors never grow from one
      step to the next (property test).
- [ ] Two runs with the same pushes give the same hash list.
- [ ] Each bad RON fixture gives an error naming the file and line.
- [ ] `cargo xtask check-modules` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: pins, collision, constraint errors, determinism.
- Unit: the hand-made bodies; the RON errors.
- Snapshot: the text dump of the rope after 120 steps.

## Completion notes
