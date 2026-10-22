---
id: "2023"
title: "The ticket lint reads every way a ticket waits on another, the settle step reads the docs too, and a done ticket has no unchecked box"
type: infra
milestone: MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2023 — The ticket lint reads every way a ticket waits on another

## Context

2017 made the ticket lint warn on a clause in an open ticket that waits
on a done one, and made the `work-ticket` skill's §5 settle such clauses
in the PR that closes a ticket. The lint
(`crates/xtask/src/tickets.rs`, `find_clauses`) knows four forms:
"either order with NNNN", "if NNNN is/are done", "when NNNN is done" and
"NNNN, if done". It matches across line breaks and Markdown emphasis.

A retro of PRs #38–#68 (2026-10-22) found clauses about done tickets that
these forms miss, so they were never settled:

- 2315: "if 2304 has landed …" and "If 2304 hasn't landed, 2304 adds
  that line". 2304's completion notes already decided it.
- 2802: "once 1133 and 1134 are done" (1133 is done), and "if 2509
  isn't done".
- 3008: "once 3006 lands".
- 2119: "2015's float ban covers it if it is done".
- 3041: "once 3040 is done" (it describes the past, so it is harmless,
  but it reads as a wait).
- From before that window: 2504 ("until 3001 exists"), 3023 ("if 3002
  has landed") and 3101 ("If 3005 has landed").

Two more gaps:

- §5 greps only `tickets/open`. A promise in the docs names a ticket too,
  and nothing settles it: `docs/README.md`'s *Pairs* table said "nothing
  yet (ticket 2017 adds `docs.rs`)" after 2017 landed. 2022 fixes that
  row; this ticket stops the next one.
- 2015 is in `tickets/done/` with all ten acceptance boxes unchecked,
  and 2002 has unchecked boxes too. Nothing checks that a done ticket's
  boxes are ticked.

## Nick input

None.

## Scope

**In:**
- **More forms in `find_clauses`**, each with a row in the test table
  beside the existing ones (`crates/xtask/src/tickets.rs`, the test
  around line 1207):
  - `if <ids> has landed` / `have landed`;
  - `if <ids> hasn't landed` / `haven't landed` / `isn't done` /
    `is not done` / `aren't done`;
  - `once <ids> lands` / `land` / `is done` / `are done` / `has landed`;
  - `until <ids> exists` / `exist` / `lands` / `is done`;
  - `<id>'s …, if it is done` / `if it has landed`, where the id is the
    nearest one before `if it` in the same sentence (2119's form).

  The doc comment of `find_clauses` lists every form. Forms that a
  sentence about the past uses ("once 3040 is done" in a *What Shuyi has*
  paragraph) still warn: the fix is to write it as what holds now.
- **The lint reads the docs too.** The same forms, plus `ticket <id>
  adds` inside a table cell, in every `*.md` under `docs/` except
  `docs/adr/` (ADRs are records of their time) and `docs/ROADMAP.md`
  (it lists done tickets on purpose). A warning names the file and line
  as for a ticket.
- **A done ticket has every acceptance box ticked.** In
  `tickets/done/`, a `- [ ]` line under `## Acceptance criteria` is an
  error: tick it, or write it as `- [x] ~~…~~ (not done: why)` and say
  why in the completion notes. Fix 2002 and 2015 in the same PR, from
  what their PRs did (`git log --grep '\[2002\]'`, `--grep '\[2015\]'`).
- **Settle what the new forms find**: every open ticket and doc the lint
  now warns on, the list in Context and any more it finds. For each,
  read the done ticket's completion notes and rewrite the clause as what
  now holds, dropping the other branch (`work-ticket` §5).
- **`work-ticket` §5:** the grep is `grep -rnw "<NNNN>" tickets/open
  docs --exclude-dir=adr`, and the list of clause forms points at
  `find_clauses`' doc comment instead of repeating it.
- **`run-gates`**: its paragraph on the ticket lint names the new check
  (done tickets' boxes) and the docs.

**Out (do not do):**
- warning on prose that only names a done ticket ("2304 added the
  edges");
- making the clause warnings errors;
- the game's ticket lint.

## Implementation steps

1. Read `crates/xtask/src/tickets.rs` (`words`, `clause_at`,
   `find_clauses`, `check_clauses` and their tests), and the
   `work-ticket` skill's §5.
2. Add the forms to `clause_at`, test first: one row per form in the
   table test, and the negative cases (`once upon`, `until 22044
   exists`, `if it is done` with no id before it in the sentence).
3. Read `docs/**/*.md` in the lint, skipping `docs/adr/` and
   `docs/ROADMAP.md`, and add `ticket <id> adds` for table cells.
4. Add the done ticket check, with tests for a ticked box, an unticked
   one, and a struck one marked "not done".
5. Run `cargo xtask ticket-lint`, then settle every warning it gives, and
   fix 2002 and 2015.
6. Update `work-ticket` §5 and `run-gates`.

## Acceptance criteria

- [ ] Each form in Scope has a passing row in the table test, and each
      negative case finds nothing.
- [ ] A clause of these forms in `docs/README.md` warns; one in
      `docs/adr/` or `docs/ROADMAP.md` doesn't.
- [ ] A done ticket with `- [ ]` under *Acceptance criteria* fails the
      lint.
- [ ] `cargo xtask ticket-lint` passes with no warning on `main`'s
      tickets after the settling.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the clause forms (table test) and the done ticket check
  (`crates/xtask/src/tickets.rs`'s tests).
- Unit: `run` over a temporary repository (`make_temp_tickets_dir`) with
  a done ticket, a clause naming it in `docs/README.md` (one warning) and
  the same clause in `docs/adr/` and `docs/ROADMAP.md` (none).
- Integration: `crates/xtask/tests/cli.rs`'s
  `ticket_lint_passes_on_the_repository` still passes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
