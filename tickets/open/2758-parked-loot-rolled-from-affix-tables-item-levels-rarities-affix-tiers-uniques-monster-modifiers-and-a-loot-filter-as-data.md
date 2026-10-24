---
id: "2758"
title: "Parked: Loot rolled from affix tables: item levels, rarities, affix tiers, uniques, monster modifiers, and a loot filter as data"
type: feature
milestone: "Engine: action RPGs"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2746", "2114"]
nick_input: none
completed:
---

# 2758 — Loot rolled from affix tables

> **Parked** (Nick, 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0034](../../docs/adr/0034-action-rpgs-a-hero-in-real-time-loot-rolled-from-affix-tables-skills-with-cooldowns-and-packs-of-monsters.md)
§5. An item drops at a level; its rarity says how many modifiers it
rolls; each modifier (an affix) comes from a table filtered by the
item's kind and level, at a tier with a range. Uniques are fixed. The
same tables give stronger monsters their modifiers. The engine names no
affix (`AGENTS.md` rule 4): what a rolled value does is read by the
game's math.

**Who uses it** (ADR-0033 §17's bar for engine work): action RPGs,
looter shooters, MMOs, roguelikes' random artefacts, and crawlers that
want rolled gear (it lives in `shuyi-rpg`, which has no real-time part).

It builds on 2746 (`Item` with its `rolled` list, item kinds and tags)
and 2114 (weighted picks, named streams).

**Either order with 2751:** if 2751 is done, a drop table entry may be
`Rolled { level, rarity_table }` and `roll_drops` rolls it here. If
not, 2751 adds that entry kind.

## Nick input

None.

## Scope

**In:**
- **`loot.ron`**, asset kind `LootDefs`, format `loot-ron`, in
  `shuyi-rpg`:
  - `affixes`: id, group (the game's words, such as two groups a game
    calls prefix and suffix), tags of item kinds it may roll on, a
    weight, and tiers, each with a least item level and a value range
    (or two ranges for a "from–to" value);
  - `rarities`: id, name key, colour role, weight, and per group how
    many affixes (a range);
  - `uniques`: id, the base kind, a fixed list of affixes with ranges,
    a least level, a weight;
  - `exclusive`: affix groups that can't share an item (a family
    rule: one of each family);
  - fuzz-style test and golden row.
- **Rolling**: `roll_item(defs, kind, level, rarity_table, rng) -> Item`
  picks the rarity, then affixes by weight among those allowed, never
  two of one family, the tier the level allows, a value in its range;
  `roll_unique`. Items with rolls stack only with equal rolls (2746).
- **Names**: an item's shown name from its rarity and affixes, by a
  text pattern in the language pack (`{prefix} {base} {suffix}`, the
  game's). **Either order with 2348:** if 2348 is done, the pattern
  may choose words by its selection rules; if not, a plain pattern, and
  2348 needs no change here.
- **Monster modifiers**: `roll_modifiers(defs, table, level, rng) ->
  Vec<(ModId, i32)>` from the same affix tables, for 2760's packs.
  **Either order with 2760:** if 2760 is done, wire its leaders:
  `spawn_area` rolls each pack leader's modifiers through
  `roll_modifiers` from the leader's modifier table in `spawns.ron`,
  with a golden test; if not, nothing to do (2760 calls
  `roll_modifiers` itself).
- **A loot filter** as data: `filter.ron`, the player's: rules in order
  (match by rarity, kind tags, affix present, level) to `show`, `hide`
  or `style` (a colour role, a sound cue); `filter(item) -> Shown`.
  Kept per profile. A game ships its default filter as data; 2764
  puts one in the `kind-action-rpg` template part.
- Determinism: a golden table of 1000 items rolled from a fixed seed,
  equal on native and wasm32.

**Out (do not do):**
- what affixes do: the game's;
- crafting (rerolling, adding affixes): a ticket of its own if a game
  asks;
- labels on the ground (2762 draws them through the filter);
- the inventory grid (2759).

## Implementation steps

1. The format and defs, with the family rule.
2. `roll_item`, `roll_unique`, names.
3. Monster modifiers.
4. The filter format and `filter`.
5. The golden table of rolls.

## Acceptance criteria

- [ ] No rolled item holds two affixes of one family, or a tier above
      its level (a property test over 10 000 rolls).
- [ ] Rarity weights hold within 2% over 10 000 fixed-seed rolls.
- [ ] The golden table of rolls is equal on x64 and wasm32.
- [ ] A filter hides and styles items as its rules say, first match
      wins (tests).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each rule of rolling; names; filters.
- Property: the two invariants above.
- Integration: the golden table.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
