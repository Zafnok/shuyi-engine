---
id: "2727"
title: "Parked: The shuyi-mystery module: clues, people and places as data, the case file and its screen"
type: feature
milestone: "Engine: detective games"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["2722", "2116"]
nick_input: none
completed:
---

# 2727 — The shuyi-mystery module and the case file

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-18 (`docs/decisions.md`): the engine should support
"detective games". [ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
sets the design. Detective games are a module, `shuyi-mystery`, built on
the visual novel module `shuyi-vn` (§1), because questioning and
testimony are told the way a visual novel is.

This ticket is ADR-0016 §1 (for `shuyi-mystery`) and §6: the crate, its
module duties, clues, people and places as data, `CaseState` and
`MysteryRules`, the dialogue commands that find clues, and the case file
screen. It builds on:

- 2721: `VnState`, `VnRules: TurnRules`, stage directions read as
  dialogue commands, characters as data;
- 2722: `VnScene` and the visual novel skin, which the case file screen
  sits on top of;
- 2116: golden tables (`shuyi_assets::golden`) and CI's `determinism`
  job, which check the formats' decode hashes on x64, ARM64 and wasm32.

The later tickets add to this crate: investigation (2728), deductions
(2729), testimony and penalties (2730), the fair-play check and hints
(2731), and the example and template (2732).

Rules that apply: the core is pure and deterministic (ADR-0002); formats
are plugins (ADR-0007); a module ships with what ADR-0009 §4 lists;
actions, never keys (`input-actions` skill). Nothing from Visions of
Shuyi and nothing from existing detective games goes into the crate or
its fixtures.

## Nick input

None.

## Scope

**In:**
- **The crate** `crates/mystery` (`shuyi-mystery`), with every module
  duty of ADR-0009 §4:
  - it depends on `shuyi-core`, `shuyi-assets` and `shuyi-vn`, and on
    `shuyi-ui` only behind its own `view` feature, which turns on
    `shuyi-vn`'s `view` feature (2721, 2722);
  - metadata under `[package.metadata.shuyi]`: concern `kind`, answer
    `mystery`; `serves` every perspective (ADR-0016 §1); `works_with`
    `shuyi-vn`; `alternatives` empty, because 2003 marks the `kind`
    concern `alternatives: false`;
  - the README in the fixed order of ADR-0009 §4. Its first section says
    it depends on `shuyi-vn` and why (ADR-0016 §1). Sections 1–3 are
    written in full; sections 4–5 point at 2732, which finishes them;
  - the `mystery` feature of the `shuyi` crate (2003), which also turns
    on `vn`;
  - a row in `docs/modules.md` (2003 wrote it as parked): set its status to
    building, with tickets 2727–2732. 2732 sets it to ready;
  - **the building status:** 2003's catalog has none
    (`shuyi_cli::catalog::Status` is ready, planned or parked). If it
    still has none, add `Status::Building`, named `building`, with its
    doc, the `status` line of `docs/modules.ron`'s header, *How to read
    it* in `docs/modules.md` and the `modules` schema. `Status` is a
    public enum: write a `Breaking:` line if CI's `api` job reports
    one;
  - the `shuyi-mystery` skill. **From 1134 (done):** a short skill,
    `skills/shuyi-mystery/SKILL.md`, in the pack's form, saying what
    this ticket built. It has no worked example yet, so its `metadata`
    has `waits_for: "2732"`: 2732 gives it its worked example and drops
    its id. It is a module's skill: `shuyi add mystery` copies it into a
    game;
  - its `needs` (ADR-0012 §1) if 3017 is done: what `shuyi-vn` needs,
    plus nothing more. If 3017 isn't done, 3017 gives it a `needs` key
    with the other modules.
- **The `kind` concern** (ADR-0015 §1, ADR-0016 §1). 2003 added it to
  `docs/modules.ron`, with `mystery` as a parked answer by
  `shuyi-mystery`. Set that answer's status to building, with tickets
  2727–2732.
- **Three asset kinds and their formats** (ADR-0007), registered by the
  module's `register(&mut Formats)`:
  - `Clue` from `*.clue.ron` (format id `clue-ron`): `id`, `name` and
    `description` as text keys, `picture` (an image asset id), `role`
    (`Evidence`, the default, or `Colour` for a clue that is there for
    the story and solves nothing; 2731 reads it), and `stages`: the
    updates a clue can go through when more is learned, each with an
    `id`, and its own `name`, `description` and `picture` where they
    change. Stage `found` is implicit and comes first;
  - `Person` from `*.person.ron` (`person-ron`): `id`, `name` and
    `description` as text keys, an optional `character` (a
    `*.character.ron` id from 2721, for the portrait), and `stages` like
    a clue's;
  - `Place` from `*.place.ron` (`place-ron`): `id`, `name` and
    `description` as text keys, and an optional `background` (a `bg`
    name from 2721). These are places to go, not the screen places
    (`left`, `center`) of `vn.ron`; the docs call them "places to go"
    where both could be meant;
  - each format: `deny_unknown_fields`, size limits, every error with
    file and line, never a panic.
- **Module settings**, `mystery.ron` beside `vn.ron`: the case's
  `closing_endings` (the `ending <id>` values of 2721 that close the
  case; empty means any ending). Later tickets add their fields with
  defaults, so old files keep loading.
- **`CaseState`** in `shuyi_mystery::case` (ADR-0016 §6): the clues
  found, each with its current stage; the people met, each with its
  stage; the deductions made (a set of ids, filled by 2729); the
  penalties taken (a count, used by 2730). Every collection is a
  `BTreeMap` or `BTreeSet`, so iteration and hashes are the same on
  every platform. `CaseState::hash()` and a text dump, one fact per
  line.
- **`MysteryRules: TurnRules`** in `shuyi_mystery::rules`: 2721's
  `VnRules` (with its `VnState`) and `CaseState` in one value
  (ADR-0016 §6), so a case is its seed plus its commands.
  - `MysteryCommand::Story(VnCommand)` passes `Advance`, `Choose` and
    `Jump` to the story. Later tickets add their own variants.
  - `check` and `legal_commands` agree, as 2721 requires of `VnRules`.
  - A **mode stack**: `Mode::Story` here; 2728, 2729 and 2730 add
    theirs.
  - A **node call**, `MysteryRules::call(label)`: run one labelled
    block of dialogue and come back to the mode and the line that called
    it when the block ends. 2728's examine and 2730's press use it. If
    2721's `VnRules` can only `Jump`, add a small call stack to
    `VnState` in `shuyi-vn` in this PR (`VnRules::call(label)`; the end
    of a called block returns instead of ending the story), with tests
    in `shuyi-vn`.
  - **Going back**: on `shuyi_core::history::History<MysteryRules>`, as
    2721's `history::back` is on `History<VnRules>`: the replay is
    `History::rewind_to`, never a copy. Only the point is the module's,
    by 2721's rule (one command is one step; `back_past_choices` stops
    after the last `Story(Choose)`). If 2721's `back` takes
    `History<VnRules>` only, make it generic in `shuyi-vn` over a small
    trait that says whether a command is a choice, so both modules
    share it, with tests in `shuyi-vn`.
  - **Saves**: a case saves as its `MysteryCommand`s. If 2724 is done,
    make `VnSave` generic over the command type (each command keeps its
    `at_line` id), so a case uses it unchanged. If 2724 isn't done,
    2724 does it when it lands (its scope says so).
  - `Event`s: `ClueFound`, `ClueUpdated`, `PersonMet`, `PersonUpdated`,
    plus the story's own.
