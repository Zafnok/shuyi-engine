---
id: "3070"
title: "The one-bit renderer's timing test fails under coverage instrumentation: a_fight_draws_fast runs only without it"
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-01
---

# 3070 — The one-bit renderer's timing test fails under coverage instrumentation

## Context

3056 made the one-bit renderer draw sprites fast, and pinned it with a
timing test: `a_fight_draws_fast` in
`crates/ui/src/frame/onebit/speed_tests/timing.rs` asserts that the fast
renderer draws the fight's frame at least 4 times faster than the plain
`reference::render`. Its completion notes measured 13 times in a debug
test build.

CI's `coverage` job runs every test under `cargo llvm-cov`, whose
counters slow down tight loops unevenly. On the PR of 2116
(`Zafnok/shuyi-engine#169`, run 40815817161, first attempt) the test
failed there: "2.855511ms fast against 10.209543ms plain", 3.6 times. A
re-run passed. Nothing in that PR touched the renderer. A test that
fails by chance makes every PR's CI flaky, and Auto-fix sessions chase
it.

A timing test checks the code as it ships, not as coverage builds it.
`cargo llvm-cov` sets `cfg(coverage)` on the crates it builds.

## Nick input

None.

## Scope

**In:**

- `a_fight_draws_fast` doesn't run under coverage:
  `#[cfg_attr(coverage, ignore = "coverage counters slow the fast
  renderer's loops more than the plain one's: the ratio means nothing
  there")]`, or `#![cfg(all(feature = "std", not(coverage)))]` on the
  module. Check the lint that names `coverage` as an unknown cfg: if
  `unexpected_cfgs` warns, declare it in `crates/ui/Cargo.toml`
  (`[lints.rust] unexpected_cfgs = { level = "warn", check-cfg =
  ['cfg(coverage)'] }`, merged with the workspace's lints as the crate
  allows) rather than allowing the lint.
- The `test` job (x64 and ARM64, debug) still runs it, with its 4
  times unchanged.

**Out (do not do):**

- Lowering the 4 times, or retrying the test.
- Any change to the renderer.

## Implementation steps

1. Add the attribute (Scope), and the check-cfg if the build warns.
2. Run `cargo llvm-cov --workspace --exclude shuyi-app --locked` (CI's
   command, not on Nick's machine if it is slow there: CI shows it) and
   `cargo test -p shuyi-ui --lib a_fight_draws_fast`.

## Acceptance criteria

- [x] `a_fight_draws_fast` is ignored under `cargo llvm-cov` and runs
      in `cargo test`.
- [x] No `unexpected_cfgs` warning, and no lint allowed to hide one.
- [x] The book needs no change (no public item changes).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- The test itself, in the `test` job; its absence from the `coverage`
  job's run, read in that job's log.

## Completion notes

- **What was done.** `a_fight_draws_fast` carries
  `#[cfg_attr(coverage, ignore = "…")]`, so the `coverage` job lists it
  as ignored, with the reason, and every other job runs it.
- **The check-cfg.** The attribute did warn (`unexpected_cfgs`), so the
  lint's `check-cfg` names `cfg(coverage)`. It is in the root
  `Cargo.toml`'s `[workspace.lints.rust]`, not the crate's as the ticket
  sketched: a crate with `[lints] workspace = true` can't add lints of
  its own. `shuyi-alloc-count`, `shuyi-app` and `shuyi-playdate` keep
  their own copy of those lints, held equal by a test, so each has the
  line too.
- **What the ticket's text had overtaken.** 2396 (done after this ticket
  was written) rewrote the measurement: the median of 51 pairs, and a bar
  of 3 times in a debug build and 4 in an optimised one (`LEAST`). So
  "its 4 times unchanged" here means `LEAST` unchanged. 2396 measured
  5.62 times on `coverage`, so the old failure is unlikely to come back
  anyway; this ticket's reason still holds, that a ratio under
  instrumentation says nothing about the code as it ships.
- **How it was checked.** `cargo test -p shuyi-ui --lib a_fight_draws_fast`
  runs and passes it. With `RUSTFLAGS="--cfg coverage"`, the flag
  `cargo llvm-cov` sets, it is ignored with the reason and nothing warns.
  `cargo llvm-cov` itself was not run on this machine (it isn't installed
  here): the `coverage` job's log on this PR shows it.
- **Gates.** Run locally: the skills' sync, fmt, clippy, the workspace's
  tests, the `no-std` tests, the ticket lint, the changelog, links, the
  docs map, the layers, typos and the engine's lint. The others (docs,
  deny, machete, the web build, the modules, compile times) don't depend
  on a test's attribute or this lint's setting, and are left to CI.
- **Docs.** None: a test's attribute and a lint's setting, no public item,
  format or rule.

No break.
