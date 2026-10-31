---
id: "2027"
title: "No handoff on one side only: the backlog audited, the rule in write-ticket and work-ticket, and a ticket-lint error on a one-way clause"
type: infra
milestone: MVP
model: reasoning
effort: xhigh
status: done
blocked_by: []
nick_input: none
completed: 2026-10-24
---

# 2027 — No handoff on one side only

## Context

Nick, 2026-10-23: 1134 "has a lot in the body that says 'waits for
ticket XXXX' but the 1134 ticket is itself not blocked by that ticket,
and going to ticket XXXX has no follow-up for the work 1134 describes if
it is completed in opposite order." He asked for the whole backlog to be
checked, the done tickets too, and for CI to check one-way references
the way it checks ticket ids.

The rules already ask for both sides. The `write-ticket` skill's *Check
the dependency chain* says each open ticket a ticket names is needed
first, either order (a clause), or not its job (**Out**), and "look the
other way too". But nothing says that the ticket a piece of work is
handed to must carry it, and nothing checks it. Here is how the work
gets lost. Ticket A says "either order with B: … if not, B adds X". If
B lands first, the PR closing B greps `tickets/open` for B
(`work-ticket` §5), finds A's clause and settles it. If A lands first,
A moves to `tickets/done/`, and B's PR greps for B. It never finds A,
and nobody adds X.

1134 says its `shuyi-net` skill's worked example needs 1140, 2529 and
1145, `shuyi-stage-field`'s needs 1119, 1120 and 1122, and
`shuyi-import-art`'s needs 1123 and 1124. Of these, only 2529 names
1134.
2231 even lists the `shuyi-animate` skill under its **Out**, while 1134
leaves it to 2231 to finish.

The same reading over the lint's own clause forms (`find_clauses`, plus
"waits for") found 226 pairs of open tickets where a clause in one names
the other, the other never names it back, and the first doesn't have it
in `blocked_by`.

## Nick input

None.

## Scope

