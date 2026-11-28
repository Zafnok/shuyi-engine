---
id: "3514"
title: The roadmap's done marks are written from the tickets, and cargo xtask tickets ready lists what can start
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2099"]
nick_input: none
completed:
---

# 3514 — The roadmap's done marks are written from the tickets, and cargo xtask tickets ready lists what can start

## Context

[`docs/ROADMAP.md`](../../docs/ROADMAP.md) is two things at once: Nick's
plan in product terms, and the list a session reads to choose its next
ticket (the docs map, `docs/README.md`, rows *The order tickets are done
in* and *The plan, in product terms*). Both readers are misled today, and
the file is the backlog's busiest merge hotspot.

Checked on 2026-11-23 against `main` at `664171e6`:

- **The done marks drift.** Of the table rows whose first cell is a
  ticket id (`| NNNN …`), 263 name a ticket in `tickets/done/`: 65 carry
  `(done)` and 198 don't (2810, 2002, 2531, 3063 and 2554 are examples).
  179 rows name an open ticket, and none of those is marked. No row names
  an id that is in neither folder. (A script over every `| NNNN` row,
  checking `(done)` in the row and the id in each folder.)
- **The file is edited by most PRs.** 34 of the last 60 first-parent
  commits on `main` changed `docs/ROADMAP.md`; 2099 counted 45 of 100,
  "many only to add a mark". Each such edit is a conflict waiting for
  the next parallel PR.
- **Nothing lists what can start.** The `work-ticket` skill, §1, has the
  session read `tickets/README.md` and the 86 KB roadmap, then work out
  by hand which tickets are `status: todo` with every `blocked_by` done,
  with a `gh api` call for `game#` ids. The order is "the order in
  `docs/ROADMAP.md`", which nothing reads as data.

2099 (a blocker here) adds `cargo xtask roadmap`, which writes the block
*Left before the release* from `tickets/`, and a ticket-lint check that
the block is current. Its **Out** keeps the `(done)` marks hand-written,
and its block checks nothing after the first release. This ticket takes the
next step on the same machinery: the marks are written by the same
command and checked by the same lint, and a new command lists the ready
tickets in order, so choosing a ticket needs neither a hand count nor an
edit of the roadmap.

The lint already reads the game's ticket ids, from `gh api` or a local
checkout (`--game-repo`): `GameSource` and `game_ticket_ids` in
`crates/xtask/src/tickets.rs`. The new command reads them the same way.

From the blind-spot audit of 2026-11-23 (its proposed ticket 86).

## Nick input

None. The roadmap's prose and its phases stay Nick's; only the `(done)`
marks change hands, from people to a command.

## Scope

**In:**
- **The marks are generated.** `cargo xtask roadmap` (2099) also rewrites
  the first cell of every table row whose first cell is a ticket id: it
  reads `NNNN (done)` when NNNN is in `tickets/done/`, and `NNNN` when it
  is open. Nothing else in the row changes. Rows whose first cell is not
  a ticket id (the comparison tables' prose rows, such as *Bevy's
  homepage*) are left alone, and so are history notes in other cells
  ("done 2026-11-14: …").
- **The lint checks them.** `cargo xtask ticket-lint` fails a row whose
  mark differs from what the command writes, naming the line and
  `cargo xtask roadmap` as the fix, and a row whose id is in neither
  folder. This check stays after the first release.
- **A `rank` field.** An optional frontmatter field, `rank: <positive
  integer>`, lower first. The lint fails a value that is not a positive
  integer. No ticket needs one; it is for a ticket whose place the
  roadmap doesn't give.
- **`cargo xtask tickets ready`.** Prints the ready tickets, one a line:
  id, milestone and title. Ready is as `tickets/README.md` says: `status:
  todo` (so never a parked ticket) and every `blocked_by` id done, a
  `game#NNNN` in the game's `tickets/done/`. Order: a ticket with a
  `rank` first, by rank; then by the first roadmap line that names it in
  a row's first cell; then by id. `--game-repo <path>` reads the game
  from a checkout, as the lint does. When the game can't be read, the
  tickets that wait only on `game#` ids are printed after the rest under
  a heading that says so and why, never silently dropped or counted
  ready. `--all` adds the open tickets that are not ready, each with
  what it waits for, as 2099's *after NNNN, MMMM*.
- Docs (step 5).

**Either order with 3513:** if 3513 is done, remove the note "read whole
by `work-ticket` §1 until 3514" from `docs/ROADMAP.md`'s report-only row
in `tools/context-budget.ron` (or drop the row if no step reads the
roadmap any more); if not, nothing to do (3513 gives the roadmap no
whole-read note if this ticket has already changed `work-ticket` §1).

**Out (do not do):**
- 2099's block *Left before the release* and its two release errors:
  2099 builds them. This ticket reuses its `roadmap` command, markers and
  `TicketFields` additions.
