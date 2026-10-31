---
id: "2717"
title: "Parked: RTS in parallel on native: a rayon feature that gives the same hashes as one thread"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2712"]
nick_input: none
completed:
---

# 2717 — RTS in parallel on native, with the same hashes

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§10. The module is single-threaded first and must meet its budgets that
way (2718). On a desktop or a Steam Deck, spare cores can make big
battles cheaper. This ticket adds a native-only `parallel` feature that
runs the per-unit passes over chunks with `rayon`.

It must give **the same result, bit for bit**. ADR-0015 §3's rule makes
that possible: every pass reads the last state and writes only its own
unit's row, so how the rows are split doesn't matter. 2711 and 2712
already test it by visiting rows in reverse.

The web build and small targets never turn it on: the web build has no
threads we use, and neither do the Playdate, the GBA or the DS (3009).

## Nick input

None.

## Scope

**In:**
- **The dependency:** `rayon`, optional, under
  `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`, with
  `default-features = false`.
  - Check its license from its `Cargo.toml` and repository on the day
    (believed MIT OR Apache-2.0, on ADR-0002 §8's list), and its
    dependencies' (`rayon-core`, `crossbeam-*`, `either`).
  - `cargo deny check` passes with the feature on.
  - Record the version and the check in the README's cost section.
- **The feature** `parallel` of `shuyi-rts`. **From 2003 (done):** the
  `shuyi` crate gets no second feature for it: `cargo xtask
  check-modules` wants each of its features to name a catalog row
  (`rts` is `shuyi-rts`), and there is no `shuyi-rts-parallel`. A game
  turns it on through `shuyi-rts` directly (`shuyi-rts = { …,
  features = ["parallel"] }` beside `shuyi`'s `rts`), and the README
  says how.
  - `compile_error!` when it is on for `wasm32` or a `no_std` build.
- **Parallel passes**, over fixed-size chunks of rows (a setting, default
  256), writing each chunk's slice of the next columns:
  - movement and steering (2711), including each push round;
  - the target pick and the damage pass (2712);
  - unit reactions (2714). **Either order with 2714:** if 2714 is
    done, split its pass too; if not, nothing to do (2714 splits it
    when it lands).
  Passes that write shared grids stay on one thread: the spatial grid
  rebuild (2708), vision stamping (2713) and flow field building (2710).
- **Events and hits** gathered per chunk and joined in chunk order, so
  their order is the one-thread order.
- **A test** that runs every fixture of the crate twice, without and
  with the feature, and compares the hash after every step. In CI it
  runs with `RAYON_NUM_THREADS` set to 1, 2 and 7.
- **CI checks:**
  - the web build of the RTS example (or the crate with all features but
    `parallel` for `wasm32-unknown-unknown`) has no `rayon` in
    `cargo tree`;
  - a build with `parallel` for `wasm32-unknown-unknown` fails with the
    `compile_error!` message;
  - **From 3004 (done):** this ticket makes `shuyi export`
    (`crates/cli/src/contract/export.rs`) refuse `parallel` for a
    target whose profile isn't `desktop` or `steam-deck`, naming the
    feature (so the desktop exporters and 3012's `steam-deck` allow
    it; `web`, `playdate` and the rest refuse), with a unit test.
- **A speed report** (not a gate): the step time of a battle fixture of
  2 × 2 500 units (made here, from 2712's fixture kinds, no AI) with and
  without the feature on the CI runner, in the README. Wall time is fine
  for a report; it is never a gate (ADR-0008 §6).

**Out (do not do):**
- threads in the web build (ADR-0008, game ADR-0034);
- parallel grid rebuilds, vision or flow fields (they write shared
  state; a later ticket if 2718 shows they matter);
- any change to results: if a pass can't be split without changing a
  hash, it stays on one thread and the README says why.

## Implementation steps

1. Check `rayon`'s license and add it behind the feature, with the
   `compile_error!`.
2. Move each per-unit pass into a function over a chunk of rows, used by
   both the one-thread loop and the parallel one.
3. Gather events and hits per chunk.
4. Write the comparison test and the CI checks.
5. Measure and write the README section.

## Acceptance criteria

- [ ] Every fixture gives the same hash at every step with and without
      `parallel`, with 1, 2 and 7 threads (CI).
- [ ] Event lists are equal with and without the feature (test).
- [ ] `cargo deny check` passes with `parallel` on, and the README
      records `rayon`'s license, version and how it was checked.
- [ ] `cargo tree` of the web build has no `rayon` (CI check).
- [ ] A `wasm32` build with `parallel` fails with the documented message
      (CI check).
- [ ] `shuyi export` refuses `parallel` on a target whose profile
      isn't `desktop` or `steam-deck` (test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: every fixture, with and without, hash per step.
- CI: the `cargo tree` and `compile_error!` checks.

## Completion notes
