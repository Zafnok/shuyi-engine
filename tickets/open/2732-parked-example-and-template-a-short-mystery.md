---
id: "2732"
title: "Parked: Example and template: a short mystery"
type: feature
milestone: "Engine: detective games"
model: reasoning
effort: high
status: blocked
blocked_by: ["2731", "2726"]
nick_input: none
completed:
---

# 2732 — Example and template: a short mystery

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0009 §4: a module claims a kind of game only when its example
passes CI. This is `shuyi-mystery`'s example, template part and
direction guide
([ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
§1). It uses everything 2727–2731 built: the case file, investigation,
both deduction shapes, testimony with a meter, and the fair-play check,
which runs on it in CI (§9).

It builds on the visual novel's example and template part (2726): a
mystery is told the way a visual novel is, so its template part sits on
top of the visual novel's. The template format (2501) and the guide
check (2004) are done before this starts, because 2726 needs them. The
wizard's `--kind` flag (2502) may not be; the template part says what
to do in each case.

Nick, 2026-10-18 (`docs/decisions.md`): inspired by, nothing copied.
The case is original, written by whoever does this ticket. No name,
character, story, puzzle, art or sound from an existing detective game,
and nothing from Visions of Shuyi.

## Nick input

None.

## Scope

**In:**
- **`examples/mystery/`**, using `shuyi` with the `mystery` feature
  only (2003). An original short case:
  - one location with **3 rooms**, each a place with an investigation
    (2728), joined by `Go` hotspots;
  - **3 people**, each with a `*.character.ron` (2721) and a
    `*.person.ron` (2727), one of whom gives the testimony;
  - about **8 clues**, at least one with a second stage, and one marked
    `Colour`;
  - **one links deduction** on the board, and **one blanks deduction**
    with at least 6 slots, so the group of 3 matters (2729);
  - **one testimony** with presses and one contradiction, and a meter
    whose `on_empty` is `RestartTestimony` (2730);
  - **hints** for both deductions (2731);
  - a closing ending, and one `other_endings` entry if the story has a
    bad ending by choice;
  - its scripts in `.dlg`, its text in an English language pack, its
    keymap with its own defaults and `playdate` and `steam-deck`
    layouts (the `input-actions` skill: the example's defaults live in
    its own keymap file). No text names a key.
- **Placeholder art made by us**, released CC0-1.0: backgrounds for the
  3 rooms, a body and two expressions per person, and a picture per
  clue. Simple shapes are enough. Each file is listed in
  `THIRD_PARTY_ASSETS.md` as our own; create the file if no ticket has
  yet (`AGENTS.md` rule 7).
- **Tests** in `examples/mystery/tests/`, driven by the Harness:
  - a playthrough from the start to the case closed, with scripted
    `UiAction`s (Harness scripts may press the example's own default
    keys), reading `CaseState` and the scene, never pixels, to check
    what happened;
  - a wrong-presentation path: a wrong clue in the testimony takes a
    penalty, runs the wrong node and returns to the testimony; emptying
    the meter restarts it; the right clue then moves the story on;
  - a replay test: the playthrough's commands, replayed, give the same
    hash at every step, and going back (2721) and loading a save (2724)
    in the middle give the same state as playing to that point;
  - `frame-png` pictures of one room with found and unfound hotspots,
    the case file, the links board, the blanks board, and a testimony
    statement with the meter;
  - `shuyi check` on the example passes, with the fair-play check and
    no warnings. CI runs it;
  - the fair-play check's state count on the example goes in the
    Completion notes, beside the counts 2731 wrote for its fixtures.
    If it passes `fair.max_states`, split the case as the module's
    README says. If it comes near it, write a ticket (parked like this
    one) for the reduction 2731 describes: examines applied together
    as one edge, with its property test;
  - `shuyi check --target playdate` and `--target steam-deck` pass on
    the example (3002).
- **The web build** of the example builds in CI, like the other
  examples.
- **The template part** (ADR-0013, 2501): `kind-mystery` in
  `templates/shuyi-default/`, `when: Picked(kind, mystery)`:
  - it `requires` 2726's `kind-visual-novel` part. Change that part's
    `when` to
    `Any([Picked(kind, visual-novel), Picked(kind, mystery)])`, so
    picking `mystery` alone brings the story stage;
  - it turns on the `mystery` module, puts `MysteryStage` (2727) in
    place of the story stage's `VnStage`, and adds a case of one room,
    two clues, one links deduction and `mystery.ron`;
  - a game made with it builds, and its test plays the case to its end
    and runs `shuyi check`;
  - if 2502 is done, `shuyi new demo --kind mystery --no-input` makes
    it; if not, `shuyi new demo --set kind=mystery` does, and 2502 adds
    the flag (as 2726 does for `visual-novel`). CI makes it in a
    temporary folder, builds it and runs its tests.
- **The direction guide** `docs/guides/mystery.md`, in the fixed order
  of ADR-0009 §5 (as 2004 writes them): which template starts it (the
  `kind-mystery` part and the command that makes it), which modules it uses
  (`shuyi-vn`, `shuyi-mystery`), what looks related but isn't for it
  (the tactics kit; dialogue alone without `shuyi-vn`), and that this
  example is the one to read first. It links the module READMEs rather
  than repeating them, and says how to write fair cases with the check.
  2004's guide check then finds it for the `mystery` answer.
- **Finishing the module**: `crates/mystery/README.md` sections 4 and 5
  (the smallest working code, taken from this example's tests, and how
  it sits beside `shuyi-vn` alone); `docs/modules.md` and
  `docs/modules.ron` mark `mystery` ready; the engine README lists
  detective games as supported, linking this example.
- **The skill** `shuyi-mystery` (`skills/shuyi-mystery/SKILL.md`, which
  2727 writes). **From 1134 (done):** finish it with a worked example,
  `skills/shuyi-mystery/example.ron`, modelled on this example, in which
  `shuyi add mystery` and a case of one room and two clues run on a
  fresh game, and drop 2732 from its `waits_for` (2727 put it there).
  It passes `check-skills`, and CI's `skills` job runs the example.

**Out (do not do):**
- new engine features; a gap found here gets a ticket (`write-ticket`,
  parked like this one);
- world investigation in a tick stage (2728, or its follow-up ticket,
  if 1122 wasn't done then);
- voices (2725 owns them; the example has none);
- bought or downloaded art, and anything from an existing detective
  game or from Visions of Shuyi.

## Implementation steps

1. Write the case on paper first, in `examples/mystery/README.md`: the
   rooms, people, clues, deductions, testimony, and the order a player
   could find them in. Run the fair-play check on the data as it grows.
2. Write the data, scripts, language pack and keymap.
3. Make the placeholder art and list it in `THIRD_PARTY_ASSETS.md`.
4. Write the Harness tests, the replay test and the `frame-png`
   pictures.
5. Write the template part and its CI step.
6. Write the direction guide and finish the README, the catalog and
   the engine README.

## Acceptance criteria

- [ ] The playthrough, the wrong-presentation path, the replay and the
      `frame-png` tests pass in CI.
- [ ] `shuyi check` passes on the example with no warnings, in CI.
- [ ] A game made with the `kind-mystery` part (by `--kind mystery` or
      `--set kind=mystery`) builds and its tests pass, in CI; picking
      `visual-novel` alone still makes 2726's game unchanged.
- [ ] Every asset is listed in `THIRD_PARTY_ASSETS.md` as CC0-1.0, made
      by us.
- [ ] `docs/guides/mystery.md` exists in the fixed order, and
      `check-modules` passes with `mystery` ready.
- [ ] The engine README lists detective games, linking the example.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the playthrough, the wrong-presentation path, the replay
  with going back and loading, `shuyi check` on the example, and the
  template's generated game.
- Snapshot: the `frame-png` pictures above.

## Completion notes
