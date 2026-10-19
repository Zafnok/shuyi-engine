---
id: "2102"
title: "Example: a small game with a rule from a script, the shuyi-script module's example"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2101"]
nick_input: none
completed:
---

# 2102 — Example: a small game with a rule from a script

## Context

ADR-0009 §4: every module ships with an example in `examples/`, a small
game driven by the Harness, with a replay determinism test and a
`frame-png` picture; a module's README claims a direction only when its
example passes CI. 2101 builds `shuyi-script` (for game#1017, the
game's scripted rules) and leaves its example to this ticket so 2101
stays one PR.

## Nick input

None.

## Scope

**In:**
- `examples/scripted-rule/`: the smallest game that shows a rule coming
  from a script in its bundle (for example, a turn game on a small grid
  where a script decides how far a piece may move from the tile it
  stands on), with its integration test.
- A replay determinism test: the same inputs give the same states with
  the scripted rule.
- A `frame-png` picture of it.
- `shuyi-script`'s README claims the directions the example shows.

**Out (do not do):**
- Changing `shuyi-script`'s API, unless the example can't be written
  without it (then say why in the completion notes).
- Visual scripting (1135).

## Implementation steps

1. Read ADR-0009 §4, `crates/script/README.md` and an existing example
   (Hello, Grid).
2. Write the example and its script.
3. The integration test, the replay test and the picture.
4. Update the module's README.

## Acceptance criteria

- [ ] The example runs and its integration test passes in CI.
- [ ] The replay determinism test passes.
- [ ] The `frame-png` picture is committed.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the example through the Harness.
- Property: replay determinism with the scripted rule.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
