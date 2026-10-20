---
id: "1144"
title: "Parked: Networking, step 5: Steam networking messages and Steam lobbies behind the steam feature"
type: feature
milestone: "Engine: networking"
model: opus-5.5
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

## Nick input

**Setup:** the Steamworks app id and SDK access, which 0903 already
asks for. Two Steam accounts that own the app, for the manual test.

## Scope

**In:**
- `SteamTransport` implements `Transport` over `networking_messages`.
- `SteamSession` implements the session API (1145, or the relay's
  interim API if 1145 isn't done) over lobbies.
- Check the GNU link. If it fails, the `steam` feature builds on the CI
  MSVC runners only. Document this in `CLAUDE.md`'s *Environment*, and
  never require MSVC locally.
- A manual test: two machines on two accounts play the toy lockstep game
  through a lobby.

**Out (do not do):**
- leaderboards (1148);
- achievements, stats and cloud saves (3036, 3037). Either order: if
  3037 is done, it added the `steam` feature and the link check, and
  this ticket reuses them;
- Remote Play Together (needs no code).

## Implementation steps

1. Add the dependency behind the feature and run `cargo deny check`.
2. Write the transport and the session.
3. Check the GNU link, run the manual test, and write the docs.

## Acceptance criteria

- [ ] The CI build with `--features steam` passes on at least the MSVC
      runner.
- [ ] The manual two-account test is noted in the PR.
- [ ] All gates pass without the feature.

## Tests required

- Manual: two accounts.
- Unit: message framing.

## Completion notes
