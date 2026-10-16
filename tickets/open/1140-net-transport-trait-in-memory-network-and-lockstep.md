---
id: "1140"
title: "Parked: Networking, step 1: the Transport trait, an in-memory network with seeded faults, and lockstep over TurnRules"
type: feature
milestone: "Engine: networking"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["game#1131"]
nick_input: none
completed:
---

# 1140 — Transport, an in-memory network, lockstep

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §1–§2 and §10. This is the first, cheap piece of networking.
It needs no real network and no server. It proves the determinism
contract: two copies of a turn-based game stay identical by sending only
commands, detect a desync by hash, and reconnect by replaying history.

It needs `TurnRules` (the game's ticket 1114) in this repository (the game's ticket 1131).

## Nick input

None (technical). Scheduling: see above.

## Scope

**In:**
- A new crate `crates/net` (package `shuyi-net`). It depends on
  `shuyi-core` and serde only. No sockets in this ticket.
- **Transport:** `trait Transport` with:
  - `PeerId`;
  - `send(peer, channel, bytes)`;
  - `poll() -> Vec<NetEvent>` (Connected, Disconnected, Message);
  - channels `Reliable` (ordered) and `Unreliable`;
  - `stats()`.
  `MemoryNetwork` hands out connected `MemoryTransport`s. Its seeded
  faults are loss, delay in steps, reordering and duplicates, and they
  only affect the unreliable channel. The reliable channel is delayed,
  never lost.
- **Lockstep:** `LockstepSession<R: TurnRules>` has:
  - a handshake (engine version, a content hash the game supplies, the
    seed);
  - who may command when (the game says whose turn it is);
  - commands sent and applied in the same order on both sides;
  - a state hash after each command (`TurnRules` gains `fn hash(&self)
    -> u64`, default unimplemented, implemented by the toy game and by
    `BattleState`);
  - desync detection;
  - reconnect by replaying the history to a rejoining peer.
- **Commit-reveal (opt-in):** a helper for rolls that both peers
  contribute to, with a forfeit rule for a peer who doesn't reveal.
- Tests:
  - two toy games (tic-tac-toe from 1116) play over `MemoryNetwork`
    with faults and end identical;
  - a forced desync is reported with the first differing command;
  - a peer that disconnects and reconnects catches up.

- Module duties (ADR-0009 §4): the metadata (concern: online play), the
  README in its fixed order, the `net` feature of the `shuyi` crate
  (2003), and the catalog row.

**Out (do not do):** real sockets (1141, 1143); servers (1141); anything
in our game (0053).

## Implementation steps

1. Write the transport trait and the in-memory network, with property
   tests: reliable messages arrive once and in order under any fault
   seed.
2. Write the lockstep session and its tests.
3. Write commit-reveal, with tests including the forfeit.
4. Add a docs page in `docs/net.md`. Lockstep between peers is
   for friends (ADR-0006 §4).

## Acceptance criteria

- [ ] The property and integration tests above pass.
- [ ] `BattleState` implements `hash`. A Harness test plays two copies
      of the test chapter's battle over `MemoryNetwork` with scripted
      commands and ends identical.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: transport guarantees.
- Integration: toy and battle lockstep, desync, reconnect.

## Completion notes
