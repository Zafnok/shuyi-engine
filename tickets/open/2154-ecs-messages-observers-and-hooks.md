---
id: "2154"
title: "Events in shuyi-ecs: typed messages between systems, component hooks, and observers of triggered events that travel up a relation, in a fixed order"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2107", "2153"]
nick_input: none
completed:
---

# 2154 — Messages, hooks and observers in shuyi-ecs

## Context

2107 builds `shuyi-ecs` with `Commands` as the only way systems talk to
each other. Its **Out** leaves "observers, and events beyond
`Commands`" to later tickets, and 2904's **Out** says building an event
queue is a follow-up. The Bevy coverage audit of 2026-11-28
(`docs/ROADMAP.md`, *Bevy's game coverage*) found this is how a whole
kind of game is written in Bevy: card games and roguelikes whose
abilities trigger ("when a unit enters play", "on death", "on hit"),
and board games with chained reactions.

Bevy has three pieces:

- **Messages**: `MessageWriter<T>` and `MessageReader<T>`, a buffer per
  type read with a cursor per reader
  (`crates/bevy_ecs/src/message/`; `examples/ecs/message.rs`);
- **Component hooks**: on add, insert, replace and remove
  (`crates/bevy_ecs/src/lifecycle.rs`; `examples/ecs/component_hooks.rs`);
- **Observers**: systems that run when an event is triggered on an
  entity, and events that propagate up `ChildOf`
  (`crates/bevy_ecs/src/observer/`; `examples/ecs/observers.rs`,
  `observer_propagation.rs`).

Here all three must keep the world deterministic: the same commands
give the same hash, in the serial and the parallel schedule (2108). So
nothing runs re-entrantly: triggers are queued and drained in a fixed
order. 2904's ADR wins where it differs from this ticket's shape.

## Nick input

None.

## Scope

**In:**
- **Messages:** `Messages<T>` as a resource, `MessageWriter<T>` and
  `MessageReader<T>` as system parameters. Writes land in schedule
  order (never in the order parallel systems finish). Each reader keeps
  its own cursor. The buffer clears at one fixed point at the end of
  each command or step, not after two frames as in Bevy. A message type
  that derives `Serialize` is saved and hashed with the world.
- **Hooks:** `on_add`, `on_insert`, `on_replace`, `on_remove`, declared
  in the component derive. A hook sees the entity and gets `Commands`
  only, never `&mut World`, so its effects queue like any system's.
- **Observers:** `world.observe(name, system)` for an event type,
  globally or on one entity. `Commands::trigger(event)` and
  `trigger_on(entity, event)` queue the event. The queue drains
  breadth first at a fixed point inside the command or step; observers
  of one event run in registration order. An event marked
  `#[event(propagate = ChildOf)]` travels up 2153's relation until an
  observer stops it. A depth cap (default 64, set per world) turns a
  loop of triggers into `ObserverError::DepthExceeded` instead of a
  hang.
- **Into the game's events:** 2108's `EcsRules` may name one message
  type whose messages become the command's `Vec<Event>` (`TurnRules`).
  **Either order with 2108:** if 2108 is done, add that option to
  `EcsRules` and a test; if not, nothing to do (2108 reads it from this
  ticket's module when it builds `EcsRules`).
- New public types follow ADR-0039: `ObserverError` is
  `#[non_exhaustive]`.

**Out (do not do):**
- run conditions on messages ("run if a message arrived").
  **Either order with 2155:** if 2155 is done, add its
  `on_message::<T>()` condition beside the messages; if not, nothing to
  do (2155 adds it);
- ECS states and their enter and exit schedules: 2155;
- observers in the arena of plain structs (1119, parked);
- scripts as observers: 2156.

**Estimated size:** about 1,500 lines (code 750, tests 600, docs 150).

**Split:** past 1,000 lines, split off observers and propagation into a
new ticket, leaving messages and hooks here.

## Implementation steps

1. Read 2904's ADR and 2107's and 2153's code.
2. Build `Messages<T>` with a per-reader cursor and the clear point;
   wire it into the schedule's end of command or step.
3. Add the hook attributes to the derive and call them from the world's
   insert, replace and remove paths, queuing their commands.
4. Build the observer registry and the trigger queue; drain it at the
   fixed point; add propagation along a relation and the depth cap.
5. Include saved messages and the pending queue (empty at rest) in the
   hash and the dump.
6. Add the `EcsRules` option as the either-order clause says.
7. Write the docs page: a card game's "when a unit enters play" written
   as a hook and an observer.

## Acceptance criteria

- [ ] Two readers of one message type each see every message once.
- [ ] Messages written by two parallel systems arrive in schedule
  order, and the hash equals the serial schedule's (2108's test).
- [ ] A hook that inserts a component runs once; its insert is visible
  to the next system, not inside the hook.
- [ ] An event triggered on a grandchild reaches the child's and the
  parent's observers in that order; one that stops it stops there.
- [ ] Two observers that trigger each other end in
  `ObserverError::DepthExceeded` at the cap.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: messages, each hook, observers, propagation, the cap.
- Property: random trigger graphs give the same hash in the serial and
  parallel schedules.
- Snapshot / integration: a small card-game fixture whose triggered
  abilities produce a fixed event list.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