- **Dialogue commands** read by `MysteryRules` (ADR-0016 §6), in every
  dialogue format (`@clue …` in `.dlg`, `<<clue …>>` in Yarn Spinner,
  `# clue …` as an Ink tag):
  - `@clue <id>`: the clue is found, at its first stage. Found twice
    does nothing the second time;
  - `@clue <id> <stage>`: the clue moves to that stage. A clue moves
    only forward: naming its current stage or an earlier one changes
    nothing and sends no event, so a story that passes the same line
    twice is safe. A clue not yet found is found at that stage;
  - `@person <id>` and `@person <id> <stage>`: the same for people.
  - They reach `MysteryRules` the way a game's own commands do in
    2721: registered with `VnRules::with_commands`, they come out as
    `VnEvent::Custom { name, args }`, and `MysteryRules` applies them
    to `CaseState`. Any other unknown command stays an error with file
    and line (ADR-0016 §2).
  - Some later commands (`@investigate`, `@deduce`, `@testimony`) must
    stop the story where they are, as a choice does. If 2721's custom
    commands can't be stops, add a flag to `with_commands` in
    `shuyi-vn` (`stops: true`: the `Advance` that reaches the command
    ends there). This ticket's commands don't stop; test the flag with a
    fixture command.
  - The Yarn Spinner and Ink forms work wherever 2721 made stage
    directions work. Add these commands' rows to
    `docs/formats/dialogue.md` beside 2721's.
