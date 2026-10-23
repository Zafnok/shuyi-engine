---
id: "2745"
title: "Parked: The shuyi-rpg module: a party of one or more, rows, a fixed member, and members' numbers as data (stats, pools, conditions and levels)"
type: feature
milestone: "Engine: dungeon crawlers"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["2003", "2116"]
nick_input: none
completed:
---

# 2745 — The shuyi-rpg module: a party of any size, numbers as data

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-22 (`docs/decisions.md`): "what I would want most is a
single player party blobber (i.e. 1 character only not 4 or 6 party)
but anyway support for any character amount is ok".
[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§1 and §7. `shuyi-rpg` doesn't depend on `shuyi-crawl`: a top-down
party RPG uses it alone, and its sheets serve the action RPGs of
[ADR-0034](../../docs/adr/0034-action-rpgs-a-hero-in-real-time-loot-rolled-from-affix-tables-skills-with-cooldowns-and-packs-of-monsters.md)
too.

**`AGENTS.md` rule 4 is the hard part.** The engine names no stat, pool,
condition, calling or lineage. A game's `sheet.ron` names them, and its
formulas live behind a trait (`PartyMath`, 2748). The module holds the
order of things and the bookkeeping, never the numbers.

What it builds on: turn rules and `SimRng` (ADR-0003 §2), formats as
plugins with golden rows (ADR-0007, 2116), module duties (ADR-0009 §4,
2003), the layers file (ADR-0032).

Items are 2746, making characters and the roster 2747, battles 2748,
the battle screen 2749, abilities 2750, encounters and chests 2751, the
hub 2752.

**Who uses it** (ADR-0033 §17's bar for engine work): dungeon crawlers, top-down party RPGs (early Dragon Quest and Final Fantasy), roguelikes with one hero, tactics games with a roster, and action RPGs' heroes (ADR-0034).

## Nick input

None.

## Scope

**In:**
- **The crate** `crates/rpg` (`shuyi-rpg`), with ADR-0009 §4's
  duties: depends on `shuyi-core` and `shuyi-assets` only; an empty
  `draw` feature (2747, 2749, 2752 fill it); metadata `kind`/`party-rpg`,
  `serves` every perspective; its `tools/layers.ron` line (pure); the
  `rpg` feature of `shuyi`; the catalog row (building, 2745–2752);
  the `party-rpg` answer in `docs/modules.ron` (add it, parked, if 2003
  didn't); the README's first two sections; the `shuyi-rpg` skill or
  its line in 1134's list.
- **`sheet.ron`**, asset kind `SheetDef`, format `sheet-ron`:
  - `stats`: names, each with a least and most value;
  - `pools`: names with a current and a most (the game's hit points,
    its magic points, its money if the party carries it per member);
  - `conditions`: names, each with flags `cannot_act`,
    `cannot_be_targeted`, `counts_as_down`, and how it ends: `after_rounds`,
    `at_rest`, `when_hit`, `never` (until cured), and which conditions
    it replaces (a worse one replaces a lighter one);
  - `levels`: an experience table (the experience to reach each level),
    and per level which pools rise by how much as a range (rolled from
    the seed) or a fixed number;
  - names resolve to `StatId`, `PoolId`, `ConditionId` (`u16`) when the
    def is built, so a typo is a build error with file and line;
  - fuzz-style test and golden row in `crates/rpg/tests/golden/formats.ron`.
- **`Sheet`** in `shuyi_rpg::sheet`: a member's numbers by id: stats,
  pools, conditions with their rounds left, level, experience, tags (the
  game's words, for example a calling, used by item requirements). It
  hashes and dumps as text with the names from the def.
  - `sheet.gain_xp(def, xp, rng) -> Vec<LevelUp>`: a level rises only
    when the game says (Wizardry rises at the inn), so it returns what
    is due and `apply_level_up` applies one;
  - `sheet.add_condition`, `sheet.tick_conditions(EndsAt)`;
  - `sheet.can_act()`, `sheet.is_down()` from the conditions' flags.
- **`party.ron`**, asset kind `PartyConfig`, format `party-ron`: `max`
  (1 to 8, default 6), `front` (how many stand in the front row; ignored
  for a party of one), `fixed_member: bool` (the first member never
  leaves), `inventory: per_member | shared` and its limits (used by
  2746).
- **`Party<M>`** in `shuyi_rpg::party`, generic over the game's member
  type `M: Member` (a trait with `sheet()`, `sheet_mut()` and `name()`),
  so a game keeps its own fields beside the sheet:
  - order, rows (`row_of(index)`), `swap(a, b)` and `move_to(a, i)`
    refused for the fixed member's place if it is fixed;
  - `add` refused past `max`, `remove` refused for the fixed member;
  - `able()` (members who can act, in order), `beaten()` when none can;
  - with `max: 1` there are no rows, and every method still works.
- **Party commands** as a small `TurnRules` piece (`PartyCommand::{Swap,
  MoveTo}`) a game's rules call, so changing the order replays.

**Out (do not do):**
- items (2746), making characters and the roster (2747), battles (2748),
  abilities (2750), encounters and chests (2751), the hub (2752);
- any stat, pool or condition named by the engine, and any formula;
- drawing a party panel (2749).

## Implementation steps

1. The crate and its duties.
2. The `sheet-ron` format and `SheetDef::build` with its errors.
3. `Sheet`: numbers, conditions, levels, the text dump.
4. `party-ron`, `Member`, `Party<M>` and the party commands.
5. Tests at `max: 1` and `max: 6` for every method.

## Acceptance criteria

- [ ] A party of one and a party of six pass the same suite of party
      tests (one parameterised test).
- [ ] The fixed member can't be removed or moved from the front.
- [ ] A condition that ends after three rounds ends after three ticks; a
      worse condition replaces a lighter one as the def says.
- [ ] Level-ups are rolled from the seed: the same seed gives the same
      pools.
- [ ] An unknown stat name in a def is an error naming file and line.
- [ ] The golden rows pass on x64 and wasm32.
- [ ] `grep` finds no stat, pool or condition name in `crates/rpg/src`
      beyond test fixtures.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: def building, sheet methods, conditions, levels, party methods.
- Property: a party never holds more than `max` members; `able()` is a
  subsequence of the order.
- Integration: the golden table on native and wasm32.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
