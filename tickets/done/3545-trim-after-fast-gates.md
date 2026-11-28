---
id: "3545"
title: A passing cargo xtask gates --fast ends with the trim of idle worktrees too
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: ["3544"]
nick_input: none
completed: 2026-11-28
---

# 3545 — A passing cargo xtask gates --fast ends with the trim of idle worktrees too

## Context

`cargo xtask trim-targets` (ticket 2076, ADR-0121) deletes the
incremental caches of the other, idle worktrees. It runs by itself only
at the end of a passing full local `cargo xtask gates`; `--fast` and
`--list` skip it (`crates/xtask/src/gates.rs`). Since ticket 3544 the
default before a push is `--fast` plus targeted clippy and tests, and
the full run is CI's (ADR-0166), so the trim now seldom runs by itself
and idle worktrees keep their caches (about half of each `target/`).

## Nick input

`None.`

## Scope

**In:**
- A passing `cargo xtask gates --fast`, locally (not with `CI` set),
  ends with `cargo xtask trim-targets` with its defaults, as a full run
  does: one line on stderr, never failing the gates, off with
  `SHUYI_NO_TRIM=1`.
- Measure how long the trim takes with nothing to delete and with a few
  idle worktrees, and say it in the completion notes. If it adds more
  than about two seconds to `--fast`, run it at most once a day instead
  (a stamp file under the repository's common `.git` folder) and say so.
- The pre-push hook runs `--fast`: a push must never wait on a long
  trim or fail because of one.
- Update `run-gates` (the trim paragraph and *Before a push*),
  `AGENTS.md`'s Environment note and ADR-0121's consequences through a
  short amendment in ADR-0166's spirit (a new ADR, or say why none is
  needed).

**Out (do not do):**
- No change to what the trim deletes or its defaults.
- `--list` still never trims.

## Implementation steps

1. In `crates/xtask/src/gates.rs`, run the trim after a passing run
   whether or not `--fast` is set, still not with `--list` or `CI`.
2. Test that the plan of a `--fast` run includes the trim and a
   `--list` one doesn't, as the existing tests do for the full run.
3. Time it (step "Measure" above) and add the daily stamp if needed.
4. Update the docs named in the Scope.

## Acceptance criteria

- [x] A passing local `cargo xtask gates --fast` prints the trim's line
      on stderr; `--list`, `CI=true` and `SHUYI_NO_TRIM=1` don't trim.
- [x] The completion notes give the trim's measured time.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the gates' plan trims after `--fast`, not after `--list`.

## Completion notes

- `cargo xtask gates` chooses the trim with the pure `trim_after`
  (`No` after `--list` or a failure, `Now` after a full run, `Daily`
  after `--fast`), tested in `a_passing_run_trims_and_a_list_never_does`.
  A full run trims as before.
- **Measured** on Nick's machine, 2026-11-28, 110 worktrees, 98 idle:
  `trim-targets` took 58 s freeing 24.4 GB, and 43 s again with nothing
  to delete (the walk, about 0.4 s a worktree, so a few idle worktrees
  would be about 2 s); `--dry-run` took 5 min 48 s, as it sizes each
  whole `target/`.
- **Deviation:** far over two seconds, so the trim after `--fast` runs at
  most once a day (the stamp `shuyi-trim/stamp` under the common `.git`
  folder, `trim::due`, tested), and also **in the background**, never
  waited for: even once a day, a minute would hold a push, which the
  Scope rules out for the pre-push hook. It runs a copy of xtask (so a
  build of xtask isn't refused on Windows meanwhile), detached and
  holding no handle of the gates' process, writing its own lines to
  `shuyi-trim/last.log` through a new `trim-targets --log <file>`. On
  Windows it is started by PowerShell's `Start-Process`: a first try
  with `DETACHED_PROCESS` returned, but a shell reading the output
  through a pipe waited 1 min 21 s for the trim, as the child inherited
  the pipe's handle, and xtask forbids the `unsafe` that would stop
  that. Starting it costs about 1.1 s once a day (`--fast` 7.6 s
  against 6.5 s with `SHUYI_NO_TRIM=1`), checked with a pipe: the gates
  returned while the trim ran on. A second start while one runs fails
  to copy the program and says so in a warning line. The gates print
  one line: started, or ran in the last day. `CI`, `SHUYI_NO_TRIM=1` and `--list` still
  never trim. Recorded in ADR-0167, which amends ADR-0121 §4 and
  ADR-0166 §4 (their status lines say so).
- Docs: `run-gates` (*Before a push* and the trim paragraph),
  `AGENTS.md`'s Environment note, the usage text of `cargo xtask gates`.
- Book: none, the book's chapter is for a game's `cargo shuyi
  trim-targets`, which this doesn't change; no public API changed.
- Skills: none, no schema, template, format or problem code changed; the
  change is the repository's own xtask.
- No break; no `Behaviour:` line, as xtask is not part of a game.
