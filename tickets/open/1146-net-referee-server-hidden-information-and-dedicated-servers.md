---
id: "1146"
title: "Parked: Networking, step 7: a referee server for turn-based games — hidden seed and fog, per-player views, listen or dedicated"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["1145", "2616"]
nick_input: none
completed:
---

# 1146 — Referee server: hidden information; listen or dedicated

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §3–§4. In lockstep between peers, both clients hold everything,
the seed included. A modified client can preview rolls and see through
fog. Competitive play needs a **referee**: a server that holds the seed
and the full state, checks every command, applies it, and sends each
player only what that player may see. The referee is the same
simulation code in a headless binary, because `shuyi-core` has no I/O.
The same code runs inside a player's game as a listen server.

Nick's decision of 2026-11-25 (`docs/decisions.md`) unparked an online
real-time example, trails, under ADR-0150. Its work creates the crate
this ticket had planned: 2616's `shuyi-server` is a headless library
(tokio, never macroquad) that runs a director's tick rooms over
WebSocket, with optional TLS (rustls on ring), limits on connections,
addresses and messages, `/healthz`, `/stats`, a cost guard and a
graceful shutdown. A game owns its binary and calls
`shuyi_server::run`. This ticket stays parked, and adds the referee as
a second room kind of that crate instead of creating a binary.
Hosting is ADR-0151's (2618), and the book's hosting page is 2856's.

## Nick input

None.

## Scope

**In:**
- `trait PlayerView: TurnRules { type View; fn view_for(&self, player)
  -> Self::View; }`. The game decides what each player sees, and events
  are filtered per player the same way (`fn events_for`).
- `RefereeSession<R: PlayerView>` holds the state, accepts commands from
  the player whose turn it is, rejects illegal ones with the rules'
  error, and broadcasts per-player views and events. It records the full
  history for replay.
- **A referee room kind in 2616's `shuyi-server`**, for example
  `shuyi_server::run_referee<R: PlayerView>(config, …)`, reusing its
  WebSocket listener, TLS, limits, health, stats and signals; the
  referee's session work stays pure in `shuyi-net`, as 2614's rooms
  are. The game owns the binary, as 2789 does for trails. A
  listen-server mode runs the same session inside a game process.
- A result record (seed, commands, outcome, hash) that a checker can
  re-simulate (`shuyi replay`) to verify.
- Toy game: a fog-of-war variant of the 1116 toy game, or "battleship".
  Tests show a client never receives hidden cells and that a forged
  command is refused.
- Docs: what a referee game adds to hosting, where the docs map
  (`docs/README.md`) puts hosting a server (2856's book page, from
  2618's deployment: systemd on a free host, no Docker image needed).
- **From 2907 (done):** `PlayerView` follows ADR-0084 §4. `type View`
  is a type of the game's own, `Serialize` (and `JsonSchema` in the
  tools), and what a player can't see is left out of it, never hidden
  with serde attributes on the state, which would change its hash.

**Out (do not do):**
- accounts and ratings (1148);
- real-time server authority: tick rooms are 2614–2616 (ADR-0150), and
  rollback and prediction stay 1147's;
- our game's fog (0053 decides if there is any).

**Either order with 2606:** if 2606 is done, `shuyi-server` takes its
`Accepts` list at start, and its docs say one process runs per build
during a rollout; if not, nothing to do (2606 adds it).

## Implementation steps

1. Write the view trait and the referee session, with tests over
   `MemoryNetwork`.
2. Add the referee room kind to `shuyi-server` and the listen mode.
3. Write the result record and its checker test.

## Acceptance criteria

- [ ] The tests prove no hidden information is sent to the client
      (inspect every message in the in-memory network).
- [ ] A forged command is refused, and a re-simulated result matches.
- [ ] Two clients play the toy game through `shuyi-server`'s referee room
      kind on `127.0.0.1:0`, and the server's limits and `/healthz` work
      as they do for tick rooms.
- [ ] All gates pass.

## Tests required

- Integration: referee matches, hidden info, forgery, result checking.

## Completion notes
