---
id: "2039"
title: The sound parity job runs only when a change can reach sound, on main too
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-01
---

# 2039 — The sound parity job runs only when a change can reach sound, on main too

## Context

Nick asked that the `sound-parity` job stop running when nothing it covers
changed: it takes 13 to 22 minutes. Ticket 2573 gave it a path filter, but
only for pull requests. Every push to main ran it whatever the push changed,
even a docs-only one, and on a pull request any file of `crates/app/` counted,
so a change to the renderer or the controllers ran it too.

## Nick input

None.

## Scope

**In:**
- `changes.yml`: on a push, `sound` is classified from what the push changed
  (`github.event.before` to the pushed commit). `code` and `crates` stay true
  on a push, for every workflow that calls it.
- `changes.yml`: `crates/app/` counts only for its audio files, `backend.rs`,
  `lib.rs`, the probe and its `Cargo.toml`.
- When the earlier commit can't be fetched (a new branch, a forced push),
  `sound` is true.
- `sound-give-up` keeps running on every push: a stray mention anywhere in the
  repository can break it, and it takes about a minute.
- The give-up footprint and the docs that describe when the job runs.

**Out (do not do):**
- Narrowing `Cargo.lock`, the root `Cargo.toml` or `ci.yml`: a dependency can
  change the sound, and the job lives in `ci.yml`.
- Changing the job's steps or budgets.

## Implementation steps

1. Compare a push with `github.event.before` in `changes.yml`.
2. Narrow the `crates/app/` patterns.
3. Keep `sound-give-up` running on a push.
4. Update `tools/sound-parity/footprint.ron`, the run-gates skill and the
   gates' README.

## Acceptance criteria

- [x] A push that changes no sound path leaves `sound=false`, so the job is skipped.
- [x] A push or pull request that changes a sound path leaves `sound=true`.
- [x] `code` and `crates` stay true on every push, and everything stays true on other events.
- [x] A push whose earlier commit can't be fetched leaves `sound=true`.
- [x] `cargo test -p xtask sound_parity` passes: giving up still matches the footprint.
- [x] The book needs no change: no public API changed.
- [x] The gates in the `run-gates` skill that this change can reach pass.

## Tests required

- The classify script run over real commit ranges in a scratch clone: no test
  harness runs a workflow's shell.

## Completion notes

Done as scoped. A push cancelled by the next one isn't compared again, as its
pull request already ran the job. Docs: the run-gates skill and the gates'
README say when the job runs.

No break. Book: none, CI only.
