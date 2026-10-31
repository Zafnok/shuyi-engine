---
id: "3036"
title: "Parked: Achievements and stats as data, and the one shape of every store service: a StoreServices set the platform hands in, a local backend saved with the profile, a test backend, an unlock notice and an achievements screen"
type: feature
milestone: "Engine: store services"
model: implementation
effort: high
status: blocked
blocked_by: ["2367", "3040"]
nick_input: decision
completed:
---

# 3036 — Achievements and stats as data

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; and 2026-10-20: "make sure steam achievements and cloud saves are covered by some ticket so it's not lost"): the game doesn't need it before parity, so this waits until after the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-20 (`docs/decisions.md`): Steam achievements and cloud
saves must have a ticket. Godot (GodotSteam) and Bevy
(`bevy_steamworks`) both reach them through a plugin over Steamworks.

This ticket is the part with no store in it: what an achievement is,
how a game unlocks one, and where it is kept when no store is there (a
DRM-free build, itch.io, the web). 3037 adds Steam behind the `steam`
feature, and other stores can plug in the same way later.

Found again by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7,
Unity 6 and Unreal 5.8, with GameMaker and Defold:

- Unreal's Online Subsystem gives every store one family of service
  interfaces. An interface the platform lacks comes back empty, and the
  game's code runs on.
- Unity's Platform Toolkit puts accounts, achievements and saves behind
  one API.
- Godot and Bevy reach Steam through GodotSteam and `bevy_steamworks`.
  GameMaker and Defold ship each store as an extension of its own.

Shuyi has no store code yet. `Platform`
(`crates/platform/src/platform.rs:189-219`) hands in storage, the sound
device and the system's keyboard, and nothing for a store. Achievements
are the first store service. Steam's (3037), leaderboards (3054) and
presence (3055) follow. So this ticket also writes down in an ADR the
shape every store service takes. Otherwise the second service would copy
the first by example, and each would add its own `Platform` method.

The notice must not stop the game under it. `ScreenStack::update`
updates the top screen only (`crates/ui/src/screen.rs:290`), so a notice
pushed as a plain screen would stop a tick stage for its few seconds,
and over a turn-based screen it would take the player's presses. 2367
(parked, in `blocked_by`) adds overlay screens that let the screen below
run, and this notice is its first consumer.

Rules it keeps:

- **Only platform backends talk to a store** (`AGENTS.md` rule 5). The
  service is a trait the platform hands in through
  `Platform::store_services()`, as it hands in `Storage` through
  `Platform::storage()` (`crates/platform/src/platform.rs:209`,
  `crates/ui/src/storage.rs`).
- **Determinism.** Unlocking is an output, like a sound cue: the rules
  never read whether something is unlocked to decide what happens.
  Replays, bots and `frame-png` run with the test backend, so they
  unlock nothing anywhere.
- **Text by key** (ADR-0002 §6): names and descriptions are text keys,
  translated like the rest.

## Nick input

**Decision**, asked with the `ask-nick` skill when this ticket is
unparked. The work doesn't wait for the answer: until Nick answers, the
ADR names Steam's backend only.

> Beyond Steam, which stores should the engine plan for: itch.io (no
> SDK; the local backend is all it needs), GOG Galaxy, the Epic Games
> Store, web portals such as Poki and CrazyGames, consoles? ADR-0006 §5
> allows no proprietary SDK except Steam's, behind its feature, so a
> store whose SDK is proprietary needs your yes before a ticket adds it.

The answer goes in `docs/decisions.md`, whose 2026-10-20 entry lists
this among the decisions left for later. The ADR names the stores he
picks, and each becomes a ticket of its own when a game ships there.

## Scope

