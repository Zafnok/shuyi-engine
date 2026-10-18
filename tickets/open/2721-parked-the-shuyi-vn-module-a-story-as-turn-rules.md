---
id: "2721"
title: "Parked: The shuyi-vn module: a story as turn rules, stage directions as dialogue commands, and characters as data"
type: feature
milestone: "Engine: visual novels"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["game#1131", "2003"]
nick_input: none
completed:
---

# 2721 — The shuyi-vn module: a story as turn rules

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-18 (`docs/decisions.md`): "I wanna be able to support ...
visual novel style games". [ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
sets the design. This ticket is its §1 (the module) and §2 (a story as
turn rules, stage directions, characters as data).

What it builds on:

- **Turn rules** ([ADR-0003](../../docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)
  §2): `check`, `apply`, `legal_commands`, `outcome`. A run is its seed
  plus its commands.
- **The dialogue the engine already has**: `.dlg` scenes (game
  ADR-0011, game ADR-0055) loaded as the `DialogueScene` asset kind,
  with speakers, choices, `@if`, variables, line ids (game ADR-0045) and
  commands that the runner passes through. Find the runner with
  `grep -rn "DialogueScene" crates/`.
- **Formats are plugins** ([ADR-0007](../../docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md)
  §2, §5): every new file kind is a `Format`, never panics, and is
  deterministic.
