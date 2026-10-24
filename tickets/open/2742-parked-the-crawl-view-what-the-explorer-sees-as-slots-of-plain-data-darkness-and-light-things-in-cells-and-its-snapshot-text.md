---
id: "2742"
title: "Parked: The crawl view: what the explorer sees as slots of plain data, darkness and light, things in cells, and its snapshot text"
type: feature
milestone: "Engine: dungeon crawlers"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2741"]
nick_input: none
completed:
---

# 2742 — The crawl view: what the explorer sees, as plain data

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§4. Every skin (lines and pictures in 2743, 3D in 2753) paints the same
plain-data view, so tests of what the player sees read the view, never
pixels (`AGENTS.md` rule 9). The automap (2744) records what the view
saw.

It builds on 2741 (`CrawlMap`, `Explorer`, `Facing`, the light effect).

**Who uses it** (ADR-0033 §17's bar for engine work): every game of 2741's map, whatever its skin: crawler RPGs, labyrinth puzzles, grid horror. The automap (2744) and the bots (2755) read it too.

## Nick input

None.

## Scope

**In:**
- **`CrawlView`** in `shuyi_crawl::view` (pure, no `draw` feature
  needed):
  - `ViewShape { depth: u8, width: u8 }`: how many cells ahead (default
    3) and to each side (default 1). It comes from a game setting in
    `crawl.ron` (below). While the explorer has the `light` effect
    (2741 keeps only the steps it lasts), the depth grows by the
    setting's `light` (default 3), capped at a `max_depth` (default 6);
  - `CrawlView::build(map, explorer, shape, things) -> CrawlView`;
  - `Slot { depth: u8, side: i8, cell: Pos, floor_look, ceiling_look,
    front: SeenEdge, left: SeenEdge, right: SeenEdge, things:
    Vec<ThingLook>, dark: bool }`, where `depth` 0 is the explorer's
    own cell and `side` is negative to the left;
  - `SeenEdge::{None, Wall(look), Door { look, open: bool }}`, as the
    edge is *seen*: an unfound secret is a wall, a one-way edge from its
    closed side is a wall, from its open side a door;
  - slots in a fixed order (by depth, far to near, then left to right),
    the order a painter draws them in.
  - `compass: Option<Facing>`: the explorer's facing while it has the
    `compass` effect (2741), else `None`. The skins show it (2743,
    2753).
- **Visibility**, written once and tested hard:
  - a slot is visible when a walk from the eye's cell reaches it
    through open edges and opened doors, staying inside the cone (each
    depth may widen by one to each side, up to `width`);
  - a wall or closed door hides everything behind it in that column;
  - a `Dark` cell is visible as `dark: true` and nothing behind it is;
    a dark cell the explorer stands in shows only `dark: true` at depth
    0 (Wizardry's "you can't see");
  - the same map, explorer and shape always give the same view.
- **Things**: `things: &[(Pos, ThingLook)]` from the game (a monster
  group, a chest, a person: `ThingLook` is a `u16` look the skin maps).
  Each lands on its visible slot, and a thing on a hidden cell isn't in
  the view.
- **Settings**: `crawl.ron`, asset kind `CrawlConfig`, format
  `crawl-config-ron`: `view` (depth, width, light, max depth), with
  the same fuzz-style test and a golden row. 2743 and 2744 add their
  own fields.
- **The snapshot**: `CrawlView`'s `Display`, one line per slot, such as
  `d2 s-1 (4,7) floor 0 ceil 0 front wall 1 left none right door 2
  open things [3]`, and a `view_dump` helper that tests compare against a
  text file (the engine's snapshot helpers, wherever 1110's Harness
  keeps them).
  While the compass is set, a first line says it (`compass north`).

**Out (do not do):**
- drawing (2743, 2753);
- recording what was seen (2744 reads this view to do it);
- field of view for top-down games (2120, a different rule: a circle
  seen through cells, not a cone through edges).

## Implementation steps

1. Write `ViewShape`, `Slot`, `SeenEdge` and the config format.
2. Write `build`: the walk from the eye, the cone, the hiding rules,
   darkness and light.
3. Add things and the snapshot text.
4. Write the tests below on a small hand-drawn map with every edge kind.

## Acceptance criteria

- [ ] Snapshot tests of the view at ten hand-picked places and facings
      (corridors, a T-junction, an open room, a door, a one-way door
      from both sides, a secret before and after `Found`, darkness ahead
      and underfoot, a light effect).
- [ ] A property test: no slot is behind a wall in its own column, and
      every slot is within the shape.
- [ ] The view of a thing on a hidden cell doesn't list it.
- [ ] The config format's fuzz-style test and golden row pass.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `SeenEdge` for each edge kind from each side; the cone at each
  depth.
- Property: the two invariants above, over random small maps.
- Snapshot: the ten views.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
