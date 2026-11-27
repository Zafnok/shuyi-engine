---
id: "2142"
title: Games get stats with stacking modifiers, timed effects and pools, and item stacks in containers, from shuyi-core
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2142 — Stats, modifiers, timed effects, pools, item stacks and containers in shuyi-core

## Context

Almost every game past a puzzle has numbers that change: a stat raised
by a ring, lowered by a curse, a speed boost that lasts five seconds, a
poison that ticks, health and stamina that are spent and refilled. It
also has things that are carried: a stack of 20 arrows, a bag that holds
12 things, a chest that holds 40 pounds. A tactics game, a platformer, a
shooter, a farming game and a roguelike all need these. Unreal ships
them as the Gameplay Ability System's attributes and effects. Unity and
Godot users use well-known stat and modifier patterns.

In this engine they are planned only inside parked genre crates:

- 2745 (`shuyi-rpg`): `sheet.ron` with stats, pools, conditions that end
  after rounds or when hit, and levels; `Sheet` with `StatId` and
  `PoolId`.
- 2746: item kinds, stacks and inventories (`Inventory { items, limit }`,
  stacking by kind).
- 2759: a grid inventory over 2746's `Inventory`.
- 2760: pack leaders with rolled monster modifiers, and monsters that
  heal when they give up a chase.

All four are parked until after Visions of Shuyi launches. So a game
that isn't an RPG either pulls in an RPG crate when they come, or writes
its own. `crates/core/src` has no stats, effects or items module today:
`character.rs` is 12 lines of ids, and `timer.rs` has `Countdown` and
`Every`. A search of the open tickets for `StatDefs`, `Modifier`,
`ItemStack`, `shuyi_core::stats` and `shuyi_core::items` finds nothing.

This ticket builds the genre-neutral parts in `shuyi-core`, integers
only, so they save, hash and replay with the state the rules own. The
parked tickets build on them when Nick unparks them (see Scope, Out).
This is a technical extraction: only unparking those genre crates needs
Nick.

**The engine names no stat, pool, effect or item** (`AGENTS.md` rule 4).
A game's data names them. The engine keeps the order of things and the
bookkeeping, never the numbers' meaning or a game's formula (as 2745
already says for its sheets).

**The game.** At the pin `50472bbcd3084555a61728a5d07518f720bd6514`,
`gh search code --repo Zafnok/visions-of-shuyi 'modifier'` and `'buff'`
find the game's own versions in its crates (`crates/core/src/combat.rs`,
`crates/core/src/skill.rs`, `crates/core/src/support.rs`,
`crates/content/src/skill.rs`). This ticket only adds API, so nothing in
the game must change. Whether the game moves onto it later is the
game's choice and a game ticket of its own.

