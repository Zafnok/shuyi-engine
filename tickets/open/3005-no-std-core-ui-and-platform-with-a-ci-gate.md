---
id: "3005"
title: "no_std with alloc for shuyi-core, shuyi-ui and shuyi-platform, with a CI gate that builds them for thumbv7em-none-eabihf"
type: infra
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["game#1131", "3003"]
nick_input: none
completed:
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

- [ ] The three crates build for `thumbv7em-none-eabihf` with
      `--no-default-features`, in CI.
- [ ] Every test passes with default features, and replays recorded
      before the change replay the same.
- [ ] No `std::` path outside `cfg(feature = "std")` in these crates
      (a grep in the gate).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the replay fixtures, unchanged.
- Build: the CI target build.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
