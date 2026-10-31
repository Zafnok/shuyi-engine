---
id: "2753"
title: "Parked: The 3D labyrinth skin: a crawl map built as meshes, the camera at eye height, and smooth steps and turns"
type: feature
milestone: "Engine: dungeon crawlers"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2742", "3102", "3103", "3104", "2109"]
nick_input: none
completed:
---

# 2753 — The 3D labyrinth skin

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches, and it needs the 3D work (3101–3104), which is parked too. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§5, and [ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
§5. Wizardry Variants Daphne draws its labyrinth in real 3D, and a step
or a turn glides. The rules still step in whole cells and quarter
turns; the glide is the view's (2743 does the same in 2D).

It builds on 2742 (the view and the explorer), 3102 (glTF meshes and
materials), 3103 (the 3D view and the software rasterizer for
`frame-png`), 3104 (the GPU painter) and 2109 (tweens).

**Either order with 2340:** if 2340 is done, reduced motion makes
steps and turns instant. If not, 2340 wires it when it lands.

**Either order with 2743:** if 2743 is done, the glide reads its
`motion_ms` in `crawl.ron`. If not, add `motion_ms` to `crawl.ron`
here (default 150; 0 is instant), and 2743's 2D motion reads the same
setting when it lands.

**Who uses it** (ADR-0033 §17's bar for engine work): crawler RPGs and labyrinth puzzle games that want Daphne's look; the same map and view as the 2D skins.

## Nick input

None.

## Scope

**In:**
- **The `3d` feature** of `shuyi-crawl`, pulling in `shuyi-3d`; its
  line in `tools/layers.ron`.
- **`crawl3d.ron`**, asset kind `Crawl3dSkin`, format `crawl3d-ron`:
  per wall, door, floor and ceiling look, a glTF mesh and material
  (3102 ids), the cell size in metres (`Fx12`), the eye height, the
  field of view, and fog (the view's depth, as fog's far distance); per
  thing look, a mesh or a billboard picture. Fuzz-style test and golden
  row.
- **The labyrinth's meshes**: built once per floor from the map: one
  draw per visible face (walls between cells drawn once, doors with
  their open state from the explorer), placed by `FxTransform`.
  Rebuilt only when a door opens or a secret is found. Only the cells
  within the view's depth plus one are sent each frame (from 2742's
  slots), so a large floor costs what a small one does.
- **The camera**: at the explorer's cell, eye height, facing; on
  `Moved` and `Turned` events a tween (2109) carries it from the old
  pose to the new over `motion_ms` (2743's setting), easing in and out;
  a bump nudges it forward and back. Camera poses are view-only: the
  rules' hash never sees them.
- **Light**: an ambient light and a point light at the eye whose range
  follows the view depth (the `light` effect deepens it), with the
  darkness of `Dark` cells as no light at all.
- **Things**: monster groups, chests and people as billboards or meshes
  at their cells.
- **The compass**: while the view's `compass` is set (2742), the
  facing's letter in a corner of the 3D view, as 2743's skins draw it.
- **Pictures**: `frame-png` through the software rasterizer at three
  places and mid-glide, and the 3D view's snapshot text.
- **The example's 3D look. Either order with 2754:** if 2754 is done,
  add a 3D look to `examples/crawler` as a third option in its
  settings: a `crawl3d.ron` with meshes and materials made for it
  (ours, CC0, or recorded in `THIRD_PARTY_ASSETS.md`), its `frame-png`
  picture, and a paragraph on swapping to it in
  `docs/guides/dungeon-crawler.md`. If not, nothing to do here (2754
  adds them when it lands).
- **The `shuyi-crawl` skill**, if it exists: a section on the 3D skin,
  and 2753 dropped from its `waits_for` (2741 put it there).

**Out (do not do):**
- free looking around (that is ADR-0011's `FirstPerson`, 3106);
- battle drawing in 3D (2749 draws over whatever is below);
- the example's other art and looks (2754 records what it uses);
- the editor of 3D labyrinths (the map stays a `crawl-ron` text).

## Implementation steps

1. The feature and the format.
2. Building meshes from a floor; culling to the view.
3. The camera rig and its tweens; the bump.
4. Light and darkness; things.
5. Pictures and snapshots.

## Acceptance criteria

- [ ] A floor of 50×50 cells sends no more draws than the view's slots
      allow (a test counts them).
- [ ] The same map and explorer give the same 3D view snapshot text.
- [ ] A step glides over `motion_ms` and is instant at 0 (a test that
      steps the clock).
- [ ] `frame-png` pictures from the software rasterizer at three places
      and mid-glide.
- [ ] The web build draws it through the GPU painter (checked in the
      browser pane, with a screenshot in the PR).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: mesh building per edge kind; culling; the camera tween.
- Snapshot: the 3D view text and `frame-png` pictures.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
