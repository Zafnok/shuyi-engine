---
id: "1147"
title: "Parked: Networking, step 8: real-time play — rollback for tick games, snapshots for many players, and how the browser joins"
type: research
milestone: "Engine: networking"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["1125", "1145"]
nick_input: none
completed:
---

# 1147 — Real-time networking: rollback, snapshots, the browser

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §1 and §6. Real-time games need one of two sync models:

- **rollback**: predict the remote input; when the real input differs,
  load an older state and step again. Good for 2–4 players. GGRS is the
  Rust port of GGPO (MIT/Apache).
- **snapshots**: a server sends state and clients interpolate. Good for
  many players, and the base for an MMO (1150).

The web build's loader blocks most browser crates. GGRS on wasm pulls
`js-sys` and the unmaintained `instant`. WebRTC (matchbox) and
WebTransport need `wasm-bindgen`. naia has a macroquad feature.

This ticket has two parts. Part one is research and an ADR, with spikes
that are not committed. Part two is implementation tickets for:

- rollback over `TickSim`;
- a snapshot model;
- the browser's path for real-time traffic.

## Nick input

None.

## Scope

**In:**
1. **Rollback.** Spike GGRS with `shuyi-net`'s `Transport` and a
   `TickSim` (save and load as clone, which the arena makes cheap). Spike
   a small rollback of our own, about 600 lines: an input queue,
   prediction by repeating the last input, a resimulate window, and
   desync hashes every N steps. Compare them on licenses, the WASM build
   under our loader, code size, and test results under seeded faults.
2. **Snapshots.** Spike naia (it works with macroquad on the web; its
   server is native) against our own snapshots over `Transport`. Our own
   version is delta-compressed state per client, interpolation buffers,
   and client prediction for the local player.
3. **The browser.** Weigh three options:
   - more JS plugins (WebRTC data channels through a signaling server,
     plus STUN/TURN: coturn is BSD-3 and self-hosted, or a paid TURN);
   - WebTransport (91.6% of browsers);
   - moving the web build to `wasm-bindgen`, which game ADR-0034 would need
     amended, and the pad plugin re-done.
4. Write an ADR with the three choices and why. Write the implementation
   tickets.

**Out (do not do):**
- committing spike code;
- MMO concerns beyond what snapshots need (1150).

## Implementation steps

1. Run the spikes on `town-and-battle` (1125)'s field stage, with two
   players.
2. Measure: WASM size, lines of code, behavior under 100 ms of delay and
   5% loss with the in-memory network.
3. Write the ADR and the tickets.

## Acceptance criteria

- [ ] The ADR exists with measurements and the alternatives.
- [ ] Implementation tickets exist, and `cargo xtask ticket-lint` passes.

## Tests required

None committed (research).

## Completion notes
