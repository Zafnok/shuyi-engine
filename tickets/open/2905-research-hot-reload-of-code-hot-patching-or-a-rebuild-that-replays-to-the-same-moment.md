---
id: "2905"
title: "Research: hot reload of a game's code: patching the running binary, or a rebuild that restarts and replays to the same moment"
type: research
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2220"]
nick_input: none
completed:
---

# 2905 — Research: hot reload of code

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): "hot reload",
as on Bevy's homepage. Bevy's homepage promises it for assets, which
2220 does. Developers also want it for code: change a rule, see it in
the running game. Bevy now has experimental code hot patching through
`subsecond` (from Dioxus).

Shuyi has something most engines don't: the whole game is
deterministic and its inputs are recorded (ADR-0002; replays, the
harness). So a rebuild can restart the game and replay the session's
inputs to the moment the developer was at, with the new code. That may
be good enough, works on every platform the game builds for, and needs
no unsafe patching.

This ticket compares the ways and writes the ADR and tickets.

## Nick input

None. This is technical.

## Scope

**In:**
- **The options**, each tried on Hello, Grid and the tutorial's game
  (2803, if done):
  1. **Rebuild and replay:** `shuyi run --watch-code` rebuilds on a
     change to `src/`, starts the new binary, and replays the record so
     far. Measure: time from save to the same frame, with 2009's
     setup; what happens when the new code makes the record diverge
     (stop at the first step whose hash differs, and say so).
  2. **Hot patching with `subsecond`:** license (check on the day),
     what it needs (its CLI, a nightly or stable toolchain), whether it
     works on Windows with the GNU toolchain (`AGENTS.md`,
     *Environment*), on the web build, and with our `no_std` crates.
  3. **A dylib of the game's rules** reloaded at run time (as
     `hot-lib-reloader` does): what state survives, and whether the
     GNU toolchain links it.
  4. **Scripts** (2101): rules in the script runtime reload as assets
     with 2220 and need no rebuild at all. Note where this already
     covers the need.
- For each: what the developer does, the time to see a change, what
  breaks it, the platforms, and the cost in our code.
- **Writes:** an ADR with the choice (more than one may be kept: replay
  as the default, patching as an option), `docs/research/code-reload.md`
  with the numbers, and the tickets that build it.

**Out (do not do):**
- building it beyond the prototypes the measurements need.

## Implementation steps

1. Prototype option 1 in a branch, measuring it.
2. Try options 2 and 3 on Hello, Grid on Windows GNU and Linux.
3. Write the notes, the ADR and the tickets.

## Acceptance criteria

- [ ] Each option has its numbers and limits in
      `docs/research/code-reload.md`.
- [ ] The ADR names the choice, and its tickets are in `tickets/open/`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the prototypes'; the tickets it writes carry the tests.

## Completion notes

*(Filled in by the session that completes the ticket.)*