**In:**
- **An ADR** (`write-adr` skill, the next free number): *store services
  have one shape*. It extends ADR-0024 (storage handed over) and says:
  - each service is a trait in `shuyi_ui`, gathered in a
    `StoreServices` set that `Platform::store_services()` hands in.
    Every member is `None` by default. The engine then falls back to a
    local backend where one exists (achievements), or the game hides
    what needs the service;
  - every service trait has `available()`, so a screen can hide or grey
    out a row while a store isn't running;
  - no call blocks. Calls return at once, and a backend keeps in order
    what comes before its store is ready (3037's queue). A service that
    answers (a leaderboard's top ten, 3054) gives its answers as events
    the game reads on a later frame;
  - outputs only: the rules never read a service to decide what happens.
    Replays, bots, the Harness and `frame-png` run on memory backends;
  - text a store shows (a name, a description, a presence line) is text
    keys (ADR-0002 §6);
  - one client per store SDK per process, owned by the platform backend.
    For Steam it is the `steam` module in `shuyi-app`, written by 3037
    or 1144, whichever is done first. `shuyi-net` and a game's own Steam
    code take its handle and never start Steam;
  - the services, by name: achievements and stats (this ticket; Steam's
    in 3037), leaderboards (3054; 1148's server is another backend) and
    presence (3055). Invites into a Steam lobby are not a member: they
    are `shuyi-net` session events (1144) that take the `steam` module's
    handle. Nor are cloud saves: they are `Storage` (3037's
    `SteamStorage`).
    Identity and sign-in, entitlements and DLC, and workshop items are
    named only, each a ticket when a game ships on a store that needs it;
  - stores: ADR-0006 §5 allows no proprietary SDK except Steam's. Any
    other store's SDK needs Nick's yes first (*Nick input*).
  Its row goes in `docs/adr/README.md`.
- **`AchievementsDef`**, an asset kind read from RON (ADR-0007,
  ADR-0019), with a property test that random bytes never panic it:
  - achievements: `id`, `name` and `description` (text keys), `hidden`,
    and an optional `progress: (stat id, goal)`;
  - stats: `id`, `kind` (`Int` only for now), `max`;
  - checks with file and line: duplicate ids, a progress naming an
    unknown stat, a goal above the stat's `max`, a text key missing from
    the language pack (through the existing text check).
- **The service**, in `shuyi_ui::achievements`:
  - `trait Achievements: Debug { available() -> bool, unlock(id),
    set_stat(id, i32), unlocked(id) -> bool, stat(id) -> i32,
    reset_all() }`, `Debug` as `Storage` is
    (`crates/ui/src/storage.rs:19`). `available` says whether the backend
    takes calls now: a store that isn't running says false, and both
    backends here always say true. `unlock` and `set_stat` return at
    once and never block. `reset_all` acts only with debug tools on;
  - `Ctx::unlock(id)` and `Ctx::set_stat(id, value)`. An unknown id is a
    debug assertion in debug builds and a `Warn` line in release
    (`Ctx::log`, category `Game`, 3040). Unlocking twice does nothing; a
    stat never goes down through `set_stat` (as stores treat them);
  - a stat reaching an achievement's goal unlocks it.
  - **Handed in by the platform** (3003 is done). A `StoreServices` set
    lives in `shuyi_ui::store_services`, a new
    `crates/ui/src/store_services.rs` beside `storage.rs`. It is not
    called `Services`, which would read as 1148's `shuyi-services` server
    crate. It is `#[non_exhaustive]` (ADR-0039),
    and `achievements: Option<Box<dyn Achievements>>` is its only member
    for now.
    - `Platform::store_services()` (`crates/platform/src/platform.rs:189`)
      returns it. Its default body returns `StoreServices::default()`,
      with every member `None`, so the macroquad backend
      (`crates/app/src/backend.rs:241`) and the test platforms need no
      change. 2358's ADR asks a default of every method added to
      `Platform` after ADR-0024, so a backend with no store, a private
      console's among them, changes nothing.
    - `App::load` (`crates/platform/src/app.rs:475`) passes it in `Setup`
      (line 117) as it passes `storage`, and `Setup::ctx` (line 140)
      gives it to the context.
    - With no `achievements` from the platform, the context uses
      `LocalAchievements` over its storage.
    - The headless backend (`crates/platform/src/headless.rs:332`),
      which the Harness and `frame-png` run on, hands in
      `MemoryAchievements`.
- **Backends:**
  - `LocalAchievements`, kept through `Storage` per player profile
    (2310's profiles), for builds with no store;
  - `MemoryAchievements`, for tests, the harness, replays and bots.
- **The notice:** an overlay screen whose `passes` is
  `Passes::Everything` (2367), so a running game under it keeps running
  and keeps getting the player's input. It shows "achievement unlocked"
  with its name for a few seconds, counted by adding up
  `FrameInput::dt()`, then pops itself. A game turns it on (a store
  build leaves it off, since the store's overlay shows its own). It
  takes its heading as a text key and its line as text, so another
  service's notice (3054's "new best") reuses it.
- **An achievements screen**, an engine screen like options and
  profiles: the list, unlocked or not, progress, and hidden ones as
  "hidden" until unlocked. **Either order with 2322:** if 2322 is done,
  it describes itself for screen readers (`Screen::describe`); if not,
  2322 describes it.
- **The toy game** has two achievements, one with progress, and its
  tests unlock both.

**Out (do not do):**
- Steam (3037), and any other store;
- leaderboards (3054, which follows this ticket's shape and reuses its
  notice; Steam's boards come with 3037 or 3054, whichever is done
  second; our own server's with 1148);
- presence, the status line friends see (3055), which follows this
  ticket's shape: a trait the platform hands in, a memory backend, text
  by key;
- float stats and average-rate stats (a follow-up if a game needs them);
- cloud saves (3037 for Steam; `Storage` already covers local saves);
- a `services` list in target profiles, and a `shuyi check --target`
  warning about it. A store is a way to ship a desktop build, not a
  target. Not until a second store backend exists, and then beside
  3004's export packages;
- voice chat: ADR-0006 §8 scopes chat, and 1149 leaves voice out;
- web portals' SDKs (Poki, CrazyGames): each needs a JS plugin of our
  own (game ADR-0034), so a ticket only if Nick names one;
- identity and sign-in, entitlements and DLC, workshop items: named in
  the ADR only, each a ticket when a game ships on a store that needs it
  (the game's 1024 plans its own Workshop, on the one Steam client);
- the Steam client (3037, or 1144 if it comes first).

## Implementation steps

1. Read `storage.rs`, `crates/platform/src/platform.rs` (`Platform`),
   `Setup` and `App::load` in `crates/platform/src/app.rs`, `Ctx`,
   ADR-0006 §5, ADR-0024, 2367's `Passes` and its ADR, the profiles
   screen (2310) and the options screen.
2. Ask Nick the *Nick input* question (`ask-nick` skill) and record the
   answer. Don't wait on it to go on.
3. Write the ADR (`write-adr` skill) and its row in
   `docs/adr/README.md`.
4. Write `AchievementsDef`, its format and checks.
5. Write `StoreServices`, `Platform::store_services()` with its default
   body, the `Setup` field, the trait, `Ctx::unlock` and `set_stat`, the
   two backends, and the headless backend's `MemoryAchievements`.
6. Write the notice (a `Passes::Everything` screen) and the achievements
   screen, with snapshot tests. If the PR would pass about 600 changed
   lines (fixtures and snapshots aside), keep everything else here and
   move the notice and the screen to a new parked ticket (`write-ticket`
   skill) blocked by this one and by 2367, and add that ticket to
   3054's `blocked_by` (its "new best" reuses the notice).
7. Add the toy game's achievements.
8. Run the gates.

## Acceptance criteria

- [ ] Unlocking twice, or setting a stat lower, changes nothing.
- [ ] A stat reaching its goal unlocks the achievement once.
- [ ] `LocalAchievements` keeps its state per profile across a restart
      (through `MemoryStorage` in a test).
- [ ] A replay of the toy game unlocks nothing outside its
      `MemoryAchievements`.
- [ ] Unless step 6 moved it: the achievements screen's snapshot (with a
      hidden one) is committed and reviewed with `frame-png`.
- [ ] Random bytes never panic the format; bad fixtures give errors with
      file and line.
- [ ] The ADR and its row in `docs/adr/README.md` are in the PR, and
      Nick's answer on stores, once given, is in `docs/decisions.md`.
- [ ] A platform that doesn't override `store_services()` gets
      `LocalAchievements` over its storage, and the headless backend gets
      `MemoryAchievements`. Both say `available()`.
- [ ] Unless step 6 moved it: Harness: the notice over a test screen
      that counts `ticks()` and Confirm presses: the count and the
      presses go on while it shows.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: unlock, stats, progress, unknown ids, `reset_all` with debug
  tools on and off; `StoreServices::default()` has every member `None`;
  `Setup` carries the platform's set into the context; the fallback to
  `LocalAchievements` without one; `available()`.
- Property: any order of `set_stat` calls ends with the largest value
  and the same unlocks.
- Snapshot / integration: the notice over a screen that keeps running,
  and the achievements screen; the toy game's run.

## Completion notes

*(Filled in by the session that completes the ticket.)*
