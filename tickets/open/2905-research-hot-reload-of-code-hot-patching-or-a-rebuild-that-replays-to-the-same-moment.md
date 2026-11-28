---
id: "2905"
title: "Research: hot reload of a game's code: patching the running binary, or a rebuild that restarts and replays to the same moment"
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2220", "2518", "2519"]
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
deterministic (ADR-0002), and a session record (2518) holds every input
that reaches it at the platform boundary, which debug builds write
(2519). So a rebuild can restart the game and replay the session's
inputs to the moment the developer was at, with the new code. That may
be good enough, works on every platform the game builds for, and needs
no unsafe patching.

2357 builds the replay to a frame and the hand-over to the device that
this needs, for going back in the debug view.

This ticket compares the ways and writes the ADR and tickets.

## Nick input

None. This is technical.

## Scope

**In:**
- **The options**, each tried on Hello, Grid and the tutorial's game
  (2803, `site/tutorial/10-ship`):
  1. **Rebuild and replay:** `shuyi run --watch-code` rebuilds on a
     change to `src/`, starts the new binary, and replays the session
     record so far (2518's `SessionRecord`, which the debug build
     writes, 2519) through 2518's `session::Playback` from the game's
     real `Start`. The replay to a frame and the hand-over to the
     device are 2357's: `App`'s rewind, a headless replay checked by
     hash, then the swap with the clock restarted, sound restarted and
     held input forgotten. If 2357 is done, the prototype reuses them.
     If not, the ADR says to reuse them rather than build a second
     replay. Measure: time from save to the same frame; what happens
     when the new code makes the record diverge (`Playback` stops at
     the first frame whose hash differs: say so). **From 2009
     (done):** measure with the setup the template has from it (no
     debug info in `dev`, Rust's own linker; no `dynamic` feature, which
     made no rebuild faster, ADR-0049), and `cargo xtask compile-times`
     for the rebuild it waits on. **From 3414:** the MCP session
     records a fingerprint of the game's code at `start` and marks a
     later answer `stale: true` with a limit when it changes. Say
     whether option 1 also serves the MCP session (rebuild the host and
     replay the session's steps headless when an answer would be
     stale), and if kept, write that ticket.
  2. **Hot patching with `subsecond`:** license (check on the day),
     what it needs (its CLI, a nightly or stable toolchain), whether it
     works on Windows with the GNU toolchain (`AGENTS.md`,
     *Environment*), on the web build, and with our `no_std` crates.
  3. **A dylib of the game's rules** reloaded at run time (as
     `hot-lib-reloader` does): what state survives, and whether the
     GNU toolchain links it.
  4. **Scripts** (2101): rules in the script runtime could reload as
     assets with 2220 and need no rebuild at all. 2145 builds that: a
     script `AssetKind` and `Format` in `shuyi-script`, and a
     `HotScript` that recompiles after each reload. Say where it
     covers the need. **Either order with 2145:** if 2145 is done,
     option 4 measures its `HotScript` reload (time from save to the
     new rule, on Hello, Grid or 2102's example) and writes no new
     ticket for scripts; if not, option 4 answers from 2145's text,
     and the ADR names 2145 as the ticket for script reload instead of
     writing one.
- For each: what the developer does, the time to see a change, what
  breaks it, the platforms, and the cost in our code.
- **Either order with 2528:** `shuyi web --serve` rebuilds the web
  build on a change and reloads the page, and the game starts over
  (2528's Out leaves keeping its state to this ticket). Say whether
  option 1 works there: the page keeps no record between loads
  (2519's web records are a download), so what would hold the inputs
  across the reload, and the time to the same frame. If 2528 is done,
  prototype it on `--serve`. If not, answer it from 2528's text, and
  the ticket this research writes for the web is blocked by 2528.
- **Writes:** an ADR with the choice (more than one may be kept: replay
  as the default, patching as an option), `docs/research/code-reload.md`
  with the numbers, and the tickets that build it.

- **From 2916 (done):** the code pad is a Codespace of the book's
  project (2837), where the reader rebuilds the step's web build
  ([ADR-0126](../../docs/adr/0126-the-code-pad-a-codespace-of-the-book-not-a-build-server.md),
  `docs/research/2916-code-pad.md`, *4. The reader's own machine*: a
  `dev` rebuild of step 2 takes 1.3 s). Say in this ticket's report
  whether the chosen reload changes that recommendation, and what a
  reader in that Codespace would see after a rebuild.
- **From 2269 (done):** a game on the short path under `run --watch`
  reloads its own part (a map, dialogue) by starting over at its first
  screen on the same context (`Ctx::restart`, `BasicReload`,
  [ADR-0164](../../docs/adr/0164-the-short-path-reloads-its-own-files-and-restarts-on-a-new-own-part.md)),
  so the player loses their place in the stage. Say in this ticket's
  report whether the rebuild-and-replay keeps that place for a data
  reload too (replay the session to the same moment on the new own
  part), and what it would take.

**Out (do not do):**
- building it beyond the prototypes the measurements need.

## Implementation steps

1. Read 2518's `SessionRecord` and `Playback`, 2519's debug-build
   records, and 2357's rewind if it is done. Prototype option 1 in a
   branch on them, measuring it.
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