- **A reference check**, `shuyi_mystery::check::references`: every
  `@clue`, `@person` and stage named in a script exists, with file and
  line. If 2724 is done, call it from `shuyi check` the way 1129's
  check operation calls `shuyi_vn::check`, when the game has the
  `mystery` feature on. If not, 2731 wires it.
- **`MysteryStage`**, behind the `view` feature: the screen that plays
  a case, like 2722's `VnStage` but owning `MysteryRules`. It reuses
  `VnPresenter`, `build_scene` and `VnSkin` (2722). If `VnStage` is
  written for `VnRules` only, make it generic in this PR over a small
  trait that gives a `&VnState` and takes a `VnCommand`, so both
  stages share it.
- **The module's actions**, `MysteryAction` in
  `shuyi_mystery::view::input`: `CaseFile` (optional) here; 2730 adds
  `Present`. A game maps its own `ActionSet` onto them through a trait
  `MysteryActions: ActionSet`, the way 2723 maps `VnAction`. If 2723 is
  done, the case file is also a row of its quick menu, so a target with
  few inputs reaches it with only the required actions bound. If 2723
  isn't done, 2723 adds that row when it lands (its scope says so).
- **The case file screen** (ADR-0016 §6), behind the `view` feature:
  - `CaseFileScene`, plain data: the tab (clues, people, places), the
    rows (id, name key, stage), the selected row, and its detail
    (picture, description key);
  - a mode: `Browse`, or `Choose { prompt }`, where confirming a row
    returns its id to the caller instead of only showing it. 2729 and
    2730 use `Choose` to pick a clue to link or present;
  - `CaseFileScreen` reads `CaseState` and never changes it. It reacts
    to `UiAction`s only (Up, Down, Left, Right, Confirm, Cancel): left
    and right change the tab. It declares its actions (`Screen::actions`,
    3002);
  - a "new" mark on rows found since the screen was last open. It is
    the screen's own state, not `CaseState`, so it never changes a
    replay (as ADR-0016 §3 keeps conveniences out of the story);
  - `CaseFileSkin`: rows, tabs and text as glyph cells (ADR-0003 §5),
    using text by key; pictures as sprite items in screen space. Colours
    by role, from the visual novel skin's palette (2722).

**Out (do not do):**
- investigation and hotspots, and the `@investigate` command (2728);
- deductions, the board, blanks, and `@deduce` (2729);
- testimony and the penalty meter (2730). `CaseState` only keeps the
  count;
- the fair-play check, the unused clue warning and hints (2731);
- the example, the template part and the direction guide (2732);
- anything else the visual novel module owns: the text box, the
  backlog, the save screens, story checks, the route chart (2721–2726);
