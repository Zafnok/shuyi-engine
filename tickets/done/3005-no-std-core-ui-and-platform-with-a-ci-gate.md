---
id: "3005"
title: "no_std with alloc for shuyi-core, shuyi-ui and shuyi-platform, with a CI gate that builds them for thumbv7em-none-eabihf"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["game#1131", "3003"]
nick_input: none
completed: 2026-10-20
---

# 3005 — `no_std` core, UI and platform

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§5. Playdate games have no standard library, so Rust there is `no_std`
with `alloc` (game ticket 1025 found the same). Closed consoles vary
too. The longer the core uses `std`, the dearer this becomes, so it
comes before the Playdate backend (3010), and the gate keeps it from
coming back.

## Nick input

None.

## Scope

**In:**
- **The crates:**
  - `shuyi-core`, `shuyi-ui` and `shuyi-platform` get
    `#![cfg_attr(not(feature = "std"), no_std)]`, with `extern crate
    alloc`;
  - a `std` feature, on by default, so nothing changes for desktop
    builds;
  - `default-features = false` builds for `thumbv7em-none-eabihf`.
- **The replacements:**
  - `std::collections::HashMap`/`HashSet` become `BTreeMap`/`BTreeSet`,
    or `hashbrown` with a fixed hasher where order doesn't matter and
    speed does (with a comment why);
  - `std::error::Error` becomes `core::error::Error`, with `thiserror` 2
    without `std`;
  - `format!` and friends come from `alloc`;
  - no `std::time`, `std::thread` or `std::sync` outside
    `cfg(feature = "std")`.
- **Dependencies.** Every dependency of these crates must build without
  `std`. Any that can't is either made optional behind `std` or
  replaced; list each decision in the PR, with its license checked under
  ADR-0002 §8.
- **The CI gate.** `cargo build -p shuyi-core -p shuyi-ui -p
  shuyi-platform --no-default-features --target thumbv7em-none-eabihf`
  runs on every PR.
- **Tests stay `std`.** Tests run with `std` on the host as before, and
  one test per crate runs the headless backend with `--no-default-features`
  on the host, to prove the `no_std` path behaves the same.
- **The `run-gates` skill** gets the new command, with the `rustup
  target add thumbv7em-none-eabihf` line. It builds on Windows with the
  GNU host toolchain. A `no_std` library build needs no linker, so check
  that and say so in the skill.

**Out (do not do):**
- `shuyi-assets`' text decoders, which may stay `std` (ADR-0008 §5);
- the pack reader's `no_std` (ticket 2204 does it);
- the Playdate backend (3010).

## Implementation steps

1. Add the `std` feature and the attributes, crate by crate (`core`
   first). Fix what breaks.
2. Replace the maps. Run the replay and determinism tests after each
   crate, because iteration order may change, and any snapshot that
   changes must be explained in the PR.
3. Add the CI job and the skill lines.

## Acceptance criteria

- [x] The three crates build for `thumbv7em-none-eabihf` with
      `--no-default-features`, in CI.
- [x] Every test passes with default features, and replays recorded
      before the change replay the same.
- [x] No `std::` path outside `cfg(feature = "std")` in these crates
      (a grep in the gate).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the replay fixtures, unchanged.
- Build: the CI target build.

## Completion notes

- **Most of it was already there.** The carve brought `shuyi-core`,
  `shuyi-ui` and `shuyi-platform` over with the `std` feature, the
  `no_std` attributes, `thiserror` 2 without `std`, `alloc` paths and the
  `libm` floats (game ADR-0070 §3). CI built core, assets and UI for
  `thumbv7em-none-eabihf`, but not the platform crate.
- **The CI gate.** The `no-std` job now runs the ticket's command for
  the three crates (assets keeps its own line), then `cargo xtask
  check-no-std`, then the three crates' tests without `std`.
- **The grep: `cargo xtask check-no-std`.** It flags every `std::` path
  in the three crates' `src` outside comments, unless the line names
  `cfg(feature = "std")` or the file starts with
  `#![cfg(feature = "std")]`. It has unit tests and a CLI test. The
  compiler enforces most of the rule anyway. The grep catches code the
  `no_std` builds never compile (code behind another feature).
- **Tests without `std`: the whole suites, not one test per crate.** I
  went further than the ticket on purpose. Every unit and integration
  test of the three crates (core 89; UI 858 + 64; platform 37 + 6,
  headless backend included) now compiles and passes with
  `--no-default-features` on the host. Doing that meant:
  - `alloc`/`core` imports in place of `std::` in the tests;
  - a `static` font and image table in place of `std::sync::LazyLock`;
  - `#[should_panic]` in place of `catch_unwind`;
  - the crates' own dev-dependencies (`shuyi-ui` → itself and
    `shuyi-platform`, `shuyi-platform` → itself and `shuyi-ui`) set to
    `default-features = false`, or Cargo would turn `std` back on through
    them.

  One test needs `std`: `a_converted_picture_draws_as_converted`, which
  uses the `std`-only converter. It is `cfg(feature = "std")`, so UI runs
  859 tests with `std` and 858 without.
- **Unused imports, allowed in one CI step only.** The test harness and
  the dev-dependencies bring `std` into the host build. `std`'s own float
  methods then win over the `Float` trait, so five `#[cfg(not(feature =
  "std"))] use crate::float::Float;` imports are unused in that build
  only. So that step alone runs with `RUSTFLAGS: -D warnings -A
  unused-imports`. The target build still denies every warning. It also
  means the host tests use `std`'s float methods, not `libm`'s.
  `float.rs`'s own tests already check that the two agree.
- **Maps.** `hashbrown` is gone. The UI crate used it in two places: the
  `Keymap`'s chord and button lookups, and the snapshot's colour keys.
  Both are now `BTreeMap`. `Chord` and `ButtonChord` were already `Ord`,
  and `Rgb` gained `PartialOrd` and `Ord` (an addition to the API, not a
  breaking change). Both maps are small, so speed doesn't matter.
  `hashbrown`'s default hasher is seeded, so its iteration order could
  differ from run to run. No code iterated these maps in an
  order-dependent way, but now none can. Nothing in the three crates uses
  `HashMap` or `HashSet` any more, tests included.
- **Dependencies (ADR-0002 §8).**
  - `serde`, `ron`, `thiserror`, `libm`: already without `std`
    (MIT/Apache).
  - `shuyi-assets`: without its `std` feature.
  - `hashbrown` (MIT/Apache): removed, and its `foldhash` 0.2 with it.

  Nothing is new, and `cargo deny check` passes.
- **Replays and snapshots.** Unchanged. No snapshot or fixture moved, and
  every insta snapshot passes both with and without `std`.
- **Linker.** On Windows with the GNU host toolchain, the target build
  needs no linker for the target (libraries only). The `run-gates` skill
  says so.
- **For the game:** no breaking change. The next pin bump needs no game
  change (only `Rgb: Ord` is new). No game PR is needed for this ticket.
- **Follow-ups:** none.
