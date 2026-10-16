---
id: "1128"
title: "Parked: Optional rigid-body physics: a shuyi-physics crate over rapier2d with cross-platform determinism"
type: feature
milestone: "Engine: more styles"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1122"]
nick_input: none
completed:
---

# 1128 — Optional rigid-body physics (rapier2d)

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0060 says the army walks around freely (then 1119–1125 become game work). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

This is ADR-0004 §5. Most of the 2D styles Nick named are better served
by the pixel-stepping kit (1122). Some games want rigid bodies: crates
to push, ropes, things that tumble.

`rapier2d` (Apache-2.0, pure Rust) gives bit-for-bit identical results
on every platform, the web included, with its `enhanced-determinism`
feature. That feature can't be combined with `simd8`.

## Nick input

None.

## Scope

**In:**
- A crate `crates/physics` (`shuyi-physics`), not a dependency of
  any other engine crate. It depends on `rapier2d` with
  `default-features = false` and `["dim2", "enhanced-determinism"]`.
  `cargo deny` must pass.
- A `PhysicsWorld` stepped from a `TickSim::step` at the fixed rate.
  Bodies and colliders are created from data. Positions cross between
  `Fx` and `f32` at this crate's edge. Events come back as engine events.
- A `hash()` over body positions and velocities (their bits), so replays
  and networking (ADR-0006) can check agreement.
- A test that runs the same scene natively and as `wasm32` and gets the
  same hashes, against a checked-in table if running wasm in CI is
  impractical.
- Measure the size it adds to a WASM build and record it.
- Module duties (ADR-0009 §4): the metadata (concern: bodies and motion;
  answer: rigid-body physics; alternatives: the default movers and
  `shuyi-procanim`), the README in its fixed order, the `physics`
  feature of the `shuyi` crate (2003), the catalog row, and the
  `shuyi-physics` skill once the skill pack (1134) exists.

**Out (do not do):**
- making physics the default;
- joints beyond what the test scene needs.

## Implementation steps

1. Write the crate, the stepping and the conversion.
2. Write the determinism test and the size measurement.
3. Add a small example scene (boxes falling on a slope) in the crate's
   tests.

## Acceptance criteria

- [ ] The same hashes come out natively and on wasm, or match the
      checked-in table.
- [ ] `cargo deny check` passes.
- [ ] The size is recorded in the Completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the determinism test.

## Completion notes
