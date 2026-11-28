---
id: "3545"
title: A passing cargo xtask gates --fast ends with the trim of idle worktrees too
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: ["3544"]
nick_input: none
completed:
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

- [ ] A passing local `cargo xtask gates --fast` prints the trim's line
      on stderr; `--list`, `CI=true` and `SHUYI_NO_TRIM=1` don't trim.
- [ ] The completion notes give the trim's measured time.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the gates' plan trims after `--fast`, not after `--list`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
