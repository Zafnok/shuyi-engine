---
id: "1125"
title: "Parked: Example: a walkable top-down town that starts a grid battle and comes back (moving between game styles)"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["1120", "1121", "1122", "1123", "1124"]
nick_input: none
completed:
---

# 1125 — Example: a walkable town that starts a grid battle

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0060 says the army walks around freely (then 1119–1125 become game work). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0003 §1: a game style is a stage, and moving between styles is moving
between screens. The README claims a style only when its example passes
CI. This example claims two things at once:

- **top-down free movement**;
- **moving between styles**, which is what Nick named for our own game:
  "we ourselves might transition between 1 and 2 for open world section".

0060 decides whether our game uses it. The example doesn't wait for that
answer.

## Nick input

None. If 0060's answer is B–D, show Nick the example running in the
browser as a preview of the engine's free movement. That is information
for him, not a sign-off.

## Scope

**In:**
- `examples/town-and-battle/` uses `shuyi-*` only, with CC0 art
  (Kenney's top-down packs, or similar, credited in
  `THIRD_PARTY_ASSETS.md`):
  - a Tiled town map (1124);
  - a walking, animated player (1123) with analog movement (1121) and
    collision (1122), followed by a camera (1120);
  - two NPCs with `.dlg` lines;
  - touching an enemy pushes a grid battle stage built from the 12×10
    tiles around the touch point, with the tactics kit (1114–1116);
  - winning pops back to the town with the enemy gone, and the town keeps
    its state.
- Tests:
  - a Harness test that walks to the enemy, wins the battle with scripted
    commands, and checks the town;
  - a replay test of the town part (tick record hashes);
  - `frame-png` snapshots of the town and the battle.
- Web build on Pages beside `hello-grid`.

**Out (do not do):** anything in our game; isometric (1126).

## Implementation steps

1. Write the field stage (a `TickSim`) and the battle stage (a
   `TurnRules`).
2. Write the example's flow screen that hands state between them.
3. Add the tests, CI and Pages steps, and a README section.

## Acceptance criteria

- [ ] The playthrough, replay and snapshot tests pass in CI.
- [ ] The web build runs in the browser pane.
- [ ] The engine README lists "top-down free movement" and "moving
      between styles" as supported, linking this example.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the playthrough, the replay, and the snapshots.

## Completion notes
