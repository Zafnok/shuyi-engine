---
id: "2857"
title: The README brought up to date and trimmed
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-27
---

# 2857 — The README brought up to date and trimmed

## Context

Nick asked for the README to be brought up to date without growing it:
add what is missing, fix what is wrong, cut what doesn't belong. The
docs map ([`docs/README.md`](../../docs/README.md)) makes the README the
home of what the engine is, its crates, and where to start. Since it was
last read whole, Snake joined the examples (2739), the template became
one template made of parts (ADR-0013), the docs map (2810), the agent
eval and the website's book landed, and several rows grew past what a
first reader needs.

## Nick input

None.

## Scope

**In:**
- The README's facts checked against the repository and fixed: the
  number of examples, the template, which crates hold `unsafe`.
- Added: a link to the book, Snake beside Hello, Grid, rows for the docs
  map and `evals/`, `targets/` named under *Targets*, and the layer check.
- Cut: the carve's leftovers (a game ADR, a game ticket, a stray ticket
  number), map rows that duplicate links elsewhere, and long rows and
  paragraphs shortened.
- The README ends no longer than it started.

**Out (do not do):**
- Any other file's docs: each keeps its own home (the docs map).
- The `shuyi-cli` row's list of operations, which
  `crates/cli/tests/it/docs.rs` reads: it keeps every operation.

## Implementation steps

1. Read the README beside `docs/ROADMAP.md`, `docs/README.md`,
   `examples/`, `templates/`, `evals/` and each crate's lints.
2. Edit `README.md`: add, amend and cut as the Scope says.
3. Run `cargo xtask gates --fast`, `cargo xtask check-links`,
   `cargo xtask check-docs` and `cargo test -p shuyi-cli --test it -- docs::`.

## Acceptance criteria

- [x] The README says two examples run on the engine, and lists both.
- [x] The `templates/` row describes the one template of parts and its
      two named sets of answers.
- [x] The `unsafe` sentence names every crate whose lints allow it: the
      backends, the sound output and the heap counter.
- [x] No game ADR or game ticket number is left in the crates section.
- [x] The README is shorter than before (293 lines).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: none; `docs::the_readme_names_every_operation` keeps passing.
- Property: none.
- Snapshot / integration: `check-links` and `check-docs` pass.

## Completion notes

`README.md` went from 293 lines to 283 (54 added, 64 removed).

- Added: the book's link; Snake in *The examples* (was *The first
  example: Hello, Grid*); map rows for `docs/README.md` and `evals/`;
  `targets/` under *Targets*; `check-layers` and `tools/layers.ron`
  under *The crates*.
- Fixed: "one example" is two; the `templates/` row; the `unsafe`
  sentence, which left out `shuyi-playdate` and `shuyi-sound-out`.
- Cut: the map rows for `docs/carve.md` and `docs/platform-backends.md`
  (the crates table links the latter); three CI upkeep rows made one;
  the Linux setup row, the heap counter's row and the `shuyi-cli` row's
  tool programs shortened; the dependents' dispatch, *Fast compiles*
  and the example's run commands tightened.

No break. No behaviour change.

Book: none, only the README changed.
Skills: none, no schema, template, format or problem code changed.
