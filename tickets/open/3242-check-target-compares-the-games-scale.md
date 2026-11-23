---
id: "3242"
title: "check --target compares each target's scale with the game's DEFAULT_SCALE"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["2333"]
nick_input: none
completed:
---

# 3242 — check --target compares each target's scale with the game's

## Context

Split off 2333 (its PR passed 1,500 counted lines with it; its step 6).
2333 made `check --target` place a frame with the window's own function
(`shuyi_ui::frame::console::scale`, ADR-0139), and gave a game
`GameSettings::DEFAULT_SCALE`, which `shuyi-app` draws. Nothing checks
that a target's `scale` in `shuyi.ron` is what the game draws: a game
whose `shuyi.ron` says `fit` and whose settings say `Integer` passes the
check at a scale its window never shows. Any game with a `shuyi.ron`
has the same gap (a platformer's `web` target as much as a tactics
game's).

## Nick input

None.

## Scope

**In:**
- `GameTools::scale(&self) -> Option<ScaleMode>`
  (`crates/cli/src/game.rs`), `None` by default.
- In `targets` (`crates/cli/src/contract/check.rs`), when the game's
  tools offer it, each asked target whose `scale` differs is an error:
  `TargetCheck::Fits`, setting `targets."<id>".scale`, message
  "`fit` here, but the game draws `integer` by default
  (`GameSettings::DEFAULT_SCALE`)". No new `TargetCheck` variant, so
  the contract's schema stays.
- Without it, the answer's limits get a line, as for the controls: "the
  scale isn't compared with the game's: the game's tools don't offer it
  (GameTools::scale)", or, with no tool program, "… it needs the game's
  tools, and shuyi.ron names no game tool program (`game`)". It goes
  after the per-target lines, and the last line becomes "only the
  frame's fit, the text's size, the controls, the scale, one-bit
  patterns and the identity are checked yet". `targets` is at clippy's
  100-line limit: put the limit in a helper.
- `Fake` (`crates/cli/src/contract/fake.rs`) gets a `scale` field.
- `BasicTools` (`crates/cli/src/basic.rs`) returns
  `Some(<() as GameSettings>::DEFAULT_SCALE)`; hello-grid's tools
  (`examples/hello-grid/src/tools.rs`) `Some(<Prefs as
  GameSettings>::DEFAULT_SCALE)`; the top-down grid template's tools
  delegate to `self.basic().scale()`. Bless the template's plans
  (`SHUYI_BLESS=1 cargo test -p shuyi-cli --lib template`).
- `docs/contract.md`'s `check --target` row says it; ADR-0139 §2's
  sentence on this ticket becomes what it does.

**Out:** the player's pick (3240); the capture (3241).

**For the game:** additive (`GameTools::scale` has a default). A game
whose tools don't offer it gets one more limit line; one whose tools do
fails a target whose `scale` differs (a `Behaviour:` line).

**Estimated size:** about 350 lines (code 150, tests 150, docs 50).

## Implementation steps

1. Add `GameTools::scale` and `Fake::scale`.
2. The comparison and the limit lines in `targets`, with a helper.
3. The tools of `BasicTools`, hello-grid and the template; bless the
   plans.
4. The docs.

## Acceptance criteria

- [ ] A check whose tools report `integer` against a `fit` target fails
      on `targets."web".scale`, with the message above. Tools that
      report `fit` pass it and fail an `integer` target.
- [ ] Tools that report nothing, and no tool program, each give their
      limit line; the existing tests of `targets` that list the limits
      gain the line.
- [ ] `check --target all` on hello-grid and on a new grid template
      passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the comparison both ways, and the two limit lines (in
  `check.rs`'s tests, with `Fake`).

## Completion notes

*(Filled in by the session that completes the ticket.)*
