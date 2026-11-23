---
id: "3513"
title: "What an agent must read before code has a byte budget, and the run-gates skill's table of gates is generated from gates.rs"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3513 — What an agent must read before code has a byte budget, and the run-gates skill's table of gates is generated from gates.rs

## Context

A blind-spot audit of the engine (2026-11-23) found that nothing measures
or caps the text a session must read before it touches code, and that
the cost grows with every retro, since each retro's fix adds prose to the
same files. Checked on this branch with `wc -c`:

| File | Bytes | Read by |
| ---- | ----: | ------- |
| `AGENTS.md` (`CLAUDE.md` imports it) | 11,055 | every session |
| `docs/README.md`, the docs map | 28,678 | `work-ticket` §0.1; the `document` skill |
| `tickets/README.md` | 9,627 | `work-ticket` §1 |
| `.claude/skills/work-ticket/SKILL.md` | 16,404 | every ticket |
| `.claude/skills/run-gates/SKILL.md` | 33,854 (684 lines) | every push (`work-ticket` §6) |
| `.claude/skills/write-ticket/SKILL.md` | 11,824 | any follow-up |
| `.claude/skills/document/SKILL.md` | 1,687 | any public change |
| `docs/ROADMAP.md` | 86,370 | `work-ticket` §1: "read `tickets/README.md` and `docs/ROADMAP.md`" |
| `docs/decisions.md` | 98,019 | searched before asking Nick (`work-ticket` §1) |

The first eight come to about 199 KB, about 50k tokens, before the
ticket itself; `docs/decisions.md` adds about 25k more when it is read
whole. Two cells of `docs/README.md`'s tables (lines 31 and 32) are 1.5
and 2.2 KB each. A retro of this repository already records gates
claimed but not run and conventions missed, and rules late in a long
file are the ones skimmed.

The largest single file is the `run-gates` skill. It has been changed in
65 commits since the repository started. Most of it is a prose manual of
CI's jobs, one `##` section per job, that repeats what the table of gates
in `crates/xtask/src/gates.rs` already holds as data: `GATES`, 35 gates,
each with its name, its `ci.yml` job, its filter, its local rule, whether
`--fast` runs it, its steps, and, for 13 of them, why CI alone runs it.
Only one gate has a `hint` (what to say after a failure). The module's
own doc comment says the skill "stays the reference for what each gate
checks and how to fix it". A test there
(`every_ci_job_is_a_gate_with_its_filter`) already holds `GATES` to
`ci.yml`, so the table is true; the prose around it is not checked.

The repository already keeps budgets as ratcheted data:
`tools/compile-budget.ron` (ticket 2009, ADR-0049, ADR-0120), whose
dependency count "a change that adds one raises ... and one that drops
some lowers". This ticket does the same for the text an agent always
reads, and makes the `run-gates` skill short: what to run, then a
generated table, with each job's long section moved to a reference file
read only when that gate fails.

Related open tickets:

- 2099 writes a block between two marker comments in `docs/ROADMAP.md`
  from data (`cargo xtask roadmap`), and fails the lint when the block
  differs. This ticket needs the same helper for the `run-gates` table.
- 3514 replaces "read `docs/ROADMAP.md`" in `work-ticket` §1 with a
  command that lists the ready tickets.
- 3516 gives the decisions ids and a standing-rules page.
- 3517 ratchets the size of Rust files and crates.
- 2844 adds `docs/architecture.md` (300 lines or fewer) to the *Read
  first* cells of the `20xx`, `21xx`, `23xx`, `30xx`, `32xx`, `33xx` and
  `35xx` rows of the docs map, so most tickets read it whole.

## Nick input

None.

## Scope

**In:**
- `tools/context-budget.ron`: each file an agent reads whole on every
  ticket, with the step that reads it and its most bytes; a total for
  them all; and report-only rows (printed with their size, never failed)
  for the files read in part or searched.
- `cargo xtask check-context`: fails a file over its cap, a cap more than
  5 % above its file's size (the ratchet: a file that shrinks locks the
  saving in), the total over its cap, a listed file that is missing, and
  a stale generated table in the `run-gates` skill. Each error names the
  file, its size, its cap and the fix, with the number to write. Bytes
  are counted with CRLF read as LF. It runs in the `tickets' checks`
  gate and CI's `tickets` job.
