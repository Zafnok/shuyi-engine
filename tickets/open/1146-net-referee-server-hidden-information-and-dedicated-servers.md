---
id: "1146"
title: "Parked: Networking, step 7: a referee server for turn-based games — hidden seed and fog, per-player views, listen or dedicated"
type: feature
milestone: "Engine: networking"
model: reasoning
effort: xhigh
status: blocked
blocked_by: ["1145"]
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
- `shuyi-server` is a headless binary (tokio, no macroquad) that runs
  referee matches on the relay protocol. A listen-server mode runs the
  same session inside a game process.
- A result record (seed, commands, outcome, hash) that a checker can
  re-simulate (`shuyi replay`) to verify.
- Toy game: a fog-of-war variant of the 1116 toy game, or "battleship".
  Tests show a client never receives hidden cells and that a forged
  command is refused.
- A Docker image and docs.
- **Either order with 2907:** if 2907 is done, `PlayerView` follows its
  ADR's rule for state and view (its point 4), which may change what
  `View` is. If not, nothing to do: 2907's ADR says how its rule fits
  `PlayerView` as written here.

**Out (do not do):**
- accounts and ratings (1148);
- real-time server authority (1147);
- our game's fog (0053 decides if there is any).

## Implementation steps

1. Write the view trait and the referee session, with tests over
   `MemoryNetwork`.
2. Build the server binary and the listen mode.
3. Write the result record and its checker test.

## Acceptance criteria

- [ ] The tests prove no hidden information is sent to the client
      (inspect every message in the in-memory network).
- [ ] A forged command is refused, and a re-simulated result matches.
- [ ] The Docker image builds.
- [ ] All gates pass.

## Tests required

- Integration: referee matches, hidden info, forgery, result checking.

## Completion notes
