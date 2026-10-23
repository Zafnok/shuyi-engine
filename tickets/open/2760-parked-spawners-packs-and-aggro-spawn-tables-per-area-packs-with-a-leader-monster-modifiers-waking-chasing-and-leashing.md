---
id: "2760"
title: "Parked: Spawners, packs and aggro: spawn tables per area, packs with a leader, monster modifiers, waking, chasing and leashing"
type: feature
milestone: "Engine: action RPGs"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2756", "2748", "2106", "2114"]
nick_input: none
completed:
---

# 2760 — Spawners, packs and aggro

> **Parked** (Nick, 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other kinds of game, and the real-time work it builds on, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0034](../../docs/adr/0034-action-rpgs-a-hero-in-real-time-loot-rolled-from-affix-tables-skills-with-cooldowns-and-packs-of-monsters.md)
§2. An action RPG's areas fill with packs from tables; some packs have
a stronger leader with rolled modifiers; monsters sleep until the hero
comes near, chase, and give up when led too far from home. How a
monster fights is the game's (its skills, 2757, and its mind); where it
is, when it wakes and when it gives up is shared bookkeeping.

**Who uses it** (ADR-0033 §17's bar for engine work): action RPGs,
MMOs, survival games, open-world action games, top-down shooters.

It builds on 2756 (bodies, teams, `ActionMath`), 2106 (routes over the
navigation mesh), 2114 (weighted picks, named streams).

**Either order with 2758:** if 2758 is done, a pack's leader rolls
modifiers through `roll_modifiers`. If not, leaders have none, and 2758
wires them.

## Nick input

None.

## Scope

**In:**
- **`spawns.ron`**, asset kind `SpawnDefs`, format `spawns-ron`: per
  area name, a density (packs per 100 open tiles or per square of
  world), a weighted table of packs (monster kinds from 2748's
  `enemies.ron` with count ranges, an optional leader and its modifier
  table), and fixed spawns by map object or mark; respawn rules (never,
  after N steps, when the area is entered again). Fuzz-style test and
  golden row.
- **Spawning**: `spawn_area(defs, area, open_tiles, level, rng)`, from
  the area's own stream (so the same seed fills the same area the same
  way); packs placed apart from each other and from the entry by a
  least distance.
- **`Aggro`** per monster in `shuyi_action::aggro`: `Asleep`,
  `Alert(target)`, `Returning`, with a wake radius (and line of sight
  through 2124's segment cast), a pack waking together, a leash distance
  from home past which it returns (and heals, if the data says).
- **Chasing**: an alert monster asks for a route (2106) to its target
  every N steps and its mover follows; a returning one walks home.
- **The mind is the game's**: `trait Mind` gives the action of an alert
  monster each step (use a skill, keep distance, flee); the default
  walks up and uses its first skill.
- A replay test of a pack waking, chasing and leashing.

**Out (do not do):**
- monster kinds and their numbers (data in a game);
- crowd steering for hundreds (the RTS work, 2711);
- bosses' scripted phases (a game's mind).

## Implementation steps

1. The format and defs.
2. `spawn_area`.
3. `Aggro` and its states; waking a pack.
4. Chasing and returning on routes; `Mind`.
5. The replay test.

## Acceptance criteria

- [ ] The same seed fills an area the same way (a golden test).
- [ ] A pack wakes together when one member sees the hero.
- [ ] A monster led past its leash returns home (a test).
- [ ] The replay test gives the same hashes twice and on wasm32.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: spawning rules, aggro transitions.
- Integration: the replay; the golden table.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
