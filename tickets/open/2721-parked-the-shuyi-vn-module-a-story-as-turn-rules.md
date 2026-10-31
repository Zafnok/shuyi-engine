---
id: "2721"
title: "Parked: The shuyi-vn module: a story as turn rules, stage directions as dialogue commands, and characters as data"
type: feature
milestone: "Engine: visual novels"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["game#1131", "2003", "2116"]
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
  ADR-0011, game ADR-0055) with speakers, choices, `@if`, variables,
  line ids (game ADR-0045) and the game's own `@` lines (its
  `Directive` type `D`). **From 2201 (done):** ADR-0007's
  `DialogueScene` kind is `shuyi_assets::dialogue::DialogueFile<D>`, a
  file's scenes, read by the format `Dlg<D>` (`docs/formats.md`, "The
  kinds of ADR-0007 §1"). The runner is `DialoguePlayer` in
  `shuyi_ui::dialogue` (`crates/ui/src/dialogue/player.rs`). It is
  pure, but it lives in `shuyi-ui`, which this crate uses only behind
  `view`; how `VnRules` drives it headless (moving it down a layer,
  with an ADR, say) is this ticket's to decide.
- **Formats are plugins** ([ADR-0007](../../docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md)
  §2, §5): every new file kind is a `Format`, never panics, and is
  deterministic. Its determinism fixture is a row in its crate's golden
  table (2116's `shuyi_assets::golden`), checked by CI's `test` job on
  x64 and ARM64 and by its `determinism` job on wasm32.
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
  - **the `kind` concern.** 2003 added it to `docs/modules.ron`
    (`alternatives: false`). Set the `visual-novel` answer's status
    from parked to building, naming this module;
  - the `vn` feature of the `shuyi` crate;
  - the catalog row in `docs/modules.md` (2003 wrote it as parked): set
    its status to building, with tickets 2721–2726. 2726 sets the row
    and the answer to ready, when the example passes CI;
  - **the `building` status.** 2003 (done) gave the catalog three
    statuses: ready, planned and parked. If `shuyi_cli::catalog::Status`
    (`crates/cli/src/catalog.rs`) has no `Building` yet, add it, named
    `building`, with its doc, to the enum,
    `crates/cli/schemas/modules.output.json`, the `status` line of
    `docs/modules.ron`'s header and the *Status* line of *How to read
    it* in `docs/modules.md`. It is a new variant of an exhaustive
    public enum, so write a `Breaking:` line if CI's `api` job reports
    one. 2118 adds it the same way if it lands first;
  - the README's first two sections in ADR-0009 §4's order (what it is
    for, what it is not for). 2726 finishes the rest;
  - the `shuyi-vn` skill. **From 1134 (done):** a short skill,
    `skills/shuyi-vn/SKILL.md`, in the pack's form, that says what this
    ticket built and that the rest of the module is to come. It has no
    worked example yet, so its `metadata` has `waits_for: "2726"`:
    2726 gives it its worked example and drops its id. It is a module's
    skill: `shuyi add vn` copies it into a game, and `shuyi new`
    doesn't;
  - `needs` (ADR-0012 §1), if 3017 is done: an empty table, with a
    comment that 2726 checks it against the example. If 3017 isn't done,
    3017 gives it a `needs` key with the other modules.
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
- **Going back** in `shuyi_vn::history`, on the engine's
  `shuyi_core::history::History<VnRules>` (`VnRules` is `TurnRules` and
  `Clone`), not a second replay: `back(history: &mut
  History<VnRules>, steps)` picks the point and calls
  `History::rewind_to`, which replays from the start and forgets the
  later commands. What is the VN's own is only the point: one step is
  one stop (one command), so one step back is the line before, and
  `vn.ron`'s `back_past_choices` (default true) says whether going back
  may undo a choice; when false, `back` stops after the last `Choose`.
  Replaying from snapshots is 2724.
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
    doesn't use the module sees its commands passed through as today;
  - **`.dlg`'s portraits are the same stage, not a second one.** `.dlg`
    already places characters: `@left <id> <expression>` and `@right …`
    (`Step::Place` in `shuyi_assets::dialogue`), `@left clear`
    (`Step::Clear`), and `id[expression]: text` (`Step::Say`'s
    `expression`). `Story::build` reads them into the same `Direction`s:
    `Place` is `show <id> <expression> at left` (or `right`), `Clear`
    hides whoever stands at that place, and a `Say` with an expression
    changes it as `show` does. So `@left` and `show` give the same
    `VnState` (a fixture checks it). `show`/`hide` are the general form:
    they add poses, other places, transitions and draw order.
- **Where directions are written:**
  - `.dlg`: `@show …`;
  - **if 2202 is done**: `<<show …>>` in Yarn Spinner and `# show …` as
    an Ink tag. Check that 2202's runner hands both to the scene as
    commands, follow 2202's rule for telling a command tag from another
    Ink tag (such as a metadata tag), and add the stage-direction rows
    to `docs/formats/dialogue.md`, with one fixture per format;
  - **if 2202 isn't done**: `.dlg` only. 2202 then adds the Yarn and
    Ink forms, their rows and their fixtures when it lands (its Scope
    In, item 5). Say which in the Completion notes.
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
  - **one list of characters.** `.dlg`'s own checks already ask the
    game who may speak and which expressions exist
    (`shuyi_assets::dialogue::SceneRules`'s `can_speak` and
    `expression_problem`, run by `check_scene`). The story's
    `CharacterDef`s answer them (a `SceneRules` built from the
    characters), so a VN game writes its characters once, in
    `*.character.ron`. Say this choice in the module's README;
  - a fuzz-style test (random bytes never panic) and a determinism
    fixture: the decoded value's hash is a row of
    `crates/vn/tests/golden/formats.ron`, a golden table (2116's
    `shuyi_assets::golden`, a dev-dependency with its `golden` feature,
    checked by a test with `golden` in its name).
- **The story's settings**: `vn.ron`, asset kind `VnConfig`, format
  `vn-ron`:
  - `places`: name to a point in screen pixels (integers). The defaults
    `left`, `center` and `right` exist unless the file replaces them;
  - `transitions`: name to a kind and a duration in ms. The kinds are
    `cut`, `fade`, `dissolve`, `slide` (with a side) and `custom`.
    Defaults exist for the first four;
  - `commands`: the game's own command names;
  - `back_past_choices`;
  - the same fuzz-style test, and a row in the same golden table.
- Both formats are registered by the module's `register(&mut Formats)`.
- **CI:** `-p shuyi-vn` joins the wasm32 `cargo test` of CI's
  `determinism` job (2116), so the golden table and the module's tests
  run on wasm32 too. A test that needs threads or unwinding gets 2116's
  WASI `ignore` attribute; any other failure there is a determinism bug.

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
   (good and bad files), the fuzz-style tests and the golden table
   (start it as `{}` and bless it with `cargo xtask golden --bless`).
   Add `-p shuyi-vn` to the `determinism` job's wasm32 run.
3. Write `Direction` and its parser over a command's words, with one
   unit test per command and per error.
4. Write `Story::build`: read every scene's commands into directions,
   with errors carrying the file and line from each scene's
   `ParsedScene` (in the `DialogueFile`).
5. Write `VnState`, `VnCommand`, `VnEvent` and `VnRules`, driving the
   engine's dialogue runner (don't write a second one), and apply each
   direction to the stage.
6. Write `history::back` over `History<VnRules>`.
7. If 2202 is done, add the Yarn and Ink forms, their fixtures, and the
   rows in `docs/formats/dialogue.md`.
8. Add the skill.

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
- [ ] A scene placed with `@left`/`@right`, `@left clear` and
      `id[expression]:` gives the same `VnState` hash at every stop as
      the same scene written with `show` and `hide`, and an unknown
      character or expression in either form is reported once, from the
      `CharacterDef`s.
- [ ] A command listed in `vn.ron`'s `commands` comes out as
      `VnEvent::Custom` with its arguments.
- [ ] `character-ron` and `vn-ron` pass their fuzz-style tests, and
      their decode hashes match their rows in a golden table (2116's
      `shuyi_assets::golden`), with `-p shuyi-vn` in the wasm32 run of
      CI's `determinism` job.
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
- Fixture and determinism: decode hashes for both formats as golden
  rows, natively in the `test` job and on wasm32 in the `determinism`
  job; the state dump of the fixture story at each stop (snapshot).
- Integration: the fixture story played to each ending.

## Completion notes
