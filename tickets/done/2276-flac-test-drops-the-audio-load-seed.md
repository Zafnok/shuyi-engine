---
id: "2276"
title: Main builds again — the FLAC test calls Audio::load without the seed
type: bug
milestone: MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-23
---

# 2276 — Main builds again — the FLAC test calls Audio::load without the seed

## Context

Two PRs merged on 2026-11-23 a minute apart and clashed. Ticket 2345
(#536) moved the choice of a cue's variant into `App` and removed
`Audio::load`'s last argument, the seed. Ticket 2234 (#537), merged a
minute later, added the test `a_flac_sound_reaches_the_backend_as_a_wav`
in `crates/platform/src/audio/player.rs`, which still passes a seed of
`1`. Each PR was green alone; together, `main` at `f66e4968` doesn't
compile `shuyi-platform`'s lib tests, so every open PR's `clippy` and
`test` jobs fail.

## Nick input

None.

## Scope

**In:**
- Drop the `, 1` from that one `Audio::load` call.

**Out (do not do):**
- Any change to `Audio::load` or to the test's assertions.
- Any change to how CI checks a PR against the tip of `main` (a merge
  queue): not this ticket's job.

## Implementation steps

1. In `crates/platform/src/audio/player.rs`, test
   `a_flac_sound_reaches_the_backend_as_a_wav`, call
   `Audio::load(&mut fake, m, &files, music, voice)`.

## Acceptance criteria

- [x] `cargo clippy --workspace --all-targets` passes.
- [x] `cargo test -p shuyi-platform` passes, the FLAC test included.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the existing `a_flac_sound_reaches_the_backend_as_a_wav` compiles and passes.

## Completion notes

Dropped the stale seed argument from the one test call. No public API
changes: no break, no behaviour change, so no `Breaking:`, `Behaviour:`
or `Book:` line is needed. The docs need no change (a test only).
