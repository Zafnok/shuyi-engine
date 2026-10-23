---
id: "2750"
title: "Parked: Abilities as data: costs by charges per level or a pool, learned by level or choice, targets, and where each works"
type: feature
milestone: "Engine: dungeon crawlers"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2748"]
nick_input: none
completed:
---

# 2750 — Abilities as data

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§10. Wizardry's spells come in levels, each level with its own charges
that an inn's rest refills; later games spend a pool of points. Spells
are learned on levelling or chosen. Some work in battle, some in the
labyrinth (light, a compass, float), some in town. What a spell does is
the game's (`AGENTS.md` rule 4); its cost, target, where it works and
its bookkeeping are the engine's.

It builds on 2748 (`BattleRules`, `PartyMath`, the `Ability` command)
and 2745 (`Sheet`, pools, levels).

**Either order with 2741:** if 2741 is done, an ability whose target is
`Explorer` names one of `shuyi-crawl`'s explorer effects or the game's,
and the example (2754) passes it to `Explorer`. This module never
depends on `shuyi-crawl` either way (ADR-0033 §1): it returns
`AbilityEvent::ExplorerEffect { name, steps }` and the game applies it.

**Either order with 2746:** if 2746 is done, an item's `uses` casts its
ability through this ticket's `cast`. If not, 2746 wires it.

**Who uses it** (ADR-0033 §17's bar for engine work): crawlers (spells by level), top-down RPGs (magic points), roguelikes (scrolls and wands cast abilities through items), tactics games' skills.

## Nick input

None.

## Scope

**In:**
- **`abilities.ron`**, asset kind `AbilityDefs`, format
  `abilities-ron`:
  - each ability: id, name (text key), `level` (its tier), `cost`:
    `Charge { tier }` (one charge of its tier), `Pool { pool, amount }`
    or `Free`; `target`: `Myself`, `Member`, `Party`, `Enemy`, `Group`,
    `AllEnemies`, `Explorer`; `works_in`: any of `Battle`, `Labyrinth`,
    `Town`; tags;
  - `learning`: per tag (the game's calling words) a table of which
    abilities come at which level, or `choose: n` per level;
  - `charges`: per tag, how many charges of each tier at each level;
  - fuzz-style test and golden row.
- **`Known`** in `shuyi_rpg::ability`, per member: abilities learned
  and charges left per tier; part of the member's sheet data (2745's
  `Sheet` gains it, or the member type carries it beside the sheet).
- **Learning**: `learn_on_level(defs, sheet, rng)` returns what is
  learned or offered on a level up; a `Choose(ability)` command takes an
  offered one.
- **Casting**: `cast(defs, member, ability, target, place) -> Result<Cast,
  AbilityError>` checks known, cost, `works_in`, target kind, and
  returns a `Cast` the caller resolves:
  - in battle, 2748's `Ability` command uses it, and the effect goes
    through a new `PartyMath::ability(caster, ability, targets, rng) ->
    Vec<BattleEvent>`, defaulting to nothing;
  - outside battle, a `CastOutside` command in the party's commands
    resolves through `PartyMath::ability_outside`, and `Explorer`
    targets return `ExplorerEffect`;
  - a zone the game marks as stopping abilities (2741's `Zone`) is the
    game's check: `PartyMath::ability_blocked(place)`.
- **Rest**: `rest(defs, party)` refills charges (and the pools the game
  says rest refills, from `sheet.ron`'s flag on a pool), for the inn
  (2752).
- A party of one with abilities passes the same tests.

**Out (do not do):**
- what any ability does: the game's;
- the menus to cast outside battle (the hub, 2752, and the example's
  camp menu, 2754);
- skills with cooldowns in real time (ADR-0034, 2757).

## Implementation steps

1. The format and defs.
2. `Known`, learning, the `Choose` command.
3. `cast` and its checks; wiring into 2748's `Ability` command.
4. Casting outside battle and the explorer effect event.
5. Rest.

## Acceptance criteria

- [ ] An ability with no charge left, the wrong place or the wrong
      target kind is refused with its own error and changes nothing.
- [ ] Charges of each tier refill at rest, and only then.
- [ ] Levelling from 1 to 10 with a fixed seed learns the same
      abilities every time (a test).
- [ ] A battle with abilities passes 2748's `legal_commands` property
      test.
- [ ] The golden row passes on x64 and wasm32.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: costs, targets, places, learning, rest.
- Property: charges never go negative.
- Integration: a bot battle with abilities ends.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
