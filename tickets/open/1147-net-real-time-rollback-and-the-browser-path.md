---
id: "1147"
title: "Parked: Networking, step 8: real-time play — rollback for tick games, snapshots for many players, and how the browser joins"
type: research
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["1125", "1140", "2604", "2614"]
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

**A third model is built already, by the decision of 2026-11-25**
(`docs/decisions.md`, the online example trails). Under ADR-0155, which
amends ADR-0006 §6, a server runs the real `TickSim` and broadcasts
each step's resolved inputs, with a hash every 15 steps and a catch-up
from the round's start on join (2614 the rooms, 2615 quick play, 2616
the server, 2617 and 1142 the clients over WebSocket). So real time in
the browser no longer waits for this ticket: it works over WebSocket,
with clients that show confirmed steps behind a 1–2 step jitter buffer
and don't predict. This ticket stays parked and keeps what that model
doesn't cover:

- rollback for 2–4 peers (GGRS against our own, with 2604's costs);
- **client-side prediction of the player's own rider**, with
  interpolation, as a named follow-up for trails: on top of 2614's
  `RoomClient`, predicting the local seat's input and re-simulating
  from the last confirmed step when the server's input differs (the
  same resimulate window as rollback);
- per-client state deltas (snapshots) for games too big for input
  broadcast, or with hidden information in real time;
- WebRTC and WebTransport for the browser.

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
   `TickSim` (save and load as clone; **from 2604:**
   `shuyi_core::tick::check_rollback` and the measured costs in
   `docs/research/2604-rollback-cost.md`, so each spike runs
   `check_rollback` on the field stage and starts from those numbers).
   Spike
   a small rollback of our own, about 600 lines: an input queue,
   prediction by repeating the last input, a resimulate window, and
   desync hashes every N steps. Compare them on licenses, the WASM build
   under our loader, code size, and test results under seeded faults.
   Both spikes keep 1119's rule that a step's events are handed on once
   (**from 2137, done:** `TickDriver::rewind_to` in `shuyi_ui::tick`,
   which sets the steps back and keeps the highest step handed on). The ADR says what happens to events that a
   correction changes. An event the predicted run handed on, which the
   corrected run doesn't make (a sound already heard), is not taken
   back. An event the corrected run adds is handed on at once, unless
   its step is more than K steps old. The ADR picks K, and says what
   `TickDriver` keeps to tell an added event from one already handed
   on, since `rewind_to` alone hands on no event of a step that runs
   again.
1b. **Own-rider prediction over 2614's rooms.** Spike predicting the
   local seat in `RoomClient` (repeat the local input, re-simulate from
   the last confirmed `Step` when the server's differs), on 2614's
   `room::testing::Dots` at 100 ms of delay, and measure the input delay it saves
   against the jitter buffer alone. The ADR says whether it ships, and
   the implementation ticket names 2790 as its first user.
2. **Snapshots.** Spike naia (it works with macroquad on the web; its
   server is native) against our own snapshots over `Transport`. Our own
   version is delta-compressed state per client, interpolation buffers,
   and client prediction for the local player.
3. **The browser.** WebSocket already carries ADR-0155's rooms (1142).
   For rollback and snapshots, where loss-tolerant datagrams matter,
   weigh three options:
   - more JS plugins (WebRTC data channels through a signaling server,
     plus STUN/TURN: coturn is BSD-3 and self-hosted, or a paid TURN);
   - WebTransport (91.6% of browsers);
   - moving the web build to `wasm-bindgen`, which game ADR-0034 would need
     amended, and the pad plugin re-done.

   **Either order with 2910:** voice chat in the browser needs the same
   path. If 2910 is done, the ADR weighs each option for a voice stream
   too, as the line 2910 added here says. If not, nothing to do: 2910
   builds on this ticket's ADR.

   **Either order with 2605:** if 2605 is done, compare the browser
   options against its row for `RelayTransport` (the toy game at 100 ms
   of delay and 5% loss), and name `RelayTransport` as the native
   transport in the rollback implementation ticket. If not, nothing to
   do (2605 measures its transport under this ticket's setup and adds
   the row to its ADR).
4. Write an ADR with the three choices and why. The ADR also says how a
   rollback session treats 2123's game speed (`Ctx::game_speed`), done
   or not: ignored while the session drives the steps, or one speed
   every peer agrees on as a session setting. It also says how the
   drawn view takes a correction: the corrected items `snap`, or the
   correction is eased over a few frames. **Either order with 2361:**
   2361 leaves that to this ticket, beyond its `snap`. If 2361 is done,
   build on its `StepViews` and `snap`. If not, the rollback
   implementation ticket names 2361 and what it needs from it, and
   2361's Out line points at that ticket. Write the implementation
   tickets.
   Each implementation ticket also says what it adds to the `shuyi-net`
   skill (1134's `skills/shuyi-net`; ADR-0005 §3 gives it lockstep,
   rollback and sessions): the rollback ticket adds its rollback
   section, written on both sides as the `write-ticket` skill asks.

**Out (do not do):**
- committing spike code;
- MMO concerns beyond what snapshots need (1150);
- the input-broadcast rooms, quick play and their server (2614–2616,
  ADR-0155): built, not re-decided here.

## Implementation steps

1. Run the spikes on `town-and-battle` (1125)'s field stage, with two
   players.
2. Measure: WASM size, lines of code, behavior under 100 ms of delay and
   5% loss with the in-memory network.
3. Write the ADR and the tickets.

## Acceptance criteria

- [ ] The ADR exists with measurements and the alternatives.
- [ ] The ADR states how the events of re-simulated steps are handed
      on, sounds first. The rollback implementation ticket's tests
      include a rollback over a step whose sound cue must play exactly
      once, checked through the Harness's `audio_log`.
- [ ] Implementation tickets exist, and `cargo xtask ticket-lint` passes.

## Tests required

None committed (research).

## Completion notes
