---
id: "1125"
title: "Parked: Example: a walkable top-down town that starts a grid battle and comes back (moving between game styles)"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["1120", "1121", "1122", "1123", "1124", "2360"]
nick_input: none
completed:
---

# 1125 — Example: a walkable town that starts a grid battle

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): after Visions of Shuyi launches, not only after the MVP (`docs/decisions.md`, 2026-10-20). Unparked early only if the game needs it: its ticket 0060 says the army walks around freely (then 1119–1125 become game work), or it asks for a piece of this through its `engine-change` skill, which is then split out and unparked alone. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

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
  - a Tiled town map (1124), drawn by 2360's tile-layer items;
  - a walking, animated player (1123) with analog movement (1121) and
    collision (1122), followed by a camera (1120) with damped follow (a
    follow fraction under `(1, 1)`);
  - **Either order with 2362:** if 2362 is done, the camera reads a
    `camera.ron` (2362's `CameraDef`) with look-ahead along the
    player's velocity, and rooms from a `rooms` object layer of the
    town map (`Rooms::from_map`) with a slide between them. If not,
    1120's damped follow alone, and nothing later: 2362 leaves the
    town's camera as it is;
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
- Guides (ADR-0009 §5): write `docs/guides/top-down-free.md` in full,
  and the move guide `docs/guides/top-down-free-to-top-down-grid.md`
  from this example. If 2004 is done, replace its planned pages. If not,
  create the files, and 2004 adds them to its list.
- The new-game wizard (ADR-0013 §4): if 2503 is done, add the move
  `top-down → top-down-grid` to the built-in template, as a small
  version of this example's touch-to-battle, with its steps in
  `tests/moves/`. If 2503 isn't done, 2503 adds it.
- **The debug view (2318).** If 2318 is done, the town calls its
  `layout` and `sort` helpers from its screen's `debug_draw`. If it
  isn't, 2318 does this.

**Out (do not do):** anything in our game; isometric (1126); mixing
the example's rules with its skin. Keep them apart: the parked 3109
redraws this example in HD-2D by changing only its skin.

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
