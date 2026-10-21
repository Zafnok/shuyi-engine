---
id: "2015"
title: "check-layers over cargo metadata, a float ban in shuyi-core, and a new game's rules crate kept pure"
type: infra
milestone: MVP
model: sonnet-5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2015 — check-layers, a float ban in shuyi-core, and a pure rules crate in new games

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. The engine's layers and its determinism (ADR-0002 §1
and §2, `AGENTS.md` rule 5) are kept mostly by review. As agents write
more of the engine and of games, an unchecked rule wears away: an `f32`
position, an `Instant::now`, a `HashMap` walked in a simulation, a
screen crate pulled into a game's rules.

**What the others have.**
- Bevy's `clippy.toml` bans `f32::sin`, `f32::powf` and the other std
  float functions "for libm determinism", each with a reason that names
  the `bevy_math::ops` function to use instead
  (https://github.com/bevyengine/bevy/blob/main/clippy.toml). Its CI
  job `check-bevy-internal-imports` fails when an example or a test
  imports an internal `bevy_*` crate instead of `bevy`.
- Unity's assembly definitions list the assemblies each one may
  reference; code can't use the others.
- Unreal declares each module's dependencies in its `.Build.cs`, and
  UnrealBuildTool builds a module against those modules only.

**What Shuyi has (checked 2026-10-20).**
- `.github/workflows/ci.yml`, job `platform-deps` (lines 187 to 207):
  `cargo tree` of `shuyi-platform` grepped for device crates
  (macroquad, miniquad, `quad-*`, gilrs, audio crates). It looks at that
  one crate. The `run-gates` skill repeats the grep.
- `cargo xtask check-no-std` (`crates/xtask/src/no_std.rs`) and the
  `no-std` job's `thumbv7em-none-eabihf` build: no `std::` path in
  `shuyi-core`, `shuyi-ui` and `shuyi-platform` outside
  `cfg(feature = "std")`. That already keeps std's `HashMap`,
  `Instant`, `SystemTime`, `env` and `thread` out of the core.
- Nothing checks which engine crate depends on which. Today's edges
  follow ADR-0002 §1 (core on none; assets on core; ui on core and
  assets; platform on assets and ui; app on assets, ui and platform;
  cli on core, assets, ui and platform), but a `shuyi-ui` to
  `shuyi-app` edge would pass CI.
- Nothing checks which outside crates a pure crate uses. `shuyi-core`
  uses `serde` only; a new `hashbrown` or `ahash` (a hash map
  `check-no-std` can't see) would pass. `deny.toml` has
  `[bans] deny = []`.
- No float lint anywhere. The root `clippy.toml` has two lines,
  `allow-unwrap-in-tests` and `allow-expect-in-tests`. `shuyi-core`'s
  only float arithmetic is the chi-square in `rng.rs`'s tests;
  `hash.rs` writes floats by `to_bits`. So a ban outside tests passes
  today (checked with `cargo clippy -p shuyi-core --lib -- -D
  clippy::float_arithmetic`).
- `templates/grid/crates/rules`, the crate `shuyi new grid` makes for a
  game's rules: `shuyi-core`, `shuyi-assets` and `serde`, the
  workspace's lints, and the template's root `clippy.toml` with the
  same two lines. Its `AGENTS.md` says "No I/O, no clock, no floats",
  and nothing checks it. `templates/blank` has no rules crate.
- ADR-0005 §1 lists a `lint` operation with "the boundary gate".
  Nothing builds it, and no ticket owns it.

**Plans.** 2003 (after the MVP) adds `cargo xtask check-modules`, which
reads `cargo metadata` for one edge rule (a default crate never depends
on a module); it builds on this ticket's reader rather than writing a
second one. Parked 3101 denies `clippy::float_arithmetic` in
`shuyi-3d`. Parked 2706's acceptance cites "the existing lint or a grep
test", and neither exists. 2109, 2113, 2115, 2117, 2118 and 2121
already point at 2015's float ban, and 3053 at its layer check.

**Why it matters.**
1. **The game first.** Visions of Shuyi's replays, bots and saves rest
   on `shuyi-core` giving the same state on native and on the web. The
   tickets after the MVP put a lot of new code into `shuyi-core` and
   `shuyi-platform` (2109 to 2115, 2518) and new outside crates with
   it. A float or a new hash map there breaks the game's records at a
   pin bump; this check catches it in the engine's PR instead. Nothing
   here touches the game's own crates.
2. **Other 2D games.** An agent asked for a puzzle or a card game
   writes its rules in the crate `shuyi new grid` made. With an `f32`
   position or a `HashMap` of tiles (std seeds its hasher per process),
   its replays and `cargo shuyi bots` differ from run to run, and no
   gate says why. A clippy error whose reason names the engine's way
   (`BTreeMap`, `SimRng`, a `Command`) is a fix an agent reads and
   follows.
3. **The end goal** (`docs/decisions.md`, 2026-10-19): "make me a snake
   game with Shuyi Engine" in under an hour. Until real time is
   unparked, the agent check measures turn-based games (2026-10-20),
   and their rules live in this crate.

It goes in the MVP next to 2006 (another gate on the engine's own
code), before 1133 and 1134 invite agents to write games: 1134's
`shuyi-gates` skill names these checks.

## Nick input

None.

## Scope

**In:**

**1. `tools/layers.ron`: the layers as data.** It goes under `tools/`,
not `docs/`, so a PR that changes only it still runs the code jobs
(`.github/workflows/changes.yml` counts `docs/*` as docs-only). It
holds:
- `devices`: the crates only a backend may reach, moved from the
  `platform-deps` pattern: `macroquad`, `miniquad`, `quad-*` (a
  trailing `*` is a prefix), `gilrs`, `gilrs-core`, `cpal`, `rodio`,
  `kira`, `alsa`, `alsa-sys`, `coreaudio-rs`, `coreaudio-sys`, `oboe`,
  `symphonia`, `lewton`, `hound`;
- `crates`: one entry per engine crate (a workspace member named
  `shuyi-*`), with:
  - `engine`: the engine crates it may depend on, in ADR-0002 §1's
    order, with `shuyi-platform` between `shuyi-ui` and `shuyi-app`
    (ADR-0024 §1);
  - `outside`: `Only([...])`, the outside crates it may use, each with
    a comment saying what for; or `Any`;
  - `devices`: `false` when no device crate may be anywhere below it;
  - `floats`: `false` when its `src/lib.rs` must deny
    `clippy::float_arithmetic` (a simulation crate).

Today's entries. They pass: checked against `cargo metadata
--all-features` on 2026-10-20.

| Crate | `engine` | `outside` | `devices` | `floats` |
| ----- | -------- | --------- | --------- | -------- |
| `shuyi-core` | none | `Only`: `serde` | false | false |
| `shuyi-assets` | core | `Only`: `serde`, `ron`, `include_dir`, `thiserror`, `once_cell`, `libm`, `png`, `postcard`, `lz4_flex` | false | true |
| `shuyi-ui` | core, assets | `Only`: `thiserror`, `serde`, `ron`, `libm` | false | true |
| `shuyi-platform` | core, assets, ui | `Only`: none | false | true |
| `shuyi-app` | core, assets, ui, platform | `Any` | true | true |
| `shuyi-cli` | core, assets, ui, platform | `Any` | false | true |

`shuyi-platform` may use `shuyi-core` though it doesn't today (2115
adds that edge). The edge points down ADR-0002 §1's table; ADR-0024 §1
said "`shuyi-assets` and `shuyi-ui` only", and the ADR below amends
that. An `engine` list says what a crate may use, not what it must:
only an `Only` list is kept exact. `shuyi-cli` never links `shuyi-app`
and reaches no device crate: the tools run headless, in CI and for
agents (ADR-0005 §1). If a ticket done in between has changed a
crate's dependencies, write the file from `cargo metadata` as it is
when you start, and say so in the PR.

**2. `cargo xtask check-layers`** (`crates/xtask/src/layers.rs`). It
runs `cargo metadata --format-version 1 --all-features --locked` with
`current_dir` set to the repository's root (the CLI tests run the binary
from a temporary folder), with the `CARGO` the xtask runs under, else
`cargo`. It reads `tools/layers.ron`, and fails on:
1. a workspace member named `shuyi-*` with no entry; an entry, or a
   name in an `engine` list, that is no such member;
2. a normal or build dependency on an engine crate that isn't in
   `engine`. Dev-dependencies are free: `shuyi-ui`'s tests use
   `shuyi-platform`'s Harness (`crates/ui/Cargo.toml`);
3. with `Only`: a normal or build dependency (optional and per-target
   ones too) that isn't on the list, and a listed crate the crate no
   longer uses, so the list stays exact;
4. with `devices: false`: a device crate anywhere below the crate over
   the resolve's normal and build edges, on every target. The message
   gives the chain (`shuyi-ui → foo → macroquad`);
5. with `floats: false`: a `src/lib.rs` with no line that starts with
   `#![` and holds `deny(clippy::float_arithmetic)`.

Each error names the crate, the dependency (or the file), the rule and
what to do, for example "shuyi-core uses hashbrown, which is not on its
`outside` list in tools/layers.ron: add it there with a comment saying
why, in this PR (ADR-NNNN)". The check is a pure function over plain
structs, `check(&Layers, &Graph, lib_rs) -> Vec<String>` (with
`lib_rs` a function from a crate's name to its `src/lib.rs` text), so
its tests need no cargo. In the resolve, a dependency's `name` is the
library name (`shuyi_core`); use its `pkg` id and the package's name.

**3. CI and the docs.**
- A `layers` job runs `cargo xtask check-layers` and replaces
  `platform-deps` (`ci.yml` lines 187 to 207), with the same `needs`,
  `if`, checkout and `rustup show`, and a comment naming this ticket
  and the new ADR. `ci-result`'s `needs` (line 309) swaps the two.
- The `run-gates` skill runs `cargo xtask check-layers` in place of the
  `cargo tree` grep, and its `description` says "the crates' layers"
  where it said "the platform crate's dependencies".
- The comments that name `platform-deps` name `check-layers`:
  `crates/platform/Cargo.toml` (line 15) and
  `docs/platform-backends.md` (line 27). ADR-0024 §1 keeps its text;
  the new ADR says what replaced the job.
- `crates/xtask/src/main.rs`'s usage and the xtask's `description` in
  `crates/xtask/Cargo.toml` list `check-layers`.

**4. The float ban in `shuyi-core`.**
`#![cfg_attr(not(test), deny(clippy::float_arithmetic))]` beside
`#![forbid(unsafe_code)]` in `crates/core/src/lib.rs` (line 11). It goes
in `lib.rs` because `[lints] workspace = true` can't be combined with
keys of the crate's own. Tests keep their floats (`rng.rs`'s
chi-square).

**5. A new game's rules crate** (`templates/grid/crates/rules`):
- **its own `clippy.toml`.** A crate's `clippy.toml` replaces the
  workspace's rather than merging, so it repeats the two lines. Each
  `reason` names what to use instead. Checked on the pinned clippy
  (0.1.98): `f32` and `f64` work in `disallowed-types`, and
  `allow-invalid = true` is accepted for paths into crates the rules
  don't depend on yet (it fires once a game adds the crate):

  ```toml
  # The rules are pure and deterministic (AGENTS.md, "The layers"). This
  # file replaces the workspace's clippy.toml for this crate, so it
  # repeats that file's two lines.
  allow-unwrap-in-tests = true
  allow-expect-in-tests = true

  disallowed-types = [
      { path = "std::collections::HashMap", reason = "its order changes from run to run, so replays differ: use BTreeMap" },
      { path = "std::collections::HashSet", reason = "its order changes from run to run, so replays differ: use BTreeSet" },
      { path = "hashbrown::HashMap", reason = "its order depends on its hasher: use BTreeMap", allow-invalid = true },
      { path = "hashbrown::HashSet", reason = "its order depends on its hasher: use BTreeSet", allow-invalid = true },
      { path = "f32", reason = "floats can come out differently on another machine or the web: count in whole tiles, turns or hundredths" },
      { path = "f64", reason = "floats can come out differently on another machine or the web: count in whole tiles, turns or hundredths" },
  ]
  disallowed-methods = [
      { path = "std::time::Instant::now", reason = "the rules have no clock: count turns, or put the time in a Command" },
      { path = "std::time::SystemTime::now", reason = "the rules have no clock: count turns, or put the time in a Command" },
      { path = "std::env::var", reason = "the rules read no settings: pass them in when the state is made, or in a Command" },
      { path = "std::env::var_os", reason = "the rules read no settings: pass them in when the state is made, or in a Command" },
      { path = "std::thread::spawn", reason = "the rules run on one thread, so a replay plays the same" },
      { path = "std::fs::read", reason = "the rules do no I/O: crates/game hands them the file's text, as parse_map takes it" },
      { path = "std::fs::read_to_string", reason = "the rules do no I/O: crates/game hands them the file's text, as parse_map takes it" },
      { path = "std::fs::write", reason = "the rules do no I/O: crates/game saves" },
      { path = "std::fs::File::open", reason = "the rules do no I/O: crates/game hands them the file's text, as parse_map takes it" },
      { path = "std::fs::File::create", reason = "the rules do no I/O: crates/game saves" },
      { path = "rand::random", reason = "the only luck is the SimRng kept in the state", allow-invalid = true },
      { path = "rand::rng", reason = "the only luck is the SimRng kept in the state", allow-invalid = true },
      { path = "rand::thread_rng", reason = "the only luck is the SimRng kept in the state", allow-invalid = true },
      { path = "getrandom::fill", reason = "the only luck is the SimRng kept in the state", allow-invalid = true },
  ]
  ```
- **`#![cfg_attr(not(test), deny(clippy::float_arithmetic))]`** in its
  `src/lib.rs`, after the `//!` lines, so `1.5 * 2.0` with no type
  written is caught too;
- **`tests/layers.rs`:** one test runs `cargo tree -p <its package>
  --edges normal,build --target all --prefix none --format {p}` (with
  `env!("CARGO")` and `env!("CARGO_PKG_NAME")`, so no placeholder is
  needed) and fails if a line's first word (the package's name) is
  `shuyi-ui`, `shuyi-platform`, `shuyi-app`, `shuyi-cli`, a device
  crate of the list above (`quad-*` as a prefix), `rand` or
  `getrandom`. A comment says the device list is a copy of `devices` in
  the engine's `tools/layers.ron`. The message names the crate, says
  that screens go in `crates/game` and that the only luck is `SimRng`,
  and gives `cargo tree -p <package> -i <crate>` to see the way in. It
  runs in `cargo test --workspace`, so it works on Windows too and adds
  no gate (a probe of it ran in 0.04 s);
- **`AGENTS.md`, *The layers*:** two sentences. Clippy and the layers
  test turn the rules above into errors, each saying what to use. A
  failure is fixed in the code, never by loosening `clippy.toml` or the
  test (a test that must use a float writes
  `#[expect(clippy::disallowed_types, reason = "...")]`).
- No `{{` in these files: `crates/cli/src/contract/new.rs` allows only
  its placeholders in a template's text.
- The template's own `.github/workflows/ci.yml` doesn't change: it runs
  clippy and the tests already.

**6. The engine's `templates` job proves the fence**, for `grid` only
(`if: matrix.template == 'grid'`), after "Run its gates", in
`${{ runner.temp }}/demo` as that step is:
- it keeps a copy of the made game's `crates/rules/src/lib.rs`, appends
  a function with a `HashMap` parameter, an `f32`, `Instant::now()` and
  `1.5 * 2.0`, runs `cargo clippy -p demo-rules --message-format=short
  -- -D warnings`, expects it to fail, and finds each of "disallowed
  type `std::collections::HashMap`", "disallowed type `f32`",
  "disallowed method `std::time::Instant::now`" and "floating-point
  arithmetic detected" in its output (these are clippy 0.1.98's words,
  seen in a probe);
