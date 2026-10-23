# ADR-0034: Action RPGs: a hero in real time, loot rolled from affix tables, skills with cooldowns, and packs of monsters

- **Status:** Accepted
- **Date:** 2026-10-22
- **Related tickets:** 2756–2764, all parked until after Visions of
  Shuyi launches; 1119 (tick simulation), 1121 (pointer input), 1122
  (collision and movers), 1126 (isometric), 2106 (paths for free
  movement), 2119 (generated levels), 2123 (hit-stop), 2124 (collision
  queries), 2745–2752 (`shuyi-rpg`, ADR-0033)
- **Extends:** ADR-0003 (stages, tick simulation, isometric), ADR-0009
  (modules), ADR-0015 §1 (the *Kind of game* concern), ADR-0033 §1 and
  §17 (`shuyi-rpg`, and the bar for engine, template and example)

## Context

Nick, 2026-10-22 (`docs/decisions.md`): "I also want support for
roguelikes, and Diablo-likes", then "ensure these are all
'engine-shaped' tickets and not 'game-shaped' tickets", and "certainly
we can build one of each genre as a demo of the engine".

What an action RPG in the style of Diablo I–IV, Path of Exile, Grim
Dawn and Torchlight is made of:

- **one hero in real time**, seen from above at an angle (isometric),
  moved by clicking where to go or by a stick, attacking what is
  clicked;
- **skills**: attacks and spells with costs, cooldowns, cast times and
  shapes (a swing, a missile, an area, an aura, a dash);
- **hits**: hitboxes against hurtboxes, damage, knockback, a moment of
  invulnerability;
- **monsters in packs**, some stronger with rolled modifiers, waking when
  the hero comes near and giving up when led too far;
- **loot**: items whose level and rarity decide how many modifiers
  (affixes) they roll from tables, unique items, and labels on the
  ground filtered by the player's rules;
- **an inventory grid** where items take shapes, and a stash;
- **areas**, many generated, a town, and waypoints to travel between;
- a **HUD**: pools as bars or orbs, a skill bar with cooldowns, numbers
  that float up from hits.

What the engine has or plans: the tick simulation, pointer input,
collision and movers, collision queries, hit-stop, isometric
projection, navigation meshes and generated levels (all parked until
the game launches), and `shuyi-rpg`'s sheets, items and abilities
(ADR-0033, parked).

## Decision

### 1. One new module, and `shuyi-rpg` grows

- **`shuyi-action`** (`crates/action`): real-time action. Hits and
  hurts, skills in real time, projectiles, spawners and aggro, moving
  to a pointed place, and the action HUD. It depends on `shuyi-rpg` for
  sheets, pools and abilities, and on the real-time kits (1119, 1122,
  2124).
- **`shuyi-rpg`** (ADR-0033) gains loot rolled from affix tables (2758)
  and an inventory grid of shapes with a stash (2759). Both serve its
  turn-based games too.
- **Waypoints** (2763) are a small part of `shuyi-core` and `shuyi-ui`,
  because open-world games and Metroidvanias use them with no RPG at
  all.
- **Concerns**: `kind` gains `action-rpg`, answered by `shuyi-action`
  with `shuyi-rpg`. The perspective is the default kits' isometric
  (1126) or top-down free (1122); `shuyi-action` serves both.
- Module duties as ADR-0009 §4 says, as in ADR-0033 §1.

### 2. The bar (ADR-0033 §17), applied

| Piece | Where | Who else uses it |
| ----- | ----- | ---------------- |
| Hits and hurts, invulnerable steps, knockback | engine | beat-'em-ups, platformers, twin-stick shooters, action roguelites |
| Skills with cooldowns, cast times and shapes; projectiles | engine | MOBAs, twin-sticks, action roguelites, platformers' abilities |
| Spawners, packs, aggro and leashing; monster modifiers | engine | MMOs, survival games, open-world action, top-down shooters |
| Moving to a pointed place, attack-move | engine | MOBAs, real-time strategy, point-and-click adventures |
| Loot rolled from affix tables; uniques; a loot filter | engine | looter shooters, MMOs, roguelikes' random artefacts |
| An inventory grid of shapes; a stash | engine | survival horror, extraction shooters, inventory puzzles |
| Waypoints unlocked by visiting | engine | open-world games, Metroidvanias, MMOs |
| Pools as bars, a skill bar with cooldowns, floating numbers | engine | MOBAs, MMOs, action roguelites |
| A town portal, a belt of potions, gold picked up by walking over it | template | Diablo's habits; a new game edits or deletes them |
| Classes, skill trees' contents, item bases, monsters, the story | the game, or the example's own | — |

### 3. Hits are data on bodies, harm is the game's

- A body (1122's mover) carries **hurtboxes** and, while a skill
  strikes, **hitboxes**, each with a team. A hit is found by 2124's
  overlap queries in the tick, in id order, once per hitbox and target.
- What a hit does goes through `ActionMath` (the game's), as
  `PartyMath` does for turns (ADR-0033 §7): harm, the chance to avoid,
  knockback, the steps of invulnerability after.
- Every number in the tick is an integer or `Fx` (ADR-0003 §3).

### 4. Skills are abilities with time

`abilities.ron` (2750) gains real-time fields: cooldown, cast and
recovery steps, and a shape (`Swing { arc, reach }`, `Missile { speed,
pierce }`, `Area { radius, delay }`, `Aura { radius }`, `Dash
{ distance }`, or the game's own). The same ability can serve a turn
game and a real-time one; each reads its own fields.

### 5. Loot rolls from tables

- An **affix** is a named modifier with tiers by item level, each with a
  range, and tags saying which item kinds may roll it. A **rarity**
  says how many affixes of each group an item rolls. Uniques are fixed
  lists of affixes on a base.
- A rolled item is 2746's `Item` with its `rolled` list filled.
  Monsters with modifiers roll from the same tables.
- What an affix does is the game's (`ActionMath` or `PartyMath` reads
  the rolled values). The engine names no affix.

### 6. The demo and the template

`examples/action-rpg`: a small isometric action RPG with a town, two
generated areas, waypoints, a handful of skills, monster packs and loot,
made for it. The template part `kind-action-rpg` copies the genre's
habits (a town portal, a belt, picking up gold by walking over it) into
the new game as code it owns.

### 7. What every target runs

Real-time action runs where the tick simulation runs. The Playdate
can run a small one in one bit within its budgets (3008); the example
measures it if 3010 is done.

## Consequences

- An action RPG is the real-time kits plus `shuyi-action`, with
  `shuyi-rpg`'s data shared with the turn-based RPGs.
- All of it waits for the real-time work, which waits for the game's
  launch.
- Online co-op isn't covered: it needs rollback networking (1147) and a
  ticket of its own when Nick asks.

## Alternatives considered

- **A `shuyi-arpg` module holding everything** — loot, the inventory
  grid and waypoints would be locked behind a module named for one
  genre, though looter shooters and Metroidvanias want them.
- **Loot in `shuyi-action`** — a turn-based roguelike's random
  artefacts would pull in real-time combat. Loot belongs with items.
- **Floats for hit detection** — breaks replays and the rule of
  ADR-0003 §3.
