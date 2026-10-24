---
id: "2752"
title: "Parked: Shops and paid services: buy, sell and stock as data, and services that restore pools, end conditions or apply level-ups for a price"
type: feature
milestone: "Engine: dungeon crawlers"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2746", "2750"]
nick_input: none
completed:
---

# 2752 — Shops and paid services

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§13 and §17. Nearly every RPG sells things and sells services: an inn
restores, a healer cures, a smith uncurses or identifies. A shop and a
**paid service** are engine (§17's bar: crawlers, top-down RPGs,
roguelikes' shops, action RPGs' vendors). **Which places a town has,
what they are called and what each offers is the game's**: Wizardry's
tavern, inn, shop and temple are the crawler template's `places.ron`
(2754), not engine types.

It builds on 2746 (items, prices, inventories), 2750 (`rest` refills
charges), 2745 (pools, levels, conditions) and the engine's menus
(2307–2309, done). Forming the party is 2747's roster screen, which a
game lists among its places.

**Either order with 2747:** if 2747 is done, a place in the list may
open its roster screen. If not, places open a shop or pop with their
name, and 2747 adds the roster to what a place may open.

**Who uses it** (ADR-0033 §17's bar for engine work): every RPG with a vendor or a healer: crawlers, top-down RPGs, roguelikes' shops, action RPGs' vendors (ADR-0034).

## Nick input

None.

## Scope

**In:**
- **`shop.ron`**, asset kind `ShopDefs`, format `shop-ron`: named
  shops, each with:
  - `stock`: items (2746 kind ids) with counts or `unlimited`, and
    whether items sold to it join the stock;
  - `sell_rate` (out of 256 of the price);
  - `services`: each with a name (text key), a price (`fixed`, or
    `base + per_level × level`), who it is for (`member`, `party`, an
    `item`) and what it does, one of:
    - `Restore { pools: all | named with shares }`,
    - `EndCondition { condition, chance: from PartyMath | always,
      worse_on_fail: Option<condition> }`,
    - `RefillCharges` (2750's `rest`),
    - `ApplyLevelUps` (2745's due level-ups),
    - `Identify`, `Uncurse` (2746),
    - `Custom(name)` (an event the game resolves);
  - money: which pool is money, and whether it is the party's or each
    member's;
  - fuzz-style test and golden row.
- **`ShopRules: TurnRules`** in `shuyi_rpg::shop`, so a visit replays
  and a bot can shop: `Buy { item, count, member }`, `Sell { item }`,
  `Service { service, target }`, `Pool` (gather money into one member
  when money is per member), `Leave`; refusals for too little money, a
  full inventory, out of stock, a target the service isn't for.
- **The shop screen** (behind `draw`): stock with prices, the party's
  items with sell prices, the services, money shown, confirmations by
  the question widget (2309). Actions only; it declares its actions
  (ADR-0025). It pops with `Leave` (ADR-0018).
- **A place list screen**: a menu of named places from a game's list,
  each opening a shop, the roster (2747) or popping with the place's
  name for the game's own screen. It is a plain menu over the game's
  list, with no place kinds of its own.
- With a party of one and money per member, everything works.

**Out (do not do):**
- named place kinds (inn, temple, tavern): the game's, through services
  and its place list;
- a walkable town (a game may draw its town as a top-down stage);
- the game's prices and numbers (data in the game).

## Implementation steps

1. The format and defs.
2. `ShopRules` with each service and refusal.
3. The shop screen and the place list.
4. Harness tests and snapshots.

## Acceptance criteria

- [ ] A Harness run buys, sells, and uses each service kind, with a
      party of one and with six.
- [ ] Every refusal changes nothing (a property test of `ShopRules`).
- [ ] A cursed item can be taken off after an `Uncurse` service.
- [ ] `frame-png` snapshots of the shop screen and the place list.
- [ ] The golden row passes on x64 and wasm32.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each command, service and refusal; price formulas.
- Property: money never goes below zero; `legal_commands` agrees with
  `check`.
- Integration: the Harness runs.
- Snapshot: both screens.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