It builds on: `shuyi_core::timer` (`Countdown`, advanced once per step
or once per turn, as `timer.rs`'s *Two kinds of time* says), the state
hash (`shuyi_core::hash`, ordered maps only), formats as plugins with
golden rows (ADR-0007, 2116), public types that can grow (ADR-0039), no
floats in a simulation (ADR-0002 §2, ADR-0032).

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::stats`**, a new module, `no_std` + `alloc`, no floats:
  - ids `StatId(u16)`, `PoolId(u16)`, `EffectId(u16)`, `TagId(u16)`
    and `Source(u32)`. A `Source` is the game's number for what put a
    modifier on (an item worn, an aura, an effect), so it can be taken
    off again by source.
  - **`StatDefs`**, built from data by `StatDefs::build(file) ->
    Result<StatDefs, StatDefsError>`: stats (a name, a base value, a
    least and a most), pools, effects and tags, each name resolved to
    its id. An unknown or repeated name is an error naming it.
  - **`Op`**: `Add(i32)`, `Percent(i32)` (percentage points of the
    value after the adds, all summed and applied once), `Scale(i32)` (in
    thousandths, 1000 = unchanged, applied one after another) and
    `Set(i32)`. **One fixed order**, written in the module's doc and in
    the book: the base, plus every `Add`, then times
    `(100 + Σ Percent) / 100`, then each `Scale` in the order the
    modifiers were added, then the last `Set` added, if any, then
    clamped to the stat's least and most. Each step works in `i64`,
    rounds a division toward negative infinity (`div_euclid`), and
    saturates its result to `i32` before the next step.
  - **`Modifier { stat, op, source }`** and **`Stats`**: a value per
    stat (its base, settable), the modifiers on it, the active effects
    and the pools. `stats.value(&defs, stat) -> i32` computes it each
    call, with no cache, so there is nothing to keep in step.
    `add_modifier`, `remove_source(source) -> usize` (how many came
    off), `set_base`.
  - **Timed effects.** An effect in the data has modifiers, a length
    (`For(u32)`, counted in whatever the rules advance it by, steps or
    turns, or `UntilRemoved`), a stacking rule (`Stack { most: u16 }`,
    `Refresh` or `Ignore`), the effects it replaces (a worse one ends a
    lighter one), the
    triggers that end it (the game's names, such as a game's "hit" or
    "rest"), tags, and an optional tick (`every` N, a pool, an amount).
    `stats.apply(&defs, effect, source) -> Applied`,
    `stats.advance(&defs, n) -> Vec<EffectEvent>` (`Ticked`, `Ended`),
    `stats.trigger(&defs, name) -> Vec<EffectEvent>`,
    `stats.remove(effect)`, `stats.remove_tagged(tag)` (a cure),
    `stats.has_tag(tag)` (a game's "can't act" reads this). An effect's
    modifiers count once per stack for `Add` and `Percent`, and apply
    once per stack for `Scale`. Timers are `Countdown`s.
  - **Pools**: a current value and a most that is a number or a stat
    (`Most::Fixed(i32)`, `Most::Stat(StatId)`), and what happens when
    the most rises (`Keep` or `AddDifference`). The current never goes
    above the most: it is clamped down when the most falls.
    `take(pool, n) -> Result<(), NotEnough>` (changes nothing when
    refused), `spend_up_to`, `give` (clamped), `fill`, `is_empty`.
  - A text dump with the names from the defs (`stats.dump(&defs)`),
    for tests and `shuyi see`.
- **`shuyi_core::items`**, a new module, same rules:
  - `ItemKindId(u16)` and **`ItemKinds`**, built from data: a name, the
    most in one stack (1 for a thing that never stacks), a weight, tags.
  - **`Stack<X = ()>`**: a kind, a count and `extra: X`, the game's own
    fields for one item (2746's known and uses left, 2758's rolls). Two
    stacks merge only when the kind and `extra` are equal
    (`X: Clone + Eq + Serialize`).
  - **`Container<X>`**: slots in order, a `Capacity` (`Slots(u16)`,
    `Weight(u32)` or both), and an optional `accepts` list of tags (a
    quiver takes only arrows). `add(stack) -> Option<Stack<X>>` merges
    into stacks already there first, then fills empty slots, and gives
    back what didn't fit; `add_all` refuses and changes nothing unless
    everything fits; `take(kind, count)`, `take_at(slot, count)`,
    `split(slot, count)`, `move_to(&mut other, slot, count)`,
    `count_of(kind)`, `weight()`, `sort(&kinds)` (by kind id, then
    count falling, a documented rule). Each refusal is an error that
    changes nothing.
- **Formats in `shuyi-assets`** (ADR-0007): asset kind `stat defs`,
  format `stats` (`stats.ron`: the stats, pools, effects and tags), and
  asset kind `item kinds`, format `item-kinds` (`item-kinds.ron`). Each
  is registered in `Formats` beside the others
  (`crates/assets/src/format.rs`), has the fuzz-style test of
  `format/builtin_tests.rs`, and a golden row in
  `crates/assets/tests/golden/formats.ron` (`stat defs: stats`,
  `item kinds: item-kinds`), checked on wasm32 by CI's `determinism`
  job.
- **Public types** (ADR-0039): `StatDefs`, `ItemKinds`, `Stats`,
  `Container`, `EffectDef` and the files' structs may grow: each is
  `#[non_exhaustive]` with a constructor. `Op`, `Most`, `Capacity`,
  `EffectEvent` and the stacking rule are `#[non_exhaustive]` enums. The
  ids are plain values, left open.
- Docs: the modules' rustdoc with a tested example each; `shuyi-core`'s
  crate doc lists the two modules; a section *Stats, effects and items*
  in the book's `site/content/learn/book/rules-as-commands.md`, after
  *Time in rules*; a row in `from-another-engine.md`'s table (Unreal's
  attributes and gameplay effects, Godot's and Unity's stat patterns).

Appendix A's table is drawn from `site/data/engine-terms.toml` (3413,
done): write the stats row there, with the paths it names; `cargo xtask
api-text --check` checks the paths.

**Out (do not do):**
- Any stat, pool, effect, item or slot named in engine code, and any
  formula (damage, levels, prices). A game's rules read the values.
- Equipment slots, item requirements, unknown names, curses, item use
  and abilities: 2746's and 2750's, in `shuyi-rpg`. Levels and
  experience stay 2745's.
- Rewriting the parked genre tickets' plans here. When Nick unparks
  them, 2745 builds its sheet's stats, pools and conditions on
  `shuyi_core::stats`, 2746 builds its items and inventories on
  `shuyi_core::items`, 2759 places 2746's stacks in its grid, and 2760
  puts a leader's rolled modifiers on its stats with one `Source`. Each
  of them names this ticket in its `blocked_by` (the back-references are
  written in the same PR as this ticket).
- A grid-shaped inventory (2759), loot rolls (2758), shops (2752).
- Scripts reading or writing stats (a later script ticket can expose
  them), the editor's inspector, and an ECS component form (2107 and
  2904 decide components).
