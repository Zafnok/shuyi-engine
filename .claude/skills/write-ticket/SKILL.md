---
name: write-ticket
description: Create a new ticket file in tickets/open/ using the repository template — for follow-ups found while working, bugs, or new features. Use whenever work is discovered that is outside the current ticket's scope, or when Nick or a user reports a problem.
---

# Write a ticket

Tickets must be good enough that an implementation-capable model can complete
them without asking questions.

## Steps

1. **Pick the block** (`tickets/README.md`). New tickets use `20xx`
   onward by area; the moved `1119`–`1150` keep their numbers. Bugs go
   in the block of the area they are in.
2. **Pick the number:** the next unused number in that block, across
   both `tickets/open/` and `tickets/done/`:
   ```bash
   ls tickets/open tickets/done | grep -E '^22[0-9]{2}-' | sort | tail -1
   ```
3. Copy `tickets/TEMPLATE.md` to `tickets/open/NNNN-short-kebab-title.md`.
   Keep a bare `@word` out of the title.
4. **Fill in every section:**
   - **Model/effort:** choose a capability and relative effort from
     `tickets/README.md`, never a provider model id.
   - **Context:** why the ticket exists. Link the ADRs, the decisions,
     and the tickets it builds on (`game#NNNN` for the game's).
   - **Nick input:** `None.` / *Decision* / *Setup* (exact clicks) /
     *Sign-off*.
   - **Scope — in / out:** be explicit about what NOT to do.
   - **Implementation steps:** numbered and concrete, naming crates,
     files, types and functions.
   - **Acceptance criteria:** checkboxes, each objectively verifiable.
   - **Tests required.**
   - A new dependency names its license and how it was checked.
   - A new file format follows ADR-0007 (a `Format`, fuzz-style test,
     determinism fixture). The fixture is a row in its crate's golden
     table (`shuyi_assets::golden`, ticket 2116: the `state_hash` of what
     it decodes, named `<kind>: <format id>`), which CI's `determinism`
     job checks on wasm32 too.
   - A new public type follows ADR-0039 (2013): one that may grow is
     `#[non_exhaustive]` with a constructor, and a new field on such a
     type is additive. Say which in the Scope.
5. **Keep it small:** one PR's worth, about 1,500 changed lines or fewer
   excluding fixtures and snapshots. Split otherwise.
6. **Check the dependency chain** (below).

## A request from the game

Most engine work starts in the game: a game session found that it needs
the engine changed (the game's `engine-change` skill), often after an
engine-friction review (game ADR-0092). Such a ticket is written by a
session that knows the game well and the engine less, and worked by one
that may know only the engine. So it must stand on its own. Its
**Context** holds, in this order:

1. **The game ticket**, as `game#NNNN`, with a link
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/NNNN-….md`).
   The game ticket is not a blocker: don't put it in `blocked_by`
   unless the engine change really needs it done first.
2. **The job in one sentence**, as a player or a reader of the code
   would say it ("page a long line of text inside a panel with a
   picture beside it").
3. **The friction numbers**, copied from the row of the game's
   `docs/engine/friction.md`: the estimate, the measured lines, the
   ratio and the outcome. If the request didn't come from a friction
   review, say so ("no friction review: a missing feature").
4. **The generic shape**: the capability in the engine's words, never
   the game's. It names no class, unit, faction, chapter, character,
   story beat or game data id (`AGENTS.md` rule 4). "A text panel with a
   portrait slot and paged text", not "the dialogue box".
5. **An example from a game that isn't tactics** that would write the
   same code: a farming game's shopkeeper, a platformer's sign post.
   If you can't find one, it may belong in the tactics kit, or in the
   game; say which and why.

Its **Scope** says what changes for the game: whether the API breaks
(CI's `api` job finds it, and each break gets a `Breaking:` line in the
ticket's completion notes, ADR-0038 §6), and that the game's PR that
moves the pin follows this one. Its **Completion notes** will name that
game PR (`work-ticket` §5).

**A small change needs no ticket here.** A change of a few lines with
nothing to decide (a doc fix, a missing re-export, a typo in an error
message), made and merged by the same game session, uses the game's
ticket as its ticket: branch `game-NNNN-<slug>`, PR title `Game NNNN:
<what it does>`, and the PR body says the game ticket and that the
game's PR moving the pin follows it. Such a change never breaks the
API: the `api` job fails a break on a branch with no ticket of this
repository.

## Check the dependency chain

A ticket is picked up when every id in `blocked_by` is done, so a
missing id means a session starts work it can't finish.

- **Each open ticket the text names** is one of three things, and the
  ticket says which:
  - *needed first*: it goes in `blocked_by`;
  - *either order works*: the text says what to do in each case, in
    the words the ticket lint reads: "**Either order with NNNN:** if
    NNNN is done, …; if not, …". The PR that closes NNNN settles it
    (the `work-ticket` skill, §5);
  - *not this ticket's job*: it goes under **Out**.

  "Waits for NNNN", "until NNNN" and "once NNNN lands" are one of the
  first two: write a clause with both branches, or a blocker. Never
  leave them as prose.
- **Write every handoff on both sides, in the same PR.** When this
  ticket leaves a piece of work to another open ticket, that ticket must
  name this one and say what it does if this one lands first. A piece is
  left by the *if not* branch of a clause ("if not, NNNN adds it"), by a
  part that waits for NNNN, or by an **Out** line saying NNNN does it.
  Write the other side in the same words: "**Either order with MMMM:**
  if MMMM is done, add …; if not, nothing to do (MMMM does it)". When
  nothing is left either way, it still says so ("if not, nothing to
  do").

  Here is why. A handoff written on one side only is lost when that side
  lands first. Its PR moves it to `done/`, and the other ticket's PR
  greps `tickets/open` (`work-ticket` §5), so it never finds it. 1134
  once left its `shuyi-net` skill's worked example to 1140 and 1145, and
  neither of them said so.

  `cargo xtask ticket-lint` fails a clause in an open ticket whose other
  open ticket never names it back (2027). It can't read an **Out** line,
  so check those yourself: open the ticket and find the piece in its
  scope.
- **Look the other way too.** Search `tickets/open/` for tickets this one
  unblocks or makes wrong, and update them in the same PR.
- **Game tickets** go in `blocked_by` as `"game#NNNN"`. If the engine
  change needs a game change, write that ticket in the game repository
  too, and link both ways.
- **A follow-up you can name now, write now**, instead of promising one
  for later. It is parked if its work is parked, and its `blocked_by`
  holds what it waits for.
- **Parked work.** A follow-up of a parked ticket is parked too: give it
  the `Parked:` title prefix and the note.
- Run `cargo xtask ticket-lint`.
