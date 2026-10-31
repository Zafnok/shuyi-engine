---
id: "3070"
title: "The one-bit renderer's timing test fails under coverage instrumentation: a_fight_draws_fast runs only without it"
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] `a_fight_draws_fast` is ignored under `cargo llvm-cov` and runs
      in `cargo test`.
- [ ] No `unexpected_cfgs` warning, and no lint allowed to hide one.
- [ ] The book needs no change (no public item changes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The test itself, in the `test` job; its absence from the `coverage`
  job's run, read in that job's log.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
