---
id: "2719"
title: "Parked: Example and template: a small RTS against the computer, and a thousand-unit battle"
type: feature
milestone: "Engine: real-time strategy"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2715", "2716", "2734", "2501", "2004"]
nick_input: none
completed:
---

# 2719 — Example and template: a small RTS, and a thousand-unit battle

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
§4: a module ships with an example, a small game of a direction it
serves, driven by the Harness, with a replay determinism test and a
`frame-png` picture. Its README claims a direction only when the example
passes CI. This is that example for `shuyi-rts`
([ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)),
plus:

- a stress scene of a thousand units, so a developer sees the scale the
  engine is tested at;
- a template part, so `shuyi new --kind rts` starts an RTS (2501's parts,
  [ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md));
- a guide in `docs/guides/` (2004's style).

Nick, 2026-10-18 (`docs/decisions.md`): "Inspired by, nothing copied."
No name, unit, map, number, art or sound comes from an existing strategy
game.

The example needs workers that gather and buildings that produce
(2733, the RTS economy), and a way for the player to place buildings
and order production (2734). Both must be done first.

## Nick input

None.

## Scope

**In:**
- **`examples/rts`**, a top-down free game:
  - a small map made in Tiled or LDtk, read through 2709's
    `TerrainGrid::from_layer`, if 1124 is done and `from_layer` exists
    (whether it does depends on which of 1124 and 2709 landed first);
    in code if not;
  - two bases, one per side, and one resource;
  - kinds, all our own and named plainly: a worker, a melee unit, a
    ranged unit, a base and one production building, as `*.unit.ron`;
  - a person against a computer player with a commander file and three
    difficulty presets (easy, normal, hard) as data;
  - win and loss: a side loses when it has no buildings left;
  - its own keymap file with default keys for every `RtsAction` (2716).
    No key is named anywhere else, help text included.
- **Art:** original placeholders made by us (plain shapes and
  colours, drawn for this example), released CC0-1.0, each listed in
  `THIRD_PARTY_ASSETS.md` as our own (create the file if it doesn't
  exist yet, as `AGENTS.md` rule 7 says). No bought or downloaded art.
- **The stress stage**, `examples/rts` stage `battle-1k`: two computer
  players with 500 units each meeting in the middle, with the camera
  over the fight.
- **Tests:**
  - a Harness integration test: the person's side, driven by a script of
    actions and pointer gestures, gathers, produces three units and
    destroys a fixture target;
  - replay determinism: a full easy match recorded and replayed, the
    same hash at every step;
  - `frame-png` snapshots of the start of the match and of the
    thousand-unit battle at step 600;
  - the web build of the example builds in CI.
- **The template part** in `templates/shuyi-default/` (2501): part
  `kind-rts`, when the `kind` answer `rts` is picked. It turns on the
  `rts` module, adds a stage with one base, a worker and one unit kind,
  and its keymap file. `shuyi new demo --set kind=rts` makes a game that
  builds and passes its tests in CI. If 2502 is done, `--kind rts` does
  the same through the wizard; if not, 2502 picks it up.
- **The guide**, `docs/guides/rts.md`, in 2004's fixed order: which
  template starts it, what it uses, what looks related but isn't for it
  (the tactics kit, 1122's movers), which example to read first. Link it
  from `docs/guides/README.md`.
- **The module's README, finished:** every section in ADR-0009 §4's
  order, the directions it claims (top-down free; isometric stays
  planned, and 2769 claims it with an isometric stage), and links to
  the example and the guide.
- **The catalog:** set the `rts` answer under `kind` in
  `docs/modules.ron` and `shuyi-rts`'s row in `docs/modules.md` to
  `ready`, now that the example passes CI (2707 moves them off
  `parked` and leaves `ready` to this ticket, ADR-0009 §4).
- **The `shuyi-rts` skill, either order with 1134:** if 1134 is done,
  the skill (2707 or 1134 wrote it) points at the example and the
  guide. If not, add to the `shuyi-rts` entry 2707 put in 1134's list,
  in this PR: its worked example reads `examples/rts`, and it links
  `docs/guides/rts.md`.

**Out (do not do):**
- an isometric example (2769, after 1126);
- multiplayer in the example (not planned: 2720 builds lockstep for
  command sims and tests it in one process over the in-memory network;
  playing over a real network also needs 1141 and 1145);
- more kinds, maps or factions than listed;
- anything from Visions of Shuyi, and anything copied from existing
  strategy games.

## Implementation steps

1. Make the kinds, the map, the commander and the presets as data.
2. Write the example's stages, its skin, its keymap and win and loss.
3. Draw the placeholder art and list it in `THIRD_PARTY_ASSETS.md`.
4. Write the stress stage.
5. Write the Harness test, the replay test and the snapshots.
6. Write the template part and its CI job.
7. Write the guide, finish the README and update the skill (or its
   entry in 1134's list).

## Acceptance criteria

- [ ] The Harness test plays the scripted match to the expected end.
- [ ] A recorded easy match replays with the same hash at every step.
- [ ] Both `frame-png` snapshots pass.
- [ ] `shuyi new demo --set kind=rts` makes a game that builds and
      passes its tests in CI.
- [ ] `docs/guides/rts.md` exists in 2004's order and 2004's guide check
      passes.
- [ ] Every asset in `examples/rts` is listed in `THIRD_PARTY_ASSETS.md`
      as CC0-1.0, made by us.
- [ ] `cargo xtask check-keys` (or its successor) passes on the example.
- [ ] The README has every section and claims only top-down free.
- [ ] The `rts` answer in `docs/modules.ron` and `shuyi-rts`'s row in
      `docs/modules.md` are `ready`, and `cargo xtask check-modules`
      passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the Harness match; the template's game in CI.
- Determinism: the recorded match replays.
- Look: the two `frame-png` snapshots.

## Completion notes
