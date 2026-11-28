---
id: "2155"
title: "Schedule control and states in shuyi-ecs: run conditions, named sets, one-shot systems by id, systems that fail, and phase states with enter and exit systems"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2107", "2108"]
nick_input: none
completed:
---

# 2155 — Schedule control and states in shuyi-ecs

## Context

2107's schedule orders systems with `before` and `after` and runs every
system every time. 2108 runs it per command (turn games, through
`EcsRules`) or per step (tick games). The Bevy coverage audit of
2026-11-28 (`docs/ROADMAP.md`, *Bevy's game coverage*) found that an
ECS game split into phases needs more: a card game's draw, main, combat
and end phases, a tactics game's rounds, a roguelike's floor and shop.
Without it each game writes its own phase code around the schedule.

Bevy has:

- run conditions and system sets
  (`crates/bevy_ecs/src/schedule/condition.rs`, `set.rs`;
  `examples/ecs/run_conditions.rs`);
- one-shot systems registered and run by id
  (`crates/bevy_ecs/src/system/system_registry.rs`;
  `examples/ecs/one_shot_systems.rs`);
- fallible systems with an error policy
  (`crates/bevy_ecs/src/error/`; `examples/ecs/error_handling.rs`,
  `fallible_params.rs`);
- states with `OnEnter`, `OnExit`, sub-states, computed states and
  entities that despawn on exit (`crates/bevy_state/src/`;
  `examples/state/*.rs`, `examples/ecs/state_scoped.rs`).

The screen stack (`crates/ui/src/screen.rs`) stays the app's structure:
menus, pause and the game. This ticket is the rules' side, inside the
world, and it must keep the world's hash the same on every run. 2904's
ADR wins where it differs from this ticket's shape.

## Nick input

None.

## Scope

**In:**
- **Run conditions:** `.run_if(cond)`, where a condition is a plain
  `fn(&World) -> bool` that reads resources only, plus `in_state(S)`,
  `resource_changed::<R>()` and `on_message::<T>()`.
  **Either order with 2154:** if 2154 is done, add `on_message`; if
  not, nothing to do (2154 adds `on_message` beside its messages).
- **Sets:** named sets, ordered against each other with 2107's cycle
  check, a condition on a set applying to all its systems.
- **One-shot systems:** `world.register_system(name, system) ->
  SystemId`, stable across save and load because it is keyed by name;
  `Commands::run_system(id)` runs it at the command's next apply point.
- **Systems that fail:** a system may return `Result<(), EcsError>`. In
  a turn game an error refuses the whole command: the world rolls back
  to the command's start and the error is the refusal (2108's
  `EcsRules`). In a tick game it is logged with the step and the step
  goes on.
- **Startup:** a `Startup` set run once from the game's `Start`.
- **States:** `State<S>` and `NextState<S>` as resources. A transition
  applies at one fixed point per command or step, running `OnExit(old)`
  then `OnEnter(new)` systems. Sub-states exist only while their parent
  state has a given value; computed states are pure functions of their
  sources. `DespawnOnExit(S)` removes tagged entities in `Commands`
  order. All of it is saved and hashed.
- New public types follow ADR-0039: `EcsError` is `#[non_exhaustive]`.

**Out (do not do):**
- script systems (2156). **Either order with 2156:** if 2156 is done, a
  script that runs out of steps fails through `EcsError` with a test; if
  not, nothing to do (2156 fails through `EcsError` when it lands).
- the app's own states (menus, pause): the screen stack, unchanged;
- custom executors and system piping: a follow-up if a game asks;
- run conditions that read the clock or input directly: a condition
  reads resources only, and input reaches the world as a step's input.

**Estimated size:** about 1,500 lines (code 750, tests 600, docs 150).

**Split:** past 1,000 lines, split off states (the last bullet) into a
new ticket.

## Implementation steps

1. Read 2904's ADR and 2107's and 2108's code.
2. Add conditions and sets to the schedule builder; reuse the cycle
   check.
3. Add the system registry and `run_system`.
4. Add `Result`-returning systems and the two error policies; add the
   rollback to `EcsRules`.
5. Add `Startup`.
6. Add `State`, `NextState`, the transition point, `OnEnter`/`OnExit`
   schedules, sub-states, computed states and `DespawnOnExit`.
7. Write the docs page: a card game's turn phases as a state.

## Acceptance criteria

- [ ] A system with a false condition doesn't run, and a set's
  condition stops all its systems.
- [ ] A one-shot system runs by id after a save and load.
- [ ] In a turn game, a system's error refuses the command and the
  world's hash equals the one before it.
- [ ] A transition from `Draw` to `Main` runs `OnExit(Draw)` then
  `OnEnter(Main)`, once, and removes the `DespawnOnExit(Draw)` entities.
- [ ] A computed state follows its source in the same command.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each condition, sets, the registry, both error policies, each
  kind of state.
- Property: random sequences of transitions give the same hash in the
  serial and parallel schedules.
- Snapshot / integration: a turn-phase fixture replayed from a record.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