- it puts the file back, appends `shuyi-ui.workspace = true` to
  `crates/rules/Cargo.toml` (its last table is `[dependencies]`), and
  expects `cargo test -p demo-rules --test layers` to fail, naming
  `shuyi-ui`.

**7. An ADR** (the `write-adr` skill, the next free number), extending
ADR-0002 §1 and §2, ADR-0008 §2 and ADR-0024 §1:
- `tools/layers.ron` is the layer table as data. A new engine crate, a
  new engine edge, or a new outside crate in a pure crate adds its line
  there in the same PR, so each is a visible choice;
- `check-layers` replaces ADR-0024 §1's `platform-deps` job, and
  `shuyi-platform` may also depend on `shuyi-core`, which is below it;
- a simulation crate denies `clippy::float_arithmetic` outside tests in
  its `lib.rs` and is marked `floats: false`: `shuyi-core` now, and
  later `shuyi-ecs` (2107) and `shuyi-procgen` (2118). `shuyi-3d`
  (3101) denies it per module outside `draw` and stays `floats: true`;
- a new game's rules crate ships with the `clippy.toml`, the float ban
  and the layers test above;
- alternatives considered:
  - cargo-deny's `[bans] deny` with `wrappers`: a wrapper list names
    the crates that may depend on a banned crate directly, with no
    difference between a normal and a dev-dependency (`shuyi-ui`'s
    tests use `shuyi-platform`); it holds no list of the outside crates
    each pure crate may use; and `deny.toml` reads the default features
    only (`[graph] all-features = false`);
  - a `clippy.toml` in `shuyi-core` banning std's hash maps and clocks:
    the `no_std` build and `check-no-std` already do it;
  - a grep test for floats: it misses types the compiler infers;
  - waiting for 2003's `check-modules`: it comes after the MVP, and
    builds on this.

