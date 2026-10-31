---
id: "1140"
title: "Parked: Networking, step 1: the Transport trait, an in-memory network with seeded faults, and lockstep over TurnRules"
type: feature
milestone: "Engine: networking"
model: reasoning
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
- A new crate `crates/net` (package `shuyi-net`). Without features it
  depends on `shuyi-core` and serde only; the fuzz driver and the debug
  channel sit behind features of their own (below). No sockets in this
  ticket.
- **Transport:** `trait Transport` with:
  - `PeerId`;
  - `send(peer, channel, bytes)`;
  - `poll() -> Vec<NetEvent>` (Connected, Disconnected, Message);
  - channels `Reliable` (ordered) and `Unreliable`;
  - `stats()`.
  `MemoryNetwork` hands out connected `MemoryTransport`s. Its seeded
  faults are loss, delay in steps, reordering and duplicates, and they
  only affect the unreliable channel. The reliable channel is delayed,
  never lost. The faults are a plain struct of whole numbers (`seed`;
  `loss`, `reorder` and `duplicate` in percent; `delay` in steps) that
  can be replaced between steps. The network moves on one step at a
  time by an explicit call (for example `MemoryNetwork::step`), and
  `stats()` returns a plain struct of whole numbers. The contract's
  `peers` operation (2529) steps the network once per frame, changes
  the faults mid-session and copies the stats into its answer.
- **Lockstep:** `LockstepSession<R: TurnRules + Serialize>` has:
  - a handshake: each peer's `Made` from 2111, which holds the engine
    version, the game's stamp and `Bundle::fingerprint()`, compared
    with `Made::check` so a refusal names what differs; and the seed
    (`shuyi_core::Made`, done in 2111);
  - who may command when (the game says whose turn it is);
  - commands sent and applied in the same order on both sides;
  - a state hash before the first command and after each one, by
    `shuyi_core::state_hash` (ADR-0022 §4), the form `replay::turns`
    already gives, kept by the session and readable from it (2529
    reads them);
  - desync detection;
  - reconnect by replaying the history to a rejoining peer.
- **Commit-reveal (opt-in):** a helper for rolls that both peers
  contribute to, with a forfeit rule for a peer who doesn't reveal.
- Tests:
  - two toy games (tic-tac-toe from 1116) play over `MemoryNetwork`
    with faults and end identical;
  - a forced desync is reported with the first differing command;
  - a peer that disconnects and reconnects catches up;
  - **Either order with 2527:** if `shuyi_core::soak` exists, the toy
    game used here first passes `soak::play` over 20 seeds: the check
    on one machine that a game rebuilt from its saved history, as a
    reconnecting peer's is, plays on as the one never rebuilt does. If
    2527 isn't done, skip it.
- **Fuzzing the messages** (2020's harness): a driver in `shuyi-net`
  behind a `fuzz` feature, as `shuyi_assets::fuzz` does it. It decodes
  random bytes as every message the session reads from a peer: the
  handshake, a command, a state hash, the reconnect history. Add a
  target `fuzz/fuzz_targets/net_messages.rs` (`fuzz/Cargo.toml` gains
  `shuyi-net` by path, with `fuzz`) and a job leg in
  `.github/workflows/fuzz.yml` (`changes.yml`'s `assets` output counts
  `crates/net/`), and keep crash inputs in
  `crates/net/tests/fuzz-regressions/`, replayed by a stable test.
  **Either order with 2020:** if 2020 isn't done, write a property test
  over random bytes per message instead, and a ticket to add the target
  once 2020 lands.

- Module duties (ADR-0009 §4): the metadata (concern: online play), the
  README in its fixed order, the `net` feature of the `shuyi` crate
  (2003), the catalog row, and a `net` channel in the debug view
  (2315). The channel draws, so it sits behind `shuyi-net`'s own
  `debug` feature (an optional `shuyi-ui`). It shows each peer's round
  trip, loss and bytes a second per channel from `stats()`, the
  lockstep turn, whether the peers' hashes agree, and a desync banner
  (a `Note` label). **Either order with 2315:** if 2315 is done, add
  the channel here; if not, write a parked follow-up ticket for it.
- **From 1134 (done):** drop 1140 from the `waits_for` of
  `skills/shuyi-net/SKILL.md` (`check-skills` fails on an id no longer
  in `tickets/open/`), and make the skill teach what this ticket built:
  `Transport`, `MemoryNetwork` and its faults, `LockstepSession` (the
  handshake, desync, reconnect) and commit-reveal, naming `docs/net.md`
  in words (a game reaches the engine's docs through its `AGENTS.md`,
  and `check-skills` fails a link that leaves the pack). Its worked
  example is not this ticket's: the later of 2529 and 1145 writes it.
- **From 2116 (done):** add `-p shuyi-net` to the wasm32 `cargo test`
  step of CI's `determinism` job, so the lockstep and message tests
  also run on wasm32, where a browser peer (1142) runs. A test that
  needs threads or unwinding is ignored there, as in 2116's crates:
  `#[cfg_attr(target_os = "wasi", ignore = "…")]`.

**Out (do not do):**
- real sockets (1141, 1143); servers (1141);
- anything in our game (0053);
- lockstep over a tick game driven by commands, such as a real-time
  strategy game (the parked 2720, ADR-0015 §2). Keep the lockstep
  driver's turn exchange separate from `TurnRules`, so 2720 can reuse
  it for command turns;
- the tool contract driving several peers (`shuyi peers`, MCP tools per
  peer): 2529, over this ticket's `MemoryNetwork` and
  `LockstepSession`;
- building the check that machines of different kinds (x64, ARM64,
  wasm32) give the same hashes: 2116 built it, in CI's `test` job
  (x64, ARM64) and its `determinism` job (wasm32), and `shuyi-net`
  joins it by the line from 2116 above. A lockstep test here runs
  both peers in one process;
- voice chat (2910 researches it). **Either order with 2910:** if 2910
  is done, read its ADR and do any line it added to this scope for
  voice over the `Unreliable` channel; if not, nothing to do (2910
  builds on this ticket's `Transport`).

## Implementation steps

1. Write the transport trait and the in-memory network, with property
   tests: reliable messages arrive once and in order under any fault
   seed.
2. Write the lockstep session and its tests, with the soak if 2527 is
   done.
3. Write commit-reveal, with tests including the forfeit.
4. Write the messages' fuzz driver, target and kept inputs, or, if 2020
   isn't done, the property test and the follow-up ticket.
5. Add the `net` debug channel if 2315 is done, or write its parked
   follow-up ticket.
6. Add a docs page in `docs/net.md`. Lockstep between peers is
   for friends (ADR-0006 §4).
7. Update `skills/shuyi-net` (Scope).

## Acceptance criteria

- [ ] The property and integration tests above pass.
- [ ] A Harness test plays two copies of `hello-grid`'s `Stage` (a
      `TurnRules`) over `MemoryNetwork` with scripted commands and ends
      with equal hashes.
- [ ] Every file in `crates/net/tests/fuzz-regressions/` runs in `cargo
      test`, and `fuzz.yml` has the `net_messages` leg; or 2020 isn't
      done, the property test over random bytes passes, and the
      follow-up ticket is in `tickets/open/`.
- [ ] Either the debug view has the `net` channel, or 2315 isn't done
      and its parked follow-up ticket is in `tickets/open/`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: transport guarantees.
- Integration: toy and `hello-grid` lockstep, desync, reconnect.
- Fuzz: the kept inputs of the messages' driver, replayed on stable
  (or, before 2020, the property test over random bytes).

## Completion notes
