---
id: "2712"
title: "Parked: RTS combat: targets in buckets, attacks, attack-move and projectiles"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2711"]
nick_input: none
completed:
---

# 2712 — RTS combat: targets, attacks, attack-move, projectiles

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§7, *Targets*, *Attacks* and *Projectiles*. Until now `Attack` and
`AttackMove` only move (2707). This ticket makes units fight:

- a unit with no attack order picks a target in its bucket (§3), from
  the spatial grid (2708): the nearest enemy it can see and hit, ties
  broken by row;
- attacks have a range, a cooldown in steps and a damage, from the
  kind's `weapons` (2707);
- projectiles are rows in a table of their own, laid out in columns
  like the units.

**Damage follows the read-last-write-next rule** (ADR-0015 §3). An
attacker never writes its target's row. Hits go into a buffer, and a
later pass adds up each unit's hits and writes its own health. Integer
sums don't depend on order, so the result is the same in any visit
order, and 2717 can split it.

**Either order with 2713** (vision): "can see" means the team's
visibility grid. If 2713 is done, the target pick asks it. If not, a
unit can see what is inside its own `sight` radius, behind a
`Visibility` trait, and 2713 swaps in the team's grid.

## Nick input

None.

## Scope

**In:**
- **Columns** added to `UnitTable`: `target: Option<UnitId>` and
  `cooldown: u16` (steps left).
- **`Weapon`** in `*.unit.ron` (2707 decodes the fields; this ticket
  uses them): `range`, `cooldown`, `damage`, `hits` (ground, air or
  both), optional `projectile { speed, homing }` and optional `area`
  (a radius).
- **Targets, `shuyi_rts::combat`:**
  - in its bucket (`RtsSettings::target_buckets`, default 8), a unit
    with a weapon and no `Attack` order picks the nearest enemy it can
    see and hit, within its acquire range (a kind setting, default its
    sight), through `SpatialGrid::nearest` with a filter. Ties go to the
    lower row;
  - a `TargetRule` hook lets a game score candidates its own way; the
    default is the nearest;
  - a target that dies, leaves range or can't be seen is dropped at the
    unit's next bucket.
- **Attacks:** a unit whose target is in range and whose cooldown is 0
  attacks: a hit at once with no projectile, or a projectile row. Its
  cooldown resets to the weapon's.
  - **Damage:** `max(damage − armour, 1)`, summed per target in a
    separate pass; health ≤ 0 queues the unit's removal (2707) and emits
    `Died { unit, by }`.
- **Orders:**
  - `Attack`: chase the target and fight it. Within a setting distance
    (default 8 cells), head straight for it; farther, ask for the flow
    field (2710) to the target's cell, asked again in the unit's bucket
    when the target has moved more than a cell. The order ends when the
    target dies;
  - `AttackMove`: move to the point; in its bucket, if an enemy can be
    acquired, fight it; when that target dies or is lost, carry on to
    the point;
  - `Hold`: fight what comes in range, never move;
  - `Patrol`: an attack-move between its ends.
- **Projectiles:** a `ProjectileTable` with the same column layout and
  serde, hash and dump: `pos`, `vel`, `target` (a unit or a point),
  `damage`, `area`, `owner`, `steps_left`. A projectile moves straight
  (or toward its target's last position if `homing`), and on arrival
  hits its target, or every unit in `area` through `within_radius` in
  the grid's fixed order. Friendly fire is a setting, off by default.
- **Events:** `Attacked`, `Died`, `ProjectileHit`.
- **The README's combat section**: the settings, and what the bucket
  delay costs.

**Out (do not do):**
- the visibility grid and `TeamView` (2713);
- unit reactions (back off, leash, focus fire) and squads (2714);
- economy and production (2733, the RTS economy; see 2707's *Out*);
- the drawing of attacks and projectiles (2716);
- threads (2717).

## Implementation steps

1. Add the columns, the `Weapon` use, the `Visibility` trait (or 2713's
   grid) and `TargetRule`.
2. Write the target pick in buckets.
3. Write attacks, the hit buffer and the damage pass.
4. Write `Attack`, `AttackMove`, `Hold` and `Patrol` behaviour.
5. Write `ProjectileTable` and area hits.
6. Write the battle fixture (2 × 100 units) and its tests.

## Acceptance criteria

- [ ] With two enemies at the same distance, the lower row is picked
      (unit test).
- [ ] In a battle fixture, the health lost by each unit equals the sum
      of the hits recorded on it (property test over random battles).
- [ ] Visiting the rows in reverse gives the same hashes (the test flag
      from 2711).
- [ ] An attack-moving unit stops to fight an enemy in range, then
      carries on to its point when the enemy dies (unit test).
- [ ] A holding unit never moves (property test).
- [ ] An area hit damages exactly the units a brute-force scan finds in
      the area (property test).
- [ ] Two replays of the 2 × 100 battle give the same hash list, and a
      snapshot of the dump after 600 steps passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: tie-breaks; damage and armour; cooldowns; each order's
  behaviour; projectile arrival.
- Property: health conservation; area hits against brute force; order
  independence; hold.
- Integration: the battle fixture replays.
- Snapshot: the battle dump.

## Completion notes
