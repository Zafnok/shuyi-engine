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
  - `join(JoinToken)`: a `JoinToken` is a room code, a session id or a
    backend's own token, with a text form (`Display` and `FromStr`) for
    links, the command line and Steam's `connect`;
  - `set_ready`;
  - `start`;
  - `leave`;
  - `metadata`;
  - `events()`, which include `SessionEvent::JoinRequested(JoinToken)`:
    a backend's own join request (Steam's, 1144) or the one from launch.
  The relay and LAN backends implement it now, and Steam's as the
  clause on 1144 at the end of this list says.
- **A join at launch:** `shuyi-app` reads `--join <token>` from the
  command line, and on the web 1142 reads `?join=`. Either becomes
  a launch join token the platform hands in at load, as a field of
  `Setup` (`crates/platform/src/app.rs`) beside `seed`. `Setup` is
  `#[non_exhaustive]`, so the field is additive. `shuyi-platform`
  doesn't depend on `shuyi-net`, so the field holds the token's text,
  and the lobby screen parses it with `JoinToken`'s `FromStr` and joins
  it on its first frame. `SystemEvent` is `Copy` today
  (`crates/platform/src/platform.rs`), so a join request that arrives
  while the game runs reaches the session through the backend's
  `events()`, not through `SystemEvent`.
  **Either order with 1142:** if 1142 is done, its `?join=` reader
  hands the room code straight to `RelaySession`; hand it to `Setup`'s
  field instead, so the lobby joins it as it joins `--join`. If not,
  nothing to do (1142 hands it to `Setup`'s field).
- Player names are strings the game gives. There are no accounts here
  (1148).
- `shuyi-ui::screens::lobby`: create or join by code, a player list,
  ready toggles, start. It uses the `UiAction`s only, and the game names
  its own text keys. `check-keys` and `check-text` must pass.
- `examples/hello-grid` (1118) gets a two-player online mode
  through the lobby, over the relay and the LAN.
- **Peers through the contract, in either order with 2529:** if 2529 is
  done, `examples/hello-grid/src/tools.rs` offers `GameTools::peers`
  through `shuyi_net::tools::MemoryPeers`, `hello-grid`'s `tools`
  feature turns on `shuyi-net/tools`, and a CI test runs `cargo shuyi
  peers 2 --net seed=1,loss=10,delay=2` on it. If 2529 isn't done, 2529
  does that.
- **Either order with 1134:** if 1134 is done, drop 1145 from the
  `waits_for` of `skills/shuyi-net/SKILL.md` (`check-skills` fails on
  an id no longer in `tickets/open/`), and teach sessions and lobbies
  there: `SessionBackend`, `JoinToken`, `--join`, the lobby screen and
  `hello-grid`'s online mode. If 2529 is done too, `waits_for` is then
  empty: drop it, and write the skill's worked example as 1134's scope
  says (two `hello-grid` peers through `peers`, with a seeded loss).
  If 2529 isn't done, 2529 writes the example. If 1134 isn't done,
  nothing to do (1134 writes the skill from what exists).
- **Either order with 3055:** if 3055 is done, the lobby screen sets the
  presence group (the session's id and its player count) and the join
  token's text (`Ctx::set_presence_group`, `Ctx::set_presence_join`)
  while the session takes players, and clears both when it closes.
  Steam then shows "Join Game" through `connect`. If 3055 isn't done,
  3055 does this when it is.
- **Either order with 1144:** if 1144 is done, move its Steam pieces
  onto this API: `SteamSession` implements `SessionBackend`; Steam's
  join callbacks become `SessionEvent::JoinRequested(JoinToken)`, with
  the lobby id as the token; `shuyi-app` hands a `+connect_lobby <id>`
  cold start in as the launch join token, beside `--join`; and the
  lobby screen offers the overlay's invite dialog when the session is
  Steam's. If not, 1144 does all of that over this API, and nothing is
  left here.

**Out (do not do):**
- matchmaking queues (1148);
- chat (1149);
- our game's lobby;
- rich presence (3055); the Steam calls themselves, the overlay's
  invite dialog and Steam's join callbacks (1144: this ticket only
  moves them onto its API, in the clause on 1144); reading the join
  link in the browser (1142).

## Implementation steps

1. Write the trait, `JoinToken` and `SessionEvent`, and adapt the two
   backends.
2. Build the lobby screen, with Harness tests over the in-memory network
   (a `MemorySession` backend for tests).
3. Add the launch join token: `Setup`'s field, `--join` in `shuyi-app`,
   and the lobby joining it on its first frame; and, if 1142 is done,
   its `?join=` reader handing its token to that field.
4. Add the example's online mode, with an integration test that runs two
   Harness games and a relay in one process.
5. Do 1134's, 1144's, 2529's and 3055's parts if they are done.

## Acceptance criteria

- [ ] Two Harness instances of `hello-grid` find each other, ready up and
      play to the end over the relay and over loopback LAN in CI.
- [ ] A game started with a launch join token joins that room on its
      first frame.
- [ ] If 2529 is done: `hello-grid`'s CI test of `cargo shuyi peers 2
      --net seed=1,loss=10,delay=2` answers `ok`.
- [ ] All gates pass.

## Tests required

- Integration: as above.
- Integration: a Harness game started with a launch join token joins
  that room on its first frame over `MemorySession`.
- Unit: a `JoinToken`'s text form reads back equal (`Display`, then
  `FromStr`).
- Snapshot: the lobby screen.

## Completion notes
