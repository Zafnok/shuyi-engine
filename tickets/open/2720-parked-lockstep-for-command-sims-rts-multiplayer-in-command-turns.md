---
id: "2720"
title: "Parked: Lockstep for command sims: RTS multiplayer in command turns"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["2706", "1140"]
nick_input: none
completed:
---

# 2720 — Lockstep for command sims: RTS multiplayer in command turns

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

This ticket belongs to two parked series: the real-time strategy module
(2705–2720) and networking (1140–1150). It waits for both: Nick unparks
it with the RTS series, and it needs 1140 from the networking one.

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§2, last bullet, and
[ADR-0006](../../docs/adr/0006-networking-in-layers-transports-sessions-sync-models-services.md)
§2: lockstep with timed commands is how RTS multiplayer has worked since
the 1990s. Each peer runs the same deterministic simulation and sends
only commands, scheduled a few steps ahead:

- peers exchange the commands for a **command turn** of N steps (default
  4);
- commands are scheduled a fixed **delay** ahead (default 2 turns, 8
  steps, about 133 ms at 60 steps a second);
- peers compare **hashes** each turn;
- a desync stops the match and both sides keep the record; a peer that
  drops out reconnects by replaying the record.

1140 built the `Transport` trait, the in-memory network with seeded
faults, and lockstep over `TurnRules`. Its transport and handshake are
reused as they are. Its `LockstepSession<R: TurnRules>` is for turn
games, so this ticket adds a session for `CommandSim` (2706) beside it.
`shuyi-net` depends on `shuyi-core` only, never on `shuyi-rts`.

Lockstep is for friends: every peer has the whole state (ADR-0006 §4).

## Nick input

None.

## Scope

**In:**
- **`shuyi_net::lockstep::CommandLockstep<S: CommandSim>`**:
  - settings: `steps_per_turn` (default 4) and `delay_turns` (default 2);
  - 1140's handshake (the peers' `Made` stamps from 2111, and the
    seed), plus the two settings, which must match. The handshake
    carries each peer's `Made` (`shuyi_core::Made`, done in 2111);
  - **sending:** a command issued during turn `t` is scheduled for the
    first step of turn `t + delay_turns`. Each peer sends one
    `TurnPacket { turn, player, commands, hash }` per turn on the
    reliable channel, even with no commands;
  - **stepping:** the steps of turn `t` run only when the packets of
    every peer for turn `t` have arrived. Until then, the driver steps
    nothing and the screen keeps drawing. The commands of a turn are
    applied in `CommandRecord` order (step, player, sent order);
  - **hashes:** each packet carries the hash after the last step of
    turn `t − delay_turns`. A mismatch is a desync: the session stops,
    both peers keep their `CommandRecord` and hash lists, and the error
    names the first turn whose hashes differ;
  - **reconnect:** a peer that comes back gets the seed and the record
    so far, replays it (as many steps a frame as it can, outside the
    frame budget), and rejoins at the next turn that hasn't been sent.
- **The driver:** `CommandDriver` (2706) gets a gate: it asks the
  session whether the next step may run, and `issue` goes through the
  session instead of the local queue. While the session drives the
  steps, a peer's `Ctx::game_speed` (2123) only makes it wait: the gate
  sets the pace. **Either order with 2123:** if it is done, add a test
  that a peer at `GameSpeed::DOUBLE` ends with the same hash list as one
  at `NORMAL`, at the slower peer's pace; if not, nothing to do here
  (2123 adds the test).
- **Tests over `MemoryNetwork`** (1140), with seeded loss, delay,
  reordering and duplicates on the unreliable channel, and delay on the
  reliable one:
  - two and four peers running 2706's `Beacons` with random commands end
    with the same hash list;
  - **either order with 2707:** if 2707 is done, the same with an
    `RtsSim` fixture; if not, add to 2707's scope in this PR: "run an
    `RtsSim` fixture through 2720's lockstep tests";
  - a forced desync is reported at the right turn, and both records are
    kept;
  - a peer that drops for 200 steps and reconnects catches up and ends
    identical;
  - a command issued at turn `t` is applied at turn `t + delay_turns`,
    never earlier;
  - no peer runs a step of a turn before it has every packet for it.
- **`docs/net.md`**: a section on lockstep for command sims, its two
  settings, what the delay costs a player, and that it is for friends.
- **The README of `shuyi-net`** lists the new session.

**Out (do not do):**
- real sockets, relays and the browser (1141, 1142, 1143);
- sessions and lobbies (1145);
- hidden information and a referee server (1146): lockstep shows every
  peer the whole state;
- rollback (1147);
- the RTS view's network status display (a later ticket if wanted);
- a game speed every peer shares as a session setting (a follow-up if a
  game asks);
- checking that machines of different kinds (x64, ARM64, wasm32) give
  the same hash lists: 2116 does it for the engine's crates, in CI's
  `test` job (x64, ARM64) and its `determinism` job (wasm32), and
  `shuyi-rts` joins the wasm32 run (2707). The tests here run every peer
  in one process.

## Implementation steps

1. Write `TurnPacket` and its serde, with a format version.
2. Write `CommandLockstep`: handshake, sending, stepping, hashes.
3. Add the gate to `CommandDriver`.
4. Write desync detection and reconnect by replay.
5. Write the tests over `MemoryNetwork`.
6. Write the docs.

## Acceptance criteria

- [ ] Two and four peers with random commands and random fault seeds end
      with identical hash lists (property test).
- [ ] A forced desync is reported with the first differing turn, and
      both records are kept (integration test).
- [ ] A peer that drops for 200 steps reconnects and ends identical
      (integration test).
- [ ] A command issued at turn `t` is applied at turn `t + delay_turns`
      (property test).
- [ ] No step of a turn runs before every peer's packet for it arrived
      (property test with the network's delay at its maximum).
- [ ] The handshake refuses peers with different settings.
- [ ] If 2123 is done: a peer at `GameSpeed::DOUBLE` and one at `NORMAL`
      end with the same hash list, at the `NORMAL` peer's pace
      (integration test).
- [ ] `shuyi-net` doesn't depend on `shuyi-rts` (`cargo tree`).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `TurnPacket` serde; turn and step arithmetic; the handshake.
- Property: identical runs under faults; the delay; waiting for packets.
- Integration: desync and reconnect.

## Completion notes
