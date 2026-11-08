---
id: "2062"
title: The tutorial steps' lockfiles match what `cargo test` writes
type: bug
milestone: After MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2062 — The tutorial steps' lockfiles match what `cargo test` writes

## Context

#304 (2576) noted, in prose only, that "the tutorial steps' `Cargo.lock`
files are stale on `main` (`cargo test` there rewrites them)". Its
second commit put them back as `main` had them. Every session that runs
the tutorial's tests gets a dirty tree and must remember to revert it.
The retro of 2026-11-08 found no ticket for it (`AGENTS.md` rule 2).
The files are `site/tutorial/01-new/Cargo.lock` to
`site/tutorial/09-test/Cargo.lock`.

## Nick input

None.

## Scope

**In:**
- Refresh the nine lockfiles to what `cargo test` writes on `main`.
- A check that keeps them fresh: the CI job that tests the tutorial
  runs with `--locked`, so a stale lockfile fails there instead of being
  rewritten. Or the job fails on a dirty tree after its tests.

**Out (do not do):**
- Changing the tutorial's dependencies.

## Implementation steps

1. Find the job that runs the tutorial's tests (`grep -n tutorial
   .github/workflows/ci.yml`) and the `run-gates` section for it.
2. Run those tests, commit the rewritten lockfiles, and add `--locked`
   (or a `git diff --exit-code site/tutorial` step).
3. Say in `run-gates` that a dependency change in the engine means
   refreshing them.

## Acceptance criteria

- [ ] Running the tutorial's tests on the branch leaves `git status`
      clean.
- [ ] A stale lockfile fails the tutorial's CI job.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The CI job itself, shown failing once on a deliberately stale lockfile
  in a scratch commit (named in the notes).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
