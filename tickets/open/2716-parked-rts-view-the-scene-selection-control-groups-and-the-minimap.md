---
id: "2716"
title: "Parked: RTS view: the scene from the camera, selection, control groups and the minimap"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2712", "2713", "1120", "1121"]
nick_input: none
completed:
---

# 2716 — RTS view: the scene, selection, control groups, the minimap

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§9. A screen with a thousand units in play must only pay for what the
camera sees and what the player selected:

- **the scene** holds the units in the camera's rectangle (2708's grid),
  seen through the local team's `TeamView` (2713), as sprite items on a
  world layer with sort keys (1120,
  [ADR-0003](../../docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)
  §5). A skin paints it (`AGENTS.md` rule 9);
- **selection is the player's own state**, outside the simulation
  (§2). It turns pointer gestures (1121) and actions into commands; it
  never changes the simulation;
- **the minimap** is drawn from coarse grids, not from every unit;
- **health bars and queued orders** are drawn for the selection only.

All of it sits behind the crate's `view` feature, which depends on
`shuyi-ui`. The simulation parts don't.

## Nick input

None.

## Scope

**In:**
- **Actions, never keys** (`AGENTS.md` rule 8, the `input-actions`
  skill):
  - `RtsAction`, the module's own list: `Select`, `AddToSelection`
    (held), `SelectAllOfKindOnScreen`, `Order` (the context order at
    the pointer), `AddToQueue` (held), `AttackMoveMode`, `PatrolMode`,
    `Stop`, `Hold`, `SaveGroup(n)`, `RecallGroup(n)`, `AddToGroup(n)`
    for `n` in 0..10, and camera panning;
  - a game maps its own `ActionSet` onto them through a function like
    `ActionSet::ui`, `fn rts(action) -> Option<RtsAction>`. The engine
    picks no keys; 2719's example has its own keymap file;
  - help text names keys through `HelpKeys`, never in a string.
- **The scene, `shuyi_rts::view::scene`:**
  - `RtsScene` built from the camera (1120) and the local team's
    `TeamView`: one `UnitItem { id, kind, owner, pos, facing, state,
    selected }` per unit in the camera's rectangle, plus projectiles and
    fog cells, on a world layer, sorted by y with 1120's sort keys;
  - plain data only; the skin (`RtsSkin`) maps kinds and states to
    sprites and owner colours to palette roles;
  - **all of it read through `TeamView`** (2713's boundary test covers
    `view/`). This ticket adds what the scene needs to `TeamView`, in
    `shuyi_rts::vision`: `facing` in `SeenUnit`; the points of own
    units' queued orders; and `projectiles_in_rect(rect, out)`, 2712's
    projectiles in visible cells, in row order;
  - bars and queued order markers for selected units only.
- **Selection, `shuyi_rts::view::select`:**
  - `Selection` (own units only, in `UnitId` order, capped by a setting);
  - a box from a pointer drag, through `TeamView::units_in_rect` in
    world coordinates, kept to own units (the camera turns screen into
    world). 2708's `in_rect` gives rows, which `view/` can't turn into
    units without the `UnitTable`;
  - a click picks the nearest own unit under the pointer within its
    radius, through `TeamView::units_within`;
  - `SelectAllOfKindOnScreen` selects own units of the clicked kind in
    the camera's rectangle;
  - control groups: save, recall and add, held in the player's view
    state; recalling drops dead units;
  - **the context order:** on open ground `Move`, on an enemy `Attack`,
    in attack-move mode `AttackMove`, in patrol mode `Patrol`.
    `AddToQueue` held sets `queued: true`. The command goes to the
    local player's `CommandDriver::issue` (2706).
- **The minimap, `shuyi_rts::view::minimap`:**
  - a small layer, one pixel per coarse cell, from the team's vision
    grid (2713) and a coarse owner grid: per cell, the owner with the
    most seen units, refreshed one band of cells per step through
    `TeamView` queries (2713's boundary test covers `view/` too);
  - the camera's rectangle drawn on it; a pointer press on it moves the
    camera.
- **A text form of the scene** for tests: one line per item.
- **Either order with 2718** (budgets): if 2718 is done, add the scene
  to its battle scenarios: each step builds the scene for a camera over
  the fight, the report says so, and the gated scenarios still pass
  (2718's tuning rule). If not, nothing to do (2718 builds the scene
  when it lands).

**Out (do not do):**
- art and the example's keymap (2719);
- a production panel, building placement, rally point markers and the
  `Gather` context order on a resource node (2734, after 2733 makes
  resources and rally points);
- sound;
- isometric drawing of the scene beyond what 1120's projection already
  gives: 1126 owns the isometric projection, and 2769 draws the scene,
  the picks and the minimap through it, with an isometric stage of
  2719's example;
- a minimap drawn through a second world layer with its own camera and
  viewport (1120 allows one): this minimap is drawn from coarse grids,
  which costs less and shows only what the team has seen.

## Implementation steps

1. Write `RtsAction` and the mapping hook, following the `input-actions`
   skill.
2. Write `RtsScene` and the skin trait, with a plain test skin.
3. Write `Selection`, box and click picks, control groups and the
   context order.
4. Write the minimap and its owner grid.
5. Write the scene tests and one look test.

## Acceptance criteria

- [ ] The scene holds exactly the units a brute-force scan finds in the
      camera's rectangle that the team can see (property test).
- [ ] No enemy in a cell the team can't see is in the scene (property
      test).
- [ ] A box drag over a fixture gives the expected `Selection`, and an
      `Order` on an enemy issues one `Attack` command with those ids
      (snapshot of the commands).
- [ ] Holding `AddToQueue` sets `queued: true` (unit test).
- [ ] A control group saved, then recalled after one member died,
      selects the living members (unit test).
- [ ] Selection never changes the simulation: a run with selection
      gestures and no orders has the same hashes as a run without them.
- [ ] Bars are built only for selected units (scene snapshot).
- [ ] `cargo xtask check-keys` (or its successor) passes: no key named in
      `crates/rts`.
- [ ] One `frame-png` look test of the test skin passes; every other
      test reads the scene.
- [ ] `shuyi-rts` without `view` doesn't depend on `shuyi-ui`
      (`cargo tree`).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each pick; control groups; the context order; queued.
- Property: the scene against brute force; fog hiding.
- Snapshot: the scene's text form; the commands from a gesture script.
- Look: one `frame-png` of the test skin.

## Completion notes
