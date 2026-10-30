---
id: "2396"
title: "The one-bit renderer's speed test holds on every runner: renders timed in pairs, and a bar a debug build's numbers support"
type: bug
milestone: After MVP
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-30
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
4. **Why it flakes.** In a debug build the speed-up is 4.4 to 5.6 times
   (the numbers are in the completion notes), so the 4 sits 10 % under
   the true value on some machines. A runner whose processor favours the
   plain renderer a little, or a burst of other tests beside it
   (`cargo test` runs a crate's tests on all cores), crosses it.

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

- [x] The test still fails when the fast renderer is as slow as the
      plain one (checked by drawing with `reference::render` on both
      sides once, locally).
- [x] The bar sits below the least speed-up measured on every runner by
      a margin the completion notes give.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Speed: `a_fight_draws_fast`, as changed.

## Completion notes

- **What was done.** `time_fight` (`speed_tests/timing.rs`) now times
  51 pairs of renders, one fast and one plain each, back to back, the
  fast one first in every other pair. Each pair gives a speed-up, and
  the test judges the median of the 51. A pause (another test, the
  scheduler) lands on one pair and the median drops it; a runner slowed
  for a while slows both renders of a pair alike. The test passes at 3
  times in a debug build and 4 in an optimised one (`LEAST`), and its
  message prints the median speed-up and the median times.
- **The numbers.** Every row is the fight-like frame of 3056, debug
  build unless it says otherwise. "Old" is 3056's statistic (the least
  of 9 tries of 4 renders, the fast tries before the plain ones),
  repeated 10 times in one run; "pairs" is the median speed-up of 200
  interleaved pairs in the same run. CI's rows are from this PR's first
  commit, which ran both and failed on purpose to print them.

  | where | old | pairs | fast, median | plain, median |
  | --- | --- | --- | --- | --- |
  | CI `determinism` (wasm32 under wasmtime), #183's attempt 1 | 4.00 | not run | 3.14 ms | 12.54 ms |
  | CI `determinism` (wasm32 under wasmtime) | 4.43–4.46 | 4.44 | 2.13 ms | 9.45 ms |
  | CI `test` (Linux) | 4.54–6.69 | 4.95 | 2.63 ms | 12.98 ms |
  | CI `test` (Windows) | 5.09–5.45 | 5.42 | 2.57 ms | 13.90 ms |
  | CI `test` (macOS) | 4.72–5.79 | 4.72 | 1.10 ms | 5.21 ms |
  | CI `coverage` (Linux, instrumented) | 4.98–6.91 | 5.62 | 2.53 ms | 13.96 ms |
  | this PC (Windows, x64), the test alone | 4.44–4.69 | 4.42–4.56 | 0.71 ms | 3.22 ms |
  | this PC, the crate's whole suite | 3.95–5.03 | 4.43–4.68 | 0.73 ms | 3.26 ms |
  | this PC, wasm32 under wasmtime 50 | 5.24–6.83 | 6.25–6.34 | 0.98 ms | 6.20 ms |
  | this PC, release build | 13.32–13.57 | 13.33 | 0.060 ms | 0.810 ms |

  Two things show. The old statistic dropped under 4 on this PC too
  (3.95, with the crate's other tests running beside it), and its
  spread on a runner is wide (Linux 4.54 to 6.69). And the speed-up
  itself is a property of the processor: the same wasm job measured
  4.00 on #183's runner and 4.44 on this PR's, each tightly. No
  measurement makes a 4.0 machine pass a 4 bar, so the bar had to
  move. The new test, run in the crate's whole suite 6 times natively
  and 5 times under wasmtime on this PC, measured 4.94 to 5.56 and 5.10
  to 5.23 times.
- **Why 3 holds and still means 3056's claim.** The least speed-up
  measured anywhere is 4.00 (#183's runner, with the old statistic); 3
  sits 25 % under it. A fast renderer that lost its byte path measures
  about 1 time: 3056's before was 0.8, and drawing with
  `reference::render` on both sides measured 0.99 here, which the test
  fails. So 3 is three times as far from a lost byte path as from the
  slowest runner. The 4 of 3056's acceptance was a release build's
  claim (17 times against before, 13 against plain); an optimised build
  keeps a bar of 4 against its 13.
- **Deviations.** None from the steps. The first commit measured on CI
  with a temporary test; the second replaced it.
- **Gates.** Run locally: fmt, clippy, the workspace's tests, the
  `no-std` tests, the ticket lint, the changelog, links, the docs map,
  typos and the engine's lint, and the test in a release build and under
  wasmtime. The others (docs, deny, machete, the web build, the modules,
  compile times) read no test file and are left to CI.
- **Docs.** None: a test's measurement, no public item, format or rule.
- Book: none, no public API changed.

No break.
