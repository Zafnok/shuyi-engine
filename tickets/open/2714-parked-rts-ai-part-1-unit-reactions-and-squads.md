---
id: "2714"
title: "Parked: RTS AI, part 1: unit reactions and squads that judge a fight before it starts"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["2712", "2713", "3009"]
nick_input: none
completed:
---

# 2714 — RTS AI, part 1: unit reactions and squads

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-18 (`docs/decisions.md`): "I would imagine thousand+ unit
count RTS games need advanced optimized AI routines provided."
[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§8 builds the AI in three layers. This ticket is layers 1 and 2; the
commander (layer 3) is 2715.

Two rules from the ADR shape everything here:

- **Every player is equal** (§2). A computer player issues the same
  commands a person does, through the same queue. Nothing it does
  writes to the simulation. So its effect is exactly its recorded
  commands, and a replay never runs the AI again.
- **Work units, never time** (§8,
  [ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
  §6). Each thinking layer implements `Think` (3009). The number of
  units per step is an AI setting, the same on every device, so a
  computer player decides the same thing at the same step everywhere.
  It does not come from the target profile.

**Where each layer runs:**

- **Layer 1, unit reactions**, is how units behave, for every player's
  units, a person's too: pick a target, back off when hurt, chase within
  a leash, return to a post. It runs inside the simulation, in buckets,
  with its state in the unit table, like 2712's target pick.
- **Layer 2, squads**, belongs to a computer player. It runs outside the
  simulation, reads only `TeamView` (2713), and issues commands.

## Nick input

None.

## Scope

**In:**
- **Layer 1, `shuyi_rts::react`:** a simulation part, so it sits
  beside `combat`, not under `ai/` (ADR-0015 §1's table puts all three
  layers under `ai`; 2713's boundary test keeps `ai/` on `TeamView`
  only, and reactions read the unit table). The README says so.
  - a `react` block in `*.unit.ron`, with defaults:
    - `leash` (how far a unit chases from where it started);
    - `back_off_below` (a health share, as a per-mille integer);
    - `return_to_post` (bool);
    - `focus_fire` (bool: when picking a target, prefer an enemy that
      allies in range already target, then the nearest, then the lower
      row);
  - a state per unit, a new column `react: ReactState` (`Idle`,
    `Guarding { post }`, `Chasing { target, from }`, `BackingOff`,
    `Returning { post }`), with fixed rules from state to state;
  - it runs in the target bucket (2712), reading the last state and
    writing the unit's own row;
  - an explicit order from its player always comes first; reactions only
    fill in when the order allows it (an idle unit, a guard, a hold).
  - **Either order with 2717** (threads): if 2717 is done, write the
    reactions pass as a function over a chunk of rows, as 2717 did for
    2712's target pick, run it in chunks under the `parallel` feature,
    and check 2717's with-and-without hash test passes on 2714's
    fixtures; if not, nothing to do (2717 splits it).
- **Driving computer players:**
  - `trait CommandSource<S: CommandSim>` in `shuyi_core::tick`: `fn
    commands(&mut self, sim: &S, now: u64, out: &mut Vec<Timed<S::Command>>)`;
  - `CommandDriver` (2706) and the Harness call each source before each
    step. Commands go into the same `CommandQueue` as a person's, for
    `now + 1` or later.
- **Layer 2, `shuyi_rts::ai::squad`:**
  - `Squad { id, members: Vec<UnitId>, goal: SquadGoal, state }`, with
    `SquadGoal` one of `MoveTo(point)`, `Attack(point)`, `Defend(point)`,
    `Hold(point)`, and the state `Gathering`, `Moving`, `Engaging`,
    `Waiting` or `Retreating`;
  - **the fight estimate**: near the squad (a radius setting), through
    `TeamView`, each side's power is its total damage per step times its
    total health, in `i64`. The ratio of the squad's power to the
    enemy's decides: engage above `engage_ratio`, wait for help between
    the two ratios, retreat below `retreat_ratio`;
  - `SquadSettings` (serde, with defaults, round-trips through RON):
    the ratios, the radius, a regroup distance. 2715's commander file
    holds them;
  - `SquadThink` implements `Think`: a work unit is one member or one
    seen enemy counted. Its decision is a list of `RtsCommand`s
    (`AttackMove`, `Attack` on a focus target, `Move` to regroup or
    retreat);
  - squads move as groups (2711) and mark a focus target with `Attack`
    commands.
- **`AiPlayer`**, a `CommandSource` for one computer player:
  - its squads, with goals set by code (2715 adds the commander);
  - `AiSettings` (serde): `think_units_per_step` (fixed, never from the
    profile), `squad: SquadSettings`;
  - it builds a `TeamView` for its team, steps each squad's `Think` with
    its share of the units, and turns finished decisions into timed
    commands.
- **The README's AI section**, layers 1 and 2.

**Out (do not do):**
- the commander, influence maps, utility scores, build orders, the
  economy and difficulty (2715);
- a game's own AI layers (2715 adds the hooks);
- drawing squads: they are a computer player's own state, never shown
  in a player's scene (a debug view is a later ticket if a game needs
  it);
- threads beyond the reactions pass above (2717).

## Implementation steps

1. Add the `react` block, the `ReactState` column and its rules.
2. Add `CommandSource` to the core and call it from `CommandDriver` and
   the Harness.
3. Write `Squad`, the fight estimate and `SquadThink`.
4. Write `AiPlayer` and `AiSettings`.
5. Write the fixtures: a squad that should engage, one that should
   wait, one that should retreat; a battle with one computer player.

## Acceptance criteria

- [ ] A Harness battle with one computer player, replayed from its
      `CommandRecord` with no AI running, gives the same hash at every
      step (integration test).
- [ ] A squad's decision is the same with slices of 1, 7 and unlimited
      units (property test with random battles and slice sizes).
- [ ] With a fixed `think_units_per_step`, two runs issue the same
      commands at the same steps (property test).
- [ ] The engage, wait and retreat fixtures give those decisions.
- [ ] A unit backs off below its share, never chases past its leash, and
      returns to its post (unit tests).
- [ ] A person's explicit order is never overridden by a reaction
      (property test).
- [ ] The 2713 boundary test still passes: nothing under `ai/` reads the
      unit table.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each reaction rule; the fight estimate on hand-made sides; each
  squad state change.
- Property: slice independence; orders before reactions; determinism.
- Integration: AI effect equals its replayed commands.

## Completion notes
