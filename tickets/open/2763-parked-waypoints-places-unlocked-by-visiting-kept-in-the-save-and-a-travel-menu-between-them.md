---
id: "2763"
title: "Parked: Waypoints: places unlocked by visiting, kept in the save, and a travel menu between them"
type: feature
milestone: "Engine: action RPGs"
model: sonnet-5
effort: medium
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2763 — Waypoints

> **Parked** (Nick, 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other kinds of game, this waits until after Visions of Shuyi launches, unless the game asks for it sooner through its `engine-change` skill. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0034](../../docs/adr/0034-action-rpgs-a-hero-in-real-time-loot-rolled-from-affix-tables-skills-with-cooldowns-and-packs-of-monsters.md)
§1. A waypoint is a place that, once visited, can be travelled to from
any other visited one. It needs no RPG and no real time, so it is a
small part of the default crates, not of a module.

**Who uses it** (ADR-0033 §17's bar for engine work): action RPGs,
open-world games (fast travel), Metroidvanias (teleport rooms), MMOs,
and turn-based RPGs with a world map.

It builds on the engine's menus (2307, 2308, done) and screen results
(ADR-0018).

## Nick input

None.

## Scope

**In:**
- **`waypoints.ron`**, asset kind `WaypointDefs`, format
  `waypoints-ron`, in `shuyi-assets`: each waypoint with an id, a name
  (text key), a group (an act, a region: the game's word) and where it
  leads (a stage name and a place the game reads: a string and a point
  in tile or world units). Fuzz-style test and golden row.
- **`Waypoints`** in `shuyi_core::waypoints` (pure): the set visited,
  `visit(id)`, `can_travel(from, to)` (both visited), serialisable for
  the save, hashed.
- **The travel screen** in `shuyi_ui::screens`: waypoints by group,
  visited ones selectable, the current one marked; it pops with the
  chosen waypoint (ADR-0018); the game's flow moves there. Actions only;
  it declares them (ADR-0025).
- Harness test and `frame-png` snapshot.

**Out (do not do):**
- what touching a waypoint looks like in a world (a game's thing);
- a town portal (the action RPG template's code, 2764);
- travel costs or times (a game's rule after the screen pops).

## Implementation steps

1. The format.
2. `Waypoints`.
3. The screen, its test and snapshot.

## Acceptance criteria

- [ ] An unvisited waypoint can't be chosen (a Harness test).
- [ ] `Waypoints` survives a save round trip.
- [ ] `frame-png` snapshot of the screen.
- [ ] The golden row passes on x64 and wasm32.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: visiting and travel rules.
- Integration: Harness; the golden table.
- Snapshot: the screen.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
