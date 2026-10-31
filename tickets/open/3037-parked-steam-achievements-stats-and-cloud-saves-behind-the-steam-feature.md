---
id: "3037"
title: "Parked: Steam achievements, stats and cloud saves behind the steam feature: Steamworks under the achievements service, and saves synced by Steam Cloud"
type: feature
milestone: "Engine: store services"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3036", "2349"]
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
SDK). 1144 needs it too, behind the same `steam` feature, for
networking. game ADR-0013 allows the Steamworks SDK behind that
feature. 1144's research could not confirm that it links with the
Windows GNU toolchain.

**One Steam client** (found by the 2026-10-20 comparison with Bevy
0.20, Godot 4.7, Unity 6 and Unreal 5.8). Steamworks expects one start
and one callback pump per process. Godot and Bevy each get one client
from their plugin (GodotSteam, `bevy_steamworks`). Here, three places
would start Steam:

- this ticket;
- 1144 (networking), which may be unparked first, when the game's 0053
  picks an online mode;
- the game's 0903, which plans to start Steam in the game's own app
  crate; the game's 1024 builds its Workshop on that.

No ticket owned the start, `SteamAPI_RestartAppIfNecessary`, the
per-frame pump or the overlay. So one `steam` module in `shuyi-app`
owns them, written here or in 1144, whichever is done first, and
everything else takes its handle. 3036's ADR records the rule.

2349 adds the system's languages (`Platform::languages`) and leaves
Steam's game language to this ticket, which waits for it. 3054 builds
leaderboards with no store in them; Steam's boards come with whichever
of 3054 and this ticket is done second.

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
- **The dependency and the Steam client.** **Either order with 1144:**
  if 1144 is done, use its `steam` feature, its link check, and the
  `steam` module below, which it wrote. If not, do three things here:
  - add `steamworks` behind a `steam` feature in `shuyi-app`, and check
    its license and default features with `cargo deny` (ADR-0004 §7);
  - check the GNU link. If GNU fails, the `steam` feature builds on CI's
    MSVC runners only. Say so in `AGENTS.md`'s *Environment*, and never
    require MSVC locally;
  - write the module.

  1144 then reuses all of it.
