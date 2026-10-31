---
id: "2108"
title: "An ECS stage in a game: a schedule run per command or per step, a parallel schedule on native with the same hashes, and an ECS example"
type: feature
milestone: After MVP
model: reasoning
effort: high
status: todo
blocked_by: ["2107"]
nick_input: none
completed:
---

# 2108 — An ECS stage in a game

## Context

2107 builds `shuyi-ecs`. This ticket puts it in a game: the state
changes only through commands (turn games) or inputs per step (tick
games), as `AGENTS.md` rule 5 says, so a schedule runs as the effect of
one of those. Bevy's homepage promises "a lock-free parallel
scheduler"; here it must give the same hash as running the systems one
by one, or it isn't allowed.

## Nick input

None.

## Scope

**In:**
- **Turn games:** an `EcsRules` adapter in `shuyi-ecs` (behind its
  `ui` feature, depending on `shuyi-core`'s turn types) that applies a
  command by inserting it as a resource and running a schedule. History
  and undo (`shuyi_core::history`) work on the world's dump or on a
  diff, whichever 2107's dump makes cheaper (measure).
- **Tick games:** **either order with 2137**. If 2137 is done,
  an `EcsTickSim` that runs a schedule per step with the step's inputs
  as a resource. If not, write a parked follow-up ticket for this
  adapter (the `write-ticket` skill), blocked by 2137, and name it in a
  section of the module's docs.
- **Views:** a system can build the frame's plain-data view from
  queries; the skin paints it as any other (`AGENTS.md` rule 9).
- **A parallel schedule:** a `parallel` feature, native only, that runs
  systems with no conflicting access at the same time (on `std`
  threads or `rayon` if 2904 allows it). Commands are applied in the
  systems' schedule order, never in finishing order, so the result is
  the same. A property test runs random schedules both ways and
  compares hashes.
  Systems that draw luck: **either order with 2114**. If 2114 is done,
  each such system gets its own stream, made with `SimRng::fork(<system
  name>)` on the world's `SimRng` in schedule order before the systems
  run, in both the serial and the parallel schedule. They then share no
  `SimRng` and may run together, and the property test includes two
  such systems. If 2114 is not done, they share the `SimRng` resource,
  and its conflicting access runs them one by one; 2114 adds the forks
  (its scope says so).
- **An example, `examples/ecs-garden/`:** a turn game on a grid where
  each turn plants grow, spread and are eaten, with a few hundred
  entities and five systems. It has a harness test, a replay test,
  and the web build. **From 2801 (done):** `pages.yml`
  builds every example whose `shuyi.ron` has a `web` section, and fails
  one with no card: add `site/content/examples/ecs-garden.md` (its
  title and two lines as its `description`), as `hello-grid.md` is.
- **From 2802 (done):** add the book's chapter "An optional ECS"
  (`site/content/learn/book/`, `extra.chapter = 12`, its `weight`
  between chapter 11's and the appendices') after its chapter 11
  (`shuyi-ecs`, `EcsRules`, the parallel schedule and `ecs-garden`),
  its code from `site/snippets/` through the `snippet` shortcode, and
  add an entity of `shuyi-ecs` to the row of chapter 11's table
  (`from-another-engine.md`, *Coming from another engine*) that maps a
  node, a GameObject and a Bevy entity, linked to the new chapter.
- **The catalog:** with the example passing CI, set `shuyi-ecs`'s
  answer in `docs/modules.ron` and its row in `docs/modules.md` to
  `ready` (2107 adds both as `planned`).

**Out (do not do):**
- moving the kit or any existing example to the ECS;
- the parallel schedule on the web (no threads there today).

## Implementation steps

1. Write `EcsRules`, with history.
2. Write the tick adapter if 2137 is done.
3. Write the parallel schedule and its property test, with a forked
   stream for each system that draws if 2114 is done.
4. Write the example, with its tests and web build.
5. Run the gates.

## Acceptance criteria

- [ ] `ecs-garden` plays natively and in the browser, and its replay
      test passes.
- [ ] Undo restores the world exactly (hash before equals hash after
      undo).
- [ ] The parallel and serial schedules give the same hash over 1 000
      random schedules (property test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a command through `EcsRules`; undo.
- Property: parallel equals serial, with two systems that draw luck
  when 2114 is done.
- Integration: the example's harness and replay tests.

## Completion notes

*(Filled in by the session that completes the ticket.)*