- **Module duties** ([ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
  §4) and the catalog, the `shuyi` crate and `shuyi add` (2003).
- **The *Kind of game* concern** ([ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
  §1): `kind`, `pick: Many`, marked `alternatives: false`. 2003 adds it
  with `visual-novel` as a parked answer.

Drawing is 2722. The player's conveniences (text speed, auto, skip, the
backlog, the going-back control) are 2723. Saves, the gallery and the
story checks are 2724. Voices are 2725. The example is 2726.

## Nick input

None.

## Scope

**In:**
- **The crate** `crates/vn` (`shuyi-vn`), with the module duties of
  ADR-0009 §4:
  - it depends on `shuyi-core` and `shuyi-assets` only. A `view`
    feature, empty here, is filled by 2722 (it pulls in `shuyi-ui`), so
    a headless tool uses the story without drawing;
  - metadata: concern `kind`, answer `visual-novel`; `serves` every
    perspective (it draws in screen space, over any stage or alone);
    `alternatives` empty (the concern is `alternatives: false`);
    `works_with` empty for now;
  - **the `kind` concern.** 2003 adds it to `docs/modules.ron`. Set the
    `visual-novel` answer's status from parked to building, naming this
    module. If the concern is missing (2003 changed after this ticket
    was written), add it as ADR-0015 §1 says: `pick: Many`, question
    "What kind of game is it?", `alternatives: false`, answers
    `tactics` (ready), `rts` (parked, 2705–2720 and 2733–2734), `visual-novel` (this
    module) and `mystery` (parked, 2727–2732);
  - the `vn` feature of the `shuyi` crate;
  - the catalog row in `docs/modules.md` (2003 wrote it as parked): set
    its status to building, with tickets 2721–2726. 2726 sets the row
    and the answer to ready, when the example passes CI;
  - the README's first two sections in ADR-0009 §4's order (what it is
    for, what it is not for). 2726 finishes the rest;
  - the `shuyi-vn` skill: if the skill pack (1134) is done, add a short
    skill that says what this ticket built and that 2722–2726 are to
    come. If not, add `shuyi-vn` to 1134's list in this PR;
  - `needs` (ADR-0012 §1), if 3017 is done: an empty table, with a
    comment that 2726 checks it against the example. If 3017 isn't done,
    3017 gives it a `needs` key with the other modules;
  - if 2004's guide check is in place, a planned page
    `docs/guides/visual-novel.md` naming 2722–2726. 2726 writes it in
    full.
- **`VnState`** in `shuyi_vn::state` (ADR-0016 §2):
  - where the story is: the scene and the line, named by **line id**
    (game ADR-0045), never by an index into the file;
  - the dialogue variables, as the runner already keeps them;
  - the *stage*: the background, the characters on screen (each with
    pose, expression, place and draw order), the music, and the event
    picture if one is showing;
  - the endings reached in this run;
  - `hash()` over stable bytes, and a text dump, one fact per line.
  - No floats, no clock, nothing from a convenience (ADR-0016 §3).
- **`VnRules: TurnRules`** in `shuyi_vn::rules`:
  - commands `VnCommand::{Advance, Choose(u16), Jump(Label)}`. Each
    command moves the story to its next *stop*: a line shown, a choice
    offered, or the end;
  - `check` refuses: `Advance` while a choice is offered or after the
    end; `Choose` with no choice offered, out of range, or on a choice
    whose condition is false; `Jump` to an unknown label. An error
    changes nothing;
  - `legal_commands()` lists exactly the commands `check` accepts;
  - `outcome()` is `Some` when the story has ended, with the endings
    reached;
  - `apply` returns events (`VnEvent`): the line shown (line id,
    speaker, text key), the choices offered, and one event per stage
    direction run (`Shown`, `Hidden`, `Moved`, `Background`,
    `EventPicture`, `Music`, `Sound`, `Voice`, `Wait`, `Shake`, `Flash`,
    `Ending`), each carrying its transition or duration as data, and
    `Custom { name, args }` for a game's own commands. The state holds
    the end result; transitions and durations live only in the events,
    for the view (2722).
- **Going back** in `shuyi_vn::history`: `back(story, seed, commands,
  steps) -> (VnState, Vec<VnCommand>)` replays the commands minus the
  last `steps` from the start. One step is one stop, so one step back is
  the line before. `vn.ron`'s `back_past_choices` (default true) says
  whether going back may undo a choice; when false, `back` stops after
  the last `Choose`. Replaying from snapshots is 2724.
- **Stage directions** in `shuyi_vn::direction` (ADR-0016 §2's table):
  - `show <who> [pose] [expression] [at <place>] [with <transition>]`,
    `hide <who> [with <transition>]`, `move <who> to <place> [over
    <ms>]`, `bg <name> [with <transition>]`, `event <name>`, `event
    off`, `music`, `sound` and `voice` (with `.dlg`'s existing
    arguments; `voice` names a line id), `wait <ms>`, `shake [<ms>]`,
    `flash`, `ending <id>`;
  - parsed into a `Direction` enum when the story is built, not when it
    plays. `Story::build(scenes, characters, config) -> Result<Story,
    Vec<ContentError>>` walks every command of every scene once;
  - **one word after `show <who>`** is a pose if the character has that
    pose, else an expression. A word that is both is an error asking
    for both words. `show` of a character already on screen changes only
    what it names;
  - names are checked against the data: an unknown character, pose,
    expression, place or transition is an error;
  - **an unknown command is an error with its file and line**, never
    ignored. A game's own commands are listed in `vn.ron`'s `commands`
    (or registered with `VnRules::with_commands`), and come out as
    `VnEvent::Custom`. The commands `.dlg` already has and the game
    still uses (such as `@music`) are in the table;
  - these commands are read only when `shuyi-vn` is used. A game that
    doesn't use the module sees its commands passed through as today.
- **Where directions are written:**
  - `.dlg`: `@show …`;
  - **if 2202 is done**: `<<show …>>` in Yarn Spinner and `# show …` as
    an Ink tag. Check that 2202's runner hands both to the scene as
    commands, follow 2202's rule for telling a command tag from another
    Ink tag (such as a metadata tag), and add the stage-direction rows
    to `docs/formats/dialogue.md`, with one fixture per format;
  - **if 2202 isn't done**: `.dlg` only. 2202 already passes unknown
    commands through and names 2721 as the owner of these rows; whoever
    lands second does the Yarn and Ink part. Say which in the
    Completion notes.
- **Characters are data**: `*.character.ron`, asset kind
  `CharacterDef`, format `character-ron` (ADR-0007):
  - `id`, `name` (a text key), `colour` (a colour role of the skin);
  - `layers` in draw order. A layer is `per: Pose`, `per: Expression`
    or `per: Always` (extras), and maps names to pictures, each with an
    offset in whole pixels. The poses and expressions are the names of
    the `Pose` and `Expression` layers;
  - `default_pose`, `default_expression`, and an `anchor` (the point of
    the picture placed on the place), in whole pixels;
  - `deny_unknown_fields`; errors name the file and line;
  - a fuzz-style test (random bytes never panic) and a determinism
    fixture (the decoded value's hash is checked in).
- **The story's settings**: `vn.ron`, asset kind `VnConfig`, format
  `vn-ron`:
  - `places`: name to a point in screen pixels (integers). The defaults
    `left`, `center` and `right` exist unless the file replaces them;
  - `transitions`: name to a kind and a duration in ms. The kinds are
    `cut`, `fade`, `dissolve`, `slide` (with a side) and `custom`.
    Defaults exist for the first four;
  - `commands`: the game's own command names;
  - `back_past_choices`;
  - the same fuzz-style test and determinism fixture.
- Both formats are registered by the module's `register(&mut Formats)`.

**Out (do not do):**
- the scene, the skin, transitions drawn, and text revealed over time
  (2722);
- text speed, auto, skip, what has been read, the backlog, and the
  action that goes back (2723);
- saves, snapshots, thumbnails, the gallery, endings in the profile,
  `shuyi check` story rules and the route chart (2724);
- playing voices, moving mouths and blinking eyes (2725);
- the example, the template part, the full README, skill and guide
  (2726);
- importing Ren'Py scripts (ADR-0016, *Alternatives*);
- anything of the detective module (2727–2732).

## Implementation steps

1. Create `crates/vn` with its `Cargo.toml` metadata, the `view`
   feature, the `vn` feature in `crates/shuyi`, the README's first two
   sections, and the catalog and `docs/modules.ron` changes. Run
   `cargo xtask check-modules`.
2. Write `CharacterDef` and `VnConfig` with their formats, fixtures
   (good and bad files), the fuzz-style tests and the determinism
   fixtures.
3. Write `Direction` and its parser over a command's words, with one
   unit test per command and per error.
4. Write `Story::build`: read every scene's commands into directions,
   with errors carrying the file and line from the `DialogueScene`.
5. Write `VnState`, `VnCommand`, `VnEvent` and `VnRules`, driving the
   engine's dialogue runner (don't write a second one), and apply each
   direction to the stage.
6. Write `history::back`.
7. If 2202 is done, add the Yarn and Ink forms, their fixtures, and the
   rows in `docs/formats/dialogue.md`.
8. Add the skill (or 1134's list entry) and, if 2004's check is there,
   the planned guide page.

## Acceptance criteria

- [ ] A fixture story of three short scenes with a choice and two
      endings plays headlessly through `VnRules` to each ending, and the
      test reads `VnState`, never a picture.
- [ ] Every command `check` refuses leaves the state's hash unchanged,
      and `legal_commands()` equals the set `check` accepts (property
      test over random command lists).
- [ ] Two runs with the same seed and commands give the same hash after
      every command.
- [ ] `back(…, 1)` after any command gives the state before that
      command (property test), and `back_past_choices: false` stops at
      the last choice.
- [ ] Each direction in ADR-0016 §2's table has a passing fixture, and
      each malformed or unknown one gives an error naming the file and
      line.
- [ ] A command listed in `vn.ron`'s `commands` comes out as
      `VnEvent::Custom` with its arguments.
- [ ] `character-ron` and `vn-ron` pass their fuzz-style tests, and
      their decode hashes match the checked-in values.
- [ ] If 2202 is done, the fixture's one scene in Yarn and one in Ink
      give the same `VnState` hashes as the same scene in `.dlg`.
- [ ] `cargo xtask check-modules` passes, with the `visual-novel`
      answer and the row building in `docs/modules.ron` and
      `docs/modules.md`.
- [ ] Without the `view` feature, `cargo tree -p shuyi-vn` lists no
      `shuyi-ui`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each direction and each parse error; the pose or expression
  rule for one word; `check` per refused command.
- Property: refused commands change nothing; `legal_commands` agrees
  with `check`; going back one step undoes one command; determinism of
  hashes.
- Fuzz-style: random bytes into `character-ron` and `vn-ron` never
  panic.
- Fixture and determinism: decode hashes for both formats; the state
  dump of the fixture story at each stop (snapshot).
- Integration: the fixture story played to each ending.

## Completion notes
