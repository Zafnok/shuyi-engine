---
id: "3422"
title: "The eval's tic-tac-toe reference: the least change to the grid template that passes its task, graded in CI"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: ["3421"]
nick_input: none
completed: 2026-11-24
---

# 3422 — The eval's tic-tac-toe reference

## Context

2524 (done) wrote the agent eval's first task, `evals/tasks/tic-tac-toe.ron`:
tic-tac-toe from `shuyi new --template grid --name Tic-tac-toe`, its
prompt naming every fact its checks read (the title first, Confirm to
start; X first; the cursor on the middle cell, stopping at the edges;
`turn` and `winner` in the scene's data). It split the reference off to
stay under the size limit, so the task has `reference: None` and
`waits_for: ["3422"]`. 3421 grades references on fresh games and in CI.

**A draft is written.** 2524's session wrote this reference before the
split, on the branch `draft-2524-eval-references` (commit `97e04ff4`,
pushed, no PR), in `evals/references/tic-tac-toe/`: the rules over
`TurnRules` (`crates/rules/src/lib.rs`, `Board`, `Mark`, `Command::Place`),
the stage screen and its view (`crates/game/src/stages/top_down_grid.rs`),
its tools (scene data, a random bot on both sides, replay), the stage's
tests, `en.ron`'s stage texts, and `crates/rules/Cargo.toml` without
`shuyi-assets`. Graded locally on Windows on 2026-11-24 with 3421's
draft command, after a fix to its rules' formatting and two pedantic
lints that are in the draft: every check passed but `Export linux`
(`SH1003`, Linux only), at +451 −725 lines over the template, 6 files.
With `--template-only`, `Check` passed and both `Scene` checks failed.
Bring it over with `git checkout draft-2524-eval-references --
evals/references/tic-tac-toe` and grade it again on `main`.

## Nick input

None.

## Scope

**In:**
- `evals/references/tic-tac-toe/`: only the files that differ from
  `shuyi new --template grid --name Tic-tac-toe`, written as an agent
  should write them, the least change to the template that passes. Its
  lines over the template are the number the PR reports. It may not
  overlay a `Cargo.toml` that says where the engine's crates are (3421
  refuses it). The template's `stage.map` stays, unread: an overlay can't
  delete a file. The workspace's `shuyi-assets`, which the rules no
  longer use, stays in the root `Cargo.toml` (cargo warns of it), which
  an overlay may not change.
- `evals/tasks/tic-tac-toe.ron`: `reference:
  Overlay("evals/references/tic-tac-toe")`, its `waits_for` emptied.
- 3421's `eval` job: a step `cargo xtask eval-reference tic-tac-toe
  --template-only --out eval-template` that must fail, its `jq` check
  that `Check` passed and every `Scene` failed, as box-pushing's.

**Out (do not do):**
- changing the grid template so the reference gets shorter;
- a Playdate target (2504: the task keeps its `unmeasured` entry).

## Implementation steps

1. Read the task, 3421's `eval-reference`, and the draft.
2. Bring the draft over, run `cargo xtask eval-reference tic-tac-toe`
   and `--template-only`, and fix what fails on `main`.
3. Set the task's reference, add the CI step, and run the gates.

## Acceptance criteria

- [x] `cargo xtask eval-reference --all` passes on tic-tac-toe in CI.
- [x] `--template-only` on tic-tac-toe fails every `Scene` check and
      passes `Check`, checked in CI.
- [x] The PR description has the reference's lines added and seconds.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- The reference's own tests (its rules, its stage through the Harness
  and the contract), which its `Gates` check runs.
- Integration: the `eval` CI job, both ways.

## Completion notes

- Brought the draft over from `draft-2524-eval-references` (`97e04ff4`)
  unchanged: on `main` it needed no fix. Graded locally on Windows on
  `3ef570a5` with `cargo xtask eval-reference tic-tac-toe`: 12 of 13
  checks passed, all but `Export linux` (`SH1003`, built on Linux only,
  which CI's `eval` job grades), at +471 −721 lines over the template,
  6 files, in 115.2 seconds (the draft measured +451 −725 before 3421;
  the reference's files are the same, so the template moved).
- `--template-only`: `Check` passed, both `Scene` checks failed (the
  template's `turn` is a number and it has no `winner`), exit 1.
- `evals/tasks/tic-tac-toe.ron` now names its reference and waits for
  nothing. The `eval` job grades the plain template too, with the same
  `jq` checks as box-pushing's; `evals/README.md` says so.
- Docs: `evals/README.md`'s *Grading* names the new CI check; no public
  API, contract or format changed, so nothing else.
- No break.
