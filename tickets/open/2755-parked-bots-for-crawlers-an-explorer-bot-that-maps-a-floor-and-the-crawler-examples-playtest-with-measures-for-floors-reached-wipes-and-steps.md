---
id: "2755"
title: "Parked: Bots for crawlers: an explorer bot that maps a floor, and the crawler example's playtest with measures for floors reached, wipes and steps"
type: feature
milestone: "Engine: dungeon crawlers"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2744", "2754"]
nick_input: none
completed:
---

# 2755 — Bots for crawlers

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

The engine's bots, playtests and autobalance (ADR-0027, ADR-0029,
2125–2130, 2532, 2541) play any `TurnRules` game, and a crawler's
battles already work with them (2748). Exploring is different: a random
walker or a short search never maps a floor, so a playtest of a whole
crawler needs a bot that explores: walk to the nearest unseen place,
fight what comes, go down the stairs when the floor is mapped.

With it, `shuyi playtest` can say how often a party of one reaches
floor 2, and `shuyi autobalance` can tune encounter rates and enemy
numbers until a band of bots lands where the game wants.

It builds on 2744 (`frontier`, `route`, travel) and 2754 (the example
whose run is one `TurnRules`).

**Who uses it** (ADR-0033 §17's bar for engine work): every game of 2741's map: crawler RPGs' playtests and autobalance, labyrinth puzzles (proving every room reachable), and auto-explore as a player's feature.

**From 2021 (done):** the example's playtest runs in CI through the reusable workflow `.github/workflows/playtest.yml`: a job in `ci.yml` beside hello-grid's `playtest` job, with `working-directory: examples/crawler` and `paths` covering the example and the crates it is made of (`shuyi-crawl`, `shuyi-rpg`).

**Either order with 2527:** if 2527 is done, the soak runs; if not, 2527 adds the example when it lands.

## Nick input

None.

## Scope

**In:**
- **`ExplorerBot`** in `shuyi_crawl::bot`: given the game's state
  through a small trait (`CrawlBotView`: the map, the explorer, whether
  the game is exploring), it picks the next command:
  - travel toward the nearest frontier cell (2744's `frontier` and
    `route`);
  - `Search` once at each dead end, if the game says secrets exist;
  - stairs down when no frontier is left, or when a goal floor is set;
  - outside exploring (a battle, a chest, the hub) it hands over to
    another bot the game names (the rollout bot, the random bot, or a
    persona, 2130).
- **An `Entrant` for playtests** (the bots module's `Entrant` trait): the
  explorer bot joined with a battle bot, so `shuyi playtest` names it.
- **The example's playtest** (2754's game): a `playtest.ron` with
  measures `floor_reached`, `wiped`, `steps`, `battles`, and bands for a
  party of one and a party of six, run in CI as the other examples'
  playtests are (2021's workflow, a caller job in `ci.yml`).
- **A soak** (2527, if done): `shuyi soak` over the example with the
  explorer bot finds no panic and no hash mismatch in 1000 runs. The
  example's tools implement `soak` over `shuyi_core::soak::play`, which
  takes any bot (2527's `soak::turns` plays random bots).
- **The `shuyi-crawl` skill**, if it exists: a section on the explorer
  bot and the example's playtest, and 2755 dropped from its `waits_for`
  (2741 put it there).

**Out (do not do):**
- tuning the example's numbers beyond making its bands pass;
- bots that make characters (2747 has its random bot);
- real-time bots (ADR-0034).

## Implementation steps

1. The `CrawlBotView` trait and `ExplorerBot`.
2. The `Entrant` that joins it with a battle bot.
3. The example's measures and `playtest.ron`.
4. CI and the soak.

## Acceptance criteria

- [ ] The explorer bot maps every reachable cell of a test floor with
      no encounters, within twice the floor's cell count in steps (a
      test).
- [ ] `shuyi playtest` runs the example's playtest at both party sizes
      and its bands pass.
- [ ] The same seeds give the same playtest report (determinism).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the bot's choice at a frontier, a dead end, a mapped floor.
- Integration: the playtest; the soak if 2527 is done.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