**What changes for the game:** nothing. No public API changes;
`shuyi-core` gains a lint attribute that passes. The game's own crates
and gates are untouched.

**Either order with 2501:** if 2501 is done, the rules crate's
`clippy.toml`, `tests/layers.rs`, float ban and `AGENTS.md` lines go in
the part that holds the rules crate; if not, 2501 moves them with the
rest of `templates/grid`.

**Either order with 2511:** if 2511 is done and its minimal template
has a rules crate, that crate gets the same `clippy.toml`, float ban and
layers test here; if not, 2511 copies them.

**Out (do not do):**
- floats in `shuyi-assets` (its lights' rounding through `libm`),
  `shuyi-ui`, `shuyi-platform` and `shuyi-app`: ADR-0002 §2 allows
  floats in drawing;
- a `clippy.toml` in `shuyi-core`: the `no_std` build and
  `check-no-std` keep std's hash maps and clocks out, and its `outside`
  list keeps `hashbrown` out;
- a list of game content the engine must not hold (rule 4; the game's
  own gate from game ticket 1102);
- Visions of Shuyi's own crates;
- `templates/blank` (no rules crate) and `examples/hello-grid` (one
  crate, with its rules beside its screens);
- 2003's `check-modules`, which builds on `layers.rs` (2003 waits for
  this ticket);
