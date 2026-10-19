---
id: "3108"
title: "Parked: Example: a small walkable 3D place in first and third person, the first-person and third-person templates, a direction menu in shuyi new, and their guides"
type: feature
milestone: "Parked: 3D"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["3104", "3106", "1129", "2004"]
nick_input: none
completed:
---

# 3108 — Example and templates: first person and third person

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
§2, and
[ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
§4–§6. A direction is supported when it has a template, a guide and an
example that passes CI. This ticket does that for **first person** and
**third person**, and finishes the parts of `shuyi-3d` that need an
example: the README's cost and start sections, and the module's skill.

ADR-0011 §2 also says `shuyi new` with no template asks which
direction, as a menu, and the same choice is a flag. Agents and scripts
never meet a prompt (ADR-0005).

**Either order with the wizard** (2501, 2502, [ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)): if 2502 is
done, the menu is the wizard's perspective question. Then write no menu,
and write the two templates as parts of the built-in template
(`stage-first-person`, `stage-third-person` in
`templates/shuyi-default`, each turning on the `3d` feature), set their
answers in `docs/modules.ron` to ready, and the CI job makes the games
with `shuyi new --no-input --perspective first-person` (and
`third-person`). If 2502 isn't done, write the menu and the template
folders as below, and 2501 and 2502 fold them in.

It builds on 3104 (the GPU painter, so the example runs in a window and
the browser), 3106 (the cameras, and through it 3105's mover), 1129
(`shuyi new` and `templates/`) and 2004 (the guides and their check).

The skill pack (1134) is MVP work and should be done by then. If it is,
write the `shuyi-3d` skill here. If it isn't, write a ticket for the
skill and link it in the Completion notes.

## Nick input

None.

## Scope

**In:**
- **`examples/courtyard`**: a small walled courtyard with stairs, a
  ramp, a raised walkway, a pillar to walk round, and a gate that opens
  when the player stands on a plate (a `Trigger3`, 3105). Two tick
  stages share the place and the mover (3105):
  - a `FirstPerson` stage with `FirstPersonLook`;
  - a `ThirdPerson` stage with `OrbitCamera` and a simple skinned
    figure with a walk clip;
  - a menu chooses the stage. Movement and look come from actions only
    (the `input-actions` skill).
- **Assets**: made for the example (CC0, ours), or under a permissive
  license (for example Kenney's CC0 3D packs) recorded in
  `THIRD_PARTY_ASSETS.md`. Nothing else.
- **Tests** (ADR-0009 §4), for each stage:
  - a Harness test that walks from the gate plate up the stairs to the
    walkway within a set number of steps;
  - a replay determinism test: the same record twice gives the same
    hash list;
  - `frame-png` pictures (3103's software painter) at three steps, and
    their 3D view snapshots.
- **The web build** of the example, and its place on the docs site's
  playable list (1132).
- **Templates** `templates/first-person` and `templates/third-person`
  for `shuyi new` (1129):
  - each turns on the `3d` feature of `shuyi`, and no other module;
  - each has one stage with the mover and its camera rig, a small room
    as `.glb`, its default bindings in a keymap file (the player's to
    change), `shuyi.ron` with `desktop` and `web` targets, and a Harness
    test and a replay test;
  - CI runs `shuyi new first-person` and `shuyi new third-person` into
    a temporary folder, builds each, and runs its tests, as 1129 does
    for `grid`.
- **`shuyi new` with no template**:
  - in a terminal, it lists the templates by direction, from
    `docs/modules.ron`, each with one line, and reads a number;
  - `shuyi new --template <name>` (or the name as before) does the same
    with no prompt;
  - with no terminal, or with `--json`, it never waits: it fails with
    the list of templates in the error (ADR-0005);
  - no new dependency for the menu. If another ticket has added the menu
    already, only add the two directions to it.
- **Guides** (ADR-0009 §5, 2004's form): `docs/guides/first-person.md`
  and `docs/guides/third-person.md`, each saying which template starts
  it, which kits and modules it uses, which look related but aren't for
  it (the 2D top-down controller, the diorama skin), and which example
  to read first. If 2004 left planned pages for them, replace those.
- **`crates/3d/README.md`**: section 3 with the WASM size the module
  adds to `courtyard`, measured against the same example's menu alone,
  and section 4 as the smallest working stage.
- **The catalog**: `shuyi-3d`'s row claims first person and third
  person as ready.
- **The debug view (2320).** If 2320 is done, the third-person example
  calls its `bones` and `collision3` helpers. If it isn't, 2320 does
  this.

**Out (do not do):**
- top-down 3D and HD-2D (3109);
- gameplay beyond walking and the gate;
- sound;
- a move guide between first and third person (write one if Nick asks
  for the move).

## Implementation steps

1. Make the courtyard's mesh, the figure, its walk clip and the gate,
   and record any asset not ours.
2. Write the two stages, the menu, and the gate trigger.
3. Write the tests and the web build.
4. Write the two templates and their CI job.
5. Add the direction menu to `shuyi new`, with tests for the terminal,
   no-terminal and `--json` cases.
6. Write the guides, the README sections and the skill. Set the catalog
   row.

## Acceptance criteria

- [ ] The Harness, replay and `frame-png` tests pass for both stages.
- [ ] The web build runs the example in the browser pane.
- [ ] CI makes a game from each template, builds it, and its tests pass.
- [ ] `shuyi new` with no template and no terminal fails at once with
      the list of templates (a test), and never waits for input.
- [ ] The README states the measured WASM size.
- [ ] `check-skills` passes on `shuyi-3d`, if the skill was written.
- [ ] `cargo xtask check-modules` passes, with both guides found by its
      guide check.
- [ ] Every asset not made for the example is in
      `THIRD_PARTY_ASSETS.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: Harness, replay and `frame-png` for each stage; each
  template built and tested in CI.
- Unit: the `shuyi new` menu's three cases.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
