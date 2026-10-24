---
id: "3109"
title: "Parked: Examples: a top-down 3D yard, and the town-and-battle example drawn in HD-2D with unchanged rules; the top-down-3d and hd-2d templates and their guides"
type: feature
milestone: "Parked: 3D"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["3106", "3107", "1125", "1129", "2004"]
nick_input: none
completed:
---

# 3109 — Examples and templates: top-down 3D and HD-2D

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
§2 and §7, and
[ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
§4–§6. This ticket supports the last two directions:

- **top-down 3D**: an overhead camera over 3D models (`Overhead3D`);
- **HD-2D**: a 2D stage drawn by `shuyi-diorama` (3107).

The HD-2D example is the proof of ADR-0011 §7. It takes 1125's
town-and-battle example, a top-down town that starts a grid battle, and
draws it with the diorama skin. Its rules don't change. Only its skin
does. That is also why moving a top-down grid game to HD-2D is a skin
change, not a stage move, and its guide says so.

It builds on 3106 (the overhead camera, and through it 3105's mover),
3107 (the diorama skin, and through it 3104's painter), 1125 (the
town-and-battle example), 1129 (`shuyi new` and `templates/`) and 2004
(the guides).

**Either order with 3108:** 3108 adds the direction menu to `shuyi
new`. If 3108 is done, add the two templates to its menu and to its
tests; if not, nothing to do (3108 lists them when it lands).

**Either order with 2501 and 2502** (the wizard,
[ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)):
- if 2501 is done, write the two templates as parts of the built-in
  template (`templates/shuyi-default`), not as folders:
  `stage-top-down-3d`, turning on the `3d` feature, and HD-2D as a part
  that applies when the *World drawing* answer `diorama` is picked over
  a grid, top-down or isometric stage. Keep `top-down-3d` and `hd-2d`
  as short names;
- if 2502 is done too, there is no menu: the wizard's perspective and
  *World drawing* questions offer them. If 2502 isn't done, the menu
  lists the short names (the paragraph above), and 2502 replaces it
  (its clause says so);
- if 2501 isn't done, write the template folders as below, and 2501
  turns them into parts (its clause says so).

Either way, set both answers in `docs/modules.ron` to ready.

Copy nothing from Square Enix's games (`docs/decisions.md`,
2026-10-17): no names, art or assets.

## Nick input

None.

## Scope

**In:**
- **`examples/yard`**, a top-down 3D tick stage (`Overhead3D`):
  - a small yard of 3D props (a fence, crates, a tree, a well), a
    skinned figure with walk and idle clips, moved by the mover (3105)
    and followed by `OverheadFollow` (3106);
  - three pickups as `Trigger3`s, and a count on screen in glyph cells;
  - movement from actions only (the `input-actions` skill).
- **HD-2D for `examples/town-and-battle`** (1125):
  - a second skin, chosen in the example's settings (`skin: flat |
    diorama`), with a `.diorama.ron` file;
  - a `height` layer added to the town's Tiled map and to the battle's
    grid, and block, cliff and slope tiles in its tileset;
  - effects: `depth_of_field` (tilt-shift), `bloom`, `fog`, `vignette`;
    `light_shafts` and `colour_grade` marked `optional`;
  - **no change to the example's rules.** Its rules crate (or module,
    as 1125 laid it out) doesn't depend on `shuyi-diorama`. If 1125
    mixed rules and drawing in one crate, split them first, in this PR,
    with no change of behaviour.
- **Assets**: made for the examples (CC0, ours), or under a permissive
  license recorded in `THIRD_PARTY_ASSETS.md`. 1125's art is already
  credited there.
- **Tests**:
  - the yard: a Harness test that collects the three pickups; a replay
    determinism test; `frame-png` pictures at three steps;
  - town-and-battle: **1125's playthrough record, replayed under the
    flat skin and under the diorama skin, gives the same hash list and
    the same scene text at every step**; `frame-png` pictures of the
    town and the battle under the diorama skin;
  - `shuyi check --target web` passes on both examples.
- **The web builds** of both, and their places on the website's
  Examples page (2801).
- **Templates** for `shuyi new` (1129):
  - `templates/top-down-3d`: the `3d` feature, one `Overhead3D` stage
    with the mover and the camera, a small room as `.glb`, tests;
  - `templates/hd-2d`: the `grid` template's stage with the `diorama`
    feature, a small map with a height layer, a skin file, tests;
  - CI makes, builds and tests a game from each, as for the others.
- **Guides** (ADR-0009 §5, 2004's form):
  - `docs/guides/top-down-3d.md`: the template, `shuyi-3d`, what looks
    related but isn't for it (the 2D top-down controller, the diorama
    skin), and `yard` to read first;
  - `docs/guides/hd-2d.md`: it says first that HD-2D is a way of
    drawing, not a perspective; that its stage is a `TopDownGrid`,
    `TopDown` or `Isometric` one, and every 2D kit works under it; and
    which targets run which effects (3017);
  - the move guide `docs/guides/top-down-grid-to-hd-2d.md`: **a skin
    change, not a stage move.** Nothing is carried across a screen
    transition, because there is none. It lists the steps: `shuyi add
    diorama`, a height layer in the map, a skin file, `optional` on the
    effects a target lacks, then the replay test that proves the rules
    held;
  - if 2004 left planned pages for these, replace them.
- **`crates/diorama/README.md`** finished in ADR-0009 §4's order, with
  the WASM size it adds to town-and-battle, measured against the flat
  skin.
- **The skill** `shuyi-diorama` in the skill pack (1134), with a worked
  example that `shuyi add diorama` and a skin file make run. If 1134
  isn't done, write a ticket for the skill instead.
- **The catalog**: `shuyi-3d` claims top-down 3D as ready, and
  `shuyi-diorama`'s row is set to ready.
- **Either order with 3025 (parked, DS hardware 3D):** if 3025 is
  done, mark the effects the `nds` profile lacks `optional` in
  town-and-battle's diorama skin (`depth_of_field` and `bloom` among
  them), so it passes `shuyi check --target nds`, then export it for
  `nds` and run it in 3025's emulator job; 3025's diorama fixture may
  then go. If not, nothing to do (3025 does it).

**Out (do not do):**
- any change to the town-and-battle example's rules or its tests,
  beyond the split above;
- an isometric HD-2D example (1126's example can get a diorama skin in
  a later ticket);
- a move guide between top-down 3D and other directions (written when
  an example makes the move).

## Implementation steps

1. Make the yard's props, the figure and its clips. Write the stage, the
   pickups and the tests.
2. In town-and-battle, check that the rules don't depend on the skin;
   split them out if they do. Add the height layer, the tiles, the skin
   file and the skin setting.
3. Write the flat-against-diorama replay test, and the pictures.
4. Add the web builds and the docs site entries.
5. Write the two templates and their CI jobs.
6. Write the three guides, the README, the skill, and set the catalog
   rows.

## Acceptance criteria

- [ ] The yard's Harness, replay and `frame-png` tests pass.
- [ ] Town-and-battle's playthrough gives the same hashes and scene text
      under both skins, and its existing tests pass unchanged.
- [ ] `cargo tree` shows the example's rules don't depend on
      `shuyi-diorama` (a CI check).
- [ ] Both examples run in the browser pane from their web builds.
- [ ] CI makes a game from each template, builds it, and its tests pass.
- [ ] The move guide says it is a skin change, not a stage move, and
      lists each step the HD-2D example took.
- [ ] The README states the measured WASM size.
- [ ] `cargo xtask check-modules` passes, with every new guide found and
      both rows ready.
- [ ] Every asset not made for the examples is in
      `THIRD_PARTY_ASSETS.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the yard's Harness, replay and `frame-png`;
  town-and-battle under both skins; each template built and tested in
  CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