- the `shuyi-gates` skill's text (1134 waits for this ticket);
- a `lint` operation in the contract that reports these findings to
  agents (ADR-0005 §1's "boundary gate"): no ticket owns it yet, so
  this ticket's step 10 writes one. Until it is done,
  `cargo clippy --message-format=json` and `cargo test` report them;
- any change to `deny.toml`.

## Implementation steps

1. Read ADR-0002 §1 and §2, ADR-0008 §2, ADR-0024 §1,
   `crates/xtask/src/no_std.rs` and `main.rs`, and the `platform-deps`
   and `templates` jobs in `ci.yml`.
2. Write `tools/layers.ron` from `cargo metadata --format-version 1
   --all-features --locked` as it stands.
3. Write `crates/xtask/src/layers.rs`: `Layers`, `CrateRule` and
   `Outside` (serde, read with `ron`); `Graph` (the members, their
   declared dependencies by kind, and the resolve's normal and build
   edges by package name) from the metadata's JSON; `check`; and
   `run(root)`. The xtask gains `serde` (with `derive`), `serde_json`
   and `ron` (with `std`): all MIT OR Apache-2.0, allowed by
   `deny.toml`, and already in `Cargo.lock`.
4. Wire `check-layers` into `main.rs` and its usage text, and into the
   xtask's `description`.
5. Add the float ban to `crates/core/src/lib.rs` and run clippy.
6. Swap the CI job, then update the `run-gates` skill and the two
   comments.
7. Write the rules crate's `clippy.toml`, the attribute, `tests/layers.rs`
   and the `AGENTS.md` lines. Format `tests/layers.rs` with `rustfmt
   --edition 2024`: `templates/` is outside `cargo fmt --all`, and CI's
   `templates` job checks it. Make a game with `cargo shuyi new grid
   <a temporary folder> --engine-path <this checkout>` and run its gates.
8. Add the proof step to the `templates` job.
9. Write the ADR and its row in `docs/adr/README.md`.
10. Unless a ticket in `tickets/open` owns it by then, write one with the
    `write-ticket` skill for ADR-0005 §1's `lint` operation:
    `check-keys`, `check-text` and these findings in one answer with file
    and line. Name it in the completion notes.
11. Run the gates.

Size: if the diff runs well past 600 changed lines, move the new game's
rules-crate guard (step 7 and step 8's proof) to a follow-up ticket that
1134 also waits for, and keep `check-layers`, the float ban and the CI
swap here.

## Acceptance criteria

- [ ] `cargo xtask check-layers` passes on the repository, and CI's
      `layers` job runs it. `platform-deps` is gone, and `ci-result`
      needs `layers`.
- [ ] Each of the five rules fails on its fixture with a message naming
      the crate, the dependency or file, and the rule; the device rule's
      message shows the chain.
- [ ] A dev-dependency on any engine crate passes, and `quad-*` matches
      `quad-snd` but not `quadtree` (fixtures).
- [ ] `crates/core/src/lib.rs` denies `clippy::float_arithmetic` outside
      tests, and `cargo clippy --workspace --all-targets --locked -- -D
      warnings` passes.
- [ ] A game made with `shuyi new grid` passes its own gates (CI's
      `templates` job).
- [ ] The `templates` job's proof step finds the four clippy messages,
      and the layers test failing on `shuyi-ui`.
- [ ] The `run-gates` skill runs `cargo xtask check-layers` and no longer
      has the `cargo tree` grep.
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] A ticket in `tickets/open` owns the contract's `lint` operation.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`layers.rs`): each rule on a small `Graph` built in code;
  `Only` and `Any`; a stale `Only` entry; a free dev-dependency; the
  device chain; the prefix match; parsing a small `cargo metadata`
  JSON sample (normal, dev and build kinds, a per-target dependency,
  the library name against the package name); `tools/layers.ron`
  parses; the repository passes (real `cargo metadata`, as
  `no_std.rs`'s `the_checked_crates_are_clean` reads the real tree);
  `templates/grid/crates/rules/tests/layers.rs` names every `devices`
  entry of `tools/layers.ron` (a text check, so the copy stays in step).
- CLI (`crates/xtask/tests/cli.rs`): `check-layers` passes on the
  repository, and fails with the usage when given an argument.
- Template: `tests/layers.rs` in the made game; the proof step in CI.
- Mutation testing in CI covers `layers.rs`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
