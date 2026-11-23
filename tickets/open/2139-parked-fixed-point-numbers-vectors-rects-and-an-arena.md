---
id: "2139"
title: "Parked: A game gets deterministic fixed-point numbers, vectors, rects, angles, square roots and an arena from shuyi-core"
type: feature
milestone: "Parked: real time"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3525"]
nick_input: none
completed:
---

# 2139 — Fixed-point numbers, vectors, rects, angles, square roots and an arena in shuyi-core

> **Parked** (the blind-spot audit, 2026-11-23): it may start when Nick unparks it in decision ticket 3525 (what 1.0 is for, among them "a numeric core"). Until then nobody starts it. Whoever unparks it sets `status: todo`, drops `Parked:` from the title, and sets the milestone 3525's answer gives.

## Context

A blind-spot audit of the engine (2026-11-23) found that a game has no
sanctioned way to write a position, a speed or a direction:

- `AGENTS.md` rule 5 bans floats in a simulation, and the template's
  rules crate denies them
  (`templates/shuyi-default/stages/top-down-grid/crates/rules/src/lib.rs`,
  line 8: `deny(clippy::float_arithmetic)`). `shuyi-core` denies them
  too (`crates/core/src/lib.rs`, line 43).
- `shuyi-core` has no fixed-point type, vector, rect, angle or arena.
  Its modules are `bots`, `character`, `ease`, `grid`, `hash`,
  `history`, `made`, `path`, `rng`, `save`, `text`, `threat`, `tick`,
  `timer`, `toy` and `turn`. Its only geometry is `grid::Pos`. The only
  rect is `shuyi_ui`'s `Rect` (`PxRect`,
  `crates/ui/src/frame/glyph_buffer.rs`, lines 129 and 177), in whole
  console pixels.
- ADR-0003 §3 and §4 name `shuyi_core::Fx`, `FxVec2`, a 256-step sine
  table and `shuyi_core::arena::Arena<T>`. None of them exists.

The maths is planned in four places, two of them parked and one of
them a planned break:

- **1119** (parked, real time): `Fx` (`i32`, 8 fraction bits), `FxVec2`,
  sine and cosine, `Lerp` for both, and `Arena<T>`. It plans no length,
  no square root and no rect.
- **2701** (parked, procedural animation): `Fx::sqrt`,
  `FxVec2::length`, `length_squared`, `normalize`, `Angle::atan2` and
  `Angle::acos`. 2106, 2122 and 2711 each say they add part of it if
  2701 hasn't.
- **3101** (parked, 3D): turns `Fx` into `Fixed<8>` of a generic
  `Fixed<const F: u32>`, with `isqrt_u64`, a generic `sqrt` and
  `atan2_turn16`. Writing `Fx` first and making it generic later is a
  change of a public type after the release.
- **2335** (open, after the MVP): the 256-step `Angle` and its sine
  table in `shuyi_core::angle`, for rotated sprites.

This ticket gathers the numbers into one ticket with no drawing and no
entity kit, so the type every real-time, physics and animation ticket
stands on is settled once. It also closes a hole the audit measured:
with 8 fraction bits in an `i32`, a squared length held in `Fx`
overflows past about 2,896 px (`sqrt(2^31 / 256)`), less than one
1080p screen width. Here, squares, dots and lengths are worked in
`i64`, so `length` is exact for every `FxVec2`.

`docs/decisions.md`, 2026-11-14: before the first release (2005) every
planned break is done or its shape is settled. Writing `Fixed<F>` here,
with `Fx = Fixed<8>`, means 3101 adds `Fx12 = Fixed<12>` without
changing `Fx`.

What the other tickets keep once this is done:

- **1119** keeps moving 2137's example sim onto `FxVec2` (with inertia
  and its regenerated `crates/core/tests/golden/tick.ron`) and the
  `shuyi-stage-field` skill (teaching `Fx` and `Arena`, dropping 1119
  from its `waits_for`). It is blocked by this ticket.
- **2701** keeps the `shuyi-procanim` crate, `Body`, `BodyDef` and the
  jitter measurement. It takes its `fx` additions from here.
- **2335** keeps the sprite's angle, pivot and `flip_y`, the renderers,
  the snapshot fields and its ADR on the pixel rule. The `Angle` type is
  written by whichever of the two goes first (below).
- **3101** keeps `Fx12`, `Angle16` and its 1,025-entry table, the 3D
  space types, the `shuyi-3d` crate and the perspective markers.

The game: Visions of Shuyi pins engine revision `50472bbc`. A search
of the game for `FxVec2` and for `shuyi_core::Angle` finds nothing;
`Arena` finds only `.claude/skills/shuyi-stage-field/SKILL.md`, the
skill pack's copy. Everything here is new and additive: nothing in the
game changes.

