---
id: "2790"
title: "trails online: quick play from the title screen, riding in a server's room, on desktop and the web"
type: feature
milestone: "Engine: examples"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2789", "2617", "1142"]
nick_input: none
completed:
---

# 2790 — trails online: quick play from the title screen

## Context

This is the player's side of Nick's decision of 2026-11-25
(`docs/decisions.md`): press "Play online", land in a room already under
way with bots and other players, take a bot's seat, and leave whenever;
a bot takes the seat back.

What it builds on:

- **Trails** (2788): `Round`, the bots, the screens and the skin.
  **`trails-server`** (2789): `impl RoomGame for Trails` behind the
  `room` feature, and the server binary.
- **`RoomClient`** (2614): follows a room from its `Step` frames, keeps
  a jitter buffer of one to two steps, steps its own copy of the
  deterministic `Round` with exactly the server's inputs, checks the
  server's hash every 15 steps, and asks for a catch-up on a mismatch.
  It doesn't predict: about one step of input delay at 15 steps a
  second (ADR-0155, 2613).
- **The director's handshake** (2615): `Hello { protocol, made }`, then
  `Play`; answers `Welcome`, `Refused { why: OtherBuild | Busy {
  retry_after_s } | Protocol }`.
- **Transports:** native, 2617's WebSocket client in `shuyi-app` behind
  its `net` feature; web, 1142's binary messages over our own miniquad
  JS plugin (`net.js`, from 2522), since the web build has no
  `wasm-bindgen` (game ADR-0034). Screens reach either through the
  `shuyi` front door's `net` feature, never a target `cfg!`
  (ADR-0060): `shuyi::net::connect(url)` gives a `Transport` on both
  (2617 native, 1142 web), and `shuyi::net::page_server()` (1142)
  reads the page's `?server=`.
- **The address.** 2618 publishes the live server as
  `site/static/trails/server.ron` (`wss://<ip>/play`), served at the
  site's `/trails/server.ron`, outside `examples/trails/` because
  `pages.yml` replaces that folder with the web build. The certificate
  is on an IP address and may be renewed or the IP changed after a
  reclaim, so the address is read at run time, never compiled in.
- **Rule 13** (decision of 2026-11-24): an optional dependency that
  makes games bigger is a toggle. 2617's `net` feature is off by
  default in `shuyi-app`; trails turns it on for its own desktop and
  web builds and says what it costs.

## Nick input

None.

## Scope

**In:** in `examples/trails`.

- **A feature `online = ["room", "shuyi/net", "shuyi-app?/net"]`**,
  in `default` for the window builds (desktop, web, Steam Deck), never
  in the Playdate's. Measure what it adds to the desktop binary
  (`--release`, stripped) and to the web build's `.wasm`, and write both
  in the README and the completion notes.
- **`src/online.rs`** (a folder `src/online/` if it grows), behind
  `online`:
  - **`Online`**, the client's state machine as a plain struct over a
    `Transport` (2613), tested with no sockets over `MemoryNetwork`:
    `Connecting` → `Hello` sent → `Finding` (`Play` sent) → `Riding {
    seat }` or `Watching` (the round's seats were all taken by living
    riders: the player waits for the next round) → `Lost`; `Busy {
    retry_in }` sends `Play` again after `retry_after_s`; `OtherBuild`
    and `Protocol` end it.
  - **Input:** each step, the arena's actions become seat 0's
    `SeatInput` as offline, sent as `Input` only when it changes, and
    `Alive` once a second otherwise (2614's messages).
  - It holds the `RoomClient<Trails>`, so the screen draws the room's
    `Round` with 2788's scene and skin.
- **Screens**:
  - The title gains "Play online" beside "Ride" when the game is built
    with `online` (a function of the feature, never a target `cfg!`).
    Anything about the target is asked of `ctx.platform`.
  - **Connecting** and **Finding a room**: a line each, Cancel back to
    the title.
  - **Online arena**: the room's round, drawn as offline; the player's
    own rider marked "you"; each bot's seat marked "bot" in the HUD (no
    names: none are sent); wins from `RoundStart`. When the player's
    rider crashes, a line "Watching until the next round". The round
    over overlay as offline. Pause is not offered (the room doesn't
    stop); Cancel asks "Leave the room?" and sends `Leave`.
  - **Messages**: `Busy`, "All rooms are full. Trying again in N s";
    `OtherBuild`, "A new version is out. Reload the page" on the web,
    "A new version is out" on native (asked of `ctx.platform`);
    connection lost, "Lost the connection", then back to the title,
    where offline play is still there.
  - Every text in `en.ron`, every action in `keymap.ron`.
