---
id: "2762"
title: "Parked: The action HUD: pools as bars or orbs, a skill bar with cooldowns, floating numbers, and labels on the ground through the loot filter"
type: feature
milestone: "Engine: action RPGs"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: blocked
blocked_by: ["2757", "2758", "2109"]
nick_input: none
completed:
---

# 2762 — The action HUD

> **Parked** (Nick, 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other kinds of game, and the real-time work it builds on, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0034](../../docs/adr/0034-action-rpgs-a-hero-in-real-time-loot-rolled-from-affix-tables-skills-with-cooldowns-and-packs-of-monsters.md)
§2. The pieces of a real-time RPG's screen, each built from scene data
the simulation already has (`AGENTS.md` rule 9): pools, skills'
cooldowns, hits, items on the ground. Which pools show and where is the
game's layout; how each piece draws is the engine's.

**Who uses it** (ADR-0033 §17's bar for engine work): action RPGs,
MOBAs, MMOs, action roguelites, twin-stick shooters (floating numbers,
cooldowns).

It builds on 2757 (`SkillState`: cooldowns, charges, refusals), 2758
(the loot filter and item names), 2311's meter (done) and the frame's
sprite and text items.

**Either order with 2329:** if 2329 is done, `PoolGauge` and `SkillBar`
implement its `Measure` and the HUD's pieces are laid out by its rows
and columns. If not, each piece takes a rectangle from the game, and
2329 gives the two their `Measure` when it lands.

## Nick input

None.

## Scope

**In** (all behind `shuyi-action`'s `draw` feature):
- **`PoolGauge`**: a pool (2745) as a bar (2311's meter) or an orb (a
  picture filled from the bottom by the pool's share, cut in whole
  pixels), with the number on hover or always, by setting.
- **`SkillBar`**: slots bound to skills, each with its icon, a cooldown
  shown as a sweep or a darkened share (whole pixels), charges as a
  number, and a flash when a use is refused (2757's event). The slots'
  actions are the player's bindings, shown by their names through the
  keymap (`input-actions` skill).
- **Floating numbers**: from `ActionEvent::Hit` (2756): the amount
  rises and fades over a few hundred milliseconds in the view only
  (2109's tweens), colour by the harm's kind (a role the game names),
  bigger for a hit the game marks heavy; a cap on how many show at once.
- **Ground labels**: an item on the ground shows its name (2758's
  names) styled by the player's loot filter; labels push apart so none
  overlap (a greedy rule, documented); an action toggles them; hiding by
  the filter hides the label, not the item.
- Snapshots: `frame-png` of each piece alone and of a busy screen.

**Out (do not do):**
- a minimap (2744's corner map serves crawlers; a free-movement minimap
  is a ticket of its own if asked);
- the screen's layout in a game (the example's).

## Implementation steps

1. `PoolGauge` and `SkillBar`.
2. Floating numbers.
3. Ground labels with the filter and the push-apart rule.
4. Snapshots.

## Acceptance criteria

- [ ] A cooldown's drawn share matches the steps left (a test reading
      the scene).
- [ ] No two ground labels overlap after pushing apart (a property
      test).
- [ ] Hidden items have no label; styled ones use the filter's role.
- [ ] `frame-png` snapshots as listed.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: gauge shares, cooldown sweeps, label placement.
- Property: labels never overlap.
- Snapshot: each piece and a busy screen.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
