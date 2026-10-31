---
id: "1128"
title: "Parked: Optional rigid-body physics: a shuyi-physics crate over rapier2d with cross-platform determinism"
type: feature
milestone: "Engine: more styles"
model: reasoning
effort: high
status: blocked
blocked_by: ["1122", "2116"]
nick_input: none
completed:
---

# 1128 — Optional rigid-body physics (rapier2d)

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): after Visions of Shuyi launches, not only after the MVP (`docs/decisions.md`, 2026-10-20). Unparked early only if the game needs it: its ticket 0060 says the army walks around freely (then 1119–1125 become game work), or it asks for a piece of this through its `engine-change` skill, which is then split out and unparked alone. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

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
- Colliders take 1122's `CollisionLayers`, mapped onto rapier's
  `InteractionGroups` (`memberships` from `layer`, `filter` from
  `mask`) as each collider's `collision_groups`, so a game that moves
  from the default movers to this module keeps its layer data. A unit
  test shows two colliders interact in rapier exactly when 1122's rule
  says they meet.
- A `hash()` over body positions and velocities (their bits), so replays
  and networking (ADR-0006) can check agreement.
- A golden table (2116's `shuyi_assets::golden`),
  `crates/physics/tests/golden/scene.ron`, of the test scene's hash per
  step. The test that checks it has `golden` in its name, as 2116 asks.
  `-p shuyi-physics` joins the wasm32 `cargo test` of CI's
  `determinism` job, so the same hashes are checked on x64, ARM64 and
  wasm32.
- Measure the size it adds to a WASM build and record it.
- Module duties (ADR-0009 §4): the metadata (concern: bodies and motion;
  answer: rigid-body physics; alternatives: the default movers and
  `shuyi-procanim`), the README in its fixed order, the `physics`
  feature of the `shuyi` crate (2003), the catalog row, and the
  `shuyi-physics` skill (**from 1134, done**): `skills/shuyi-physics/`
  in the pack's form, with a worked example, `example.ron`, that `shuyi
  add physics` and a few lines make run on a fresh game, which CI's
  `skills` job runs. It is a module's skill: `shuyi add physics` copies
  it into a game.
- **From 2003 (done):** a game steps the `PhysicsWorld` from its own
  `TickSim::step`, a library call (ADR-0040 §1), and a hook never
  changes a stage's state (§3). So this module needs no `Module` and no
  line in `shuyi::enabled_modules`; its README's "how to start" shows
  the call. If it adds a format, set `formats` in its
  `[package.metadata.shuyi]` to the function a game's `formats()`
  calls, so `shuyi add` prints that line (ADR-0041 §6).
- **Either order with 2704:** if 2704 is done, add a few rigid crates
  to `examples/cave-crawl` beside its creatures (ADR-0010 §1), update
  its pictures and hashes, and drop the README line saying this ticket
  adds them. If not, nothing to do: 2704 adds them.

**Out (do not do):**
- making physics the default;
- joints beyond what the test scene needs;
- drawing its colliders in the debug view: 2373 (parked).

## Implementation steps

1. Write the crate, the stepping, the conversion and the layer mapping.
2. Write the golden table's test, add `-p shuyi-physics` to the
   `determinism` job, and measure the size.
3. Add a small example scene (boxes falling on a slope) in the crate's
   tests.

## Acceptance criteria

- [ ] The scene's hashes match its golden table in CI's `test` and
      `determinism` jobs.
- [ ] `cargo deny check` passes.
- [ ] The size is recorded in the Completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the layer mapping, against 1122's rule.
- Golden: the scene's table, natively in CI's `test` job (x64, ARM64)
  and on wasm32 in its `determinism` job.

## Completion notes