- The `run-gates` skill split in two. Its long per-job sections move,
  as they are, to `.claude/skills/run-gates/reference.md` (a `git mv` of
  `SKILL.md`, so `cargo xtask pr-size`'s `-M` counts it as a rename). A
  new, short `SKILL.md` keeps the start (`cargo xtask gates`, `--list`,
  `--fast`, `--base`), the hooks in a few lines, the environment facts,
  and the generated table; its cap is 8,000 bytes.
- A new field on `Gate`, `section`: the heading in `reference.md` that
  holds the gate's detail. A failed gate prints the section's path and
  heading after its output, so the detail is read when it is needed.
- `cargo xtask check-context --write`: rewrites the table between two
  marker comments in the new `SKILL.md`, from `GATES`. Columns: gate,
  CI job, `--fast`, when a local run skips it (the `Local` rule's text),
  CI only (the reason), and the `reference.md` section.
- An ADR (`write-adr`): the text every session reads whole has a byte
  budget kept as data and ratcheted down, and the `run-gates` table is
  generated from `GATES`.
- No public API changes: everything is in `crates/xtask` and the
  repository's own docs. Nothing in the game changes; the game pins no
  file of `.claude/skills/run-gates/`.

**Either order with 2099:** if 2099 is done, move its marker-block
functions (the read, compare and rewrite of the block between two marker
comments, CRLF read as LF, the file's own line endings kept) out of
`crates/xtask/src/tickets/release.rs` into a new
`crates/xtask/src/generated.rs`, and use them for both blocks; if not,
write `crates/xtask/src/generated.rs` here with those functions, and
2099 uses it.

**Either order with 3514:** if 3514 is done and `work-ticket` §1 no
longer reads `docs/ROADMAP.md` whole, give it no row (or a report-only
one, if a step still reads part of it); if not, `docs/ROADMAP.md` is a
report-only row, with the note "read whole by `work-ticket` §1 until
3514", and 3514 drops that note.

**Either order with 2844:** if 2844 is done, `docs/architecture.md` is a
capped row, read by "the *Read first* cell of most blocks", with its cap
by the 5 % rule, and the total counts it; if not, nothing to do (2844
adds that row to `tools/context-budget.ron` and raises the total by its
cap).

**Out (do not do):**
- The command that lists ready tickets, and the `work-ticket` skill's
  §1 pick that uses it: 3514.
- `docs/decisions.md`: a report-only row. Its ids and a standing-rules
  page short enough to read whole are 3516's.
- Size ratchets on Rust files and crates: 3517.
- Editing `docs/ROADMAP.md` or `docs/decisions.md` in any way.
- Shortening `AGENTS.md`, `work-ticket`, `write-ticket`,
  `tickets/README.md` or `docs/README.md` now: each gets its cap at
  today's size by the 5 % rule, and only the ratchet holds them. Rewriting
  what a section of `reference.md` says: it moves as it is, but for
  the lines that become the short `SKILL.md`.
- Changing what any gate runs, when it runs, or a `ci.yml` job, beyond
  adding the `check-context` step.
- The shipped skill pack in `skills/` (a game's skills, read by a game's
  agent): its own budget, if any, is a later question.
- Counting tokens: bytes are stable across models and tokenizers, and
  the cap is about the size, not the price.

**Estimated size:** about 1,300 lines (code 450, tests 400, docs 400,
data 50). The move of `SKILL.md` to `reference.md` counts as a rename.

**Split:** past 1,000 lines, split off steps 5 and 6 (the `section`
field, the generated table and the failure pointer) into a new ticket;
this PR then keeps the budget, the check and the split skill, with the
short `SKILL.md`'s table of gates written by hand.

## Implementation steps

1. **The marker helper.** Settle the clause with 2099 above:
   `crates/xtask/src/generated.rs` with
   `block(text, begin, end) -> Option<&str>`,
   `replace_block(text, begin, end, body) -> Result<String, String>`
   (keeps the file's line endings) and
   `check_block(text, begin, end, body, path, command) -> Option<String>`
   (an error naming the file and the command to run).
2. **The budget file.** `tools/context-budget.ron`, read with `ron` into
   `#[derive(Deserialize)] struct Budget { files: Vec<Row>, total: u64,
   report: Vec<Report> }`, `Row { path, read_by, max_bytes }`,
   `Report { path, read_by }`. A comment at the top says what it is, the
   5 % rule, and that a raise carries a comment saying why, as
   `tools/compile-budget.ron` does. Capped rows: `AGENTS.md`,
   `CLAUDE.md`, `docs/README.md`, `tickets/README.md`, and the
   `SKILL.md` of `work-ticket`, `run-gates`, `write-ticket` and
   `document`, and `docs/architecture.md` as the clause with 2844
   says. Each cap is the file's size after this PR times 1.05,
   rounded up to the next 100 bytes. The total is the sum of those sizes
   times 1.05, rounded the same way. Report rows: `docs/ROADMAP.md` (as
   the clause with 3514 says), `docs/decisions.md`, and
   `.claude/skills/run-gates/reference.md` ("read a section, when that
   gate fails").
3. **The check.** A new `crates/xtask/src/context.rs` with
   `run(root) -> Vec<String>` (errors) and `report(root) -> String` (a
   table of every row: path, bytes, cap, read by), and `main.rs`'s
   `check-context` and `check-context --write`, with their usage lines.
   The errors, each one line:
   - `<path>: 9,912 bytes, over its cap of 9,800 (tools/context-budget.ron). Move detail to where it is read when needed, or raise the cap with a comment saying why.`
   - `<path>: its cap of 12,000 is more than 5 % above its 10,100 bytes; write 10,700 (tools/context-budget.ron).`
   - the same two for the total;
   - `<path>: listed in tools/context-budget.ron but missing`;
   - the stale table, from `check_block`, naming `cargo xtask check-context --write`.
4. **The skill split.** `git mv .claude/skills/run-gates/SKILL.md
   .claude/skills/run-gates/reference.md`, then give `reference.md` a
   heading and a first paragraph saying it is read a section at a time,
   when a gate fails or a gate is run alone. Write the new `SKILL.md`
   with the same front matter (name and description unchanged, so the
   loader in `.agents/skills/run-gates/` and
   `scripts/sync-agent-skills.ps1 -Check` stay as they are): the
   shared-skill check, *Start with `cargo xtask gates`* cut to the
   commands and what the summary block is for, the hooks in about ten
   lines with a link to `reference.md`'s hook detail, the environment
   facts, the table's markers, and one line saying that a new gate's
   detail goes in `reference.md` under a heading its `section` names.
   Remove from `reference.md` only what moved into `SKILL.md`.
5. **The `section` field.** `Gate.section: &'static str` with a
   `const fn section(...)` builder; every entry of `GATES` names the
   `##` heading of `reference.md` its job's detail is under (for
   example `"The code gates (CI's ci.yml)"`, `"Golden tables (CI's
   determinism job)"`). In `run_gate`, a failure's output ends with
   `See .claude/skills/run-gates/reference.md, "<section>".` after any
   `hint`. Update the module's doc comment: the commands are this
   table's, the skill's table is generated from it, and `reference.md`
   holds each gate's detail.
6. **The generated table.** `gates::skill_table() -> String`: one row
   per gate in `GATES` order, with `|` in a cell escaped. `cargo xtask
   check-context --write` writes it between
   `<!-- The gates: written by cargo xtask check-context --write from crates/xtask/src/gates.rs; do not edit. -->`
   and `<!-- End of the gates. -->`. Run it and commit the block.
7. **The gate.** Add `"cargo xtask check-context"` to the `tickets'
   checks` gate's steps in `GATES`, and a `run: cargo xtask
   check-context` step to the `tickets` job in
   `.github/workflows/ci.yml`, next to `check-docs`.
8. **The ADR**, with the `write-adr` skill, and its row in
   `docs/adr/README.md`.
9. **Docs** (the `document` skill): a row in `docs/README.md`'s
   table of homes for `tools/context-budget.ron` (what agents always
   read, and its budget) and `reference.md`; the row for *The local
   gates* names both files of the skill. `work-ticket` §6 is unchanged:
   it still says to run the `run-gates` skill.
10. Run `cargo xtask check-context` and `pwsh -File
    scripts/sync-agent-skills.ps1 -Check`; fix what they find.

## Acceptance criteria

- [ ] `tools/context-budget.ron` lists the capped files of step 2 (eight,
      or nine with `docs/architecture.md` if 2844 is done),
      each with the step that reads it, and the report rows of step 2.
- [ ] `cargo xtask check-context` passes on the PR's tree, and prints
      each row's bytes and cap.
- [ ] One byte appended to a capped file past its cap fails it, with the
      file, its size and its cap in the error.
- [ ] A cap raised to 6 % above its file fails, with the number to write
      (the size times 1.05, rounded up to the next 100); a cap 5 % above
      passes.
- [ ] The total over its cap fails, though every file is under its own.
- [ ] A file with CRLF endings counts the same bytes as with LF.
- [ ] A report row's file, however large, never fails the check.
- [ ] The new `.claude/skills/run-gates/SKILL.md` is at most 8,000 bytes
      and holds a row for every gate in `GATES`, between the markers.
- [ ] A hand edit inside the markers fails `check-context`, naming
      `cargo xtask check-context --write`; running it restores the block.
- [ ] A gate added to `GATES` without regenerating the table fails
      `check-context`.
- [ ] Every gate's `section` is a `##` heading of `reference.md`.
- [ ] A failing gate's output ends with the path of `reference.md` and
      its section.
- [ ] `cargo xtask pr-size` counts the move of `SKILL.md` to
      `reference.md` as a rename, not as its whole text added and removed.
- [ ] `scripts/sync-agent-skills.ps1 -Check` passes with the loader
      unchanged.
- [ ] `cargo xtask check-docs` and `cargo xtask check-links` pass with
      the new files.
- [ ] The ADR is written and in the index.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not (the book is for people making games;
      this is the repository's own process).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`context.rs`): over the cap; exactly at the cap; the ratchet's
  5 % edge on both sides, with the number written; the total; a missing
  file; CRLF and LF counted equal; a report row never failing; a
  malformed `tools/context-budget.ron` naming the file.
- Unit (`generated.rs`): a block found, replaced keeping CRLF or LF, a
  missing marker, an end marker before the begin marker, and an equal
  block passing.
- Unit (`gates.rs`): `skill_table()` has one row per gate in order and
  escapes `|`; every `section` names a heading that exists in
  `.claude/skills/run-gates/reference.md` (read from the repository);
  a failed gate's text ends with the section line.
- Integration: `cargo xtask check-context` on the repository passes, and
  the generated block in `SKILL.md` equals `skill_table()`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