- **The Steam client**: a `steam` module in `shuyi-app`, behind the
  feature. It is the only code in the engine that starts Steam (3036's
  ADR):
  - the game gives its app id, for example in an optional field of
    `AppConfig` (`crates/app/src/lib.rs:90`) that `AppConfig::new`
    (2013) leaves `None`, so only the game's Steam build sets it. The PR
    says so for the game's pin bump;
  - `SteamAPI_RestartAppIfNecessary` comes first, as early as the
    backend can. When it says Steam must relaunch the game, `run`
    (`crates/app/src/lib.rs:148`) returns before its loop;
  - then Steam starts. If Steam isn't running, the build starts anyway
    (the fallback below);
  - the callback pump runs once per frame in `run`'s loop, before
    `App::frame`;
  - Steam's `GameOverlayActivated_t` callback becomes
    `SystemEvent::FocusLost` when the overlay opens and `FocusGained`
    when it closes, sent from `Macroquad::system_events`
    (`crates/app/src/backend.rs:272`). The game's hook hears it like any
    focus change. **Either order with 3042:** if 3042 is done, the
    overlay's open and close feed its `Focus` tracker
    (`crates/app/src/focus.rs`), so held input is let go, pads are
    ignored while the overlay is open, and no focus event comes twice;
    if not, send the two events straight, and 3042 feeds them to its
    tracker (its clause says so);
  - one handle for everything else: this ticket's backends, `shuyi-net`'s
    Steam transport and lobbies (1144), presence (3055), and the game's
    own Steam code (its on-screen keyboard, and its Workshop in the
    game's 1024). None of them starts Steam or pumps callbacks. If the
    game's 0903 already starts Steam in its own app crate, the PR says
    that the game's pin bump drops that start and takes this handle.
- **The fake of the Steam calls** is a small trait over the calls this
  ticket uses, in one module of `shuyi-app`, so later Steam services
  (3055's rich presence) add their calls to it.
- **`SteamAchievements`**, implementing 3036's `Achievements` over
  `UserStats`: it asks for the current stats at start, unlocks with
  `set_achievement` and `store_stats`, sets stats, and shows progress
  toward a goal with `indicate_achievement_progress`. Calls before
  Steam has answered are queued, then sent.
- **Either order with 3054 (leaderboards):** if 3054 is done, add
  `SteamLeaderboards`, implementing its `Leaderboards` over `UserStats`:
  - each board is found or made once with `find_or_create_leaderboard`
    (its sort from 3054's `BoardDef::best`, its display from `shown`);
  - scores are uploaded with "keep best". Steam's answer becomes
    `Submitted`: `new_best` from whether the score changed, `rank` from
    the new global rank;
  - entries are read globally and around the player, plus a new
    `Range::Friends` read among friends (3054's local backend answers
    it with `Failed`);
  - calls are queued until Steam answers, with the fake.

  Without Steam running, keep 3054's `LocalLeaderboards`. If 3054 is not
  done, leave leaderboards out: 3054 adds `SteamLeaderboards` over this
  ticket's `steam` feature.
- **Steam's game language** (`ISteamApps::GetCurrentGameLanguage`):
  with the `steam` feature, `shuyi-app` puts it first in
  `PlatformFacts::languages` (2349, in 3053's facts), mapped from Steam's API language names to tags
  (`schinese` to `zh-Hans`, `tchinese` to `zh-Hant`, `brazilian` to
  `pt-BR`, `latam` to `es-419`, `koreana` to `ko`, the others by their
  language), so a player who set the game's language in Steam starts in
  it until they pick one in the game's options (2349's order).
- **Cloud saves:**
  - **Auto-Cloud by default.** The docs (`docs/targets/steam.md`) name
    the folder the native backend saves into on each OS, which the game
    enters on the Steamworks site. No code. The patterns the docs give
    sync each save's `<key>.ron` and its `<key>.ron.bak` (3045: synced
    without its backup, a save deleted on one machine would come back
    from the backup on another), and leave out the `<key>.ron.tmp` a
    crash may leave, which stays on the machine.
  - The `window` record that 3044's backend keeps in the same folder
    (`window.ron`) is per computer: the Auto-Cloud setup in the docs
    leaves it out of the synced files. `SteamStorage` never sees it,
    since 3044 writes it through the native `FileStorage`, not through
    the storage handed to the game.
  - **`SteamStorage`**, implementing `Storage` over `RemoteStorage`
    (write, read, delete, list), for a game that wants each save in the
    Cloud API. It reports Steam's quota errors as `StorageError`.
  - The docs say which to pick: Auto-Cloud unless the game must control
    each file.
- **Handed in through 3036's `StoreServices`.**
  `Macroquad::store_services()` hands in `SteamAchievements` when Steam
  started. Its `available()` is false until Steam has sent the current
  stats, and its calls queue until then.
- **Without Steam running:** the build starts anyway.
  `store_services()` hands in no achievements, so 3036's fallback gives
  `LocalAchievements`. The storage is the normal file storage, and the
  log says why.
- **The toy game** behind `--features steam`, with app id 480 for the
  manual test.
- **A manual test**, written down in the docs: the toy game unlocks an
  achievement and the Steam overlay shows it; a save made on one machine
  appears on another (or after deleting the local copy).

**Out (do not do):**
- the partner account, the store page and the game's own achievements
  (the game's 0903);
- lobbies (1144), and our own server's leaderboards (1148);
- the Steam auth ticket for 1148's accounts and Steam's text filter
  for 1149's chat. Each asks this ticket's handle if it comes after
  this ticket; one that comes first leaves its Steam part to 1144.
  Nothing to do here either way;
- other stores (GOG Galaxy, Epic, consoles): each a ticket of its own
  under 3036's `StoreServices`, when a game ships there and Nick has
  said yes to its SDK (ADR-0006 §5 allows Steam's alone; 3036's *Nick
  input*);
- presence and Steam rich presence (3055, over this ticket's `steam`
  feature, fake and fallback), and invites into a lobby (1144): both
  take the `steam` module's handle;
- Steam Input action sets (3012's Out list).

## Implementation steps

1. Read 3036 and its ADR, 1144 (if done), 3054 (if done), `storage.rs`,
   and in `shuyi-app` the native storage backend, `run`
   (`crates/app/src/lib.rs`), `backend.rs` and 2349's `languages.rs`.
2. Add or reuse the dependency and the `steam` module, and check the
   license and the GNU link. Write a fake of the Steam calls for tests.
3. Write `SteamAchievements` over the fake, and put Steam's game
   language first in the platform's languages.
4. Write `SteamStorage`, with the same fake. If the PR would pass about
   600 changed lines (fixtures aside), move `SteamStorage` to a new
   parked ticket (`write-ticket` skill) blocked by this one. Auto-Cloud
   needs no code.
5. If 3054 is done, write `SteamLeaderboards` with the same fake. If
   that takes the PR past about 600 changed lines, move it to a new
   parked ticket blocked by this one, as step 4 does for `SteamStorage`.
6. Write the fallback without Steam, and `docs/targets/steam.md`.
7. Run the manual test with app 480 if a Steam account is at hand; say
   in the Completion notes whether it was done.
8. Run the gates.

## Acceptance criteria

- [ ] `cargo deny check` passes with the `steam` feature.
- [ ] The GNU link result is recorded, and `AGENTS.md` says what it
      means.
- [ ] With the fake, an unlock before Steam answers is sent once Steam
      does.
- [ ] Only the `steam` module starts Steam and pumps its callbacks: a
      search of the workspace for the start call finds it there alone.
- [ ] With the fake: a relaunch answer stops the start, an overlay that
      opens and closes gives `FocusLost` then `FocusGained`, and the pump
      runs once per frame.
- [ ] If `SteamLeaderboards` is written here: with the fake, a score
      submitted before Steam found the board is uploaded once it does.
- [ ] With the fake, Steam's `schinese` comes first in the platform's
      languages as `zh-Hans`, before the system's.
- [ ] Without Steam running, the toy game starts with local achievements
      and saves.
- [ ] `docs/targets/steam.md` names the save folder per OS and the
      Auto-Cloud patterns (`.ron` and `.ron.bak` synced, `.ron.tmp` and
      the `window` record left out), and when to use `SteamStorage`.
- [ ] All gates in the `run-gates` skill pass, with and without the
      `steam` feature.

## Tests required

- Unit: the queue before Steam answers; quota errors; the fallback;
  the start (relaunch, not running, running); the overlay as focus
  events; one pump per frame; Steam's language names as tags; if
  written here, a score queued until its board is found.
- Integration: the toy game's achievements through the fake.
- Manual: the overlay and a synced save, as above.

## Completion notes

*(Filled in by the session that completes the ticket.)*
