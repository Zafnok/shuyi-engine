---
id: "2751"
title: "Parked: Encounters and drops: encounter tables per area, fixed encounters by mark, and drop tables"
type: feature
milestone: "Engine: dungeon crawlers"
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["2748", "2746", "2114"]
nick_input: none
completed:
---

# 2751 — Encounters and drops

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§11 and §17. Random encounters by area and what a won fight leaves are
shared by crawlers (Wizardry, Etrian Odyssey), top-down party RPGs
(early Dragon Quest and Final Fantasy) and monster-collecting games, so
they are engine (§17's bar). **Trapped chests to inspect and disarm are
Wizardry's own habit**, so they aren't here: the crawler example and
its template part carry them as code the game owns (2754).

Encounters are keyed by an **area name**, a plain string, so this works
with `shuyi-crawl`'s `Entered { area }` and `Encounter(mark)` events
(2741) and with a top-down game's own areas. `shuyi-rpg` doesn't
depend on `shuyi-crawl` (ADR-0033 §1).

It builds on 2748 (`BattleState`, `EnemyDefs`' drop table id), 2746
(`Item`) and 2114 (weighted picks).

**Who uses it** (ADR-0033 §17's bar for engine work): crawlers, top-down RPGs with random encounters, monster-collecting games, and roguelikes' drops.

## Nick input

None.

## Scope

**In:**
- **`encounters.ron`**, asset kind `EncounterDefs`, format
  `encounters-ron`:
  - per area name: `chance` per step (out of 256), and a weighted table
    of entries, each up to the battle's group limit of `(enemy kind,
    count range, range)`, with an optional surprise bias;
  - `fixed`: per mark name, an entry and `once` or `until_beaten`;
  - `drops`: named drop tables of weighted entries, each an item (2746
    kind id, count range, `known: bool`), money as a range, or the
    game's own drop by name (`Custom(name)`, which the game turns into
    whatever it likes: a chest, a key, a story flag);
  - fuzz-style test and golden row.
- **`Encounters`** in `shuyi_rpg::encounter`:
  - `after_step(defs, area, rng) -> Option<BattleSetup>` rolls the
    area's chance, then the table;
  - `at_mark(defs, mark, done) -> Option<BattleSetup>` for fixed
    encounters, with the done-set the game keeps and saves;
  - `BattleSetup` builds a 2748 `BattleState`.
- **Drops**: on `Won`, `roll_drops(defs, groups, rng) -> Drops { money,
  items, custom: Vec<String> }`, one roll per fallen enemy's table.
- Neither the chance nor the table knows party size; tests run with a
  party of one and of six anyway.

**Out (do not do):**
- trapped chests and any other way of handing out a drop: the game's
  (the crawler template's chest, 2754);
- loot rolled with affixes (ADR-0034, 2758);
- encounters that walk on the map (a game's things, 2742's things list).

## Implementation steps

1. The format and defs.
2. `after_step`, `at_mark`, `BattleSetup`.
3. `roll_drops`.

## Acceptance criteria

- [ ] Over 10 000 seeded steps an area's encounter rate is within 2% of
      its chance (a test with a fixed seed list).
- [ ] A `once` fixed encounter never comes back after it was met; an
      `until_beaten` one comes back until won.
- [ ] The same seed gives the same drops.
- [ ] The golden row passes on x64 and wasm32.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: rolls, fixed encounters, drops, `Custom` drops passed through.
- Integration: the golden table.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
