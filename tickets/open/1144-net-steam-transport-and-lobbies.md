---
id: "1144"
title: "Parked: Networking, step 5: Steam networking messages and Steam lobbies behind the steam feature"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["1140", "game#0903"]
nick_input: setup
completed:
---

# 1144 — Steam transport and lobbies

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §1 and §5. Steam gives several things for free:

- lobbies (up to 250 members, metadata, lobby chat);
- P2P relayed through Valve's network (Steam Datagram Relay), which hides
  IP addresses and gets through home routers;
- leaderboards.

`steamworks-rs` (MIT/Apache over the SDK) wraps `networking_messages`
and `Matchmaking`. game ADR-0013 allows the Steamworks SDK behind the `steam`
feature. 0903 (Steam readiness) brings the SDK and the app id into the
game.

The research could not confirm that `steamworks-rs` links on the Windows
GNU toolchain. Its build script links `steam_api64` without telling
MSVC and GNU apart.

**One Steam client** (found by the 2026-10-20 comparison with Bevy
0.20, Godot 4.7, Unity 6 and Unreal 5.8). Steamworks expects one start
and one callback pump per process. 3037 (Steam achievements and cloud
saves) needs Steam too, and Nick may unpark either ticket first. So one
`steam` module in `shuyi-app` starts Steam, pumps its callbacks and
turns the overlay into focus events (3037's *The Steam client*).
`shuyi-net` takes its handle and never starts Steam. 3036's ADR records
the rule.

## Nick input

**Setup:** the Steamworks app id and SDK access, which 0903 already
asks for. Two Steam accounts that own the app and are friends on Steam,
for the manual test (the invite and the join from the friends list).

## Scope

**In:**
- `SteamTransport` implements `Transport` over `networking_messages`.
- `SteamSession` implements the session API (1145, or the relay's
  interim API if 1145 isn't done) over lobbies.
- **Invites and joins from Steam's friends list.** Each way in becomes
  1145's `SessionEvent::JoinRequested(JoinToken)`, with the lobby id as
  the token (or, for a `connect` string, the token it carries):
  - the overlay's invite dialog for the current lobby
    (`Friends::activate_invite_dialog(lobby)` in `steamworks` 0.13),
    which the lobby screen offers when the session is Steam's. A friend
    who accepts comes in by one of the two ways below;
  - the `GameLobbyJoinRequested` callback, for a friend's lobby picked
    while the game runs, and `GameRichPresenceJoinRequested`, for a
    rich-presence `connect` string (3055) picked while it runs;
  - a cold start from `+connect_lobby <id>` on the command line, read in
    `shuyi-app`, the platform backend (`AGENTS.md` rule 5), and handed
    in as the launch join token (1145).

  **Either order with 1145:** if 1145 is done, use its `JoinToken`,
  `JoinRequested`, launch join token and lobby screen as above. If not,
  map the joins onto the relay's interim session API's join by id, and
  the toy lockstep game's screen opens the invite dialog on an action
  of its own; 1145 then moves the joins onto its API and the dialog
  onto its lobby screen (1145's clause on this ticket says so).
- **The Steam client. Either order with 3037:** if 3037 is done, its
  `steam` module in `shuyi-app` starts Steam and pumps its callbacks,
  and this ticket takes the handle it gives. If not, write that module
  here, with its tests over a fake, as 3037's *The Steam client* says:
  - `SteamAPI_RestartAppIfNecessary` first, then the start;
  - one callback pump per frame in `run`'s loop;
  - the overlay as `FocusLost` and `FocusGained`;
  - one handle.

  3037 then reuses it. If the game's 0903 (done before this ticket)
  starts Steam in its own app crate, the PR says that the game's pin
  bump drops that start and takes the handle. Either way,
  `SteamTransport` and `SteamSession` take the handle in their
  constructors and never start Steam or pump callbacks. `shuyi-net` gets
  the handle as a value and never depends on `shuyi-app`. Lobby, join
  and message callbacks reach them through the one pump.
- Check the GNU link, unless 3037 did. If it fails, the `steam` feature
  builds on the CI MSVC runners only. Document this in `AGENTS.md`'s
  *Environment*, and never require MSVC locally.
- **Either order with 1148:** if 1148 is done and its completion notes
  say its accounts are device ids only, add the Steam auth ticket: the
  client asks for it through the handle, and `shuyi-services` checks it
  with Steam. If not, nothing to do (1148 does it).
- **Either order with 1149:** if 1149 is done and its completion notes
  say its chat filter is `rustrict` alone, use Steam's text filter
  instead when the `steam` feature is on and Steam runs, asked through
  the handle. If not, nothing to do (1149 does it).
- A manual test: two machines on two accounts play the toy lockstep game
  through a lobby.

**Out (do not do):**
- leaderboards: the service is 3054's, Steam's boards come with 3037 or
  3054 (whichever is done second), and our own server's with 1148;
- achievements, stats and cloud saves (3036, 3037). Either order: if
  3037 is done, it added the `steam` feature, the link check and the
  Steam client, and this ticket reuses them. If not, this ticket adds
  them (In) and 3037 reuses them;
- rich presence, the status line friends see, and its `connect` key
  (3055);
- Remote Play Together: a guest's pad arrives as one more local pad. It
  needs no code for hotseat, since every pad drives the game as one
  today (2406's `Shared`). A game where each player's pad acts only for
  that player needs input seats (2406).

## Implementation steps

1. If 3037 is done, read its `steam` module and take its handle. If
   not, add the dependency behind the feature, run `cargo deny check`,
   and write the module (In).
2. Write the transport and the session, then the invites and joins.
3. Check the GNU link, run the manual test, and write the docs.

## Acceptance criteria

- [ ] The CI build with `--features steam` passes on at least the MSVC
      runner.
- [ ] The manual two-account test is noted in the PR. In it the second
      account joins once through the overlay's invite dialog, and once
      from the friends list with the game closed (the `+connect_lobby`
      cold start).
- [ ] `shuyi-net` never starts Steam or pumps its callbacks: its Steam
      types take the handle from `shuyi-app`'s `steam` module.
- [ ] All gates pass without the feature.

## Tests required

- Manual: two accounts.
- Unit: message framing.
- Unit: `+connect_lobby <id>` parsed into a token (with a malformed one
  ignored), and each callback mapped to `JoinRequested`, through a fake
  of the Steam calls.
- Unit, if this ticket writes the Steam client: the start, the overlay
  as focus events and one pump per frame, over a fake (as 3037 lists).

## Completion notes
