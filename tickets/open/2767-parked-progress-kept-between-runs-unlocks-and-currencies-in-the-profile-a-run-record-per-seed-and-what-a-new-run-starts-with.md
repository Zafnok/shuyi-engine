---
id: "2767"
title: "Parked: Progress kept between runs: unlocks and currencies in the profile, a run record per seed, and what a new run starts with"
type: feature
milestone: "Engine: roguelikes"
model: sonnet-5
effort: medium
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2767 — Progress kept between runs

> **Parked** (Nick, 2026-10-20, choosing roguelikes as a kind of game the engine serves, and 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other roguelike tickets, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found on 2026-10-22 when the backlog was checked for roguelike gaps
([ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§17's bar). 2735's roguelike forgets an ended run. Roguelites keep
something: unlocks (a new item in the tables, a new starting choice),
a currency spent between runs, and a record of each run (its seed, how
far it got, how it ended). Which unlocks exist and what they do is the
game's; keeping them, spending, and recording runs is shared.

**Who uses it**: roguelites, deck-builders (2737's card battle across
runs), arcade games that unlock modes, action roguelites, survival games
with a meta-progression.

Achievements (3036, parked) are the player's trophies, seen by stores;
this is game state the game reads at the start of a run. The two don't
share storage.

**Either order with 2110 and 3045:** if 2110 is done, `Meta`'s saved form migrates through it, and if 3045 is done, writes are flushed through it; if not, a version number and a plain write, and each adds its part when it lands.

## Nick input

None.

## Scope

**In:**
- **`Meta`** in `shuyi_core::meta` (pure): unlocks (a set of the game's
  ids), currencies (named counters, never below zero), and run records
  (`RunRecord { seed, started_at_run: u32, measures: Vec<(String, i64)>,
  ended: String }`, the measures and ending named by the game), with a
  cap on records kept.
  - `unlock(id)`, `spend(currency, amount) -> Result`, `earn`, `record`;
  - `hash()`; serialisable with a format version (2110's migrations if
    done).
- **Kept in the profile** through the storage the engine has, one key
  per profile, written when a run ends (and flushed, as 3045 says if it
  is done).
- **The start of a run**: `Meta` is an argument to the game's run setup,
  so a run is still its seed, its starting `Meta` hash and its commands:
  a record of a run names the `Meta` hash it started from, and a replay
  checks it.
- **A runs screen** in `shuyi_ui::screens`: past runs with their seed,
  measures and ending; choosing one copies its seed to start the same
  floors again. Actions only.
- Harness test and `frame-png` snapshot.

**Out (do not do):**
- what any unlock does: the game's;
- a shop between runs (2752's shop works with a currency pool if the
  game wants);
- online leaderboards (3054).

## Implementation steps

1. `Meta` and its rules.
2. Storage and the run-start contract.
3. The runs screen; tests.

## Acceptance criteria

- [ ] Spending more than a currency holds is refused and changes
      nothing.
- [ ] `Meta` survives a save round trip.
- [ ] A replay of a run started from a different `Meta` hash fails with
      a clear error.
- [ ] A Harness test restarts a past run's seed from the runs screen.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: unlocks, currencies, the record cap.
- Integration: storage round trip; the Harness test.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
