---
id: "2748"
title: "Parked: Party battles as turn rules: enemy groups at ranges, a choosing phase per member, resolution by the game's math, surprise, running, and unknown enemies"
type: feature
milestone: "Engine: dungeon crawlers"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["2745", "2114"]
nick_input: none
completed:
---

# 2748 — Party battles as turn rules

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§9. Wizardry's battle: up to four enemy groups ("3 Orcs, 2 Kobolds"),
each member picks an action in turn, then everyone acts in an order
from speed and luck, and the log says what happened. Enemies are known
by an unknown name ("Man in robes") until identified. The same shape
serves front-view JRPG battles (early Dragon Quest).

Battles are turn rules, so the bots (ADR-0027, ADR-0029), playtests
and autobalance (2126–2130, 2532) play them as they are, through
`legal_commands()`. A party of one is the case Nick wants most: one
choice a round, and it must feel complete.

It builds on 2745 (`Party`, `Sheet`, conditions, rows) and 2114
(ranges and weighted picks).

**Either order with 2746:** if 2746 is done, `Item` actions use its
inventories and `Use` command here, `reach` gets the member's
`Equipment`, and `PartyMath` takes 2746's `ItemMath` as a supertrait.
If not, the `Item` action is declared and refused with
`BattleError::NoItems`, `reach` takes no equipment, and 2746 wires
the action and `Equipment` and adds the supertrait.

**Who uses it** (ADR-0033 §17's bar for engine work): dungeon crawlers and front-view party RPGs (early Dragon Quest, Etrian Odyssey); a party of one makes it a one-hero front-view RPG.

## Nick input

None.

## Scope

**In:**
- **`enemies.ron`**, asset kind `EnemyDefs`, format `enemies-ron`: each
  kind with an id, a name and an unknown name (text keys), a `Sheet` of
  the game's stats and pools (2745's def), tags, the experience it
  gives, a drop table id (2751 reads it), and a look id for the skin.
  Fuzz-style test and golden row.
- **`BattleState`** in `shuyi_rpg::battle`: the party's members (by
  index into the game's `Party`, with their sheets), up to four
  `Group { kind, members: Vec<Sheet>, range: u8, known: bool }`, the
  round, the phase (`Surprise(side)`, `Choosing(member)`, `Resolving`,
  `Over(outcome)`), the picks so far, and the battle's `SimRng`
  stream. It hashes and dumps as text.
- **The game's numbers**: `trait PartyMath` with
  `initiative(&Sheet) -> i32`, `to_hit(attacker, target) -> u8` (out of
  256), `harm(attacker, target, &Action, &mut impl RandomSource) -> Harm`,
  `run_chance(party, groups) -> u8`, `surprise_chance(...) -> (u8, u8)`,
  `identify(kind, party) -> u8`, `reach(member_row, &Equipment-or-none,
  group_range) -> bool`, `xp_share(...)`. Each has a default good enough
  for tests (the defaults name no stat: they read the def's first stat
  and first pool, and say so in their docs).
- **The enemies' minds**: `trait EnemyMind` with `choose(state, group,
  member, rng) -> Action`; the default picks a random legal action.
- **`BattleRules<M: PartyMath, E: EnemyMind>: TurnRules`**:
  - commands `Fight(group)`, `Defend`, `Ability { id, target }` (2750
    resolves abilities; until then refused with `NoAbilities`), `Item {
    item, target }`, `Run`, `Custom(name, target)`, `Back` (undo the
    last pick this round), and `Next` to start resolving when every
    able member has picked (or automatically on the last pick, by a
    setting in `battle.ron`);
  - `Fight` is refused when `reach` says the member can't reach the
    group (back row, by default, reaches no group in melee);
  - resolution orders every member's and enemy's action by initiative,
    ties broken by the RNG; an action whose actor or target fell earlier
    is skipped or retargeted to the same group, as `battle.ron` says;
  - events (`BattleEvent`): `Acted`, `Missed`, `Harmed { pool, amount }`,
    `Condition { added | ended }`, `Fell`, `GroupGone`, `Identified`,
    `Ran`, `RunFailed`, `Surprised`, `Won { xp, drops }`, `Lost`;
    `drops` holds the fallen enemies' drop table ids, and 2751's
    `roll_drops` rolls them;
  - outcome: `Won`, `Ran`, `Lost` (2745's `beaten()`);
  - conditions tick at the end of each round (`after_rounds`).
- **`battle.ron`**, format `battle-ron`: groups at most (default 4),
  ranges, retarget rule, auto-start of resolution, whether known enemies
  stay known across battles (kept by the game).
- **Unknown names**: a group shows its unknown name until `known`; the
  start of each round rolls `identify` for unknown groups.
- **Bots**: a test plays 1000 seeded battles with the rollout bot
  (2125) and with the random bot, at party sizes one and six, and each
  ends.

**Out (do not do):**
- the battle screen and its log (2749);
- abilities (2750), encounter tables and drops (2751);
- chests: the game's own code (the crawler example's, 2754);
- any formula beyond the trait's placeholder defaults;
- real-time battles (ADR-0034).

## Implementation steps

1. The enemies format and defs.
2. `BattleState`, the traits and their defaults.
3. Choosing, `Back`, reach.
4. Resolving: order, skips, retargets, events, outcomes.
5. Surprise, running, identifying.
6. Bot tests at sizes one and six.

## Acceptance criteria

- [ ] `legal_commands()` equals the commands `check` accepts in every
      phase (a property test over random battles).
- [ ] A party of one plays a full battle to `Won`, `Ran` and `Lost` in
      three tests.
- [ ] The same seed and commands give the same events and hash.
- [ ] A back-row member can't `Fight` by default; with a `reach` that
      allows it, it can.
- [ ] 1000 seeded battles with the rollout bot end, at sizes one and
      six.
- [ ] The golden rows pass on x64 and wasm32.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each command and refusal; ordering; skips; surprise; running.
- Property: `legal_commands` agrees with `check`; pools never go below
  their least.
- Integration: the bot runs.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