- Moving the game onto these types (the game's choice).

**Estimated size:** about 1,900 lines (code 900, tests 750, docs 250).

**Split:** past 900 lines, split off steps 6–8 (`shuyi_core::items`,
the `item-kinds` format and their tests and docs) into a new ticket in
the `21xx` block, blocked by this one, and give 2746's back-reference to
it too.

## Implementation steps

1. `crates/core/src/stats.rs` (and `stats/` for its parts): the ids,
   `StatDefs` and its file struct with `build` and `StatDefsError`.
2. `Op`, `Modifier`, `Stats::value` with the fixed order, the `i64`
   sums, `div_euclid` and the clamp. Write the order once, in the
   module's doc, and test it case by case.
3. Effects: `EffectDef`, the stacking rules, `replaces`, triggers,
   ticks, `apply`, `advance`, `trigger`, `remove`, `remove_tagged`,
   `has_tag`, on `Countdown`.
4. Pools: `Most`, the rule when the most rises or falls, `take`,
   `spend_up_to`, `give`, `fill`. `dump`.
5. The `stats` format in `crates/assets/src/stats.rs`: the asset kind,
   `StatsRon`, registration, the fuzz-style test, the golden row.
6. `crates/core/src/items.rs`: `ItemKindId`, `ItemKinds` and its file
   struct, `Stack<X>`, `Capacity`, `Container<X>` and its methods.
7. The `item-kinds` format in `crates/assets/src/item_kinds.rs`, as in
   step 5.
8. Exports at `shuyi-core`'s root for the main types, the crate doc,
   the rustdoc examples, the book's section and the table's row.

## Acceptance criteria

- [ ] A stat of base 10 with `Add(5)`, `Percent(20)`, `Scale(500)` and
      no `Set` is 9: (10 + 5) × 120 / 100 = 18, then 18 × 500 / 1000 = 9.
      With a `Set(3)` added after, it is 3. With a most of 8, the first
      case is 8.
- [ ] `Add(-7)` on a base of 10 with `Percent(-50)` gives 1: 3 × 50 /
      100 rounds 1.5 down to 1. A `Scale(333)` on a value of -3 gives
      -1 (`div_euclid` of -999 by 1000 is -1). Each rounding case is a
      test.
- [ ] A value that would pass `i32::MAX` saturates and then clamps,
      with no panic: a base of 1,000,000 with `Scale(i32::MAX)` twice
      gives the stat's most.
- [ ] `remove_source` takes off every modifier of that source and no
      other, and says how many.
- [ ] An effect `For(3)` ends on the third `advance(1)` and on one
      `advance(5)`, with one `Ended` event each time. A ticking effect
      `every` 2 for 6 ticks 3 times.
- [ ] `Stack { most: 3 }` applied 5 times holds 3 stacks; `Refresh`
      applied again restarts its countdown and keeps one stack;
      `Ignore` leaves the first; an effect that replaces another ends
      the other with an `Ended` event.
- [ ] `trigger` ends exactly the effects that list that trigger.
- [ ] A pool whose most is a stat is clamped down when the stat falls,
      and with `AddDifference` gains what the most gained. `take` past
      the current refuses and changes nothing.
- [ ] `Container::add` of 25 of a kind that stacks to 10, into a
      container of 2 slots holding 4 of it, fills both slots (10 and
      10) and gives back 9. `add_all` of the same refuses and changes
      nothing.
- [ ] A container with `Weight(30)` refuses a stack whose weight would
      pass 30. One that `accepts` a tag refuses a kind without it.
- [ ] Two stacks of one kind with different `extra` never merge.
- [ ] An unknown stat, pool, effect, tag or item kind name in a file is
      an error naming it; the format reports it with the file's path.
- [ ] `Stats` and `Container` serialize, deserialize back equal, and
      `state_hash` gives the same value on x64 and wasm32 (a golden row
      or a test in `crates/core/tests/golden`).
- [ ] The golden rows `stat defs: stats` and `item kinds: item-kinds`
      pass on x64 and wasm32.
- [ ] `shuyi-core` still builds without `std` (the no-std gate).
- [ ] A `grep` finds no stat, pool, effect or item name of any game in
      `crates/core/src/stats*` or `crates/core/src/items*` beyond test
      fixtures.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the order of ops case by case, each rounding and saturation
  case, each stacking rule, `replaces`, triggers, ticks, pools' rules,
  each container method and refusal, `sort`.
- Property: `value` always lies within the stat's least and most; a
  pool's current never passes its most after any sequence of calls;
  `move_to` and `split` keep the total count of each kind across both
  containers; no refused call changes the state (compare `state_hash`
  before and after); a container never holds more than its capacity.
- Integration: the two formats' fuzz-style tests, the golden rows, and
  a replay-style test that applies effects over 600 steps from a fixed
  list of calls and gives the same hash twice and on wasm32.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
