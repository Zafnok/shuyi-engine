---
id: "3036"
title: "Parked: Achievements and stats as data: a service with no store in it, a local backend saved with the profile, a test backend, an unlock notice and an achievements screen"
type: feature
milestone: "Engine: store services"
model: sonnet-5
effort: high
status: blocked
blocked_by: []
nick_input: none
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

Rules it keeps:

- **Only platform backends talk to a store** (`AGENTS.md` rule 5). The
  service is a trait the platform hands in, as `Storage` is
  (`crates/ui/src/storage.rs`).
- **Determinism.** Unlocking is an output, like a sound cue: the rules
  never read whether something is unlocked to decide what happens.
  Replays, bots and `frame-png` run with the test backend, so they
  unlock nothing anywhere.
- **Text by key** (ADR-0002 §6): names and descriptions are text keys,
  translated like the rest.

## Nick input

None.

## Scope

**In:**
- **`AchievementsDef`**, an asset kind read from RON (ADR-0007,
  ADR-0019), with a property test that random bytes never panic it:
  - achievements: `id`, `name` and `description` (text keys), `hidden`,
    and an optional `progress: (stat id, goal)`;
  - stats: `id`, `kind` (`Int` only for now), `max`;
  - checks with file and line: duplicate ids, a progress naming an
    unknown stat, a goal above the stat's `max`, a text key missing from
    the language pack (through the existing text check).
- **The service**, in `shuyi_ui::achievements`:
  - `trait Achievements { unlock(id), set_stat(id, i32), unlocked(id)
    -> bool, stat(id) -> i32, reset_all() }`. `reset_all` acts only with
    debug tools on;
  - `Ctx::unlock(id)` and `Ctx::set_stat(id, value)`. An unknown id is a
    debug assertion in debug builds and a log line in release. Unlocking
    twice does nothing; a stat never goes down through `set_stat` (as
    stores treat them);
  - a stat reaching an achievement's goal unlocks it.
  - **Either order with 3003:** if 3003 is done, the platform hands the
    service in through `Platform`; if not, `Game` takes it like
    `Storage`, and 3003 moves it.
- **Backends:**
  - `LocalAchievements`, kept through `Storage` per player profile
    (2310's profiles), for builds with no store;
  - `MemoryAchievements`, for tests, the harness, replays and bots.
- **The notice:** an overlay screen that shows "achievement unlocked"
  with its name for a few seconds, which a game turns on (a store build
  leaves it off, since the store's overlay shows its own).
- **An achievements screen**, an engine screen like options and
  profiles: the list, unlocked or not, progress, and hidden ones as
  "hidden" until unlocked. It describes itself for screen readers if
  2322 is done.
- **The toy game** has two achievements, one with progress, and its
  tests unlock both.

**Out (do not do):**
- Steam (3037), and any other store;
- leaderboards (1148, through the networking services);
- float stats and average-rate stats (a follow-up if a game needs them);
- cloud saves (3037 for Steam; `Storage` already covers local saves).

## Implementation steps

1. Read `storage.rs`, `Ctx`, the profiles screen (2310) and the options
   screen.
2. Write `AchievementsDef`, its format and checks.
3. Write the trait, `Ctx::unlock` and `set_stat`, and the two backends.
4. Write the notice and the achievements screen, with snapshot tests.
5. Add the toy game's achievements.
6. Run the gates.

## Acceptance criteria

- [ ] Unlocking twice, or setting a stat lower, changes nothing.
- [ ] A stat reaching its goal unlocks the achievement once.
- [ ] `LocalAchievements` keeps its state per profile across a restart
      (through `MemoryStorage` in a test).
- [ ] A replay of the toy game unlocks nothing outside its
      `MemoryAchievements`.
- [ ] The achievements screen's snapshot (with a hidden one) is committed
      and reviewed with `frame-png`.
- [ ] Random bytes never panic the format; bad fixtures give errors with
      file and line.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: unlock, stats, progress, unknown ids, `reset_all` with debug
  tools on and off.
- Property: any order of `set_stat` calls ends with the largest value
  and the same unlocks.
- Snapshot / integration: the notice and the achievements screen; the
  toy game's run.

## Completion notes

*(Filled in by the session that completes the ticket.)*
