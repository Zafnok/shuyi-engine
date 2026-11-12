---
id: "2073"
title: '`cargo xtask gates` fails a stale tutorial step lock before CI does'
type: feature
milestone: After MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: done
blocked_by: ["2072"]
nick_input: none
completed: 2026-11-12
---

# 2073 — `cargo xtask gates` fails a stale tutorial step lock before CI does

## Context

Each step in `site/tutorial/` has its own checked-in `Cargo.lock` with
a path dependency on the engine's crates. CI's `tutorial` job builds and
tests each step `--locked` (2062), so a PR that changes the engine's
dependencies and forgets the ten locks fails there. Locally,
`cargo xtask gates` lists that gate as CI-only
(`crates/xtask/src/gates.rs`, `gate("tutorial", …).ci_only(…)`): the
build and tests of ten games take long. But whether a lock is stale
takes seconds to find: `cargo metadata --locked` in the step fails on
a lock it would have to change.

2072 refreshed the locks after #379 (2589) added `gif` to `shuyi-cli`.
That break came from three merges in one minute (#381, #378, #379):
#379's CI ran before 2062's `--locked` was on `main`, so no gate of its
own could have seen it. A local check catches the ordinary case: a
dependency PR branched after 2062, which today learns of the stale
locks only from CI.

## Nick input

None.

## Scope

**In:**
- A local gate, `tutorial locks`, under CI's `tutorial` job, with one
  step per step folder: `site/tutorial/<step>: cargo metadata --locked
  --format-version 1` (output thrown away). It runs when the diff
  touches a `Cargo.toml` or a `Cargo.lock` under `crates/`, or the root
  `Cargo.lock`, or `site/tutorial/`.
- On failure, the gate's message gives the refresh command from the
  `run-gates` skill's section "The tutorial's steps".
- That section says the local gate now finds a stale lock.

**Out (do not do):**
- Building or testing the steps locally: the `tutorial` gate stays CI-only.
- Requiring a PR's branch to be up to date with `main` before merging:
  a repository setting, Nick's (it would have stopped the 2072 race).

## Implementation steps

1. In `crates/xtask/src/gates.rs`, add the gate beside
   `gate("tutorial", …)`, with steps in the `"<dir>: <command>"` form
   the template gates use. If the ten folder names are listed as data,
   read them from `site/tutorial/` rather than writing them twice.
2. Give it a filter that fires on the paths in Scope (the classifier,
   `classify`, and its outputs; add one if none fits).
3. Update the `run-gates` skill's section and the gates' list there.
4. A test in `gates.rs`'s tests that the gate plans to run for a diff
   touching `crates/cli/Cargo.toml` and is skipped for a docs-only diff.

## Acceptance criteria

- [x] With one step's lock put back to a stale copy (a scratch commit),
      `cargo xtask gates` fails the new gate, naming the step.
- [x] With fresh locks it passes in under 30 seconds on a warm cache.
- [x] A docs-only diff doesn't run it.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the gate's plan for the two diffs above.
- The scratch-commit run, named in the notes.

## Completion notes

Added the `tutorial locks` gate in `crates/xtask/src/gates.rs`, under CI's
`tutorial` job: one `cargo metadata --locked --format-version 1` step per
step folder, run when the diff touches a `Cargo.toml` or `Cargo.lock` under
`crates/`, the root's, or `site/tutorial/`. A failure names the step and
gives the refresh command. The `run-gates` skill's section says so.

Deviations:
- A new `Local::When(text, fn)` filter, since `Local::Under` matches
  prefixes and the rule needs file names under `crates/`. The root
  `Cargo.toml` counts too (workspace dependencies move the engine's
  resolution), beyond the ticket's list.
- The ten folders are listed in the steps (they are `'static` data); a test
  holds the list equal to the folders of `site/tutorial/`.
- A `Gate::hint` field carries the refresh command into the failure text.

Evidence: with `site/tutorial/03-walk/Cargo.lock` stripped of `gif` in a
scratch commit (dropped), `cargo xtask gates` failed `tutorial locks` naming
`site/tutorial/03-walk` after 1 s; on fresh locks the gate takes seconds.
A docs-only diff skips it (unit test).

Book: none, a gate of the repository's own checks; no public API changed.
No ticket: the repository setting that requires branches up to date is Nick's.
