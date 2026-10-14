---
id: "1145"
title: "Parked: Networking, step 6: sessions and lobbies — one API over the relay, the LAN and Steam, with a lobby screen"
type: feature
milestone: "Engine: networking"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1141", "1143"]
nick_input: none
completed:
---

# 1145 — Sessions and lobbies

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §1 defines the Session layer: who is in a match. It covers
creating a room, listing rooms, joining by invite code, getting ready,
starting, leaving, and room metadata.

The relay (1141) and the LAN (1143) each have a session of their own,
and Steam (1144) has lobbies. This ticket puts them behind one trait. It
also adds a generic lobby screen (game ADR-0054: logic, view and skin), so a
game gets a working lobby out of the box.

## Nick input

None. Our game's lobby look comes later with 0053.

## Scope

**In:**
- `trait SessionBackend` with:
  - `create(settings)`;
  - `list(filter)`;
  - `join(code or id)`;
  - `set_ready`;
  - `start`;
  - `leave`;
  - `metadata`;
  - `events()`.
  The relay and LAN backends implement it now. Steam's does too, if
  1144 is done; if 1144 isn't done, 1144 implements it later.
- Player names are strings the game gives. There are no accounts here
  (1148).
- `shuyi-ui::screens::lobby`: create or join by code, a player list,
  ready toggles, start. It uses the `UiAction`s only, and the game names
  its own text keys. `check-keys` and `check-text` must pass.
- `examples/hello-grid` (1118) gets a two-player online mode
  through the lobby, over the relay and the LAN.

**Out (do not do):**
- matchmaking queues (1148);
- chat (1149);
- our game's lobby.

## Implementation steps

1. Write the trait and adapt the two backends.
2. Build the lobby screen, with Harness tests over the in-memory network
   (a `MemorySession` backend for tests).
3. Add the example's online mode, with an integration test that runs two
   Harness games and a relay in one process.

## Acceptance criteria

- [ ] Two Harness instances of `hello-grid` find each other, ready up and
      play to the end over the relay and over loopback LAN in CI.
- [ ] All gates pass.

## Tests required

- Integration: as above.
- Snapshot: the lobby screen.

## Completion notes
