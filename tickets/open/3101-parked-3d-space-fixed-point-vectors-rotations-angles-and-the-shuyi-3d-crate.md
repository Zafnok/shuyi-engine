---
id: "3101"
title: "Parked: 3D space: Fixed<F> and Fx12, vectors, rotations and Angle16, the shuyi-3d crate, and the 3D perspective markers"
type: feature
milestone: "Parked: 3D"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["1119", "1120", "2003"]
nick_input: none
completed:
---

# 3101 — 3D space: fixed-point vectors, rotations and the `shuyi-3d` crate

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
§1–§3. 3D is a module, `shuyi-3d`, and its simulation is in integers
like the 2D one (ADR-0003 §3). This ticket lays the ground every other
3D ticket stands on:

- `Fx` (1119) becomes one case of a generic `Fixed<F>`, with no change
  of behaviour;
- `Fx12` (20.12, one unit a metre) is the 3D number;
- vectors, rotations and a 16-bit angle with its own sine table;
- the crate, its metadata, its feature in `shuyi`, and the three 3D
  perspective markers (ADR-0011 §2, ADR-0009 §3).

It builds on 1119 (`Fx`, `FxVec2`, and its golden table of `Fx`
results on 2116's `shuyi_assets::golden`), 2335 (the 256-step `Angle`
and its sine table, in units of 1/65 536 like this ticket's), 1120 (the
`Perspective` trait) and 2003 (the `shuyi` crate, module metadata, the
catalog and `cargo xtask check-modules`).

Two open tickets touch the same ground. Either order works with each:

- **2701** adds `Fx::sqrt`, `Angle::atan2` and `Angle::acos`. If 2701
  has landed, make those functions generic over `F` here, keeping their
  results for `Fx` (2701's tests must pass unchanged). If it hasn't,
  write them generic here, and 2701 uses them instead of adding its own.
- **3005** makes `shuyi-core` `no_std`. If 3005 has landed, add
  `shuyi-3d --no-default-features` to its CI gate. If not, write the
  crate `no_std`-clean anyway (below), and 3005 adds it to the gate.

## Nick input

None.

## Scope

**In:**
- **`Fixed<const F: u32>`** in `shuyi_core::fx` (ADR-0011 §3):
  - an `i32` with `F` fractional bits, `#[repr(transparent)]`;
  - `Add`, `Sub`, `Neg`, `Mul`, `Div` and their `*Assign` forms, with
    1119's rounding (toward negative infinity) and its saturation rules,
    written once for every `F`;
  - `from_int`, `from_raw`, `raw`, `floor`, `ceil`, `from_ratio(n, d)`,
    and `convert::<G>()` between precisions (rounding toward negative
    infinity);
  - `serde` as the raw `i32`, as `Fx` does today.
- **`type Fx = Fixed<8>`.** Every method `Fx` has today keeps its name
  and its results. Methods that only make sense for 2D pixels
  (`from_px`, `floor_px`) stay on `Fixed<8>` alone. `FxVec2` stays as
  it is.
- **`type Fx12 = Fixed<12>`**: one unit is a metre, a step is 1/4096 m.
- **Integer algorithms, written once** in `shuyi_core::fx::int`:
  - `isqrt_u64` (the floor of the exact root) and `Fixed::sqrt` on it,
    for every `F`;
  - `atan2_turn16(y: i64, x: i64) -> u16`: the angle in 1/65 536 of a
    turn. Use octant reduction and a binary search of the 3D sine table
    below, or CORDIC with 16 rounds; pick one, and document its error
    bound;
  - `Angle::atan2` on 2335's 256-step `Angle` (and `acos`, if 2701
    added it) is built on `atan2_turn16`, rounded to its 256 steps.
- **The crate** `crates/3d` (`shuyi-3d`), following ADR-0009 §4:
  - `#![cfg_attr(not(feature = "std"), no_std)]` with `extern crate
    alloc`, and a `std` feature on by default;
  - a `draw` feature, empty here, filled by 3103;
  - `#![deny(clippy::float_arithmetic)]` on every module outside
    `draw`, so no float enters the simulation part;
  - its entry in `tools/layers.ron` (2015): `engine` and `outside` as
    built, `devices: false`, and `floats: true`, because the ban is per
    module outside `draw`, not in `lib.rs`;
  - metadata under `[package.metadata.shuyi]`: concern *Perspective*,
    answers first person, third person and top-down 3D; concern *World
    drawing*, answer full 3D, alternative `shuyi-diorama`; `serves`:
    `first-person`, `third-person`, `top-down-3d`. If 2003's
    `ModuleInfo` takes only one `concern` and one `answer`, let it take
    a list of `[concern, answer]` pairs too (the single form stays
    valid), with a fixture for `check-modules`. If 3017 has landed,
    add `needs` with `three_d: software`; if not, 3017 adds it to
    every module that exists then;
  - the `3d` feature in the `shuyi` crate;
  - its row in `docs/modules.md` (2003 lists it as parked) set to
    building;
  - `crates/3d/README.md` in ADR-0009 §4's order: sections 1, 2 and 5
    in full, section 4 with a doc-tested snippet of the space types,
    and section 3 (cost) saying it is measured in 3108.
