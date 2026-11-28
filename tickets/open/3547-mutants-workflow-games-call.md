---
id: "3547"
title: "The diff-mutation workflow is one a game calls: mutants.yml takes workflow_call inputs, as playtest.yml does"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3547 — The diff-mutation workflow is one a game calls

## Context

The redundancy sweep of 2026-11-28 found the mutation-testing pipeline
written twice:

- the engine's `.github/workflows/mutants.yml` (325 lines, plus
  `.github/mutants-diff.sh`);
- the game's `.github/workflows/mutants.yml` (500 lines, public
  repository).

The two share the job graph: `changes`, `plan`, `baseline`, `pr`,
`mutants-result`, `weekly-baseline` and `weekly`. 104 non-comment lines
match verbatim. The game adds `gate` and `save` jobs and an outcome
cache. The engine adds `adapters` and `weekly-adapters`.

Three open tickets each plan to change one of the copies:
- engine 2055: mutants looked at before the push, and a plain verdict
  after;
- game#0134: a shard logs its disk and memory;
- game#0155: the weekly full run finishes inside six hours.

Games already call engine workflows this way. The grid template's
`playtest.yml` calls
`Zafnok/shuyi-engine/.github/workflows/playtest.yml@{{engine_ref}}`. The
engine's `changes`, `compile-times`, `export`, `playtest` and
`sound-parity` workflows are `workflow_call`.

About 250 lines go across the two repositories. Engine 2055 and the
game's 0134 and 0155 then change one file.

## Nick input

None.

## Scope

**In:**
- **`mutants.yml` gains `workflow_call`**, keeping its own
  `pull_request` and `schedule` triggers. Its inputs:
  - the packages and features to mutate;
  - the system packages to install;
  - the proptest seed;
  - the outcome cache (on or off);
  - the gate job's command;
  - whether to save the results.

  The defaults are the engine's values.
- **The game's `gate`, `save` and outcome-cache jobs** become inputs, or
  jobs that run when their input is set.
- **The engine's `adapters` jobs** run only when `adapters: true` (the
  engine's own call).
- **The caller pins by commit**, as `playtest.yml` does. A game's call
  goes in the template only if the template has mutation testing
  today; otherwise its docs say how to add the call.
- **Either order with 2055:** if 2055 is done, keep its verdict step
  in the called workflow. If not, 2055 writes it there (its clause says
  so).
- **The game's adoption is game#1425.**

**Out (do not do):**
- Changing what is mutated, or the engine's thresholds.
- The game's own workflow file: game#1425 replaces it with a call.

**Estimated size:** about 500 lines (workflow 250, tests of the inputs
in the workflow lint 100, docs 150).

## Implementation steps

1. Read the game's `mutants.yml` (public repository) and list its jobs
   and steps that the engine's lacks.
2. Add the `workflow_call` inputs, and move the game's extra jobs in
   behind inputs.
3. Keep the engine's own run equal: run it on this PR, and compare the
   job list and the mutants' counts with the last run on `main`.
4. Document the call in the `shuyi-test` skill and the CI guide.

## Acceptance criteria

- [ ] The engine's `mutants` run on this PR has the same jobs and
      finds the same mutants as the last run on `main`.
- [ ] zizmor passes on the changed workflow.
- [ ] A test caller in the workflow lint (or a fixture repository)
      calls it with the game's inputs and passes the action lint.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The workflow lint's checks on the inputs.
- The engine's own run on the PR is the integration test.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
