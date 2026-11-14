---
id: "2080"
title: Set a new game's dependency budget to the 143 crates it has
type: bug
milestone: MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2080 — Set a new game's dependency budget to the 143 crates it has

## Context

`tools/compile-budget.ron` holds the most crates a new `blank` game's
dependency tree may have (ticket 2009, ADR-0049). A count over it fails
CI's `compile-times` job; a count under it only warns "lower it", so the
number has to be lowered by hand.

It was 146 since ticket 2589 added `gif` and `weezl`. Then:

- ticket 3033 (#386) removed `quad-storage`, `quad-storage-sys`,
  `nanoserde`, `nanoserde-derive` and `lazy_static` from `Cargo.lock`
  and left the number at 146, so the jobs counted 141 and warned (seen
  on #413's run 43555899983);
- ticket 2816 (#411) gave `shuyi-cli` `getrandom` for `reveal commit`,
  which brings 2 crates into the tree, so `main`'s jobs count 143 and
  still warn (run 43557116904).

## Nick input

None.

## Scope

**In:**
- `dependencies: 143` in `tools/compile-budget.ron`, its comment saying
  why, and that a change that drops crates lowers it.

**Out (do not do):**
- The `rebuild` seconds: no measurement says to move them.
- Making a count under the budget fail the job: ADR-0049 chose a warning.

## Implementation steps

1. Set `dependencies` to 143 and update its comment.

## Acceptance criteria

- [x] `tools/compile-budget.ron` says 143, and
      `cargo xtask compile-times --budget tools/compile-budget.ron --count-only`
      prints `compile-times: ok` with no "lower it" warning.
- [x] Book: no chapter changes: the budget is CI's, not a game maker's.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- CI's `compile-times` job, which reads the file.

## Completion notes

**What was done.** `dependencies` set from 146 to 143: 5 crates fewer
from ticket 3033, 2 more from ticket 2816, the count `main`'s jobs and
a local `--count-only` print. The comment says a change that drops
crates lowers the number too.

**Deviations.** The first draft said 141, the count on the base of
#413; `main` had moved to 143 with #411 by the time this branch was
cut.

No break, no behaviour change for a game.

Book: none, the budget is CI's, not a game maker's.