- Rewriting the roadmap's prose, its phases or its comparison tables'
  *State* cells, or moving rows. Order stays where it is; `rank` only
  adds to it.
- `site/data/features.toml`, which copies the *Bevy's homepage* states:
  its own row in the docs map.
- An MCP tool for the ready list. The MCP server (`crates/cli/src/mcp`)
  serves a game's tools to game developers, and the engine's own backlog
  is not a game's; a session runs the command in its shell.
- Byte budgets for the files a session must read, and a generated
  run-gates table: 3513.
- A short shape and a freshness field for parked tickets: 3515. Both
  tickets add frontmatter fields; neither needs the other.

**Estimated size:** about 750 lines (code 330, tests 330, docs 90).

**The readiness walk is 2099's** (the redundancy sweep of 2026-11-28): `ready.rs` calls the
waits-for walk 2099 writes in `tickets/release.rs` rather than walking
the blockers again.

## Implementation steps

1. `crates/xtask/src/tickets.rs`: `TicketFields` gains `rank:
   Option<u32>`, read with the other scalars in `parse_ticket`; a value
   that isn't a positive integer is an error on that ticket.
2. The module 2099 adds (`crates/xtask/src/tickets/release.rs`), or a new
   `crates/xtask/src/tickets/roadmap.rs` beside it if that reads better:
   - `marked(roadmap, tickets) -> String`: the file with each ticket
     row's first cell rewritten as **In** says, line endings kept;
   - `check_marks(roadmap, tickets) -> Vec<String>`: one error a line
     that differs, and one for an unknown id, each with its line number.
   A ticket row is a line starting `|`, whose first cell, trimmed, is
   four digits, optionally followed by ` (done)`.
3. `crates/xtask/src/main.rs`: `roadmap` runs `marked` before it writes
   2099's block; `run` in `tickets.rs` calls `check_marks` next to 2099's
   `check_roadmap`.
4. A new `tickets ready` command in `main.rs`, its logic in a new
   `crates/xtask/src/tickets/ready.rs`: `ready(tickets, roadmap, game)`
   returns the ordered rows, `GameTickets` read through the existing
   `game_ticket_ids`, only when some candidate names a `game#` id. Its
   usage line in the module's doc comment and in the usage text.
5. Run `cargo xtask roadmap` and commit the marks it writes (about 198
   added). Then docs:
   - the `work-ticket` skill, §1: run `cargo xtask tickets ready` instead
     of listing ready tickets by hand, and take its first row unless the
     one sentence about the pick says why not;
   - `tickets/README.md`: `rank` in *Frontmatter*; *Picking the next
     ticket* names the command and its order;
   - the `write-ticket` skill: a new ticket's roadmap row is written
     without a mark, and the `(done)` marks are never written by hand;
   - `docs/README.md`, the row *The order tickets are done in*: the marks
     are written by `cargo xtask roadmap`;
   - the module doc comments.
   Then `pwsh -File scripts/sync-agent-skills.ps1 -Check`.
6. Run `cargo xtask ticket-lint`.

## Acceptance criteria

- [ ] `cargo xtask roadmap` on `main` marks every row of a done ticket
      `(done)`, and leaves every line that isn't a ticket row as it was.
- [ ] Run a second time, it changes nothing.
- [ ] A faked roadmap with a done ticket's row unmarked fails the lint,
      naming the line and `cargo xtask roadmap`; one with an open
      ticket's row marked `(done)` fails too.
- [ ] A row whose id is in neither folder fails the lint.
- [ ] With 2005 in `done/`, the marks' check still runs.
- [ ] `rank: 0`, `rank: -1` and `rank: soon` each fail the lint; `rank:
      3` passes.
- [ ] In a faked tree, `tickets ready` lists a todo ticket whose blockers
      are done, and not one with an open blocker, a `status: blocked`
      ticket, a parked ticket or an in-progress one.
- [ ] Its order is rank first, then the first roadmap row, then id, shown
      on a faked tree where each rule decides one pair.
- [ ] With the game unreadable, a ticket waiting on `game#NNNN` is
      printed under the heading that says so, with the reason, and not
      among the ready rows.
- [ ] `--all` prints an open ticket that waits with *after* and its open
      blockers.
- [ ] `cargo xtask ticket-lint` passes on `main` with the marks written.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, in the module of step 2: `marked` and `check_marks` on roadmaps
  written in the test (a done row, an open row, a marked open row, a
  prose row, a row in a table whose first column isn't ids, CRLF lines).
- Unit, in `ready.rs`: each case of the acceptance criteria above, on
  tickets parsed from strings as `tickets/size.rs`'s tests do, with the
  game given as a fixed `GameTickets`.
- Unit: `rank` parsing, each value above.
- Integration: `run` on a temporary repository with `tickets/{open,done}`
  and a `docs/ROADMAP.md`, as the tests at the end of
  `crates/xtask/src/tickets.rs` build one: an unmarked done row fails,
  and `roadmap`'s rewrite then passes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
