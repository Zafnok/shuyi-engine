---
id: "2023"
title: "The ticket lint reads every way a ticket waits on another, the settle step reads the docs too, and a done ticket has no unchecked box"
type: infra
milestone: MVP
model: implementation
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-26
---

# 2023 — The ticket lint reads every way a ticket waits on another

## Context

2017 made the ticket lint warn on a clause in an open ticket that waits
on a done one, and made the `work-ticket` skill's §5 settle such clauses
in the PR that closes a ticket. The lint
(`crates/xtask/src/tickets.rs`, `find_clauses`) knows five forms:
"either order with NNNN", "if NNNN is/are done", "when NNNN is done",
"NNNN, if done" and "waits for NNNN" (2027). It matches across line
breaks and Markdown emphasis.

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
- **The new forms count for 2027's error too.** 2027's `check_mirrors`
  fails a clause in an open ticket whose other open ticket never names
  it back, and it reads every form `find_clauses` knows. So a clause the
  new forms find between two open tickets must be written on both sides:
  where the lint fails, write the other side in the ticket the clause
  names (`write-ticket`, *Check the dependency chain*), as for the rest.
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

- [x] Each form in Scope has a passing row in the table test, and each
      negative case finds nothing.
- [x] A clause of these forms in `docs/README.md` warns; one in
      `docs/adr/` or `docs/ROADMAP.md` doesn't.
- [x] A done ticket with `- [x]` under *Acceptance criteria* fails the
      lint.
- [x] `cargo xtask ticket-lint` passes with no warning on `main`'s
      tickets after the settling.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the clause forms (table test) and the done ticket check
  (`crates/xtask/src/tickets.rs`'s tests).
- Unit: `run` over a temporary repository (`make_temp_tickets_dir`) with
  a done ticket, a clause naming it in `docs/README.md` (one warning) and
  the same clause in `docs/adr/` and `docs/ROADMAP.md` (none).
- Integration: `crates/xtask/tests/cli.rs`'s
  `ticket_lint_passes_on_the_repository` still passes.

## Completion notes

- `find_clauses` reads `once … lands / is done / has landed`, `until …
  exists / lands / is done`, `if … has landed / hasn't landed / isn't
  done / is not done`, and `<id>'s …, if it is done` (the nearest id in
  the sentence, found with a new `stop` flag on `Word`). A typographic
  apostrophe reads as a plain one. Its doc comment lists all eight forms.
- The lint reads `docs/**/*.md` except `docs/adr/` and `docs/ROADMAP.md`
  (`read_docs`, `find_doc_clauses`), and `ticket <id> adds` in a table
  cell warns when that ticket is done. Warnings name the file and line.
- A done ticket with a `- [ ]` under *Acceptance criteria* is an error
  (`check_done_boxes`). Besides 2002 and 2015 it found 2011, 2031, 2555
  (all done as the notes say, now ticked) and 2358, 2801, 3039, 3063
  (moved or Nick's sign-off, written `- [x] ~~…~~ (not done: why)`).
  2031's second box was ticked, since the workflow's runs on `main` are
  green; its token box is struck, as the token is Nick's setup.
- Settled: 2368 and 2531 (clauses on done 3065 and 2513). The new forms
  also failed 2027's mirror check on 12 clauses; the other side is written
  as a **From NNNN (open)** line in 2114, 2218 (for 2223, 2342, 2343),
  2322, 2332, 2335, 2343, 2514, 3008, 3017 and 3023. The Context's
  list of clauses (2315, 2802, 3008, 2119, 3041, 2504, 3023, 3101) had
  mostly been settled since; the lint found the rest.
- `work-ticket` §5 and `run-gates` are updated. No docs map change: no
  new file. No `Breaking:` (xtask is tooling, `publish = false`).