- **`shuyi_3d::space`**, all in `Fx12`:
  - `FxVec3`: add, sub, neg, scale, `dot` and `cross` (products in
    `i64`, sums of three in `i128`, rounded once), `length_squared`,
    `length`, `normalize` (zero stays zero);
  - `FxMat3` (rotation and scale): `IDENTITY`, `rot_x`, `rot_y`,
    `rot_z` from an `Angle16`, `from_axis_angle`, `scale`, `mul`,
    `mul_vec`, `transpose`;
  - `FxTransform { rot: FxMat3, pos: FxVec3 }`: `apply`, `compose`, and
    `inverse_rigid` (for transforms without scale; documented);
  - `FxQuat`: `from_axis_angle`, `mul`, `conjugate`, `rotate`,
    `to_mat3`, `nlerp(a, b, t)` and `renormalize`;
  - **one rounding rule for normalising**, written in the module docs:
    the length is `isqrt` of the `i128` squared length, and each
    component is divided by it rounding toward negative infinity, as
    `Fixed` division does.
- **`Angle16(u16)`** in `shuyi_3d::space`: 65 536 steps a turn, wrapping
  add and sub, `from_degrees(i32)`, `sin` and `cos` returning `Fx12`,
  and `atan2(y: Fx12, x: Fx12)` on `atan2_turn16`.
  - The table `SIN_QUARTER: [i32; 1025]` holds 1 024 entries of a
    quarter wave plus its end point (exactly 1), in units of 1/65 536.
    A lookup takes the top 10 bits of the angle within its quarter and
    interpolates the low 4 bits in integers, then rounds once to `Fx12`.
  - The table is generated by `cargo xtask gen-sin-table` (floats are
    fine there; it is a tool, not a simulation) and checked in as
    `crates/3d/src/space/sin_table.rs`.
- **The markers** `FirstPerson`, `ThirdPerson` and `Overhead3D` in
  `shuyi_3d::perspective`, implementing
  `shuyi_core::perspective::Perspective` (1120). If 1120's trait asks
  every marker for a 2D `Projection`, split it: `Perspective` stays the
  common trait, and a `Perspective2d: Perspective` carries the
  projection. Kits that take any 2D `P` then take `P: Perspective2d`.
  Record the split in the Completion notes.

**Out (do not do):**
- meshes and their formats (3102);
- the 3D view and drawing (3103, 3104);
- collision (3105) and cameras (3106);
- slerp (`nlerp` is enough until a camera shows it isn't);
- the module's skill and example (3108).

## Implementation steps

1. Copy today's `Fx` into `crates/core/tests/fx_before.rs` as a frozen
   reference type, `FxBefore`. It is test code only.
2. Write `Fixed<F>` and make `Fx = Fixed<8>`. Run every test: every
   fixture, snapshot and replay hash of the engine and its examples
   must pass unchanged. Add a property test that compares `Fx` with
   `FxBefore` for every operator.
3. Write `fx::int` with `isqrt_u64` and `atan2_turn16`, and the generic
   `sqrt`. Build `Angle::atan2` for 2335's `Angle` (and 2701's other
   functions, if they exist) on them.
4. Create `crates/3d`: `Cargo.toml` with the metadata and features, the
   README, its `tools/layers.ron` entry, the `3d` feature in
   `crates/shuyi`, and the catalog row. Run `cargo xtask check-modules`
   and `cargo xtask check-layers`.
5. Write `xtask gen-sin-table`, check in the table, and write
   `Angle16`.
6. Write `FxVec3`, `FxMat3`, `FxTransform` and `FxQuat`, each with unit
   tests on hand-worked values.
7. Add the markers, with a unit test that each implements `Perspective`.
8. Add a golden table of `Fx12` and `shuyi_3d::space` operations in the
   crate's `tests/golden/` (`crates/3d/tests/golden/space.ron`), through
   2116's `shuyi_assets::golden` (a dev-dependency on `shuyi-assets`
   with `default-features = false, features = ["golden"]`, as 1119's).
   Add `-p shuyi-3d` to the wasm32 `cargo test` of CI's `determinism`
   job, and to what the `run-gates` skill's *Golden tables* section says
   runs there.

## Acceptance criteria

- [ ] `Fx` is `type Fx = Fixed<8>`. Every existing test, fixture,
      snapshot and replay hash passes unchanged.
- [ ] The property test of `Fx` against `FxBefore` passes 10 000 cases
      per operator.
- [ ] `sqrt` is the floor of the exact root for every non-negative input,
      for `Fixed<8>` and `Fixed<12>` (property test).
- [ ] For all 65 536 angles (an exhaustive test): `sin² + cos²` is
      within 2 steps of 1; `sin` of a quarter turn is exactly 1; `sin`
      never decreases across the first quarter; and
      `atan2(sin a, cos a)` is within the documented bound of `a`.
- [ ] Rotation round trips hold within a documented bound (property
      tests): `q.conjugate().rotate(q.rotate(v))` returns `v`;
      `m.transpose().mul_vec(m.mul_vec(v))` returns `v`;
      `q.to_mat3()` agrees with `from_axis_angle` for the same axis and
      angle.
- [ ] After 1 000 random `FxQuat` products with `renormalize` every
      step, the length stays within 2 steps of 1 (property test).
- [ ] `Fx12` and `shuyi_3d::space` results match their golden table in
      CI's `test` and `determinism` jobs (x64, ARM64, wasm32).
- [ ] `shuyi-3d` builds with `--no-default-features`, and
      `clippy::float_arithmetic` finds nothing outside `draw`.
- [ ] `cargo xtask check-modules` passes, with the `3d` feature and the
      row set to building.
- [ ] `cargo xtask check-layers` passes with `shuyi-3d`'s entry.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: `Fx` against `FxBefore`; `sqrt`; rotation round trips;
  quaternion drift.
- Exhaustive: `Angle16` over every angle.
- Unit: hand-worked vectors, matrices and transforms; the markers.
- Determinism: the golden table of `Fx12` and `shuyi_3d::space`
  results, in CI's `test` and `determinism` jobs.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
