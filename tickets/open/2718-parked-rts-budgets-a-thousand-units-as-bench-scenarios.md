---
id: "2718"
title: "Parked: RTS budgets: a thousand units as bench scenarios in CI, and the module's needs per target"
type: infra
milestone: "Engine: real-time strategy"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2715", "3008"]
nick_input: none
completed:
---

# 2718 — RTS budgets: a thousand units as bench scenarios

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-18 (`docs/decisions.md`): "'Thousand+' is a number the
engine is tested against": 1 000 units moving and fighting at 60 steps
a second on the Steam Deck, with 5 000 as the stretch.
[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§11 turns that into bench scenarios for 3008's `shuyi bench`, each a
fixed `CommandRecord`:

| Scenario | Units | Gate |
| -------- | ----- | ---- |
| `rts-march-1k` | 1 000 ordered across a map | within `steam-deck`'s `frame_instructions` per step, in CI |
| `rts-battle-1k` | 2 × 500 fighting, both sides computer players | within `steam-deck`'s `frame_instructions` per step, in CI |
| `rts-battle-5k` | 2 × 2 500 | measured and reported in CI, not gated |
| `rts-ai-think` | 4 commanders, 1 000 units | each commander's work units per step within its budget |

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§6: CI gates on instruction counts and peak heap, never on wall time,
and every number is provisional until the Steam Deck device ticket
(3013) measures it.

The module also states its `needs` (ADR-0012 §1). It doesn't fit the
Playdate, the GBA or the DS at a thousand units.

## Nick input

None.

## Scope

**In:**
- **A bench stage** in `crates/rts/benches/` (or an example folder 3008
  can run, whichever 3008's scenario format expects): a 256 × 256 map
  made in code, the fixture kinds, a fixture commander, and a setup per
  scenario. Nothing from 2719's example, so this ticket doesn't wait for
  it.
- **The four scenarios** as `.bench.ron` files (3008's format), each
  playing a checked-in `CommandRecord`:
  - `rts-march-1k`: one player's 1 000 units ordered in 10 groups to the
    far side, measured over every step until they arrive;
  - `rts-battle-1k`: two computer players with 500 units each, from the
    first contact for 1 800 steps;
  - `rts-battle-5k`: the same with 2 500 each;
  - `rts-ai-think`: four computer players, 250 units each.
- **What 3008's format needs**, added here if missing (a version bump
  with defaults):
  - `targets`: the profiles a scenario is checked against
    (`rts-march-1k` and `rts-battle-1k` against `steam-deck` and
    `desktop` only, never `playdate`);
  - `gate: false` for a scenario that is reported, not gated
    (`rts-battle-5k`).
- **The `rts-ai-think` check:** every commander's `Think` reports the
  units it used per step; the scenario fails if any step uses more than
  its difficulty's `think_units_per_step`. The instructions per step are
  reported beside it.
- **Peak heap** for each scenario, including the flow field cache at its
  limit (ADR-0015, *Consequences*).
- **Scene building, either order with 2716:** if 2716 is done, the
  battle scenarios also build the scene for a camera over the fight
  each step, and the report says so. If not, they step the simulation
  only, and 2716 adds the scene when it lands (its scope says so).
- **Tuning, not cheating:** if a gated scenario is over budget, make the
  module cheaper (bucket counts, `flow_cells_per_step`, the code). Never
  raise the budget (`AGENTS.md` rule 6). If it can't be met in one PR's
  worth of work, write a ticket for the cost, link it here, and don't
  merge until it lands; a failing gate never merges.
- **Needs, either order with 3017:** if 3017 is done, set the module's
  `needs`: `ram_bytes` from `rts-battle-1k`'s peak heap, rounded up,
  marked provisional. If 3017's `Needs` can't say "too slow for this
  target", record that in the README and write a ticket for a CPU need.
  If 3017 isn't done, add these numbers to 3017's scope in this PR.
- **The device, either order with 3013:** if 3013 is done, run the four
  scenarios on the Steam Deck and add p50, p99 and the worst frame to
  `docs/targets/steam-deck.md`; the goal for `rts-battle-5k` is p99 ≤
  16.7 ms. If 3013 isn't done, add the four scenarios to 3013's list in
  this PR.
- **The README's cost section:** the numbers per scenario, and the
  settings they were measured with.

**Out (do not do):**
- the parallel feature and its speed-ups (2717; scenarios run on one
  thread);
- the example's own scenarios (not planned: this bench stage measures
  the module without 2719's example);
- raising any budget.

## Implementation steps

1. Write the bench stage and record the four `CommandRecord`s.
2. Add `targets` and `gate` to 3008's scenario format, if missing.
3. Write the four `.bench.ron` files and the `rts-ai-think` check.
4. Run `shuyi bench` in CI and tune until the gated scenarios pass.
5. Set the `needs`, or add to 3017. Run on the Deck, or add to 3013.
6. Write the README's numbers.

## Acceptance criteria

- [ ] `shuyi bench --scenario rts-march-1k` and `rts-battle-1k` pass
      against `steam-deck` in CI.
- [ ] `rts-battle-5k`'s numbers appear in the CI table and don't fail
      the job.
- [ ] `rts-ai-think` fails on a fixture commander set to use more units
      than its budget, and passes on the real one.
- [ ] Two CI runs on one commit give the same instruction counts (3008's
      rule).
- [ ] No budget in `targets/` changed in this PR.
- [ ] The `needs` key is set, or 3017's scope has the numbers.
- [ ] `docs/targets/steam-deck.md` has the four scenarios, or 3013's
      scope lists them.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the four scenarios under `shuyi bench` in CI.
- Unit: the `rts-ai-think` check on a fixture over budget; the new
  scenario fields' defaults.

## Completion notes