**In:**
- **The audit, settled in this PR.** Every open ticket and every done
  one, read against every ticket it names. Each piece of work handed to
  another ticket (a clause's *if not* branch, "waits for", an **Out**
  line, a done ticket's completion notes) is carried by that ticket,
  in its words: "**Either order with NNNN:** if NNNN is done, …; if
  not, …", or "**From NNNN (done):** …" for a done one. Also fixed:
  - a missing `blocked_by`;
  - a blocker that isn't one;
  - two tickets that each hand a piece to the other;
  - two that both claim it;
  - a wrong id;
  - a promised follow-up with no ticket;
  - a clause about a done ticket.

  The completion notes list what changed, by kind.
- **The rule in the skills.**
  - `write-ticket`, *Check the dependency chain*: a handoff is written
    in both tickets in the same PR, the other side saying what it does
    if this one lands first ("if not, nothing to do" when nothing is
    left). "Waits for NNNN" is written as a clause or a blocker, never
    left as prose.
  - `work-ticket` §5: a new last step hands over what the ticket
    leaves. For each piece left to an open ticket, the closer checks
    that ticket names this one and says it does the piece, and writes
    it there if not. After the move to `done/` no grep finds this
    ticket, so this is the last chance.
- **The check in CI.** `cargo xtask ticket-lint`, which CI's `tickets`
  job runs (`crates/xtask/src/tickets.rs`):
  - `find_clauses` also reads `waits for <ids>` and `wait for <ids>`.
    The warning on a clause about a done ticket covers them too.
  - A new error, `check_mirrors`. A clause in an open ticket A names an
    open ticket B that isn't in A's `blocked_by`, and B's body never
    names A. The error names A's file and line, the clause and B, and
    sends the reader to `write-ticket`. Done tickets' clauses are
    history and aren't read: the other side was written while both
    were open, and it stays when A moves to `done/`.
  - A ticket "names" an id when the id appears in its body as a word
    of four digits. It is not part of a longer number or word, and not
    after `#` or `-` (so `game#1131` and `ADR-0005` aren't names).
    `1134's` counts.
  - `run-gates`' paragraph on the ticket lint names the new error, and
    `write-ticket` says the lint checks the rule.
- **Either order with 2023:** 2023 adds more clause forms to
  `find_clauses`. If 2023 is done, the new error reads its forms too,
  and its tests get a row for one of them. If not, 2023's forms join
  this check with nothing more to do: both read `find_clauses`.

**Out (do not do):**
- checking that an **Out** line's ticket carries the piece: no lint can
  read that, so the skills say it and the audit did it once;
- warning on a ticket that only mentions another;
- the game's ticket lint.

## Implementation steps

1. Audit the backlog (open and done tickets) and settle every finding
   in the ticket files.
2. Write the two skill rules.
3. Add the forms and `check_mirrors` to `crates/xtask/src/tickets.rs`,
   test first, and run `cargo xtask ticket-lint`. It passes with no
   error.
4. Update `run-gates`.

## Acceptance criteria

- [x] `cargo xtask ticket-lint` passes on the repository, and fails on a
      clause whose other ticket never names it back.
- [x] Each of 1119, 1120, 1122, 1123, 1124, 1140, 1145, 2103, 2231 and
      2529 says what it does for 1134's skills if 1134 lands first.
- [x] Every finding of the audit is settled or listed in the completion
      notes with why not.
- [x] `write-ticket` and `work-ticket` §5 state the rule.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `waits for` and `wait for` in the clause table test, and their
  negative cases (`waits for the`, `waiting for 2204`).
- Unit: `check_mirrors` over tickets built in the test:
  - a one-way clause (one error, with file, line and clause);
  - a mirrored one (none);
  - one naming its own blocker (none);
  - one naming a done ticket (none, only the existing warning);
  - a clause in a done ticket (none);
  - a name written `1134's` (counts) and `ADR-1134` (doesn't).
- Integration: `crates/xtask/tests/cli.rs`'s
  `ticket_lint_passes_on_the_repository` still passes.

## Completion notes

**The audit.** A script listed every reference from one ticket to
another (4,026 in all). Agents then read each open ticket against every
ticket it names (86 batches), and each done ticket against the open
tickets it names (19 batches). The new lint added the 260 one-way
clauses it found, and the 2 clauses that wait on a done ticket in the
new "waits for" form. That gave 1,253 findings. A second agent re-read
both tickets of each one and planned the edits. A third tried to
refute each change to a `blocked_by` or to parking. It refuted 5, and
those got a clause on both sides instead.

1,242 findings held up. Many are the same problem seen from both
tickets. By kind:

- 447 handoffs written on one side only (1134's case, and the done
  tickets' handoffs that were already lost: 3002's crank as an axis
  for 1121, 3001's provisional budgets for 3008, 3038's keymap button
  tables for 2504, 2372's module screens for 2367, …);
- 260 one-way clauses (the lint's list);
- 115 contradictions between two tickets (names, flags, owners,
  order);
- 87 others;
- 82 missing blockers;
- 69 promised follow-ups with no ticket;
- 54 clauses about done tickets;
- 45 pieces each ticket handed to the other;
- 42 open tickets building on what a done ticket planned rather than
  what it built (2501 and 2009 on `--template`, 3011 and 3013 on
  profile statuses, 3009 on where `Think` is implemented, …);
- 29 wrong ids or descriptions;
- 8 pieces claimed by two tickets;
- 2 blockers that weren't;
- 2 milestone remarks.

**The edits.** About 1,700 edits to 311 open tickets, counting the
ones that point promises at the new follow-ups. Done tickets are
history and weren't edited: their handoffs are now written in the open
tickets that inherit them, as "**From NNNN (done):** …". Most missing
blockers became a clause on both sides. 18 `blocked_by` lists changed:

- 1123 and 1124 no longer wait on 1119;
- 1124 no longer waits on 2207;
- 2507 waits on 2220, not 1124;
- new blockers: 2325 (2321), 2327 (2220), 2360 (2335), 2371 (3053),
  2506 (2907), 2725 (2723), 2729 (2405), 2740 (3039), 2754 (2747),
  2760 (2757), 3010 (3007), 3011 (3028), 3020 (3017), 3032 (2513),
  3107 (2220, 2405).

No ticket that isn't parked waits on a parked one.

**Follow-ups written now** (write-ticket: a follow-up you can name now
is written now), each one promised by a ticket that had no id for it:
2028, 2029, 2134, 2135, 2136, 2234, 2235, 2236, 2373–2382, 2409,
2549–2552, 2769–2772, 2811, 3057–3062 and 3112. The tickets that
promised them now point at them.

**Not settled, and why.**

- 11 findings were judged not real. In most, the scenario can't
  happen: the other ticket is always done first through a chain of
  blockers. In a few, a split or a promise is left to the PR that
  makes it, by `work-ticket` §5 and §6.
- 2005 and 2808 say MVP while some of their blockers say "Engine:
  public". `docs/ROADMAP.md` lists them all in phase 2 in a workable
  order, so this is a label, not a wrong order. I left the labels as
  they are.
- 1135's milestone ("Engine: public", while it is parked research
  that the editor waits for) was left alone too.
- 1148's server-side Steam check will need a Steam Web API publisher
  key, which its *Nick input* doesn't list yet. 1148 is parked, so
  that question waits until it is unparked.

**The check.** `find_clauses` reads `waits for <ids>` and `wait for
<ids>`. A new error, `check_mirrors`, fails a clause in an open ticket
that names an open ticket outside its `blocked_by` whose body never
names it back. `named_ids` reads a four-digit word, so "1134's" counts,
while `ADR-1134`, `game#1134` and `v1134` don't.

No public API changed: `xtask` is not published.
