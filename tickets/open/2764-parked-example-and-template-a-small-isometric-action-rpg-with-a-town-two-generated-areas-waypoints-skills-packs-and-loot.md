---
id: "2764"
title: "Parked: Example and template: a small isometric action RPG with a town, two generated areas, waypoints, skills, packs and loot"
type: feature
milestone: "Engine: action RPGs"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["2747", "2752", "2759", "2760", "2761", "2762", "2763", "1126", "2119", "2501", "2511", "2004"]
nick_input: sign-off
completed:
---

# 2764 — Example and template: a small action RPG

> **Parked** (Nick, 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other kinds of game, and the real-time work it builds on, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-22: "I also want support for roguelikes, and
Diablo-likes", and "certainly we can build one of each genre as a demo
of the engine". [ADR-0034](../../docs/adr/0034-action-rpgs-a-hero-in-real-time-loot-rolled-from-affix-tables-skills-with-cooldowns-and-packs-of-monsters.md)
§2 and §6, and [ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§17: this is the **demo** of the genre, a small complete game made only
of the engine, with its own story, numbers and art made for it. Nothing
comes from Diablo or any other game (no names, classes, items,
monsters or maps).

The **template part** copies the genre's habits into a new game as
code the game owns: a town portal, a belt of quick-use items, money
picked up by walking over it. They are not engine types (ADR-0034 §2).

It builds on the whole `shuyi-action` set (2756–2762), waypoints
(2763), isometric drawing and picking (1126), generated levels (2119),
template parts (2501), the short path (2511) and guides (2004).

## Nick input

**Sign-off:** Nick plays the example in the browser (the link is in the
PR) and says whether it feels like the Diablo-likes he likes. His
comments become tickets.

## Scope

**In:**
- **`examples/action-rpg`**, a tick game on the isometric projection:
  - one hero made from a short `make.ron` (2747) of one choice;
  - **a town** with a vendor (2752's shop) and a stash (2759);
  - **two areas** generated from the run's seed (2119's `rooms` and
    `caves`, repaired), each with a waypoint (2763), packs from
    `spawns.ron` (2760) and a stronger leader with rolled modifiers;
  - **four skills** (2757): a swing, a missile, an area and a dash, on
    the skill bar (2762), moved and aimed by pointing (2761) or the
    stick;
  - **loot** (2758) with three rarities, a dozen affixes of the
    example's own, two uniques, the inventory grid (2759), ground labels
    and a default loot filter;
  - **the HUD** (2762): two pools, the skill bar, floating numbers;
  - its formulas: an `ActionMath` of its own, with its numbers in its
    data;
  - **the template's habits** as the example's own code: a portal back
    to town, a belt, money picked up by walking.
- **Assets** made for the example (ours, CC0) or permissively licensed
  and recorded in `THIRD_PARTY_ASSETS.md`.
- **Tests**: a Harness run from town through area 1 to area 2's
  waypoint with a scripted input record; a replay determinism test;
  `frame-png` pictures of town, an area mid-fight, the inventory.
- **The web build** and its place on the Pages index (the loop over
  `examples/*/shuyi.ron`), and the website's Examples page if 2801 is
  done.
- **The template part** `kind-action-rpg`, `when: Picked(kind,
  action-rpg)`: turns on `action` and `rpg`, adds one area with one
  pack, one skill, a little loot, and copies the habits above as code
  and data the game owns or deletes. CI's `templates` job makes and
  tests it.
- **The guide** `docs/guides/action-rpg.md` in 2004's order: the
  template part, the modules, what looks related but isn't for it (the
  crawler's turn-based battles, 2748; RTS units, 2707), the example.
- **READMEs** of `shuyi-action` finished (with its measured WASM size),
  the skill, the catalog row and the `action-rpg` answer ready.

**Out (do not do):**
- online co-op (ADR-0034, *Consequences*);
- a skill tree screen (a game's data over 2750's learning);
- crafting.

## Implementation steps

1. The data: make, shop, spawns, skills, loot, waypoints.
2. The stages: town, areas, inventory, travel.
3. The game's `ActionMath` and the template habits.
4. Tests, the web build.
5. The template part, its CI job, the guide, the README, the skill, the
   catalog.

## Acceptance criteria

- [ ] The Harness run reaches area 2's waypoint.
- [ ] The replay test gives the same hashes twice and on wasm32.
- [ ] `frame-png` pictures as listed.
- [ ] The web build runs in the browser pane.
- [ ] CI makes a game from `kind-action-rpg`, builds it and its test
      passes.
- [ ] `cargo xtask check-modules` passes with the guide found.
- [ ] Every asset not made for the example is in
      `THIRD_PARTY_ASSETS.md`.
- [ ] Nick has signed off on the feel (the PR says so).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the Harness run, the replay, `frame-png`, the template's
  game in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
