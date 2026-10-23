---
id: "2761"
title: "Parked: Moving to a pointed place: click to move on a route, held to follow, attack-move on a target, and the stick beside it"
type: feature
milestone: "Engine: action RPGs"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["1121", "2106", "2757"]
nick_input: none
completed:
---

# 2761 — Moving to a pointed place

> **Parked** (Nick, 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other kinds of game, and the real-time work it builds on, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0034](../../docs/adr/0034-action-rpgs-a-hero-in-real-time-loot-rolled-from-affix-tables-skills-with-cooldowns-and-packs-of-monsters.md)
§2. In Diablo the player clicks where to go and the hero walks a route
there; holding the button keeps following the pointer; clicking a
monster walks into reach and attacks. A pad moves the hero directly
with a stick. Both are actions, never keys (`AGENTS.md` rule 8, the
`input-actions` skill); the pointer is in world pixels in the tick's
input (1121).

**Who uses it** (ADR-0033 §17's bar for engine work): action RPGs,
MOBAs, real-time strategy (a unit's move order), point-and-click
adventures walking their hero.

It builds on 1121 (the pointer in `TickInput`, mouse buttons as
bindable inputs), 2106 (routes and a follower) and 2757 (using a skill
on a target).

**Either order with 1126:** if 1126 is done, the pointed place is
picked through the isometric projection's picking. If not, top-down
only, and 1126 adds isometric picking here.

## Nick input

None.

## Scope

**In:**
- **`PointMove`** in `shuyi_action::point_move`, per controlled body:
  - action `move_to` (pressed): a route (2106) to the pointed world
    place, the nearest reachable one if it can't be reached;
  - held: a new route every N steps (a setting) toward the pointer;
  - pointing at a body of another team (picked by a small radius
    around the pointer, 2124's point query): walk until in reach of the
    chosen skill (the skill's shape says its reach), then `Use` it
    (2757); keep using it while held;
  - `stand_still` (held): skills aim at the pointer without moving;
  - a stick action moves the body directly and drops any route; aiming
    by the right stick when the game binds it.
- **Feedback for the view**: an event for the pointed place (a marker
  the skin draws) and for an unreachable click.
- **Settings** in the player's keymap and options: which buttons do
  what, the follow interval.
- A Harness test that clicks across a room with a pillar, and one that
  attack-moves onto a dummy.

**Out (do not do):**
- selecting several units (RTS, 2716);
- touch gestures (2407);
- the HUD (2762).

## Implementation steps

1. `PointMove` with routes and held following.
2. Targets and attack-move through 2757.
3. The stick path; settings; events.
4. Harness tests and a replay test.

## Acceptance criteria

- [ ] A click across a room with a pillar walks round it (Harness).
- [ ] An attack-move walks into reach and uses the skill (Harness).
- [ ] The stick drops a route at once.
- [ ] A replay of a recorded session gives the same hashes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: target picking; the reach rule.
- Integration: the Harness tests and the replay.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
