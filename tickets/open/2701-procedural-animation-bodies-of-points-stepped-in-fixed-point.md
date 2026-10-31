---
id: "2701"
title: "Parked: Procedural animation: bodies of points and rules, stepped in fixed-point, in a shuyi-procanim crate"
type: feature
milestone: "Engine: procedural animation"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["1119", "1122", "2003"]
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
- **`shuyi_core::fx` additions** (ADR-0010 §1):
  - `Fx::sqrt` (the floor of the exact root, through `i64`).
    **Either order with 3101:** if 3101 is done, `Fx` is `Fixed<8>`
    and 3101 wrote `Fixed::sqrt` and `Angle::atan2` (`fx::int`): use
    them, and build the rest below on them. If not, write them here,
    and 3101 makes `sqrt`, `atan2` and `acos` generic, keeping their
    results;
  - `FxVec2::length`, `length_squared` and `normalize` (zero stays
    zero);
  - `Angle::atan2(y, x)` and `Angle::acos(Fx)`, to 2335's 256-step
    `Angle` (`shuyi_core::angle`) and its sine table.
  - **Either order with 2106, 2122 and 2711:** if this ticket isn't
    done first, each of them adds what it needs of these (2106:
    `Fx::sqrt`; 2122: `Fx::sqrt`, `FxVec2::length` and `Angle::atan2`,
    to 2335's `Angle`; 2711: `Fx::sqrt`, `FxVec2::length`,
    `length_squared` and `normalize`) as specified here. Use what
    exists, add the rest, and keep one golden table of their results,
    as step 1 says.
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
    it* in `docs/modules.md` and the `modules` schema. `Status` is a
    public enum: write a `Breaking:` line if CI's `api` job reports
    one;
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

1. Add the `fx` functions with property tests and a golden table of
   results (2116's `shuyi_assets::golden`), checked by CI's `test` job
   (x64, ARM64) and `determinism` job (wasm32). Add the crate's `-p` to
   that job if the functions live outside `shuyi-core`.
2. Create the crate, its metadata, its feature in `shuyi`, the README's
   start, and the catalog's building status.
3. Write `Body` and `step`, with unit tests on hand-made bodies: a
   pendulum, a rope, and a box of four points with diagonals.
4. Write `BodyDef`, its format plugin, and error fixtures.
5. Do the jitter measurement.

## Acceptance criteria

- [ ] `Fx::sqrt` is the floor of the exact root for every non-negative
      input (property test), and the trig table matches its golden
      table in CI's `test` and `determinism` jobs.
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

- Property: the `fx` additions, pins, collision, constraint errors,
  determinism.
- Unit: the hand-made bodies; the RON errors.
- Snapshot: the text dump of the rope after 120 steps.

## Completion notes
