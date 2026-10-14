---
name: write-ticket
description: Create a new ticket file in tickets/open/ using the repository template — for follow-ups found while working, bugs, or new features. Use whenever work is discovered that is outside the current ticket's scope, or when Nick or a user reports a problem.
---

# Write a ticket

Tickets must be good enough that a weaker model (Sonnet) can complete
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
     determinism fixture).
5. **Keep it small:** one PR's worth, roughly 600 changed lines or fewer
   excluding fixtures and snapshots. Split otherwise.
6. **Check the dependency chain** (below).

## Check the dependency chain

A ticket is picked up when every id in `blocked_by` is done, so a
missing id means a session starts work it can't finish.

- **Each open ticket the text names** is one of three things, and the
  ticket says which:
  - *needed first*: it goes in `blocked_by`;
  - *either order works*: the text says what to do in each case;
  - *not this ticket's job*: it goes under **Out**.
- **Look the other way too.** Search `tickets/open/` for tickets this one
  unblocks or makes wrong, and update them in the same PR.
- **Game tickets** go in `blocked_by` as `"game#NNNN"`. If the engine
  change needs a game change, write that ticket in the game repository
  too, and link both ways.
- **Parked work.** A follow-up of a parked ticket is parked too: give it
  the `Parked:` title prefix and the note.
- Run `cargo xtask ticket-lint` (after 2002 lands).
