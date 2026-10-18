---
id: "2713"
title: "Parked: RTS vision: fog of war per team, and TeamView, what each side knows"
type: feature
milestone: "Engine: real-time strategy"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2708"]
nick_input: none
completed:
---

# 2713 — RTS vision: fog of war per team, and TeamView

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§7, *Vision* and *What a team knows*. Each team sees only what its units
see. Nick, 2026-10-18 (`docs/decisions.md`): "A computer opponent sees
only what its side sees, unless its difficulty settings say otherwise."

`TeamView` is how that rule holds: the scene for a person (2716) and the
AI of a computer player (2714, 2715) both read enemy state through it,
and only through it.

It builds on 2707's units and kinds (`sight`) and 2708's grid.

**Either order with 2712** (combat): if 2712 is done, replace its
`Visibility` stand-in (sight radius) with the team's grid. If not, 2712
uses the grid when it lands.

## Nick input

None.

## Scope

**In:**
- **Teams:** `RtsSettings::teams` maps each player to a team; allies
  share vision.
- **`shuyi_rts::vision::VisionGrid`**, one per team:
  - a coarser grid than the costs: the cell size is a setting (default
    two terrain cells);
  - per cell a `u16` count of units that see it, and an `explored` bit;
  - a cell is *visible* when its count is above 0, *explored* when it
    was ever visible, *unseen* otherwise.
- **Stencils:** for each sight radius in the loaded kinds (in vision
  cells), the offsets inside the circle (`dx² + dy² ≤ r²`), computed
  once, in a fixed order.
- **Stamping in buckets** (`RtsSettings::vision_buckets`, default 4):
  - each unit remembers where it last stamped (a column: the cell and
    the stencil);
  - in its bucket, it takes its old stamp off (counts down) and puts a
    new one on at its position (counts up, sets `explored`);
  - a removed unit takes its stamp off when it is removed;
  - counts make the result independent of the order units stamp in.
- **Whole-map vision:** `RtsSettings::sees_all: Vec<TeamId>`, empty by
  default. A team in it sees every cell as visible. The docs call it a
  cheat, and 2715's difficulty names it so.
- **`TeamView<'a>`**, built from the sim and a team:
  - `cell(pos) -> Visibility` (unseen, explored, visible);
  - `units_in_rect(rect, out)` and `units_within(center, radius, out)`:
    own and allied units always, enemy units only in visible cells, in
    the grid's fixed order;
  - `unit(id) -> Option<SeenUnit>`: kind, owner, position, health share
    and order kind for own and allied units; kind, owner, position and
    health share for a visible enemy; `None` for an enemy that isn't
    visible;
  - nothing in it hands out `&UnitTable` or `&RtsSim`.
- **A boundary test:** a test reads the source files under
  `crates/rts/src/ai/` and `crates/rts/src/view/` and fails if one names
  `UnitTable`, `RtsSim::units` or `ProjectileTable`. The AI and the view
  get a `TeamView` and nothing else.
- **A text dump** of a team's grid: one character per cell (` `
  unseen, `.` explored, `#` visible).

**Out (do not do):**
- memory of buildings seen earlier (ghosts in explored cells), heights
  and line of sight blocked by terrain: later tickets if a game needs
  them;
- the drawing of fog and the minimap (2716);
- the AI that reads `TeamView` (2714, 2715);
- threads (2717).

## Implementation steps

1. Add teams and `sees_all` to `RtsSettings`.
2. Write `VisionGrid`, stencils and stamping in buckets, with the stamp
   column in `UnitTable`.
3. Write `TeamView` and its queries over 2708's grid.
4. Replace 2712's stand-in, if 2712 is done.
5. Add the boundary test and the dump.

## Acceptance criteria

- [ ] After every unit has stamped once, the visible cells equal the
      union of the sight circles at the units' stamp positions, computed
      by brute force (property test).
- [ ] With the units standing still, bucket counts of 1 and 8 give the
      same grid after 8 steps (property test).
- [ ] An explored cell stays explored (property test).
- [ ] `TeamView` never returns an enemy unit in a cell that isn't
      visible (property test), and returns every own unit.
- [ ] A team in `sees_all` sees every enemy (unit test).
- [ ] The boundary test passes, and fails on a fixture file that names
      `UnitTable` under `ai/`.
- [ ] Two replays of a fixture give the same hash list, and a snapshot of
      a team's dump after 300 steps passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: stencils for radii 0 to 8; stamping on and off; removal.
- Property: brute-force visibility; bucket independence; explored;
  `TeamView` hiding.
- Integration: the boundary test; replay.
- Snapshot: the vision dump.

## Completion notes
