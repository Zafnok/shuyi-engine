---
id: "2715"
title: "Parked: RTS AI, part 2: commanders with influence maps, utility scores, build orders and difficulty as data"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["2714", "2733"]
nick_input: none
completed:
---

# 2715 — RTS AI, part 2: commanders

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-18 (`docs/decisions.md`): computer opponents "that manage
an economy and an army", provided by the engine.
[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§8, layer 3: one commander per computer player, thinking every N steps:

- **influence maps** (threat, control, value) on a coarse grid, updated
  in buckets;
- **utility scores** of goals (Dave Mark, Kevin Dill, Game AI Pro), each
  a weighted sum of considerations, with the weights as data in a
  `*.commander.ron`, so a game ships several personalities;
- **build orders** as data, followed until the scores say to leave them;
- **the economy**: workers to resources, production to the plan;
- it hands 2714's squads their goals, and issues commands only.

**Difficulty is settings**, never special code: how often each layer
thinks, a delay before reacting, a cap on commands a minute, and
whether it sees the whole map (2713's `sees_all`, a cheat). A game can
replace any layer with its own `Think`, or add considerations, without
forking the module.

The economy part needs workers that gather and buildings that produce.
That simulation is 2733, the RTS economy, which must be done first.

## Nick input

None.

## Scope

**In:**
- **Influence maps, `shuyi_rts::ai::influence`:**
  - one per computer player, on a coarse grid (a setting, default the
    vision grid's cells), built only from that player's `TeamView`;
  - three layers of `i32` per cell: **threat** (seen enemy damage per
    step, spread over its range), **control** (own minus seen enemy
    unit value nearby), **value** (resources and buildings seen);
  - updated in buckets: each seen unit whose `UnitId` index falls in
    this step's bucket takes off its old contribution and adds its new
    one, as 2713 stamps vision. An enemy that is no longer seen keeps
    its last contribution until it is seen again or known dead. Never
    rebuilt whole.
- **Utility, `shuyi_rts::ai::utility`:**
  - goals: `Expand`, `Defend(area)`, `Attack(area)`, `Scout(area)`,
    `BuildUp`. Candidate areas are the top cells of the maps, in a fixed
    order;
  - a `Consideration` reads one input (threat, control or value at the
    area; distance from the base; army power ratio; resources; supply)
    and maps it through a response curve: points joined by straight
    lines, in `Fx`;
  - a goal's score is the weighted sum of its considerations. The
    highest wins; ties go to the goal's order, then the area's;
  - a game adds a consideration through a `Consideration` trait
    registered by name, so a commander file can name it.
- **Build orders:** a list of steps, each `Produce` or `Build` a kind,
  a count, and a condition (resources at least, supply at least, step
  at least, has a kind). Followed in order; left when `Defend` or
  `Attack` scores above a threshold in the file; picked up again after.
- **The economy:** idle workers go to the nearest seen resource; workers
  are produced up to a target per resource node; army units are
  produced to the plan; a rally point is set at the base.
- **Its own economy, through `TeamView` only** (2713's rule for
  `ai/`): the considerations, build-order conditions and the economy
  read the player's own stock, supply (used, cap) and buildings'
  production queues (2733) from its `TeamView`, never from `RtsSim`.
  If `TeamView` doesn't offer them when this ticket starts, add them to
  it here, for its own player only, never another's.
- **Scouting:** a cheap unit is sent to the unseen or oldest-explored
  cell with the highest value guess, every N steps.
- **`*.commander.ron`:** name, the goals' considerations and weights,
  the build order, workers per node, scouting interval, `SquadSettings`
  (2714) and the difficulty. The format (ADR-0007): `commander`,
  extension `.commander.ron`, with a fuzz-style test and a determinism
  fixture.
- **Difficulty**, a block in the commander file or a separate preset
  referenced by name:
  - `think_every` (steps between commander decisions);
  - `think_units_per_step` for each layer (the same on every device);
  - `reaction_delay` (steps added before each command's step);
  - `commands_per_minute` (a cap; commands over it wait for the next
    allowed step, in order);
  - `sees_whole_map` (turns on 2713's `sees_all` for its team, named a
    cheat in the docs).
- **`Commander`** implements `Think` (a work unit = one cell scored or
  one consideration evaluated), and `AiPlayer` (2714) gains it.
- **Hooks:** `AiPlayer::with_layers` takes a game's own `Think` for the
  squads, the commander or both. A test replaces the commander with a
  scripted one.
- **The README's AI section**, layer 3, difficulty and the hooks.

**Out (do not do):**
- the economy simulation itself (2733, the RTS economy);
- behaviour trees and GOAP (ADR-0015, *Alternatives*; a game can plug
  its own in through the hooks);
- bench scenarios (2718);
- the example's commanders (2719);
- threads (2717).

## Implementation steps

1. Write the influence maps and their bucket updates.
2. Write considerations, response curves and goal scoring.
3. Write build orders, the economy and scouting.
4. Write the commander format, difficulty, and `Commander` as `Think`.
5. Wire it into `AiPlayer`, with the hooks.
6. Write the fixtures: an AI-vs-AI match and a commander against a
   player that does nothing.

## Acceptance criteria

- [ ] A commander's decision is the same with slices of 1, 7 and
      unlimited units (property test with random states and slice
      sizes).
- [ ] After every unit has been in a bucket once, the influence maps
      equal a full rebuild from the same `TeamView` (property test).
- [ ] An AI-vs-AI fixture of 6 000 steps, replayed from its
      `CommandRecord` with no AI running, gives the same hash list
      (integration test).
- [ ] In a fixture, a commander destroys every unit and building of a
      player that does nothing within 18 000 steps (integration test).
- [ ] The commands-per-minute cap is never exceeded, and the reaction
      delay is applied to every command (property tests).
- [ ] A scripted commander passed through the hook replaces the
      engine's (unit test).
- [ ] The commander fixtures decode to their checked-in hash on native
      and web; bad fixtures name the file and field; arbitrary bytes
      never panic.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: response curves; scoring and tie-breaks; build-order conditions;
  the cap and the delay.
- Property: slice independence; bucket maps against a rebuild.
- Integration: AI against AI replays; a commander beats an idle player.
- Fuzz-style and fixture: the `commander` format.

## Completion notes
