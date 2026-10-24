---
id: "2757"
title: "Parked: Skills in real time: cooldowns, cast and recovery steps, costs from pools, and shapes (swings, missiles, areas, auras and dashes)"
type: feature
milestone: "Engine: action RPGs"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2756", "2750"]
nick_input: none
completed:
---

# 2757 — Skills in real time

> **Parked** (Nick, 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other kinds of game, and the real-time work it builds on, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0034](../../docs/adr/0034-action-rpgs-a-hero-in-real-time-loot-rolled-from-affix-tables-skills-with-cooldowns-and-packs-of-monsters.md)
§4. A skill in real time is an ability (2750) with time: it costs a
pool, waits through a cast, strikes in a shape, recovers, and cools
down. The shapes recur across genres; what each skill does when it
lands is the game's (`ActionMath`, 2756).

**Who uses it** (ADR-0033 §17's bar for engine work): action RPGs,
MOBAs, twin-stick shooters, action roguelites, platformers' abilities
(a dash, a thrown blade).

It builds on 2756 (hitboxes, strikes, `ActionMath`) and 2750
(`abilities.ron`, `Known`, costs from pools).

## Nick input

None.

## Scope

**In:**
- **Real-time fields** in `abilities.ron` (2750's format, read only
  when `shuyi-action` is used): `cooldown`, `cast`, `recovery` (steps),
  `charges` (several uses before cooling: a count per skill, not 2750's
  charges per tier), and a `shape`:
  - `Swing { arc: Angle, reach: Fx }`: a hitbox for the strike's
    steps, in front of the user;
  - `Missile { speed, range, pierce, radius }`: a projectile body (a
    mover of 1122) with a hitbox, gone at a wall, at its range, or after
    `pierce` hits;
  - `Area { radius, delay, lasts }`: a hitbox at a point after a delay;
  - `Aura { radius }`: a hitbox around the user while the skill is on,
    striking every `period` steps;
  - `Dash { distance, steps }`: the user's mover pushed, invulnerable
    if the data says;
  - `Custom(name)`: an event the game resolves.
- **`SkillState`** per body in `shuyi_action::skill`: cooldowns,
  charges, the skill being cast and its phase. Commands come from the
  tick's actions: `Use { skill, aim: FxVec2 or target body }`; a use
  while casting, cooling or short of the cost is refused for that step
  (an event says why, for the HUD).
- **Projectiles** as arena entities, stepped in the tick, in id order.
- The golden fixture of the format gains rows with the new fields.

**Out (do not do):**
- what skills do: the game's;
- skill trees and points: a game's data over 2750's learning;
- the skill bar (2762).

## Implementation steps

1. The fields in the format.
2. `SkillState`, phases, costs, refusals.
3. Each shape, with a test per shape.
4. Projectiles; a replay test with every shape.

## Acceptance criteria

- [ ] Each shape strikes where and when the data says (one test each).
- [ ] A skill can't be used while cooling, casting or short of its cost.
- [ ] A missile stops at a wall and after `pierce` hits.
- [ ] A replay of 600 steps using every shape gives the same hashes
      twice and on wasm32.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: phases and refusals; each shape.
- Integration: the replay test.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
