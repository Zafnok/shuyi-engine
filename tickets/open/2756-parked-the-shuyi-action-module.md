---
id: "2756"
title: "Parked: The shuyi-action module: hits and hurts in real time, hitboxes and hurtboxes by team, harm through the game's math, knockback and invulnerable steps"
type: feature
milestone: "Engine: action RPGs"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["1119", "1122", "2124", "2745", "2003"]
nick_input: none
completed:
---

# 2756 — The shuyi-action module: hits and hurts in real time

> **Parked** (Nick, 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other kinds of game, and the real-time work it builds on, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0034](../../docs/adr/0034-action-rpgs-a-hero-in-real-time-loot-rolled-from-affix-tables-skills-with-cooldowns-and-packs-of-monsters.md)
§1–§3. Every real-time fighting game asks the same questions each step:
which hitbox touches which hurtbox, of which team, for the first time
this swing; what harm it does; which way the target is knocked; how
long it can't be hit again. The answers' numbers are the game's; the
bookkeeping is the engine's.

**Who uses it** (ADR-0033 §17's bar for engine work): action RPGs,
beat-'em-ups, platformers with enemies, twin-stick shooters, action
roguelites, top-down Zelda-likes.

It builds on 1119 (the tick simulation, `Fx`, the arena), 1122 (bodies
and movers), 2124 (overlap queries and layer filters), 2745 (`Sheet`
and pools, so harm lands on a pool the game named) and 2003 (module
duties).

A hit may ask for hit-stop (2123's `Freeze`): see *Hit-stop from a
hit* in the scope.

## Nick input

None.

## Scope

**In:**
- **The crate** `crates/action` (`shuyi-action`), with ADR-0009 §4's
  duties: depends on `shuyi-core`, `shuyi-assets` and `shuyi-rpg`; an
  empty `draw` feature (2762 fills it); metadata `kind`/`action-rpg`,
  `serves` top-down free and isometric; its `tools/layers.ron` line
  (pure, no floats); the `action` feature of `shuyi`; the catalog row
  (building, 2756, 2757, 2760–2762, 2764); the `action-rpg` answer of
  `kind` in `docs/modules.ron`; the README's first two sections; the
  skill (**from 1134, done**): `skills/shuyi-action/SKILL.md` in the
  pack's form, saying what this ticket built. It has no worked example
  yet, so its `metadata` has `waits_for: "2764"`: 2764 gives it its
  worked example and drops its id.
  2003's catalog has no building status (`shuyi_cli::catalog::Status`
  is ready, planned or parked): if it still has none, add
  `Status::Building`, named `building`, with its doc, the `status` line
  of `docs/modules.ron`'s header, *How to read it* in `docs/modules.md`
  and the `modules` schema, and a `Breaking:` line if CI's `api` job
  reports one.
- **`Hurtbox`** and **`Hitbox`** in `shuyi_action::hit`: boxes relative
  to a body, with a `TeamId`, a layer mask (2124's filters), and for a
  hitbox a `strike: StrikeId` (one swing or missile) so a target is hit
  once per strike. A hitbox may also be a circle, asked with 2124's
  `overlap_circle` if 2124 shipped it. A hurtbox is always a box,
  since 2124's circles are shapes to ask with, never bodies.
- **`Hits`**: `step(world, bodies, &mut log) -> Vec<Hit>` finds every
  hitbox–hurtbox overlap of different teams, in body-id order, skipping
  targets this strike already hit and targets still invulnerable.
- **The game's numbers**: `trait ActionMath` with `harm(attacker,
  target, strike, rng) -> Harm { pool, amount, conditions }`,
  `avoid_chance(...) -> u8`, `knockback(...) -> FxVec2`,
  `invulnerable_steps(...) -> u16`, with defaults for tests that name no
  stat (as 2748's do).
- **Hit-stop from a hit. Either order with 2123:** if 2123 is done, add
  `ActionMath::hit_stop(attacker, target, strike) -> u16` (default 0).
  Each `Hit` carries it, and a sim that keeps 2123's `Freeze` holds it
  for that many steps (a hold during a hold keeps the longer one), with
  a test; if not, nothing to do (2123 adds it).
- **Results as events**: `ActionEvent::{Hit, Avoided, Fell, Knocked}`,
  harm applied to the target's `Sheet` pool, knockback handed to the
  mover (1122) as an impulse it resolves against walls.
- Determinism: the same seed and inputs give the same events and hash
  (a replay test over 600 steps of two teams fighting).

**Out (do not do):**
- skills and projectiles (2757), spawners and aggro (2760), moving to a
  pointed place (2761), the HUD (2762);
- the game's formulas;
- physics bodies (1128) as targets.

## Implementation steps

1. The crate and its duties.
2. The boxes, teams and strikes.
3. `Hits::step` with 2124's queries.
4. `ActionMath`, events, harm, knockback, invulnerability.
5. The replay test.

## Acceptance criteria

- [ ] A strike hits a target once, however many steps they overlap.
- [ ] Bodies of the same team never hit each other.
- [ ] A target is not hit during its invulnerable steps.
- [ ] Knockback never moves a body into a wall (1122 resolves it).
- [ ] If 2123 is done: a hit whose `hit_stop` is 6 holds a test sim's
      `Freeze` for 6 steps.
- [ ] The replay test gives the same hash list twice, and on wasm32.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: overlaps, strike memory, teams, invulnerability.
- Integration: the replay test; the golden hashes on native and wasm32.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
