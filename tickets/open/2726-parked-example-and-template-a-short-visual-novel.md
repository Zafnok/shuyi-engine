---
id: "2726"
title: "Parked: Example and template: a short visual novel with three endings"
type: feature
milestone: "Engine: visual novels"
model: reasoning
effort: high
status: blocked
blocked_by: ["2724", "2725", "2501", "2004"]
nick_input: none
completed:
---

# 2726 — Example and template: a short visual novel

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
§4: a module is done when an example shows a direction it serves, and
[ADR-0003](../../docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)
§1: the example is also its integration test. This ticket finishes
`shuyi-vn` (2721–2725,
[ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md))
with a short visual novel, a template part, the direction guide, and
the README, skill and catalog row.

Nick, 2026-10-18 (`docs/decisions.md`): "Inspired by, nothing copied."
The story, characters, names, art and sounds are written and made for
this example. Nothing comes from an existing visual novel, and nothing
from Visions of Shuyi (`AGENTS.md` rule 4).

Templates are data made of parts (2501, ADR-0013 §3). The guide follows
2004's form (ADR-0009 §5).

## Nick input

None.

## Scope

**In:**
- **`examples/visual-novel`**, a game on `shuyi` with the `vn` feature:
  - **an original short story**, written by whoever does this ticket:
    about 15 minutes of reading, two characters, at least three choices
    that matter, and three endings. Keep it light and self-contained;
  - **two characters**, each one pose and three expressions (layers as
    2721 defines), with mouths and blinks (2725);
  - **three backgrounds** and one event picture per ending, so the
    gallery has something to unlock;
  - **a few voiced lines** per character, with baked envelopes, so the
    voice path runs in CI. The voices are ours: recorded by a
    contributor who releases them, or made with a free tool;
  - **placeholder art and sound made by us**, released CC0-1.0, listed
    in `THIRD_PARTY_ASSETS.md` as our own (created if it doesn't exist).
    Simple shapes are fine; the point is the engine, not the art;
  - **the story written once in `.dlg`**. If 2202 is done, one scene in
    Yarn Spinner and one in Ink, in the same story, so every dialogue
    format's stage directions run in CI. If not, `.dlg` only, and say so
    in the README;
  - `vn.ron` with places, transitions and the layout;
  - its keymap file with default layouts, and `playdate` and
    `steam-deck` layouts (the `input-actions` skill; the engine's code
    names no key);
  - the flow: a title screen, the story, and the save, load, gallery,
    endings and route chart screens (2724).
- **Tests** of the example:
  - a Harness playthrough to each of the three endings, through actions
    only, checking `VnState`'s outcome;
  - a replay determinism test: each ending's command record gives the
    same hash list twice;
  - `frame-png` pictures at three stops (a line with a speaker, a
    choice, an event picture);
  - a save made mid-story restores to the same state;
  - `shuyi check` passes on the example with no problems, and
    `shuyi route-chart` matches a checked-in Mermaid file;
  - `shuyi check --target playdate` and `--target steam-deck` pass.
- **The template part** (2501): in `templates/shuyi-default/`, a part
  `kind-visual-novel`, `when: Picked(kind, visual-novel)`:
  - it turns on the `vn` module, adds a story stage, a starter `.dlg`
    scene of a few lines with one choice, one character file, one
    background, `vn.ron`, and keymap layouts;
  - a game made with it builds, and its test plays the starter scene to
    its end;
  - if 2502 is done, `shuyi new demo --kind visual-novel` makes it. If
    not, `shuyi new demo --set kind=visual-novel` does, and 2502 adds
    the flag.
- **The direction guide** `docs/guides/visual-novel.md`, in full and in
  2004's order:
  - which template starts it (the part above);
  - which kits and modules it uses: `shuyi-vn`, the dialogue formats,
    2231's clips;
  - what looks related but isn't for it: the plain dialogue box of a
    turn or tick game, cinematics (timed shots, not a story the player
    reads), and `shuyi-mystery` for clues and deductions (2727–2732);
  - which example to read first.
- **The README** of `crates/vn`, finished in ADR-0009 §4's order: the
  cost section with the WASM size it adds, measured on the example
  against the same build without the `vn` feature; how to start, as the
  smallest working code; how it sits beside other `kind` answers (a
  strategy or tactics game telling its story through it). Include how a
  gamepad, the Steam Deck and the Playdate reach each action, by action
  name only.
- **The skill** `shuyi-vn` in the skill pack
  (`skills/shuyi-vn/SKILL.md`, which 2721 writes). **From 1134
  (done):** finish it, with a worked example,
  `skills/shuyi-vn/example.ron`, that `shuyi add vn`, a character file
  and a scene make run on a fresh game, and drop 2726 from its
  `waits_for` (2721 put it there). It passes `check-skills`, and CI's
  `skills` job runs the example.
- **`needs`** (ADR-0012 §1), if 3017 is done: check the module's needs
  against what the example uses, and correct them.
- **The catalog**: set `shuyi-vn`'s row in `docs/modules.md` and the
  `visual-novel` answer in `docs/modules.ron` to ready.
- **The website** (2801, done): the example's card,
  `site/content/examples/<name>.md`, on the Examples page.

**Out (do not do):**
- a long story, more characters or more art than listed;
- anything taken from an existing visual novel or from Visions of Shuyi;
- the detective example (2732);
- new engine features. A gap the example shows becomes a ticket
  (`write-ticket` skill).

## Implementation steps

1. Write the story's outline (scenes, choices, endings) and check it
   with `shuyi route-chart` before writing the lines.
2. Write the story, the characters, `vn.ron`, and make the placeholder
   art and sounds. Bake the voice envelopes.
3. Write the flow and the screens' wiring, and the keymap file.
4. Write the tests.
5. Write the template part and its CI build.
6. Measure the WASM size. Write the README, the guide and the skill.
   Set the catalog row.

## Acceptance criteria

- [ ] Each of the three endings is reached by a Harness test through
      actions only.
- [ ] Each ending's replay gives the same hash list twice.
- [ ] The `frame-png` pictures are checked in, and a mid-story save
      restores to the same hash.
- [ ] `shuyi check` reports nothing on the example, and the route chart
      matches its checked-in file.
- [ ] A game made from the `kind-visual-novel` part builds and passes
      its test in CI.
- [ ] `docs/guides/visual-novel.md` exists, and 2004's guide check
      passes.
- [ ] The README states the measured WASM size, and the catalog row is
      ready; `cargo xtask check-modules` passes.
- [ ] `THIRD_PARTY_ASSETS.md` lists every picture and sound of the
      example.
- [ ] `check-keys` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the three Harness playthroughs; replay determinism;
  `frame-png`; the mid-story save; `shuyi check` and `shuyi route-chart`
  on the example; the template part's build and test.
- Doc test: the README's smallest working code.

## Completion notes
