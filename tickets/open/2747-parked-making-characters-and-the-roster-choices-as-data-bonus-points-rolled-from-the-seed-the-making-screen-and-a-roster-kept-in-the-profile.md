---
id: "2747"
title: "Parked: Making characters and the roster: choices as data, bonus points rolled from the seed, the making screen, and a roster kept in the profile"
type: feature
milestone: "Engine: dungeon crawlers"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2745", "2114"]
nick_input: none
completed:
---

# 2747 — Making characters and the roster

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§12. In Wizardry a character is made by picking a lineage, rolling bonus
points, spending them on stats, and then picking a calling the stats
allow. Daphne makes its protagonist the same way. The names of the
choices, their options and what they give are the game's; the steps,
the roll and the rules of spending are the engine's.

It builds on 2745 (`SheetDef`, `Sheet`, tags, `Party`), 2114 (ranges
and weighted picks on the RNG), the name entry widget and the profiles
screen (2310, done), the list cursor (2307, done) and menu items with
values (2308, done).

**Either order with 2746:** if 2746 is done, an option's starting
items are 2746 kind ids, resolved when the def is built, and
`outcome()` holds the made items. If not, options have no starting
items and `outcome()` is the sheet and its tags, and 2746 adds both.

**Who uses it** (ADR-0033 §17's bar for engine work): crawlers (Wizardry's bonus points), tabletop-style RPGs, roguelikes that make a hero before a run, and tactics games that recruit by making units.

**Either order with 2110:** if 2110 is done, the roster's saved form has a version and a migration through it; if not, a version number only, and 2110 adds the migration.

## Nick input

None.

## Scope

**In:**
- **`make.ron`**, asset kind `MakeDef`, format `make-ron`:
  - `steps` in order. A step is one of:
    - `Choose { name: TextKey, options }`: each option with an id, a
      name, stat changes, starting tags, starting items (2746 ids), and
      `needs` (stats at least, after spending, or tags);
    - `Roll { points }`: bonus points as a weighted table or a range
      (2114), with `reroll: unlimited | count | none`;
    - `Spend`: the rolled points spent on stats within each stat's
      least and most, as long as points remain;
    - `Name { max_len }`;
  - an option whose `needs` aren't met is shown greyed (2307's disabled
    rows) and refused;
  - fuzz-style test and golden row.
- **`MakeRules: TurnRules`** in `shuyi_rpg::make`, so making a
  character replays and a bot can do it:
  - commands `Pick(option)`, `Reroll`, `Raise(stat)`, `Lower(stat)`,
    `SetName(String)`, `Back`, `Done`;
  - `check` refuses an option not met, a raise with no points or past
    the most, a lower below the start, `Done` before every step is done;
  - `outcome()` is the made `Sheet` with its tags and items;
  - rolls come from the seed (the game passes a stream from its
    `SimRng`).
- **The roster**: `Roster<M>` in `shuyi_rpg::roster`: the characters
  made, by a stable id, kept in the profile through the storage the
  engine already has (one key per roster), with a format version and a
  migration path (2110's, if it is done). Commands `Join(id)` and
  `Leave(id)` move a character into and out of the `Party` (refused for
  the fixed member, or past `max`), and `Retire(id)` removes one.
- **The making screen** (behind `draw`): built from the engine's menus:
  a page per step, the sheet so far beside it, the points left, the
  options with disabled rows, the name entry from 2310, `Back` on every
  page. Actions only (`input-actions` skill). It pops with the made
  sheet or nothing (ADR-0018).
- **The roster screen** (behind `draw`): the roster on one side, the
  party on the other, join and leave, the fixed member marked. With
  `max: 1` it shows the one place.
- **Either order with 2752:** if 2752 is done, let its place list
  open the roster screen as one of a game's places; if not, nothing
  to do (2752 does it).

**Out (do not do):**
- portraits and their pickers (the game's pictures through its skin;
  a slot for one is in the screen's layout);
- the hub's other places (2752);
- what a lineage or calling means beyond its numbers and tags.

## Implementation steps

1. The format, defs and errors.
2. `MakeRules` with tests of every refusal, and a bot that makes a
   character from `legal_commands()` with a seed.
3. `Roster` and its storage.
4. The two screens and their Harness tests and snapshots.

## Acceptance criteria

- [ ] The same seed and commands make the same character (a test).
- [ ] A calling whose needs aren't met can't be picked, and becomes
      pickable after raising the stat (a test).
- [ ] A random legal bot always finishes making a character within 200
      commands, over 100 seeds (a property test).
- [ ] The roster survives a save and load through the headless
      storage.
- [ ] Harness tests make a character and form a party of one and a
      party of six; `frame-png` snapshots of both screens.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each step kind, refusals, the roster's commands.
- Property: the random bot finishes; points never go negative.
- Integration: Harness runs of both screens; storage round trip.
- Snapshot: both screens.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
