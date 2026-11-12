---
id: "2062"
title: The tutorial steps' lockfiles match what `cargo test` writes
type: bug
milestone: After MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-11
---

# 2062 — The tutorial steps' lockfiles match what `cargo test` writes

## Context

#304 (2576) noted, in prose only, that "the tutorial steps' `Cargo.lock`
files are stale on `main` (`cargo test` there rewrites them)". Its
second commit put them back as `main` had them. Every session that runs
the tutorial's tests gets a dirty tree and must remember to revert it.
The retro of 2026-11-08 found no ticket for it (`AGENTS.md` rule 2).
The files are `site/tutorial/01-new/Cargo.lock` to
`site/tutorial/10-ship/Cargo.lock`.

## Nick input

None.

## Scope

**In:**
- Refresh the ten lockfiles to what `cargo test` writes on `main`.
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

- [x] Running the tutorial's tests on the branch leaves `git status`
      clean.
- [x] A stale lockfile fails the tutorial's CI job.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- The CI job itself, shown failing once on a deliberately stale lockfile
  in a scratch commit (named in the notes).

## Completion notes

- All ten tutorial locks refreshed with `cargo metadata` in each step, which
  adds what the engine's crates now need (`image` 0.25 with `moxcms`,
  `pxfm` and `byteorder-lite`; `sapp-jsutils` and `winapi` in
  `shuyi-app`) and moves nothing else. `cargo test --workspace --locked`
  then passes in each step and leaves `git status` clean.
- CI's `tutorial` job no longer runs `cargo update -p shuyi -p shuyi-cli
  -p shuyi-app` first, which rewrote a stale lock and hid it. Clippy and
  the tests run `--locked`, and a last step, `git diff --exit-code` over
  the step's folder, fails on anything else the job changed.
- `run-gates` has a section on the tutorial's steps: a change to the
  engine's dependencies refreshes the ten locks, with the command.
  `site/README.md` and the gate's line in `crates/xtask/src/gates.rs` say
  `--locked`.
- Shown failing: scratch commit `b9ef6d4f` put back `main`'s `01-new` lock,
  and CI's `tutorial (01-new)` failed at Lint: "cannot update the lock
  file … because --locked was passed" (run 43160958228, job 130993658597).
  The next commit put the fresh lock back.

Deviations:
- The ticket named nine locks, `01-new` to `09-test`; `10-ship` (added
  after it was written) was stale too, so it is refreshed and checked
  like the others, and the ticket's Context and Scope now say ten.
- The web build's boot step (`ci.yml`) and the Pages build still run
  `cargo update -p …` before `cargo shuyi web`. With fresh locks it
  moves nothing, and the `tutorial` job now fails a stale one first.

Book: none, the reader's own project makes its own lock; nothing the
book says about the steps changed.

No break. No `Behaviour:` line: only CI and the tutorial's locks changed;
the game has no tutorial.

Nick input: none.
