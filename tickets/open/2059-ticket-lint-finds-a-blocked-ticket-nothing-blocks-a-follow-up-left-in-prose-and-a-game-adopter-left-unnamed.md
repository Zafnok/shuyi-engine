---
id: "2059"
title: "Ticket lint finds a blocked ticket nothing blocks, a follow-up left in prose, and a game adopter left unnamed"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2059 — Ticket lint finds a blocked ticket nothing blocks, a follow-up left in prose, and a game adopter left unnamed

## Context

The retro of engine PRs #254–#313 (2026-11-08) found three things the
ticket lint (`crates/xtask/src/tickets.rs`) doesn't check, each of
which let work go missing.

- **`status: blocked` with every blocker done.** 2397, 3032 and 3082 had
  this at the retro; 2397 and 3032 had been unparked. The retro's PR set
  them to `todo`. 2005's `blocked_by` had gone stale the other way: it
  read as ready while five MVP tickets were open, until #281 fixed it.
- **A follow-up promised in prose and never ticketed** (`AGENTS.md`
  rule 2):
  - #296's "if it comes back, it is worth a ticket", the Windows
    `cargo-semver-checks` crash, now 2061;
  - #267's messages that say "characters" where they count cells, now
    2257;
  - #284's "the live link and capture plans refuse it for now", now 2583;
  - #304's stale tutorial `Cargo.lock` files, now 2062;
  - #302's flaky MCP forwarding test, now 2063.
  
  By contrast, 2577 was ticketed when 2914 hit it and fixed in #311.
- **A done ticket that names `game#NNNN` but not the game PR or ticket
  that moves the pin** (`work-ticket` §5.2). This was missing in 5 of 17
  sampled tickets: 2315, 3046, 2226, 2350 and 2391. 3046 and 3209 don't
  link back to the game's 1245 and 1252.

## Nick input

None.

## Scope

**In:**
- An error for a ticket that isn't parked, is `status: blocked`, and has
  every `blocked_by` id done (this repository's in `done/`, a `game#`
  id in the game's `done/` when the lint can read it).
- A warning for a phrase in a done ticket's `## Completion notes` that
  promises later work with no four-digit ticket id on the same line or
  bullet: "worth a ticket", "for now", "follow-up", "later", "not yet",
  "out of scope", "TODO". The warning names the line. A `No ticket:`
  prefix with a reason silences it.
- An error for a ticket moved to `done/` by the PR whose body names
  `game#NNNN` with no line in its completion notes naming the game PR or
  game ticket that adopts it, or `Game: nothing to adopt` with a reason.

**Out (do not do):**
- The game's lint (game#0147 does the game's half).
- Reading prose follow-ups in open tickets.

## Implementation steps

1. Add `check_blocked_with_done_blockers` beside the existing
   `blocked_by` checks. It reuses the game-id reading the lint already
   does for `game#` blockers.
2. Add `check_prose_follow_ups(body)` over `## Completion notes`, run
   only for tickets the PR moves to `done/` (the PR's ticket as
   `tickets::pr` reads it from `--pr-title` and `--pr-branch`), so the
   old done tickets don't all warn.
3. Add `check_game_adopter` for the same set.
4. Update `work-ticket` §5 to say what each check wants, then run
   `pwsh -File scripts/sync-agent-skills.ps1`.

## Acceptance criteria

- [ ] An open non-parked ticket `status: blocked` whose blockers are all
      in `done/` fails, naming them.
- [ ] A ticket moved to `done/` whose notes say "worth a ticket if it
      comes back" with no id warns; with `No ticket: …` it doesn't.
- [ ] A ticket moved to `done/` naming `game#1245` with no game PR or
      ticket line fails.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each check over small ticket strings, both ways.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
