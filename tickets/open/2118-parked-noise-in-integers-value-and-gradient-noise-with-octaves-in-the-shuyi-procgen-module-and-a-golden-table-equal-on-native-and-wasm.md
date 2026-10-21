---
id: "2118"
title: "Parked: Noise in integers: value and gradient noise with octaves, in a new shuyi-procgen module, with a golden table equal on native and wasm"
type: feature
milestone: "Engine: roguelikes"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2003", "2013", "2015", "2114", "2116"]
nick_input: none
completed:
---

# 2118 — Noise in integers, and the shuyi-procgen module

> **Parked** (Nick, 2026-10-20, choosing roguelikes as a kind of game the engine serves; Visions of Shuyi is "the first game I want to be launched with the engine"): like real-time strategy, visual novels and detective games, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and Construct 3. All of them but Bevy have noise built in:

- Godot's `FastNoiseLite`: value, Perlin, simplex and cellular noise,
  with fractal octaves
  (https://docs.godotengine.org/en/stable/classes/class_fastnoiselite.html);
- Unity's `Mathf.PerlinNoise`
  (https://docs.unity3d.com/ScriptReference/Mathf.PerlinNoise.html);
- Unreal's `FMath::PerlinNoise2D`, and its PCG framework for generated
  content
  (https://dev.epicgames.com/documentation/en-us/unreal-engine/procedural-content-generation-overview);
- Construct 3's Advanced Random plugin.

Bevy has none; its users take the `noise` crate. Every one of these
works in floats. Shuyi bans floats in a simulation (`AGENTS.md` rule 5,
ADR-0003 §3), so a game can't use them in a seeded run that must replay
the same on native and on the web. Noise that is the same everywhere is
something only the engine can give conveniently.

What Shuyi has today:

- no noise anywhere: `grep -rni "noise\|perlin" crates/` finds only a
  test's bytes named `noise` in `crates/assets/src/pack/tests.rs`;
- `shuyi_core::Grid<T>` and `Pos` (`crates/core/src/grid.rs`); `Grid`
  has no `map` method yet;
- `SimRng`, a PCG32 written in the crate, and the `RandomSource` trait
  (`crates/core/src/rng.rs`). 2114 adds `mix64` and `mix_at` there, the
  engine's one SplitMix64, and an ADR that freezes their outputs;
- `Fx` is planned in 1119 (parked) as an `i32` with 8 fraction bits.
  This ticket needs none of it.

Why it matters:

- **The game first:** Visions of Shuyi needs no generated levels, which
  is why this waits for its launch.
- **Other 2D games:** Nick added roguelikes to the kinds of game the
  engine serves on 2026-10-20 (`docs/decisions.md`). A roguelike's
  floors come from a seed, so a shared seed gives every player the same
  dungeon. That only works if the generation is the same on every device
  and stays the same for a seed. Farming and survival games use noise
  for terrain too.
- **The end goal** (2026-10-19): an agent asked for a roguelike gets
  tested, deterministic pieces instead of writing float noise of its
  own.

This ticket starts the `shuyi-procgen` module, with what ADR-0009 §4
asks of a module. 2119 adds the level generators and weighted tables,
2120 adds field of view (in `shuyi-core`), and 2735 the example and
template that prove the direction.

It is blocked by:

- 2003: the catalog, module metadata, `docs/modules.ron` with the
  `kind` concern (ADR-0015 §1), `cargo xtask check-modules` and the
  `shuyi` crate;
- 2013: the ADR on public types that can grow, which `NoiseKind` and
  `Noise` follow;
- 2015: `tools/layers.ron`, `cargo xtask check-layers`, and the float
  ban a simulation crate carries in its `lib.rs`;
- 2114: `mix_at`, which the lattice hash uses;
- 2116: `shuyi_assets::golden`, `cargo xtask golden` and CI's
  `determinism` job, which runs the golden table on wasm32.

## Nick input

None.

## Scope

**In:**
- **The crate** `crates/procgen` (`shuyi-procgen`), with the module
  duties of ADR-0009 §4 (the root `Cargo.toml`'s `crates/*` already
  makes it a workspace member):
  - its dependencies are `shuyi-core` (`default-features = false`) and
    `serde` only. It is `no_std` with `alloc` and a default `std`
    feature (`std = ["shuyi-core/std", "serde/std"]`), as
    `crates/core/Cargo.toml` does. Its dev-dependencies are `proptest`,
    `ron` (with `std`) and `shuyi-assets` with `default-features =
    false, features = ["golden"]`, so the no-std test run stays without
    `std`. All are in the workspace already: no new dependency;
  - metadata: concern `kind`, answer `roguelike`; `serves` the top-down
    grid perspective; `alternatives` empty (the `kind` concern is
    `alternatives: false`); `works_with` empty for now;
  - **the `kind` concern** in `docs/modules.ron`: 2003 writes it
    without roguelikes. Add the answer `roguelike`, status building,
    naming this module and tickets 2118–2120 and 2735. If a ticket in
    between added it as parked, set it to building. If 2003's statuses
    are only planned, parked and ready, add building (2701 and 2721 use
    it too);
  - the `procgen` feature of the `shuyi` crate;
  - a row in `docs/modules.md` (2003's first rows don't include this
    module): `shuyi-procgen`, building, tickets 2118, 2119 and 2735.
    2735 sets the row and the answer to ready when its example passes
    CI;
  - the README's first two sections, in ADR-0009 §4's order:
    - what it is for: maps made from a seed for grid games, roguelikes
      first;
    - what it is not for: hand-made levels (the `.map` format), field of
      view (`shuyi_core::sight`, 2120), worlds without a grid.

    2735 writes the rest;
  - the `shuyi-procgen` skill: if the skill pack (1134) is done, a short
    skill that says what exists and that 2119 and 2735 are to come. If
    not, add `shuyi-procgen` to 1134's list in this PR;
  - if 2004's guide check is in place, a planned page
    `docs/guides/roguelike.md` naming 2119, 2120 and 2735. If not,
    nothing: 2735 writes the guide after 2004.
- **`shuyi_procgen::noise`**, integers only:
  - `pub const ONE: i32 = 1 << 16`. Noise values are in `-ONE..=ONE`,
    with 16 fraction bits;
  - the maths runs in `i64`. Every division is `div_euclid` by a
    positive divisor, so it rounds toward negative infinity, as `Fx`
    will (ADR-0003 §3). Never `/` on a value that can be negative;
  - no `as` cast that pedantic clippy flags, and no `#[allow]`:
    `i32::cast_unsigned` for the lattice index, `(h >> 32) as u32` for a
    high half (clippy accepts it), and `i32::try_from(..).unwrap_or(..)`
    after a clamp, as `SimRng::roll_percent` narrows;
  - coordinates wrap at the `i32` edges (`wrapping_add`), so no position
    panics, `i32::MAX` included;
  - `pub fn lattice(seed: u64, x: i32, y: i32) -> u32`: the hash of one
    lattice point, through the engine's one mixing function: the high
    32 bits of `shuyi_core::rng::mix_at(seed, index)` (2114), with
    `index = (u64::from(x.cast_unsigned()) << 32) |
    u64::from(y.cast_unsigned())`. No second mixer is written here.
    There is no `usize` anywhere in the maths, so a 32-bit target
    (wasm32) gives the same bits;
  - `pub fn value(seed: u64, period: u16, pos: Pos) -> i32`: value
    noise.
    - A period of 0 is read as 1 (documented), so nothing divides by 0.
    - The lattice cell is `pos` divided by `period` with `div_euclid`,
      so negative positions work. The far corners are the cell plus 1,
      with `wrapping_add`.
    - The fraction of each coordinate `c` is
      `(i64::from(c.rem_euclid(period)) << 16).div_euclid(period)`, in
      `0..ONE`.
    - It is faded by `3t² − 2t³`: `(t * t * (3 * ONE - 2 * t))
      .div_euclid(ONE * ONE)` fits `i64` in one product.
    - Each corner's value is `i64::from(lattice(..) >> 15) -
      i64::from(ONE)`, in `-ONE..ONE`.
    - The four corners are mixed bilinearly: `lerp(a, b, s) = a + ((b -
      a) * s).div_euclid(ONE)`, along `x` then `y`;
  - `pub fn gradient(seed: u64, period: u16, pos: Pos) -> i32`: gradient
    noise, in the manner of Perlin's improved noise.
    - The cell, the fraction and a period of 0 as in `value`.
    - Each corner picks a gradient by the top 3 bits of its lattice hash
      (`h >> 29`) from `[(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1),
      (-1, 1), (1, -1), (-1, -1)]`, and takes its dot product with the
      offset from that corner (`(tx, ty)`, `(tx - ONE, ty)`, `(tx, ty -
      ONE)`, `(tx - ONE, ty - ONE)`).
    - The fade is `6t⁵ − 15t⁴ + 10t³`. It overflows `i64` as one
      product, so divide by `ONE` after each multiplication:
      `t2 = (t * t).div_euclid(ONE)`, `t3 = (t2 * t).div_euclid(ONE)`,
      then `s = (t3 * ((t * (6 * t - 15 * ONE)).div_euclid(ONE) + 10 *
      ONE)).div_euclid(ONE)`.
    - The dots are mixed with `lerp`, as in `value`. The result is
      clamped to `-ONE..=ONE` as the last step, with no other scaling;
  - **`NoiseKind`** (`Value`, `Gradient`), `#[non_exhaustive]`: a set
    that will grow (2013), so a new kind is not a breaking change;
  - **`Noise`, the settings as data** (ADR-0009 §4). Its fields are
    private, since they keep their ranges (2013), with a getter each:
    - `kind: NoiseKind`;
    - `period: u16`: tiles across one lattice cell of the first octave,
      at least 1;
    - `octaves: u8`: 1 to 8, default 1;
    - `gain_percent: u8`: at most 100, default 50.

    `Noise::new(kind, period, octaves, gain_percent) -> Result<Noise,
    NoiseError>` is the one check. Reading goes through it too: serde
    `try_from` a raw form with `deny_unknown_fields` and the two
    defaults, as `Grid` reads through `RawGrid` in
    `crates/core/src/grid.rs`. `NoiseError`'s message names the field
    and its range;
  - `Noise::sample(&self, seed: u64, pos: Pos) -> i32`: the octaves,
    summed:
    - octave `i` has the period `max(1, period >> i)`. Octave 0 uses
      `seed`, and octave `i ≥ 1` uses `mix_at(seed, i)`, so neighbouring
      seeds (one per floor, say) share no octave;
    - its weight is `amp(i)`, with `amp(0) = ONE` and
      `amp(i + 1) = (amp(i) × gain_percent).div_euclid(100)`;
    - the weighted sum is divided by the sum of the weights (at least
      `ONE`), so the result stays in `-ONE..=ONE`;
  - `Noise::grid(&self, seed: u64, origin: Pos, width: u16, height: u16)
    -> Grid<i32>`: a rectangle of samples from `origin`, row-major.
- **`Grid::map`** in `crates/core/src/grid.rs`:
  `pub fn map<U>(&self, f: impl FnMut(&T) -> U) -> Grid<U>`, the same
  size, row-major. A game turns samples into its own terrain with it.
  If another ticket has added it first, use that one.
- **The golden table**, `crates/procgen/tests/golden/noise.ron`, through
  2116's `shuyi_assets::golden`, checked by
  `crates/procgen/tests/golden.rs` (its test's name has `golden` in it,
  as 2116 asks):
  - seeds `0` and `0xDEAD_BEEF`, both kinds, 1 and 4 octaves, periods 8
    and 13, gain 50, the 12 × 12 samples of `Noise::grid` from
    `(-6, -6)`;
  - one row per configuration (seed, kind, octaves, period), named
    `"<seed in 16 hex digits> <kind> octaves <n> period <nn>"` (as in
    `"00000000deadbeef gradient octaves 4 period 13"`), its value the
    144 samples on one line, row-major, separated by spaces;
  - it is blessed with `cargo xtask golden --bless`, never by an ignored
    test. It changes only in a PR with a `Breaking:` line (the ADR
    below);
  - add `-p shuyi-procgen` to the wasm32 `cargo test` of CI's
    `determinism` job (`.github/workflows/ci.yml`), so CI checks it on
    x64, ARM64 and wasm32.
- **An ADR** (the `write-adr` skill, the next free number):
  - roguelikes as the `kind` answer `roguelike`, served by
    `shuyi-procgen` (noise here; generators and weighted tables in
    2119), the core's field of view (2120), and the example and template
    (2735);
  - why a module and not `shuyi-core`: a game without generated maps
    would pay for it in concepts to learn (ADR-0009 §1);
  - why not `Fx`, though ADR-0009 §4 says a module's simulation part is
    in `Fx`: `Fx` is 1119's, parked with real time, and a turn game's
    map needs none of it. 16 fraction bits keep a period of up to
    65 535 tiles moving at least one unit per tile, where `Fx`'s 8 bits
    stop at 256. `n >> 8` converts to `Fx`;
  - **what a seed promises**, as 2114's ADR does for the RNG: for the
    same seed and settings, a noise value or a generated map never
    changes. New behaviour goes in a new `NoiseKind` or a new function,
    never in place of an old one. A change that still moves a value (a
    bug fix) is a breaking change: its PR carries 2010's `Breaking:`
    line and blesses the golden tables (2116) in the same PR;
  - integers only: values as `i32` with 16 fraction bits, positions in
    whole tiles, a period in tiles.
- **Gates:**
  - add `"crates/procgen"` to `CRATES` in `crates/xtask/src/no_std.rs`
    (line 21);
  - add `-p shuyi-procgen` to the `no-std` job's build for
    `thumbv7em-none-eabihf` (`.github/workflows/ci.yml`, line 174) and
    to its test run without `std` (line 185);
  - put `#![cfg_attr(not(test), deny(clippy::float_arithmetic))]` in
    `crates/procgen/src/lib.rs`, and give `shuyi-procgen` an entry in
    `tools/layers.ron` (2015) with `floats: false`, `devices: false`,
    `engine` `shuyi-core`, and `outside` `Only` `serde`.

**Out (do not do):**
- simplex, cellular (Worley) and ridged noise, and domain warping: a new
  `NoiseKind` each, when a game asks;
- 3D noise. A game that wants noise over time passes the time as `y`;
- `Fx` inputs or outputs. `Fx` (1119) has 8 fraction bits, so `n >> 8`
  converts. If 1119 is done, add a one-line `to_fx` with a test; if not,
  nothing;
- level generators and weighted tables (2119);
- field of view (2120);
- the example, the template, the guide and the rest of the README
  (2735);
- `shuyi_assets::golden`, `cargo xtask golden` and the `determinism`
  job (2116), and `check-layers` (2015): this ticket only adds the
  crate to them;
- noise drawn in the debug view (2315): a follow-up if a developer asks.

## Implementation steps

1. Read ADR-0009 §4, 2003's catalog and metadata, 2013's ADR, 2114's
   `mix_at` and ADR, 2116's `shuyi_assets::golden` and `determinism`
   job, and 2015's `tools/layers.ron`.
2. Create `crates/procgen`: its `Cargo.toml` with the dependencies,
   features and metadata above, `src/lib.rs` with the float ban, and
   the README's first two sections. Add the `procgen` feature of
   `shuyi`, the catalog row, the `docs/modules.ron` answer and the
   `tools/layers.ron` entry.
3. Add `Grid::map` to `shuyi-core`, with a test.
4. Write `lattice`, `value`, `gradient`, `NoiseKind`, `Noise` and
   `NoiseError`, each with its unit and property tests.
5. Write the golden table's test, bless it with `cargo xtask golden
   --bless`, and add `-p shuyi-procgen` to the `determinism` job.
6. Add the crate to the `no-std` job (build and tests) and to
   `no_std.rs`'s list. Write the ADR and its row in
   `docs/adr/README.md`. Write the skill, or the entry in 1134's list,
   and the planned guide page if 2004's check is in place.
7. Run the gates.

**Size:** about 600 to 650 changed lines without the golden table. If
the diff runs long, `NoiseKind::Gradient` (`gradient`, its tests and
its golden rows) moves to a parked follow-up ticket (the `write-ticket`
skill, blocked by this one); `NoiseKind` is `#[non_exhaustive]`, so
adding it later breaks nothing. Add that ticket to 2735's `blocked_by`
(its floors use gradient noise) and say so in the completion notes.

## Acceptance criteria

- [ ] `cargo build -p shuyi-procgen --no-default-features --target
      thumbv7em-none-eabihf` passes, and its tests pass without `std`,
      in CI's `no-std` job. `cargo xtask check-no-std` covers
      `crates/procgen`.
- [ ] Every row of the golden table matches in CI's `test` job (x64 and
      ARM64) and its `determinism` job (wasm32).
- [ ] Samples stay in `-ONE..=ONE` for any seed, settings and position
      (property test).
- [ ] `Noise` reads from RON. A period of 0, 0 or 9 octaves, a gain over
      100 and an unknown field are refused, and the first three with a
      message naming the field. `Noise::new` refuses the same values.
- [ ] `cargo xtask check-modules` and `cargo xtask check-layers` pass
      with the new crate.
- [ ] `crates/procgen/src/lib.rs` denies `clippy::float_arithmetic`
      outside tests, and `grep -rnE "f32|f64" crates/procgen/src` finds
      nothing.
- [ ] The ADR is in `docs/adr/` and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - `value` at a lattice point equals that corner's value;
  - with period 1, `value` gives the corners' values;
  - `gradient` is 0 at every lattice point, for any period;
  - period 0 gives the same as period 1;
  - one octave of `Noise` equals the base kind;
  - negative positions: with period 8, `(-1, -1)` is in the cell
    `(-1, -1)`;
  - positions at `i32::MIN` and `i32::MAX` give values in range, with no
    panic;
  - `Noise` from RON, `Noise::new`, and each of their errors;
  - `Grid::map` keeps the size and the order.
- Property:
  - the range, above;
  - the same inputs give the same value;
  - with one octave, neighbouring samples (`x` and `x + 1`) differ by at
    most `3 × ONE / period + 8` for value noise and
    `20 × ONE / period + 8` for gradient noise. The bounds come from the
    fade curves' slopes; the test's comment shows the sum.
- Golden: the table, natively in CI's `test` job and on wasm32 in its
  `determinism` job.

## Completion notes

*(Filled in by the session that completes the ticket.)*