- **The server's address**, resolved in this order by a pure function
  (`online::address`) with unit tests:
  1. the launch's own: `?server=<url>` in the page's query on the web
     (`shuyi::net::page_server()`, 1142), `--server <url>` on
     native; a malformed one is ignored with a log line;
  2. a `server.ron` read at run time: on the web, the site's
     `../../trails/server.ron` relative to the page (2618's file),
     through macroquad's loader; on native, `server.ron` beside the
     executable; never packed into the game;
  3. the default in trails' `files/online.ron`:
     `ws://127.0.0.1:7777/play`, the dev server of 2789.

  A `ws://` address from a page served over `https://` is refused with
  a message (browsers block it); `wss://` is required there.
- **README**: "Play online": how to point a build at a local server
  (`--server`, `?server=`), and the sizes measured above.
- `Book:` and `Skills:` lines in the completion notes (2854's chapter
  walks through this screen; 2855's skill teaches it).

**Out (do not do):**
- Predicting the player's own rider: a follow-up under 1147 (rollback,
  2604).
- Names, chat, friends, room codes, a room browser, spectating without
  playing.
- Reconnecting into the same seat: a reconnect is a fresh `Play`.
- The server, its deploy and its address file (2789, 2618).
- Online play on the Playdate (no network there).

**Estimated size:** about 900 lines (code 500, tests 330, text and
docs 70).

## Implementation steps

1. Read 2614's `RoomClient` and wire, 2615's handshake, 2617's native
   client and 1142's web transport, and 2788's screens and skin.
2. `Online`, the state machine, and its tests over `MemoryNetwork`
   with seeded loss and delay.
3. The screens and their text.
4. `online::address` and its tests; the run-time `server.ron` read on
   both targets.
5. A native end-to-end test against `trails-server` on 127.0.0.1.
6. The web check in the browser pane (two tabs, below).
7. The size measurement, the README, the completion notes.

## Acceptance criteria

- [ ] In one process, a `Director<Trails>` (2615) and three `Online`
      clients over a `MemoryNetwork` with seeded loss and delay ride
      the same room, and every client's `Round` hash equals the
      server's at every checked step.
- [ ] A client that leaves (or goes silent past 2614's timeout) is
      replaced by a bot at the next step; a client that joins takes the
      lowest bot seat with a living rider, or watches until the next
      round when none is alive.
- [ ] Integration tests through the Harness read the scene: title →
      "Finding a room" → the arena with the player's seat → "Lost the
      connection" when the server stops → the title.
- [ ] `Busy` shows its line and retries after `retry_after_s`;
      `OtherBuild` shows its line and does not retry.
- [ ] `online::address` picks the launch's address, then `server.ron`,
      then the default, and refuses `ws://` from an `https://` page.
- [ ] A native end-to-end test plays 300 steps against `trails-server`
      on 127.0.0.1 in Linux CI.
- [ ] Two browser tabs of the web build on a local `trails-server` ride
      in one room and see each other's riders (checked by hand in the
      browser pane with 1142's harness; recorded in the completion
      notes).
- [ ] The desktop and web size added by `online` is in the README and
      the completion notes.
- [ ] Offline play is unchanged: 2788's goldens and record pass, and the
      Playdate build has no `online`.
- [ ] `cargo shuyi check-keys` and `check-text` pass.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass, `wasm` included.

## Tests required

- Unit: `online::address` (each source, the order, a malformed one,
  `ws://` from `https://`); `Online`'s transitions.
- Integration: `tests/online.rs` (director and clients over
  `MemoryNetwork`); `tests/it/online_screens.rs` (the screens through
  the scene).
- End to end: `tests/e2e.rs`, against `trails-server`, run in Linux CI
  only (`#[cfg_attr(not(target_os = "linux"), ignore)]` with a reason).
- Manual: two browser tabs, recorded.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
