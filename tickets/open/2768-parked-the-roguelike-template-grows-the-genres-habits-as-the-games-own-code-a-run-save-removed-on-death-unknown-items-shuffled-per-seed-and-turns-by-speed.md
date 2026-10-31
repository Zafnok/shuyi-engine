---
id: "2768"
title: "Parked: The roguelike template grows: the genre's habits as the game's own code, a run save removed on death, unknown items shuffled per seed, and turns by speed"
type: feature
milestone: "Engine: roguelikes"
model: implementation
effort: high
status: blocked
blocked_by: ["2735", "2765", "2766", "2767", "2746"]
nick_input: none
completed:
---

# 2768 — The roguelike template grows

> **Parked** (Nick, 2026-10-20, choosing roguelikes as a kind of game the engine serves, and 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other roguelike tickets, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§17: a genre's habits go in its template, copied into the new game as
code the game owns, never as engine types. Nick, 2026-10-22: "ensure
these are all 'engine-shaped' tickets and not 'game-shaped' tickets".

Three habits of roguelikes fail the engine bar (each is one genre's
rule), so they belong in the `kind-roguelike` template part that 2735
writes:

- **one save per run, removed when the run ends** (permadeath): the
  engine's saves and storage do it; the rule is the genre's;
- **unknown items that look different each run** (this run's blue
  potion heals, the next run's poisons): 2746's unknown names, with the
  look-to-kind table shuffled from the run's seed;
- **turns by speed** (2765), the message log (2766) and progress between
  runs (2767), wired into the template so a new roguelike starts with
  them.

It builds on 2735 (the example and the `kind-roguelike` part), 2765,
2766, 2767 and 2746 (items with unknown names, used by a hero alone:
`shuyi-rpg` with `max: 1`).

## Nick input

None.

## Scope

**In** (in `templates/shuyi-default/`'s `kind-roguelike` part, and the
same in `examples/roguelike`):
- **The run save**: saved on quitting, loaded on starting, deleted when
  the run ends in death or victory; no other save slot. A setting in the
  game's own code turns it off for a "practice" mode.
- **Shuffled unknown looks**: the template's `items.ron` with a few
  potions and scrolls whose unknown names come from a list of looks,
  shuffled per run by `SimRng::stream(run_seed, "looks")` (2114), and
  an identify-by-use rule: using one makes its kind known for the run.
  This is the template's code over 2746, not a field of the engine.
- **Turns by speed**: the example's foes and hero on 2765's `Schedule`,
  one fast foe kind.
- **The log**: 2766's widget under the map, the game's events pushed to
  it.
- **Progress between runs**: one unlock (a third foe kind joins the
  tables after the first win), recorded runs on 2767's runs screen.
- The example's tests updated: its Harness run, replay and `frame-png`
  pictures, with the run save and the shuffled looks covered.
- The roguelike guide (`docs/guides/roguelike.md`, 2735) says which of
  these are the template's habits and how to delete each.

**Out (do not do):**
- any of the three habits as engine types;
- new generators (2119) or field of view (2120).

## Implementation steps

1. The run save in the template and the example.
2. Shuffled looks and identify-by-use.
3. The schedule, the log, the unlock and the runs screen.
4. Tests and the guide.

## Acceptance criteria

- [ ] Dying deletes the run save (a Harness test with the headless
      storage).
- [ ] Two seeds give two different look tables; the same seed gives the
      same one (tests).
- [ ] A game made from `kind-roguelike` builds and its test passes in
      CI's `templates` job.
- [ ] `grep` finds no permadeath or shuffled-look type in `crates/`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the example's Harness, replay and `frame-png`; the
  template's game in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
