---
id: "2396"
title: "The one-bit renderer's speed test holds on every runner: a measurement that ignores other tests and the runner's noise, and a bar a debug build's numbers support"
type: bug
milestone: After MVP
model: opus-5.5
effort: medium
status: in-progress
blocked_by: []
nick_input: none
completed:
---

# 2396 — The one-bit renderer's speed test holds on every runner

## Context

1. **The test:** `a_fight_draws_fast`
   (`crates/ui/src/frame/onebit/speed_tests/timing.rs`, ticket 3056)
   times the fight-like frame drawn by the fast one-bit renderer and by
   `reference::render`, the plain one, and asserts `fast * 4 <= slow`.
   `time_fight(4)` takes the least of 9 tries of 4 renders each, the
   fast renderer's 4 renders all timed before the plain renderer's.
2. **The failure:** CI's `determinism` job (the engine crates' tests on
   `wasm32-wasip1` under wasmtime) failed on PR
   Zafnok/shuyi-engine#183 (run 40998839300, attempt 1), a change to
   `shuyi export` that touches no renderer, with
   `3.138761ms fast against 12.544307ms plain`: 3.997 times, against
   the bar of 4. The re-run passed.
3. **What the 4 times meant.** 3056's acceptance criterion is "the
   fight-like frame renders at least 4 times faster *on the PC than
   before*", and its completion notes measured it in a release build:
   60.7 µs against 1.07 ms before (17 times) and against 805 µs plain
   (13 times). In a test (debug) build the same notes measured 0.53 ms
   against 3.15 ms plain, 5.9 times. The test then asserts 4 times
   against the plain renderer in whatever build runs it, which is a
   debug build on every CI job. What the test guards is that the byte
   path stays: before 3056 the fast renderer drew a sprite a pixel at a
   time, as slow as the plain one (4.02 ms against 3.15 ms in a debug
   build, 0.8 times).
4. **Why it flakes.** Measured on this PC (Windows, x64, debug test
   build), the speed-up is about 4.45 times today, not 5.9: the 4 sits
   10 % under the true value, so any runner whose cache or code
   generation favours the plain renderer a little, or any burst of
   other tests running beside it (`cargo test` runs a crate's tests on
   all cores), crosses it. The full numbers are in the completion notes.

## Nick input

None.

## Scope

**In:**
- In `timing.rs`, a measurement that holds under noise: the two
  renderers timed in pairs, one render each, which goes first changing
  each pair, so the runner's speed changing over the run (another test,
  the clock's frequency) lands on both; the speed-up taken from many
  pairs, not one try.
- A bar the numbers support on every runner that runs the test, kept
  well above what a lost byte path would measure (about 1 time); the
  numbers measured natively and under wasmtime, locally and on CI's
  runners, in the completion notes.
- The failure message prints the numbers it judged by.

**Out (do not do):**
- The renderer itself (`onebit.rs`): no change to how it draws.
- An optimised profile for tests (`[profile.dev.package…]`): it changes
  every test's build to fix one test's measure.
- Other timing tests (`crates/app`, `crates/cli`): they time waits, not
  a speed-up.

## Implementation steps

1. Measure the current statistic and candidates (least of pairs,
   median of pairs) natively in a debug and a release build, under
   wasmtime, and on CI's runners (`test` on Linux, Windows and macOS,
   `coverage`, `determinism`).
2. Rewrite `time_fight` as the paired measurement and
   `a_fight_draws_fast`'s bar from those numbers.
3. Run the crate's tests many times in a row natively and under
   wasmtime and record the least speed-up seen.

## Acceptance criteria

- [ ] The test still fails when the fast renderer is as slow as the
      plain one (checked by drawing with `reference::render` on both
      sides once, locally).
- [ ] The bar sits below the least speed-up measured on every runner by
      a margin the completion notes give.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Speed: `a_fight_draws_fast`, as changed.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
