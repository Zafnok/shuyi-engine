---
id: "2746"
title: "Parked: Items as data: kinds, slots, stacks, requirements, unknown names and identifying, cursed equipment, and inventories per member or shared"
type: feature
milestone: "Engine: dungeon crawlers"
model: implementation
effort: high
status: blocked
blocked_by: ["2745"]
nick_input: none
completed:
---

# 2746 — Items as data

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§8. Wizardry's items: worn in slots, needing a calling, unknown until
identified, sometimes cursed so they can't come off. Roguelikes (2735)
and action RPGs ([ADR-0034](../../docs/adr/0034-action-rpgs-a-hero-in-real-time-loot-rolled-from-affix-tables-skills-with-cooldowns-and-packs-of-monsters.md))
use the same parts, and ADR-0034's loot adds rolled modifiers to an item
(2758), so an item here is a kind plus room for what was rolled.

It builds on 2745 (`Sheet`, tags, `PartyConfig`'s inventory setting).
The engine names no item, slot or tag (`AGENTS.md` rule 4).

**Either order with 2747:** if 2747 is done, give `make.ron`'s
options their starting items (kind ids from here, resolved when the
def is built) and put the made items in `MakeRules::outcome()`; if
not, nothing to do (2747 does it).

**Either order with 2748:** if 2748 is done, wire its `Item { item,
target }` command (refused with `BattleError::NoItems` until now) to
this ticket's inventories and `Use`, pass the member's `Equipment` to
`PartyMath::reach`, and make `ItemMath` a supertrait of `PartyMath`;
if not, nothing to do (2748 does these when it lands).

**Either order with 2750:** if 2750 is done, a kind's ability name
resolves to 2750's `AbilityId` when the def is built, and `Use` casts
it through 2750's `cast`; if not, `Use` returns an event with the
ability's name, and 2750 wires it.

**Who uses it** (ADR-0033 §17's bar for engine work): every RPG with gear: crawlers, top-down RPGs, roguelikes (unknown potions and scrolls), action RPGs (ADR-0034, which adds rolled modifiers), survival games' inventories.

## Nick input

None.

## Scope

**In:**
- **`items.ron`**, asset kind `ItemDefs`, format `items-ron`:
  - `slots`: the game's slot names (for example the game's hands, head,
    body), each with how many of it a member has;
  - `kinds`: each with an id, a name (text key), an unknown name (text
    key; none means always known), a slot or none, `two_slots` (a list
    of slots it fills at once), a stack size (1 for gear), tags,
    `needs` (stat at least a value, a tag the member has, or any of
    several tags), `cursed: bool`, a price, and `uses` (a count, or
    none) and the ability it casts when used (an `AbilityId` 2750
    resolves; a name here);
  - names resolve to ids when built; fuzz-style test and golden row.
- **`Item`** in `shuyi_rpg::item`: `{ kind: ItemKindId, count: u16,
  known: bool, uses_left: Option<u16>, rolled: Vec<(ModId, i32)> }`.
  `rolled` stays empty here; ADR-0034's loot (2758) fills it. Make the
  struct one that can grow (2013's rule, `#[non_exhaustive]` and a
  constructor).
- **Inventories**: `Inventory { items, limit }` per member or one shared
  (as `party.ron` says), with stacking by kind (and by `rolled`, which
  must be equal to stack).
- **Equipment**: `Equipment` per member, by slot. `equip` checks the
  slot, `needs` against the member's sheet and tags, and takes the
  displaced item back to the inventory (refused if it is full). An
  equipped cursed item can't be taken off until `uncurse` (a temple in
  2752, an ability in 2750). Putting on a cursed item makes it known.
- **Commands** as a `TurnRules` piece a game's rules call:
  `Equip`, `Unequip`, `Use` (returns an event with the kind's ability,
  or casts it through 2750's `cast`, as the clause with 2750 says),
  `Give(from, to)`, `Drop`, `Identify` (made known; who may and at what
  chance is the game's, through a small trait of this ticket,
  `ItemMath::identify_chance`, defaulting to always, which 2748's
  `PartyMath` takes as a supertrait), `Split(count)`. Each refusal is
  an error that changes nothing.
- **Names shown**: `item.name_key(defs) -> TextKey`, the unknown name
  until known.
- With a party of one and a shared inventory, everything works (tests).

**Out (do not do):**
- shops and prices paid (2752), drops (2751), chests (the crawler
  template's own code, 2754), affixes and
  rolled modifiers (2758), an inventory grid of shapes (2759);
- what an item does when used or worn: the game's formulas read
  `Equipment` (2748's `PartyMath`).

## Implementation steps

1. The format and defs.
2. `Item`, `Inventory`, stacking.
3. `Equipment` and the curse rule.
4. The commands with their refusals.
5. Tests at party sizes one and six, shared and per member.

## Acceptance criteria

- [ ] Equipping refuses a wrong slot, an unmet need and a full
      inventory, each with its own error and no change.
- [ ] A cursed item, once on, can't come off until uncursed.
- [ ] An unknown item shows its unknown name until `Identify`.
- [ ] Stacks merge and split; gear doesn't stack.
- [ ] The golden row passes on x64 and wasm32.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each command and refusal; stacking; the curse.
- Property: no command changes the total count of a kind except `Use`
  and `Drop`.
- Integration: the golden table.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
