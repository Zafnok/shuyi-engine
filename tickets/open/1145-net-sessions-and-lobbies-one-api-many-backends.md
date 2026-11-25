---
id: "1145"
title: "Parked: Networking, step 6: sessions and lobbies — one API over the relay, the LAN and Steam, with a lobby screen"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["1141", "2615"]
nick_input: none
completed:
---

# 1145 — Sessions and lobbies

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.
> Still parked after Nick's decision of 2026-11-25, which unparked the online trails example: trails uses quick play (2615) and needs no lobby.

## Context

ADR-0006 §1 defines the Session layer: who is in a match. It covers
creating a room, listing rooms, joining by invite code, getting ready,
starting, leaving, and room metadata.

The relay (1141) and the LAN (1143) each have a session of their own,
and Steam (1144) has lobbies. This ticket puts them behind one trait. It
also adds a generic lobby screen (game ADR-0054: logic, view and skin), so a
game gets a working lobby out of the box.

Quick play (2615, from Nick's decision of 2026-11-25) is another kind of
session: a black-box director on a server seats a player in a room with
a free seat, with no lobby, ready step, room code or invite. So this
ticket's explicit lobby model is for invite and friend play, and a
`QuickPlay` backend of `SessionBackend` wraps 2615's `Director` over its
server (2616): `join` with no token asks for quick play, `list` is
empty, and `set_ready` and `start` do nothing, since the server starts
rounds. The LAN (1143) is not needed for sessions, so it is no longer a
blocker, and its session backend is a follow-up (Scope).

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
  The relay backend implements it now, the LAN's as the LAN line below
  says, and Steam's as the clause on 1144 at the end of this list says.
- **Quick play:** `QuickPlay`, a backend over 2615's `Director`, through
  2616's server and the client transports (2617 on native, 1142 on the
  web), as the Context says. Its refusals (2615's `Refusal`:
  `OtherBuild`, `Busy`, `Protocol`) become `SessionEvent`s the lobby
  screen shows with keys the game names.
- **The LAN's backend is a follow-up:** write, in this PR, a parked
  ticket blocked by 1143 and this one, in which the LAN's session
  implements `SessionBackend` and `hello-grid`'s online mode also runs
  over loopback LAN.
- **A join at launch:** `shuyi-app` reads `--join <token>` from the
  command line, and on the web this ticket reads `?join=` from the
  page's address (the reader moved here from 1142 on 2026-11-25, when
  1142 was unparked for trails, which has no join links). Either becomes
  a launch join token the platform hands in at load, as a field of
  `Setup` (`crates/platform/src/app.rs`) beside `seed`. `Setup` is
  `#[non_exhaustive]`, so the field is additive. `shuyi-platform`
  doesn't depend on `shuyi-net`, so the field holds the token's text,
  and the lobby screen parses it with `JoinToken`'s `FromStr` and joins
  it on its first frame. `SystemEvent` is `Copy` today
  (`crates/platform/src/platform.rs`), so a join request that arrives
  while the game runs reaches the session through the backend's
  `events()`, not through `SystemEvent`.
  **Either order with 1142:** if 1142 is done, add to its `net.js`
  plugin a call that hands Rust the page's query, read its `join`
  parameter with a pure function, and hand the token to `Setup`'s
  field, so the lobby joins it as it joins `--join`. If not, do the
  native half only, and write a follow-up ticket in this PR, blocked by
  1142, for the web half.
- Player names are strings the game gives. There are no accounts here
  (1148).
- `shuyi-ui::screens::lobby`: create or join by code, a player list,
  ready toggles, start. It uses the `UiAction`s only, and the game names
  its own text keys. `check-keys` and `check-text` must pass.
- `examples/hello-grid` (1118) gets a two-player online mode
  through the lobby, over the relay.
- **Peers through the contract, in either order with 2529:** if 2529 is
  done, `examples/hello-grid/src/tools.rs` offers `GameTools::peers`
  through `shuyi_net::tools::MemoryPeers`, `hello-grid`'s `tools`
  feature turns on `shuyi-net/tools`, and a CI test runs `cargo shuyi
  peers 2 --net seed=1,loss=10,delay=2` on it. If 2529 isn't done, 2529
  does that.
- **From 1134 (done):** drop 1145 from the `waits_for` of
  `skills/shuyi-net/SKILL.md` (`check-skills` fails on an id no longer
  in `tickets/open/`), and teach sessions and lobbies there:
  `SessionBackend`, `JoinToken`, `--join`, the lobby screen and
  `hello-grid`'s online mode. If 2529 is done too, `waits_for` is then
  empty: drop it, and write the skill's worked example,
  `skills/shuyi-net/example-lockstep.ron` (beside 2855's
  `example.ron` for rooms; two `hello-grid` peers through
  `peers`, with a seeded loss, which CI's `skills` job runs). The
  runner, `crates/xtask/src/skills.rs`, makes a fresh `shuyi new` game
  for each example, and this one runs on `hello-grid`: give `Example`
  a field naming an example of the engine's to run in, in the same PR.
  If 2529 isn't done, 2529 writes the example.
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
- **Either order with 2605:** if 2605 is done, carry its `RelayPaths`
  option from `RelaySession` onto the relay session backend, and show
  each player's path (`PathChanged`) on the lobby screen. If not,
  nothing to do (2605 adds the option to this ticket's relay backend
  and the path to its lobby screen).
- **Either order with 2606:** if 2606 is done, `SessionBackend` is
  opened with a purpose id and refuses with `Refused::NoConsent` when
  `Consent::allows` doesn't allow it, and the lobby screen shows
  `UpdateRequired` and `OtherBuild` with keys the game names. If not,
  nothing to do (2606 adds both).
- **Either order with 2920:** if 2920 is done, add the hooks its report
  names to `SessionBackend`: an online check before `create` and
  `join`, answered allowed, denied or restricted, and a platform
  account id a session carries. The relay and LAN backends answer
  "allowed". If not, nothing to do (2920 adds them to this scope when
  it lands).

**Out (do not do):**
- matchmaking queues (1148); quick play's seating policy, room cap and
  handshake (2615: this ticket only wraps its director);
- chat (1149);
- our game's lobby;
- rich presence (3055); the Steam calls themselves, the overlay's
  invite dialog and Steam's join callbacks (1144: this ticket only
  moves them onto its API, in the clause on 1144); the browser's
  WebSocket plugin itself (1142).

## Implementation steps

1. Write the trait, `JoinToken` and `SessionEvent`, and adapt the relay
   backend and the `QuickPlay` backend over 2615's director.
2. Build the lobby screen, with Harness tests over the in-memory network
   (a `MemorySession` backend for tests).
3. Add the launch join token: `Setup`'s field, `--join` in `shuyi-app`,
   and the lobby joining it on its first frame; and, if 1142 is done,
   the `?join=` reader through its plugin, handing its token to that
   field.
4. Add the example's online mode, with an integration test that runs two
   Harness games and a relay in one process.
5. Do the `shuyi-net` skill's part (from 1134), and the parts of 1144,
   2529, 2605, 2606, 2920 and 3055 that are done.

## Acceptance criteria

- [ ] Two Harness instances of `hello-grid` find each other, ready up and
      play to the end over the relay in CI.
- [ ] A `QuickPlay` session over `MemoryNetwork` and a 2615 director
      joins with no token and is seated; a director at its room cap
      gives the lobby screen a `Busy` event.
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
- Unit: the `join` parameter read from a page's query, with and without
  one, and among other parameters.
- Integration: `QuickPlay` over `MemoryNetwork`, seated, then `Busy`.
- Snapshot: the lobby screen.

## Completion notes
