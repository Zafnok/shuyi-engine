---
id: "3037"
title: "Parked: Steam achievements, stats and cloud saves behind the steam feature: Steamworks under the achievements service, and saves synced by Steam Cloud"
type: feature
milestone: "Engine: store services"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["3036"]
nick_input: setup
completed:
---

# 3037 — Steam achievements, stats and cloud saves

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; and 2026-10-20: "make sure steam achievements and cloud saves are covered by some ticket so it's not lost"): after the MVP in `docs/ROADMAP.md` and after 3036. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-20 (`docs/decisions.md`): Steam achievements and cloud
saves must have a ticket. 3036 builds the achievements service with no
store in it. This ticket puts Steam under it, and puts the player's
saves in Steam Cloud.

What stays the game's (decisions, 2026-10-15, and game ticket 0903):
the Steam partner account, the app id, the store page, and setting up
the achievements, stats and cloud quota on the Steamworks site. The
engine brings the code; the game brings the account.

Steamworks in Rust: `steamworks-rs` (MIT OR Apache-2.0, over Valve's
SDK). 1144 adds it behind the `steam` feature for networking. game
ADR-0013 allows the Steamworks SDK behind that feature. 1144's research
could not confirm that it links with the Windows GNU toolchain.

Steam Cloud works two ways (Steamworks documentation, *Steam Cloud*):

- **Auto-Cloud:** the game's save folder is named on the Steamworks
  site, and Steam syncs it with no code. Steam shows its own screen when
  two machines disagree.
- **The Cloud API** (`ISteamRemoteStorage`): the game reads and writes
  each file through Steam.

## Nick input

**Setup** (when unparked; the game's 0903 asks for the same account):

1. In the Steamworks partner site, open the app, then *Stats &
   Achievements* → *Achievements*, and add the toy game's two test
   achievements with the ids from 3036's toy file (or use Valve's test
   app 480, Spacewar, for the code; then no setup is needed until the
   game's own build).
2. *Application* → *Steam Cloud*: set a quota (for example 10 files,
   1 MB), and for Auto-Cloud add the save folder this ticket's docs
   name.
3. A Steam account that owns the app, for the manual test.

## Scope

**In:**
- **The dependency.** **Either order with 1144:** if 1144 is done, use
  its `steam` feature and its link check. If not, add `steamworks`
  behind a `steam` feature in `shuyi-app` here: check its license and
  default features with `cargo deny` (ADR-0004 §7), and check the GNU
  link. If GNU fails, the `steam` feature builds on CI's MSVC runners
  only; say so in `AGENTS.md`'s *Environment*, and never require MSVC
  locally. 1144 then reuses this.
- **`SteamAchievements`**, implementing 3036's `Achievements` over
  `UserStats`: it asks for the current stats at start, unlocks with
  `set_achievement` and `store_stats`, sets stats, and shows progress
  toward a goal with `indicate_achievement_progress`. Calls before
  Steam has answered are queued, then sent.
- **Cloud saves:**
  - **Auto-Cloud by default.** The docs (`docs/targets/steam.md`) name
    the folder the native backend saves into on each OS, which the game
    enters on the Steamworks site. No code.
  - **`SteamStorage`**, implementing `Storage` over `RemoteStorage`
    (write, read, delete, list), for a game that wants each save in the
    Cloud API. It reports Steam's quota errors as `StorageError`.
  - The docs say which to pick: Auto-Cloud unless the game must control
    each file.
- **Without Steam running:** the build starts anyway, falls back to
  3036's `LocalAchievements` and the normal file storage, and logs why.
- **The toy game** behind `--features steam`, with app id 480 for the
  manual test.
- **A manual test**, written down in the docs: the toy game unlocks an
  achievement and the Steam overlay shows it; a save made on one machine
  appears on another (or after deleting the local copy).

**Out (do not do):**
- the partner account, the store page and the game's own achievements
  (the game's 0903);
- leaderboards and lobbies (1144, 1148);
- other stores (GOG Galaxy, Epic, consoles): each a ticket of its own
  under 3036's trait, when a game ships there;
- Steam Input action sets (3012's Out list).

## Implementation steps

1. Read 3036, 1144 (if done), `storage.rs` and the native storage
   backend in `shuyi-app`.
2. Add or reuse the dependency; check the license and the GNU link.
3. Write `SteamAchievements`, with a fake of the Steam calls for tests.
4. Write `SteamStorage`, with the same fake.
5. Write the fallback without Steam, and `docs/targets/steam.md`.
6. Run the manual test with app 480 if a Steam account is at hand; say
   in the Completion notes whether it was done.
7. Run the gates.

## Acceptance criteria

- [ ] `cargo deny check` passes with the `steam` feature.
- [ ] The GNU link result is recorded, and `AGENTS.md` says what it
      means.
- [ ] With the fake, an unlock before Steam answers is sent once Steam
      does.
- [ ] Without Steam running, the toy game starts with local achievements
      and saves.
- [ ] `docs/targets/steam.md` names the save folder per OS for
      Auto-Cloud, and when to use `SteamStorage`.
- [ ] All gates in the `run-gates` skill pass, with and without the
      `steam` feature.

## Tests required

- Unit: the queue before Steam answers; quota errors; the fallback.
- Integration: the toy game's achievements through the fake.
- Manual: the overlay and a synced save, as above.

## Completion notes

*(Filled in by the session that completes the ticket.)*
