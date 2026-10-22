# ADR-0032: The layers as data, checked over cargo metadata, a float ban in the simulations, and a new game's rules crate kept pure

- **Status:** Accepted
- **Date:** 2026-10-22
- **Related tickets:** 2015; 2003 (its `check-modules` builds on this
  check), 2107 and 2118 (new simulation crates), 3101 (`shuyi-3d`)

## Context

ADR-0002 §1 says which engine crate may depend on which, and §2 keeps
floats, clocks and unseeded luck out of a simulation. ADR-0008 §2 and
ADR-0024 §1 let only platform backends touch devices. Before this ADR
these rules were kept mostly by review:

- CI's `platform-deps` job grepped `cargo tree` of `shuyi-platform` for
  device crates, and looked at that crate only;
- `cargo xtask check-no-std` and the `no_std` builds keep std's
  `HashMap`, `Instant`, `env` and `thread` out of `shuyi-core`, `shuyi-ui`
  and `shuyi-platform`;
- nothing checked which engine crate depends on which (a `shuyi-ui` to
  `shuyi-app` edge would have passed), which outside crates a pure crate
  uses (a `hashbrown` in `shuyi-core` would have passed), or floats
  anywhere;
- a new game's rules crate (`shuyi new grid`) said "no I/O, no clock, no
  floats" in its `AGENTS.md`, and nothing checked it.

As agents write more of the engine and of games, an unchecked rule wears
away. Bevy bans std's float functions in its `clippy.toml`, each with a
reason naming what to use, and fails CI when an example imports an
internal crate; Unity's assembly definitions and Unreal's `.Build.cs`
files list the modules each one may use.

## Decision

### 1. `tools/layers.ron` is the layer table as data

[`tools/layers.ron`](../../tools/layers.ron) holds:

- `devices`: the crates only a platform backend may reach (macroquad,
  miniquad, `quad-*`, gilrs, the audio crates); a trailing `*` is a
  prefix;
- `crates`: one entry per engine crate (a workspace member named
  `shuyi-*`), with `engine` (the engine crates it may depend on),
  `outside` (`Only([...])`, each with a comment saying what for, or
  `Any`), `devices` and `floats`.

A new engine crate, a new edge between engine crates, or a new outside
crate in a pure crate adds its line there in the same PR, so each is a
visible choice. It lives under `tools/`, not `docs/`, so a PR that
changes only it runs the code jobs.

`shuyi-platform` may depend on `shuyi-core`, which is below it in
ADR-0002 §1's table: this amends ADR-0024 §1's "`shuyi-assets` and
`shuyi-ui` only". `shuyi-cli` may use every crate but `shuyi-app`, and
reaches no device: the tools run headless (ADR-0005 §1).

### 2. `cargo xtask check-layers` replaces the `platform-deps` job

`crates/xtask/src/layers.rs` reads `cargo metadata --format-version 1
--all-features --locked` and the file, and fails on:

1. an engine crate with no entry, or an entry or `engine` name that is
   no engine crate;
2. a normal or build dependency on an engine crate not on `engine`
   (dev-dependencies are free: `shuyi-ui`'s tests use `shuyi-platform`'s
   Harness);
3. with `Only`, a normal or build dependency not on the list (optional
   and per-target ones too), and a listed crate no longer used;
4. with `devices: false`, a device crate anywhere below the crate over
   the resolve's normal and build edges, on every target, with the chain
   in the message;
5. with `floats: false`, a `src/lib.rs` with no `#![...]` line that
   denies `clippy::float_arithmetic`.

Each error names the crate, the dependency or file, the rule and what to
do. The check is a pure function over plain structs, so its tests need
no cargo. CI's `layers` job runs it in place of `platform-deps`, which
looked at one crate with a grep; 2003's `check-modules` reuses its
reader.

### 3. A simulation crate denies float arithmetic

A simulation crate writes `#![cfg_attr(not(test), deny(clippy::float_arithmetic))]`
in its `src/lib.rs` and is marked `floats: false`: `shuyi-core` now, and
later `shuyi-ecs` (2107) and `shuyi-procgen` (2118). It goes in `lib.rs`
because `[lints] workspace = true` can't be combined with a crate's own
lint keys. Tests keep their floats (`rng.rs`'s chi-square). `shuyi-3d`
(3101) denies it per module outside `draw` and stays `floats: true`.
Drawing crates (`shuyi-assets`, `shuyi-ui`, `shuyi-platform`,
`shuyi-app`) may use floats, as ADR-0002 §2 says.

### 4. A new game's rules crate ships fenced

`templates/grid/crates/rules` ships with:

- its own `clippy.toml`, which bans std's and hashbrown's hash maps and
  sets, `f32` and `f64`, the clocks, `env::var`, `thread::spawn`, the
  file reads and writes, and `rand`'s and `getrandom`'s luck, each with
  a reason that names what to use (`BTreeMap`, a `Command`, `SimRng`).
  Paths into crates the rules don't depend on yet carry
  `allow-invalid = true`, so they fire once a game adds the crate. A
  crate's `clippy.toml` replaces the workspace's, so it repeats that
  file's two lines;
- the float ban of §3 in its `src/lib.rs`, so `1.5 * 2.0` with no type
  written is caught too;
- `tests/layers.rs`, which runs `cargo tree` on the crate and fails on
  `shuyi-ui`, `shuyi-platform`, `shuyi-app`, `shuyi-cli`, a device crate
  (a copy of `devices`, which a test of the xtask keeps in step), `rand`
  or `getrandom`. It runs in `cargo test --workspace`, so it adds no
  gate to the game.

CI's `templates` job proves the fence on the made game: clippy fails
with each kind of message on a function that breaks the rules, and the
layers test fails on a `shuyi-ui` dependency.

## Consequences

- A layer is a line in a file, and breaking one fails CI with a message
  that says what to do. New crates and new outside dependencies of pure
  crates become choices a PR shows.
- Every engine crate's outside dependencies are in two places for the
  pure ones (`Cargo.toml` and `tools/layers.ron`); the check keeps them
  equal, so adding or removing one is a two-file change.
- A game made with `shuyi new grid` hears about a `HashMap`, a float or
  a screen crate in its rules from clippy and `cargo test`, before its
  replays differ.
- ADR-0005 §1's `lint` operation, which would report these findings
  through the tool contract, is still to be built (its own ticket).

## Alternatives considered

- **cargo-deny's `[bans] deny` with `wrappers`** — a wrapper list names
  the crates that may depend on a banned crate directly, with no
  difference between a normal and a dev-dependency (`shuyi-ui`'s tests
  use `shuyi-platform`); it holds no list of the outside crates each
  pure crate may use; and `deny.toml` reads the default features only
  (`[graph] all-features = false`).
- **A `clippy.toml` in `shuyi-core` banning std's hash maps and clocks**
  — the `no_std` build and `check-no-std` already do it, and the
  `outside` list keeps `hashbrown` out.
- **A grep test for floats** — it misses the types the compiler infers.
- **Waiting for 2003's `check-modules`** — it comes after the MVP, and
  builds on this.
