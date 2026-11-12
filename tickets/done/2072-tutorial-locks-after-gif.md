---
id: "2072"
title: The tutorial steps' locks hold `gif`, which `frame-png --gif` added
type: bug
milestone: After MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-12
---

# 2072 — The tutorial steps' locks hold `gif`, which `frame-png --gif` added

## Context

CI's `tutorial` job fails on every leg, `tutorial (01-new)` to
`tutorial (10-ship)`, on `main` since `0a126f96`: cargo refuses with
"cannot update the lock file …/site/tutorial/<step>/Cargo.lock because
--locked was passed" (run 43243529181). PR #382 (2064) shows the same.

2062 (#381, `08c5f5a7`) made the job run `--locked`, so a stale step
lock fails it. #379 (2589, `0a126f96`) added `gif` 0.14 (and through it
`weezl`) to `shuyi-cli`, a dependency of every step. The three merges
landed within one minute: #379's own CI ran before 2062's `--locked`
was on `main`, so nothing refreshed the ten locks. #378 (3300, `991a7aaa`)
only added features (`packed`) to `hello-grid` and the template; the
steps' locks don't move for it.

The `run-gates` skill's section "The tutorial's steps" gives the
refresh: `cargo metadata` in each step.

## Nick input

None.

## Scope

**In:**
- Refresh the ten locks, `site/tutorial/01-new/Cargo.lock` to
  `site/tutorial/10-ship/Cargo.lock`, with the `run-gates` command. Each
  adds `gif` 0.14.2 and `weezl` 0.1.12 and moves nothing else.

**Out (do not do):**
- Changing the tutorial's or the engine's dependencies.
- A local gate that would catch a stale step lock before CI: a ticket of
  its own.

## Implementation steps

1. `for d in site/tutorial/*/; do (cd "$d" && cargo metadata --format-version 1 > /dev/null); done`
2. Check `git diff --stat site/tutorial`: ten files, each adding the
   two packages and `gif` in `shuyi-cli`'s dependencies, nothing else.
3. Run one step as CI does (`run-gates`, "The tutorial's steps").

## Acceptance criteria

- [x] Each step's lock adds `gif` and `weezl` and changes no other
      package.
- [x] CI's `tutorial` job passes on every leg of the PR.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- CI's `tutorial` job, which builds and tests each step `--locked` and
  fails on a dirty step afterwards.

## Completion notes

- The break came from #379 (2589, `0a126f96`): `gif` 0.14 (and `weezl`)
  in `shuyi-cli`, a dependency of every step. #378 (3300) only added
  `packed` features to `hello-grid` and the template; the steps' locks
  don't move for it. #381 (2062), #378 and #379 merged within a minute,
  and #379's CI ran before `--locked` was on `main`.
- The ten locks refreshed with `cargo metadata` in each step, as
  `run-gates` says. Each adds `gif` 0.14.2 and `weezl` 0.1.12 and `gif`
  under `shuyi-cli`, nothing else; `cargo metadata --locked` then
  passes in all ten, and `cargo test --workspace --locked` in `10-ship`
  leaves the step clean.
- Follow-up: 2073, a local gate (`cargo metadata --locked` in each step)
  that fails a stale step lock before CI does. Whether a PR must be up
  to date with `main` before it merges, which would have stopped this
  race, is a repository setting: No ticket: Nick's call, not raised.

Book: none, only the tutorial's locks changed; the reader's project
makes its own.

No break. No `Behaviour:` line: only the tutorial's locks changed; the
game has no tutorial.

Nick input: none.