## Nick input

None (technical). Unparking is 3525's question.

## Scope

**In:**
- **`shuyi_core::fx`** (`crates/core/src/fx.rs` and a folder for its
  parts), `no_std`, no floats outside tests:
  - `Fixed<const F: u32>`: an `i32` with `F` fraction bits,
    `#[repr(transparent)]`, deriving `Copy`, `Eq`, `Ord`, `Hash`,
    `Debug` and `Default`; serde as the raw `i32`
    (`#[serde(transparent)]`) and the `schema` feature as `grid::Pos`
    has it.
  - `Add`, `Sub`, `Neg`, `Mul`, `Div` and their `*Assign` forms, written
    once for every `F`. Products and quotients are worked in `i64` and
    round toward negative infinity. A result outside `i32` saturates;
    the module docs say so, and `checked_mul`, `checked_div`,
    `wrapping_add` and `saturating_add` say what each does. Division by
    zero panics, as integer division does, and `checked_div` returns
    `None`.
  - `from_int`, `from_raw`, `raw`, `floor`, `ceil`, `round`,
    `from_ratio(n, d)` (rounding toward negative infinity), `abs`,
    `min`, `max`, `clamp`, `ZERO`, `ONE`, `MIN`, `MAX`, and
    `convert::<G>()` between precisions (rounding toward negative
    infinity, saturating).
  - `sqrt` for every `F`: the floor of the exact root, through
    `isqrt_u64`. A negative input returns `ZERO` and the doc says so.
  - `type Fx = Fixed<8>`, re-exported as `shuyi_core::Fx`. `from_px(i32)`
    and `floor_px() -> i32` live on `Fixed<8>` alone.