- an optimized format for these files. They are small text; ADR-0007 §4
  allows one later, derived and tested equal.

If the PR grows well past about 600 changed lines, split the case file
screen and `MysteryStage` into a parked ticket of their own
(`write-ticket` skill), add it to the `blocked_by` of 2728, 2729 and
2730 (their screens sit on them), and say so in the Completion notes.

## Implementation steps

1. Create `crates/mystery` with its `Cargo.toml` metadata, the `view`
   feature, the README's first three sections, the `mystery` feature in
   `crates/shuyi`, the catalog row and the `kind` answer. Run
   `cargo xtask check-modules`.
2. Write `shuyi_mystery::data`: `Clue`, `Person`, `Place`, `ClueStage`,
   `ClueRole`, their ids as newtypes over `String`, and the three
   formats in `shuyi_mystery::formats`, with good and bad fixtures in
   `crates/mystery/tests/fixtures/`. The good fixtures' decode hashes
   go in a golden table, `crates/mystery/tests/golden/formats.ron`,
   through 2116's `shuyi_assets::golden` (a dev-dependency on
   `shuyi-assets` with its `golden` feature), checked by a test with
   `golden` in its name. Add `-p shuyi-mystery` to the wasm32 run of
   CI's `determinism` job.
3. Write `CaseState` with `find_clue`, `update_clue`, `meet_person`,
   `update_person`, `hash` and `dump`.
4. Write `MysteryRules`: the command enum, the mode stack, `@clue` and
   `@person` through `VnRules::with_commands`, and the `stops` flag and
   the call stack in `shuyi-vn` if 2721 lacks them.
5. Go back with `History<MysteryRules>` and 2721's `back` (made
   generic if it needs to be), and the save change in 2724's code if
   2724 is done.
6. Write `check::references` and its fixtures.
7. Write `MysteryStage`, `MysteryAction`, and `CaseFileScene`,
   `CaseFileScreen` and `CaseFileSkin` behind `view`.
8. Add a small story fixture (a few `.dlg` lines that find two clues,
   update one, and meet a person) for the rules and screen tests.

## Acceptance criteria

- [ ] `cargo xtask check-modules` passes with `shuyi-mystery`, its
      metadata, its README and its catalog row, and the `kind` concern
      lists `mystery`.
- [ ] `cargo build -p shuyi-mystery --no-default-features` builds
      without `shuyi-ui`.
- [ ] Each good fixture decodes to its row in a golden table (2116's
      `shuyi_assets::golden`), and `-p shuyi-mystery` joins the wasm32
      run of CI's `determinism` job.
- [ ] Each bad fixture gives an error naming its file and line.
- [ ] Playing the story fixture gives a checked-in `CaseState` dump,
      and replaying its commands gives the same hash at every step.
- [ ] Moving a clue to an earlier stage changes nothing and sends no
      event.
- [ ] A node call runs its block and comes back to the line that
      called it, and `history::back` by one command gives the state
      before that command (property test).
- [ ] An unknown `@clue` id in a script is reported with file and line.
- [ ] A Harness test opens the case file over the visual novel screen,
      changes tab and row with `UiAction`s, and reads the scene.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each `CaseState` method; each command, legal and refused;
  `references` on good and bad scripts; the screen's tab and row moves;
  the `stops` flag and the call stack in `shuyi-vn`, if added here.
- Property: random bytes into each of the three formats never panic;
  random command lists never leave a clue at an earlier stage than
  before; `legal_commands` equals the set `check` accepts; a refused
  command leaves the hash unchanged.
- Golden: decode hashes per format in `formats.ron`, natively in CI's
  `test` job (x64, ARM64) and on wasm32 in its `determinism` job.
- Fixture and determinism: the story fixture's dump and per-step
  hashes.
- Snapshot: the `CaseFileScene` dump; one `frame-png` picture of the
  case file skin (a test of a look, so it reads cells and sprites).

## Completion notes
