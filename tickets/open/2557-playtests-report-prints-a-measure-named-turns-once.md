---
id: "2557"
title: "playtest's report prints a measure named turns once, not beside the try's own turns"
type: bug
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2557 — `playtest`'s report prints a measure named `turns` once

## Context

Found by the skill pack's authors (1134), writing `shuyi-balance-bots`'
worked example. A game whose measures include one named `turns` sees it
twice on every try of `cargo shuyi playtest`'s report for people:

```text
try 3 · hero · … · turns 21 · foe trips 2 · turns 21 · won 1
```

- `PlaytestReport`'s `Display` (`crates/core/src/bots/playtest.rs`)
  calls `PlaytestReport::write_table`, whose loop over `entrant.tries`
  writes `turns {}` from the try's own `Tried::turns`, then every
  measure as ` · {name} {value}`.
- Hello, Grid's measures (`examples/hello-grid/src/tools.rs`,
  `StageMeasure::finish`) include `("turns", report.turns)`, the same
  number, because a check reads measures by name (a `Median` of
  `turns`, `examples/hello-grid/playtest.ron`). A game that compares its
  bots' speed does the same.

The JSON answer (`--json`, `--report`) is right as it is: `turns` and
`measures` are separate fields there.

## Nick input

None.

## Scope

**In:**
- In `PlaytestReport`'s `Display`, a measure named `turns` whose value
  is the try's own `turns` is left out of the try's line. A measure
  named `turns` with another value (a game that counts only its own
  side's turns) is still written, so nothing a game measures is hidden.
- The report's tests: a try with a `turns` measure equal to its turns
  reads `… · turns 21 · won 1`, and one with a different `turns` measure
  keeps both.
- **Either order with 2544** (`bots` folded into `playtest`): it
  rewrites `playtest`'s text for a plan with no checks. If 2544 is
  done, apply this rule to every place that writes a try's line, with
  the same tests; if not, nothing to do (2544 keeps the rule in each
  try line it writes).

**Out (do not do):**
- renaming the try's own `turns`, or the JSON's fields (the contract's
  schemas don't change);
- `bots`' text (`crates/cli/src/contract/bots.rs`), which prints no
  per-try line.

## Implementation steps

1. In `crates/core/src/bots/playtest.rs`, skip the measure `turns` in the
   try's line when it equals `t.turns`.
2. Add the tests to `crates/core/src/bots/playtest/tests.rs`.
3. Look at `cargo shuyi playtest` on `examples/hello-grid` (each try
   line has one `turns`), and run the gates.

## Acceptance criteria

- [ ] `cargo shuyi playtest` on Hello, Grid prints each try's turns
      once.
- [ ] A `turns` measure that differs from the try's turns is still
      printed (a test).
- [ ] `playtest --json`'s answer is unchanged (the schemas' test
      passes with no new version).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the report's text with a `turns` measure equal to the try's
  turns, with one that differs, and with none.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