- **`shuyi_core::fx::int`**: integer algorithms written once.
  - `isqrt_u64(n: u64) -> u64` and `isqrt_u128(n: u128) -> u128`, the
    floor of the exact root (`u64::isqrt` and `u128::isqrt`, stable
    since Rust 1.84; the workspace's `rust-version` is 1.100);
  - `atan2_turn16(y: i64, x: i64) -> u16`: the angle of `(x, y)` in
    1/65,536 of a turn, clockwise on screen (y points down), as 2335's
    `Angle` turns. CORDIC in vectoring mode, 16 rounds, after an octant
    reduction, on `i64`. Its 16 arctangent constants are written out as
    integers; a test computes them in `f64` and checks each. The doc
    states the error bound the exhaustive test measures. `(0, 0)` is 0.
- **`FxVec2 { x: Fx, y: Fx }`**: public fields, exhaustive on purpose
  (a vector never grows, as `grid::Pos` doesn't). Add, sub, neg,
  scale by `Fx` and by `i32`, `ZERO`, `new`, `from_px`, `floor_px`.
  - `dot(a, b) -> i128` and `length_squared() -> u64`, in raw units
    squared (1/65,536 of a pixel squared), so they never overflow: each
    square is at most 2^62, and the sum of two at most 2^63, one past
    `i64::MAX` but inside `u64`. The doc says why they aren't `Fx`.
  - `length() -> Fx`: `isqrt_u64` of `length_squared`, exact for every
    vector, saturating at `Fx::MAX` (only a vector longer than
    8,388,607 px reaches it).
  - `distance(a, b) -> Fx` and `distance_squared(a, b) -> u128`, the
    difference worked in `i64` first (it can need 33 bits), its root
    through `isqrt_u128`.
  - `normalize(len: Fx) -> FxVec2`: the vector scaled to length `len`
    (zero stays zero). Each component is `x · len / length`, worked in
    `i64` (`i128` where the product needs it) and rounded toward
    negative infinity, as `Fixed` division does.
  - `angle() -> Angle` through `Angle::atan2`, and
    `from_angle(Angle, len: Fx) -> FxVec2`.
- **`FxRect { x: Fx, y: Fx, w: Fx, h: Fx }`**: public fields,
  exhaustive. `new`, `from_px(x, y, w, h)`, `right`, `bottom`,
  `contains(FxVec2)` (left and top edges in, right and bottom out, as
  `PxRect` and 2335's `dest_pixel` keep them), `overlaps`,
  `intersection -> Option<FxRect>`, `union`, `translate`, and
  `floor_px() -> (i32, i32, i32, i32)`. A negative `w` or `h` is empty;
  the doc says so. 2386 (parked) already names an `FxRect`; this is it.
- **`shuyi_core::angle`**:
  - **Either order with 2335:** if 2335 is done, `Angle(u8)`, its
    65-entry quarter-wave table, `sin`, `cos`, `rotate` and the
    `shuyi_core::Angle` re-export exist: use them, and add only what is
    below. If not, write the module exactly as 2335's scope lists
    (`Angle(u8)`, 256 steps, `UNIT = 65_536`, the table, `sin`, `cos`,
    `rotate`, the constants, wrapping `Add`, `Sub` and `Neg`, serde as a
    number, the re-export) with 2335's tests of the table, and 2335 adds
    the sprite on it.
  - Added here either way: `Angle::sin_fx()` and `cos_fx()` returning
    `Fx`, the table's 1/65,536 shifted to `Fx`'s 1/256 with an
    arithmetic shift (rounding toward negative infinity), not a table
    of their own; `Angle::atan2(y: Fx, x: Fx) -> Angle` on
    `atan2_turn16`, rounded to the nearest of its 256 steps (halves
    up, wrapping 256 to 0); and `Angle::acos(Fx) -> Angle`, its input
    clamped to `-1..=1`, as `atan2(sqrt(1 − c²), c)`.
- **`Lerp`** (from 2109, done; `crates/core/src/ease.rs`, line 402) for
  `Fx` and `FxVec2`: `mix(from, to, p)` with `p` in 1/65,536
  (`ease::ONE`), worked in `i128` as `mix_wide` does and rounded as
  `Lerp` for integers rounds (to the nearest, halves up), so a tween of
  an `Fx` moves as a tween of an `i32` does. A test shows `mix` hits
  both ends exactly.
  - **Either order with 2118:** if 2118 is done, add its one-line
    `shuyi_procgen::noise::to_fx` (a noise value as an `Fx`, `n >> 8`)
    with a test; if not, nothing to do (2118 adds it).
- **`shuyi_core::arena`**: `Arena<T>` with generational `Id<T>` (an
  index and a generation, both `u32`, read through `index()` and
  `generation()`; `Copy`, `Eq`, `Ord` and `Hash` written by hand so `T`
  needs none of them). `insert`, `remove`, `get`, `get_mut`,
  `contains`, `len`, `is_empty`, `clear`, iteration (shared and
  mutable) in id order, serde, and a text dump with one line per live
  entry (`#3.1 <Debug of T>`).
  - The serde form keeps the free slots and their generations, so after
    a save and a load the next `insert` returns the same id it would
    have without them. Two runs with the same inserts and removes give
    the same ids.
  - `Arena<T>` names no ECS. **Either order with 2904:** if 2904 is
    done, read its ADR first and follow what it says the arena shares
    with `shuyi-ecs` (kits name neither store; a shared store trait only
    as the ADR says, as a follow-up ticket written here, parked with
    1119); if not, nothing to do (2904's ADR describes this arena as
    planned here).
- **An ADR** (the `write-adr` skill, the next free number): the numeric
  core. `Fixed<F>` with `Fx = Fixed<8>`; products in `i64` rounding
  toward negative infinity; which operations saturate; squares, dots
  and lengths in `i64`; one angle and one sine table, in
  `shuyi_core::angle`; `atan2` by CORDIC and its bound; the arena's ids
  and dump. It amends ADR-0003 §3 and §4 (change only their `Status:`
  line and row in `docs/adr/README.md`) and says that ADR-0011 §3's
  `Fixed<F>` is now this ticket's, with `Fx12` left to 3101.
- **Public types (ADR-0039):** `Fixed<F>`, `FxVec2`, `FxRect` and
  `Angle` are values that never grow and stay exhaustive, as `Pos`
  does; the ADR says so. `Arena<T>` and `Id<T>` have private fields.
  All of it is additive: no `Breaking:` line.
- **Determinism:** a golden table, `crates/core/tests/golden/fx.ron`,
  of a fixed list of operations (each operator at its edges, `sqrt`,
  `length`, `normalize`, `atan2`, `acos`, `sin_fx`, `mix`, and an arena
  after a fixed script of inserts and removes, by its dump's hash),
  through 2116's `shuyi_assets::golden` (already a dev-dependency of
  `shuyi-core`, `crates/core/Cargo.toml`, line 36). CI's `test` job
  checks it on x64 and ARM64, and its `determinism` job on wasm32
  (`.github/workflows/ci.yml`, line 1003, already runs `-p shuyi-core`).

**Out (do not do):**
- drawing anything, or any change to `shuyi-ui`'s `PxRect` or the
  frame;
- the sprite's angle, pivot and `flip_y` and the renderers (2335);
- 2137's example sim on `Fx` and the `shuyi-stage-field` skill (1119);
- `Fx12`, `Angle16`, 3D vectors, matrices and quaternions (3101);
- bodies, rules and limbs (2701, 2702);
- collision, `Aabb` and movers (1122; it may build `Aabb` on `FxRect`,
  its call);
- world layers and cameras (1120);
- curves (2122), navigation (2106) and steering (2711): each comes
  after this through 1119 or 2701, and uses what is here;
- an ECS (2107) and measuring stores at scale (2705);
- a 64-bit fixed-point type. If a game shows `Fx`'s range
  (±8,388,607 px) is too small, a new ticket adds `Fixed64<F>`.

**Estimated size:** about 1,800 lines (code 900, tests 750, docs 150).

**Split:** past 1,100 lines, split off step 7 (the arena) into a new
ticket, parked with this one and put in 1119's `blocked_by` beside it.

## Implementation steps

1. Read ADR-0003 §3–§4, ADR-0011 §3, `crates/core/src/ease.rs`
   (`Lerp`, `mix_wide`), `crates/core/tests/golden.rs` and 2335's,
   1119's, 2701's and 3101's scopes. Write the ADR.
2. Write `Fixed<F>`, its operators, conversions, serde and schema, with
   property tests of the algebra within the documented rounding and of
   saturation at `MIN` and `MAX`.
3. Write `fx::int` (`isqrt_u64`, `isqrt_u128`, `atan2_turn16`) and
   `Fixed::sqrt`.
4. Write `shuyi_core::angle` if 2335 hasn't, then `sin_fx`, `cos_fx`,
   `atan2` and `acos`.
5. Write `FxVec2` and `FxRect`, with unit tests on hand-worked values
   and property tests.
6. Implement `Lerp` for `Fx` and `FxVec2`.
7. Write `Arena<T>` and `Id<T>`, with property tests and a snapshot of
   the dump.
8. Write the golden table and its test.
9. Re-export `Fx`, `Fixed`, `FxVec2`, `FxRect`, `Angle`, `Arena` and
   `Id` from `crates/core/src/lib.rs`. Run `cargo xtask check-no-std`
   and the `thumbv7em-none-eabihf` build of `shuyi-core`.
10. The book (2802) gets a short page on positions and angles in a
    simulation, or a `Book:` line says why not.

## Acceptance criteria

- [ ] An ADR records the numeric core, and ADR-0003's status line points
      to it.
- [ ] `Fixed<F>`'s `Mul` and `Div` equal the floor of the exact result
      wherever it fits `i32`, and saturate where it doesn't (property
      tests, 10,000 cases each, for `F` = 8 and 12).
- [ ] `sqrt` is the floor of the exact root for every non-negative
      input, for `Fixed<8>` and `Fixed<12>` (property test).
- [ ] `FxVec2::length` of `(3000, 4000)` px is exactly 5,000 px, and of
      `(8_000_000, 0)` px is exactly 8,000,000 px: no overflow past
      2,896 px.
- [ ] `length` is the floor of the exact length for every `FxVec2`,
      `(Fx::MIN, Fx::MIN)` included (property test against `u128`
      arithmetic, plus that case by hand).
- [ ] `normalize(len)` of a non-zero vector has a length within 2 raw
      units of `len` (property test); zero stays zero.
- [ ] For each of the 256 angles, `Angle::atan2(sin_fx a · k, cos_fx a ·
      k)` returns `a` for `k` in 1, 100 and 10,000 px.
- [ ] `atan2_turn16` is within its documented bound of the `f64`
      `atan2` over a circle of 65,536 points (an exhaustive test).
- [ ] `acos(Fx::ONE)` is `ZERO`, `acos(-Fx::ONE)` is `HALF`, and
      `acos(0)` is `QUARTER`.
- [ ] `Lerp::mix` of `Fx` and `FxVec2` hits both ends exactly.
- [ ] `FxRect::contains` keeps the left and top edges and drops the
      right and bottom ones; `intersection` of two rects that only touch
      is `None` (unit tests).
- [ ] `Arena`'s ids are never reused while live, iteration is in id
      order, and a save and load gives the same next id (property
      tests).
- [ ] `fx.ron` matches in CI's `test` (x64, ARM64) and `determinism`
      (wasm32) jobs.
- [ ] `shuyi-core` builds for `thumbv7em-none-eabihf`, and
      `cargo xtask check-no-std` passes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: `Fixed<F>`'s algebra and saturation; `sqrt`; `length`
  against `u128`; `normalize`; `Arena`'s ids, order and save and load.
- Exhaustive: `atan2_turn16` over a circle; `atan2` over the 256
  angles.
- Unit: hand-worked vectors and rects; `acos` at its ends; the CORDIC
  constants against `f64`; `mix` at both ends.
- Snapshot: the arena's text dump.
- Golden: `crates/core/tests/golden/fx.ron` on x64, ARM64 and wasm32.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
