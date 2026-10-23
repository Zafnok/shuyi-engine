---
id: "2765"
title: "Parked: Turn order by speed: an energy schedule where faster actors act more often, with ties broken by a fixed rule"
type: feature
milestone: "Engine: roguelikes"
model: sonnet-5
effort: medium
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2765 — Turn order by speed

> **Parked** (Nick, 2026-10-20, choosing roguelikes as a kind of game the engine serves, and 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other roguelike tickets, this waits until after Visions of Shuyi launches, unless the game asks for it sooner through its `engine-change` skill. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found on 2026-10-22 when Nick asked for roguelikes again and the
backlog was checked for gaps
([ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§17's bar). 2735's roguelike moves every foe once per hero command, in
the order they were placed. Most roguelikes let a fast creature act
twice for a slow one's once: each actor gains energy by its speed each
tick of game time and acts when it has enough. Tactics games with a
charge time (Final Fantasy Tactics) and some crawlers' battles order
turns the same way.

**Who uses it**: roguelikes, tactics games with charge time, turn-based
battles ordered by speed, and simulation games with actors on different
clocks.

It is pure bookkeeping in `shuyi-core`; no other ticket is needed first.

## Nick input

None.

## Scope

**In:**
- **`Schedule<K>`** in `shuyi_core::schedule`, over the game's actor
  keys `K: Ord + Copy`:
  - `add(key, speed: u16)`, `remove(key)`, `set_speed(key, speed)`;
  - `next() -> K`: advances game time until an actor's energy reaches
    the threshold (a setting, default 100), and returns it; the caller
    then calls `spent(key, cost)` with its action's cost (default the
    threshold), so a quick action costs less;
  - ties: the actor with the most energy first, then by a fixed rule:
    the order added (default) or the key's order;
  - `time()`: the game time so far;
  - integer only, no floats; `hash()`; serialisable for the save.
- A doc test with a hero at speed 100 and a foe at 200 acting twice per
  hero turn.
- 2735 needs no change: its foes keep their simple order, and 2768
  moves the roguelike template onto this schedule.

**Out (do not do):**
- what speed means for a game (its numbers);
- real-time turn bars drawn on screen (a game's or a skin's).

## Implementation steps

1. `Schedule` and its tie rules.
2. Tests and the doc test.

## Acceptance criteria

- [ ] Over 10 000 actions, an actor at twice another's speed acts
      twice as often (exactly, with the default threshold).
- [ ] Ties follow the documented rule (tests).
- [ ] The same adds and costs give the same order and hash.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: speeds, costs, ties, removal mid-round.
- Property: no actor waits more than `threshold / speed + 1` ticks of
  game time between actions with the default cost.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
