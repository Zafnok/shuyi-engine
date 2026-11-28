---
id: "2099"
title: "The ticket lint fails an open MVP ticket the first release doesn't wait for, and the roadmap lists what is left before the release"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2099 — The ticket lint fails an open MVP ticket the first release doesn't wait for, and the roadmap lists what is left before the release

## Context

Nick's rule of 2026-11-05 (`docs/decisions.md`, *the first release waits
for everything before going public*): a ticket's `blocked_by` names
everything it waits for, and "waits for the MVP" means the MVP's open
tickets are in the list. The first release, 2005, waits for the MVP. No
gate checks this, and the retro of 2026-11-17 (the last 100 PRs, #379 to
#484) found two ways it slipped:

- **An MVP ticket outside the list.** 2090 (`milestone: MVP`, a row in
  the roadmap's phase 2 table) was written by 2089, one of 2005's
  blockers, and never joined 2005's `blocked_by`. The ticket-only PR of
  that retro added it by hand.
- **A blocker's own fix filed after the MVP.** 3218, which blocks 2005,
  put the game's Playdate frame over its budget, and its fix, 3219, was
  `milestone: After MVP`. It was done the same day (#485), but only
  because the game's bench caught it. Nothing
  would have caught an open ticket in 2005's list whose milestone says
  it comes after the release.

And nobody can see what is left. Phase 2's table in
[`docs/ROADMAP.md`](../../docs/ROADMAP.md) has about 104 rows, of which
about 9 are open. Its `(done)` marks are written by hand and drift:
2087 and 2347 are done, and their rows don't say so. 45 of those 100 PRs
edited the roadmap, many only to add a mark. "What is left before the
release, and what of it is ready" was worked out by hand each time, and
the retro's problem 5 found about 13k lines of work after the MVP done
while four release blockers were ready.

This ticket makes both checks a gate, and writes the list of what is
left from `tickets/`, so it is never written by hand.

**Where the list goes.** `ROADMAP.md` is Nick's file in the docs map
(`docs/README.md`, *Nick*: the plan, in product terms) and also the
engine developers' (*The order tickets are done in*). A generated list
of tickets, by their titles, is allowed there: it is the same kind of
fact as the phase 2 table already in it, so nothing technical goes in
Nick's file (the docs map's rule 4), and a list the code can produce is
generated rather than written (rule 2). It goes at the top of phase 2,
where Nick reads the plan and where a session choosing a ticket looks
first.

**Worth doing only before the release.** After the release, with 2005 in
`done/`, both checks have nothing to check. The milestone is After MVP
because the release doesn't need it, not because it should wait.

## Nick input

None.

## Scope

**In:**
- Two errors of `cargo xtask ticket-lint`, read while 2005 is in
  `tickets/open/`:
  1. an open ticket with `milestone: MVP`, other than 2005, that 2005's
     `blocked_by` doesn't name;
  2. an open ticket that 2005 waits for, directly or through a
     blocker's own `blocked_by`, whose milestone is neither `MVP` nor
     `"Engine: public"` (the book and playground tickets of 2026-11-05
     have that milestone). A `game#NNNN` id is not followed.

  2005 naming a done ticket of any milestone is fine: its list keeps the
  history of what it waited for. An open `"Engine: public"` ticket that
  2005 doesn't name is fine too: the blog (2805) and sponsors (2806)
  don't hold the release up (2026-11-05).
- **The list *Left before the release*,** at the top of phase 2 in
  `docs/ROADMAP.md`, between two marker comments. It is written by a new
  command, `cargo xtask roadmap`, from `tickets/` alone (no network), and
  the ticket lint fails it when it differs from what the command would
  write, with the command to run in the error.
- What each row of the list says: the ticket's id, its title, and one of
  *ready*, *in progress*, or *after NNNN, MMMM* (the open engine tickets
  it waits for). A `game#NNNN` blocker is named as it is and never looked
  up, so the list never changes without a change here. Ready rows come
  first, then the rest, each by id.
- One line each in the docs that say where facts live and how to pick
  a ticket (step 6).

**Out (do not do):**
- The `(done)` marks and the order of phase 2's table: they stay
  hand-written here; 3514 generates the marks after this ticket.
- Whether work after the MVP waits for the release: a question for Nick
  (the retro's question 2), asked with `ask-nick` by whoever takes it up.
- Checking the game's tickets that 2005's blockers name.
- Any other release than 2005.
- Changing any ticket's milestone or 2005's `blocked_by`: the retro's
  ticket-only PR already put 2090 there. If the lint finds
  another, fix it in this PR (add it to 2005, or move its milestone if
  it really comes after the release) and say which in the completion
  notes.

**A stale list on `main`.** Two PRs that each close a blocker and merge
one after the other without the second being brought up to date leave
`main`'s list one change behind, and the lint then fails every PR until
one runs `cargo xtask roadmap`. That is a one-command fix any PR may
carry, since the command changes only the block. Once Nick's click in
2005 ("Require branches to be up to date before merging") is on, it
can't happen. The error says so in one clause.

**Either order with 3513:** if 3513 is done, use its
`crates/xtask/src/generated.rs` (the read, compare and rewrite of a block
between two marker comments) for the roadmap's list instead of writing
your own in `tickets/release.rs`; if not, write the marker functions in
`tickets/release.rs` as planned (3513 moves them to `generated.rs` when
it lands second).

**Estimated size:** about 700 lines (code 300, tests 320, docs 80).

**Either order with 3516 and 2844:** if one of them is done, move its
marker-block splice onto `generated.rs`. If not, nothing to do (they use
`generated.rs`, their clauses say so).

## Implementation steps

1. `crates/xtask/src/tickets.rs`: `TicketFields` gains `milestone` and
   `title` (`scalar_string(map, "milestone")`, and the title that
   `parked` already reads). Pass the parsed tickets to the new module's
   checks in `run`, next to `size::check_estimates`, and read
   `docs/ROADMAP.md` there for the list's check.
2. A new `crates/xtask/src/tickets/release.rs`, with `const RELEASE:
   &str = "2005"` and:
   - `check_blockers(tickets) -> Vec<String>`: the two errors above,
     each naming the ticket's path, its milestone and the fix ("add it
     to 2005's `blocked_by`, or give it the milestone it really has").
     The walk through blockers' `blocked_by` keeps a seen set, so a
     cycle doesn't loop;
   - `left(tickets) -> Option<String>`: the block between the markers,
     or `None` with 2005 in `done/`. A Markdown table, `| Ticket | What |
     Ready |`, a `|` in a title escaped as `\|`, and a first line *N
     tickets are left before the first release (2005).*;
   - `check_roadmap(roadmap, tickets) -> Option<String>`: an error when
     the markers are missing or what is between them differs from
     `left`, CRLF read as LF.
   The markers are `<!-- Left before the release: written by cargo xtask
   roadmap from tickets/; do not edit. -->` and `<!-- End of the list. -->`.
3. `crates/xtask/src/main.rs`: the `roadmap` command, which rewrites the
   block in `docs/ROADMAP.md` in place (and adds it under the heading
   `## Phase 2: the MVP, parity with the game` when the markers are
   missing), keeping the file's line endings. Its usage line in the
   module's doc comment and in the usage text.
4. Run `cargo xtask roadmap` and commit the block it writes.
5. Run `cargo xtask ticket-lint`. On today's tickets both errors should
   find nothing; fix any it finds as **Out** says.
6. Docs:
   - `docs/README.md`: the row *The order tickets are done in* says the
     list *Left before the release* is written by `cargo xtask roadmap`
     and checked by the ticket lint;
   - `tickets/README.md`, *Picking the next ticket*: a ready ticket of
     that list is the first to look at;
   - the `work-ticket` skill, §1: read that list first, and say in the
     one sentence about the pick why it isn't one of its ready rows
     when it isn't;
   - the `write-ticket` skill, *Check the dependency chain*: an MVP
     ticket goes in 2005's `blocked_by`, and the lint fails it
     otherwise;
   - the module doc comment at the top of `crates/xtask/src/tickets.rs`.
   Then `pwsh -File scripts/sync-agent-skills.ps1 -Check`.

## Acceptance criteria

- [ ] A faked tree whose open MVP ticket is missing from 2005's
      `blocked_by` fails the lint, naming the ticket and 2005.
- [ ] A faked tree where 2005 names an open `After MVP` ticket fails;
      one where 2005 names an open MVP ticket that waits for an open
      `After MVP` ticket fails too, naming both.
- [ ] 2005 naming done tickets of any milestone, and an open
      `"Engine: public"` ticket outside 2005's list, pass.
- [ ] With 2005 in `done/`, neither error is read and the list's check
      passes without the block.
- [ ] `cargo xtask roadmap` run twice changes nothing the second time,
      and leaves every line of `docs/ROADMAP.md` outside the block as it
      was.
- [ ] A list one ticket behind fails the lint with the command to run.
- [ ] A row says *after 2405* for a ticket that waits for open 2405,
      and *ready* once 2405 is in `done/`.
- [ ] `cargo xtask ticket-lint` passes on `main`, with the block in
      `docs/ROADMAP.md`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, in `crates/xtask/src/tickets/release.rs`: each error above and
  each case that passes, on tickets parsed from strings as
  `tickets/size.rs`'s tests do; a cycle in `blocked_by`; a title with a
  `|`.
- Unit: `left`'s table for a fixed set of tickets, compared with the
  expected text written out in the test.
- Integration: `run` on a temporary repository with
  `tickets/{open,done}` and a `docs/ROADMAP.md`, as the tests at the
  end of `crates/xtask/src/tickets.rs` build one: a stale block fails,
  and `roadmap`'s rewrite then passes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
