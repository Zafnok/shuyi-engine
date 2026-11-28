---
id: "2140"
title: Story variables live in the save, and dialogue can set and test them
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2140 — Story variables live in the save, and dialogue can set and test them

## Context

A story remembers what happened: the player met the smith, paid the
toll, has 3 keys. Today the engine keeps none of it. A blind-spot audit
of the engine (2026-11-23) found this, and the code agrees:

- **`.dlg` tests only who is there.** `@if <character>` is the only
  condition (`Step::If { character, then, otherwise }` in
  `crates/assets/src/dialogue.rs`, parsed by `open_if` in
  `crates/assets/src/dialogue/parse.rs`, which refuses anything but one
  character id). `Scene::resolved` (`dialogue/present.rs`) settles every
  `@if` block when the scene starts, as game ADR-0055 says.
- **The player keeps no variables.** `DialoguePlayer`
  (`crates/ui/src/dialogue/player.rs`) keeps the portraits, the caption,
  the music and the game's `Stage` (`DialogueHooks::Stage`, what the
  game's own directives set). Nothing that a script sets and a later
  line reads.
- **A save holds only what the game puts in it.** `shuyi_core::save`
  names a save's version; "what a save holds, and where it is kept, are
  the game's". There is no story type for a game to put there.

Three places already assume variables that don't exist:

- [ADR-0016](../../docs/adr/0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md)
  §2: `VnState` holds "the dialogue variables `.dlg` already has", and
  §5 checks "a variable read before any line sets it".
- 2721 (parked, `shuyi-vn`) says `.dlg` has "variables" (line 31) and
  that `VnState` keeps "the dialogue variables, as the runner already
  keeps them" (line 102). It doesn't.
- 2202 (Yarn Spinner and Ink) must map each format's variables and
  conditions onto our scenes, and there is nothing to map them onto.
  2728 and 2731 (parked, the mystery module) read "a dialogue variable"
  too.

A game that isn't tactics needs the same: a farming game's shopkeeper
who greets you differently after the festival (`@if $festival_done`),
an adventure's guard who lets you pass once `$paid_toll` is set.

This ticket gives the engine one story state, typed and deterministic,
that a game keeps in its save, and the two `.dlg` lines that use it.
The event sequences and quests built over it are 2141's.

## Nick input

None.

## Scope

**In:**

- **`shuyi_core::story`**, pure and `no_std` (ADR-0002: no floats, no
  clock):
  - `VarName`: `$` then `[a-z][a-z0-9_]*`, at most 32 bytes (the
    sigil is Yarn Spinner's, so a bare id stays a character id);
  - `Value`: `Bool(bool)`, `Int(i64)`, `Text(String)`. No float: a
    fractional number is refused where it is read;
  - `StoryDecls`: each variable's name and default (the default gives
    its type), built in code (`StoryDecls::new().bool(..)`, `.int(..)`,
    `.text(..)`) or read from `@var` lines (below);
  - `StoryState`: a `BTreeMap<VarName, Value>`, `Serialize` and
    `Deserialize`, so a game puts `story: StoryState` in its own save
    type and `state_hash` sees stable bytes. `StoryDecls::start()` is
    every default. `StoryState::complete(&decls)` adds the defaults a
    save made before a variable was declared lacks, so a new variable
    needs no new `SaveFormat::VERSION`;
  - `Change { name, value }`, `StoryState::changes_since(&before)` and
    `StoryState::apply(&[Change])`, so a turn game sends what a scene
    set as one command and the state changes only through commands
    (`AGENTS.md` rule 5);
  - `Test` and `Expr`: a small expression language, parsed from text
    (`Test::parse`, `Expr::parse`), type-checked against `StoryDecls`,
    and evaluated with `Test::holds(&StoryState, &dyn Fn(&CharacterId)
    -> bool)`. Literals `true`, `false`, integers, `"text"` (ASCII);
    `$name`; a bare id is "this character is there"; `not`, `and`,
    `or`, parentheses; `== != < <= > >=`; `+` and `-` on integers,
    saturating at the `i64` bounds. At most 16 levels deep. A variable
    missing from the state (never declared, or a hand-edited save)
    makes its comparison false and its arithmetic 0: evaluating never
    panics.
  - New public types follow ADR-0039: `StoryDecls`, `StoryState` and
    `Change` are `#[non_exhaustive]` with constructors; `Value` and
    `Expr` are `#[non_exhaustive]` enums.
- **`.dlg` lines** (`shuyi_assets::dialogue`):
  - `@var $name <literal>` at a file's top, outside any `@scene`:
    declares a variable and its default (Yarn's `<<declare>>`, Ink's
    `VAR`). Files' declarations merge; one name declared twice with
    another default or type is an error naming both files and lines;
  - `@set $name = <expr>` anywhere a step goes (a scene, a reaction, an
    `@if` part): `Step::Set { name, value: Expr }`;
  - `@if <test>` where the test is more than one bare id:
    `Step::IfTest { test, then, otherwise }`, evaluated **when playback
    reaches it**, after the `@set` lines before it. `@if scout` stays
    `Step::If`, settled when the scene starts as today (game ADR-0055
    is unchanged for the game). A test that mixes the two
    (`@if scout and $trust > 2`) is an `IfTest`, and
    `Scene::conditions` lists its characters too;
  - `print_scene` writes `@var`, `@set` and `@if <test>` back, and
    parsing them gives the same steps.
- **The checks** (`check_scene`, `ScriptsCheck`): a variable read or set
  but never declared, a type mismatch (`$gold == true`, `@set $met = 3`
  for a bool), a fractional or out-of-range number, a test past 16
  levels. Each error has the file and line. With declarations in place,
  ADR-0016 §5's "read before any line sets it" is "never declared".
- **The player** (`shuyi_ui::dialogue`): `DialoguePlayer::with_story(
  scene, hooks, present, story)`, `story()`, and
  `story_changes()` (`changes_since` the state it started with). `new`
  starts from an empty state. `from_text_box` plays the `@set` lines
  before the box it starts at.
- **Text boxes over a story:** `ParsedScene::text_boxes_with` and
  `text_box_at_with`, taking a `&StoryState` as well as `Present`; the
  old two play over `StoryState::default()`.
- **The preview** (`crates/ui/src/dialogue/preview.rs`): its *Switches*
  tab gets one switch per bool variable the scene's tests read, at its
  declared default first. Other variables play at their defaults.
- **An ADR** (the `write-adr` skill picks the number): story variables
  are a typed `StoryState` in `shuyi-core` that the game keeps in its
  save; `.dlg` declares them with `@var`, sets them with `@set` and
  tests them with `@if <test>` as playback reaches it; a presence-only
  `@if` is still settled at the start. It amends ADR-0016 §2 (`VnState`'s
  variables are a `StoryState`) and §5 (the check is "never declared").
- **What changes for the game.** The game pins `50472bbc` of this
  repository (`shuyi-engine.rev` on its main). It matches `Step`
  exhaustively, `Step::If { character, .. }` included, in
  `crates/content/src/dialogue/presence.rs` (line 305), `deaths.rs`
  (118), `openings.rs` (56) and `stand_in.rs` (83, 166), read through
  the contents API on 2026-11-23 (`gh search code` was rate-limited).
  So the new variants break it: `Step` becomes `#[non_exhaustive]` in
  the same break, so later steps don't. It gets a `Breaking:` line
  with a `migration` block. `DialogueFile` and `Scripts` gain a
  `story: StoryDecls` field, additively: **from 2096 (done)** both are
  `#[non_exhaustive]`, built with `DialogueFile::new(scenes)` and
  `Scripts::default()`.
  The game's directives are `place`, `opening`, `dies` and `joins`
  (`crates/content/src/dialogue.rs`, `impl Directive for Stage`), so
  the engine taking `@var` and `@set` shadows none of them. The new
  syntax is ASCII (game ADR-0045 keeps `.dlg` text ASCII). No `.dlg`
  file of the game changes meaning: `@if $x` and `@set` are errors
  today. The game's PR that moves the pin follows this one.
- **Either order with 2237** (problem codes on `.dlg` errors): if 2237
  is done, each new parse and check error gets its code from the
  content codes as 2237 set them out, and its help line; if not,
  nothing to do (2237 gives every `.dlg` problem a code, these too).
- **Either order with 2383** (the ready-made talk screen): if 2383 is
  done, its talk screen takes the game's `StoryState`, plays the scene
  over it with `with_story`, and hands back `story_changes()` when the
  scene ends, with one test; if not, nothing to do (2383 builds its
  screen on `with_story`).
- **2273 (writing a scene back into its file).** Each `Step` this
  ticket adds (`@set`, `@if <test>`) is printed by `print_step` in
  `crates/assets/src/dialogue.rs`, whose match over `Step` is
  exhaustive, so `print_scene` and 2273's `dialogue::write_scene` write
  it back. Whether or not 2273 is done, nothing more to do.

**Out (do not do):**

- Yarn Spinner's and Ink's variables and conditions: 2202 maps them
  onto `shuyi_core::story` (this ticket becomes its blocker). Whether a
  condition on one reply option, or a variable's value shown in a line
  (`{$gold}`), is needed is 2202's research to say; it writes the
  ticket if so.
- Event sequences that set and test story variables, and quests and
  objectives as data over them: 2141 (blocked by this ticket).
- `VnState` and the visual novel's checks: 2721 (parked) keeps a
  `StoryState` in `VnState` when it lands.
- `@elif`, functions, random numbers, and floats in the language.
- Any script language: `shuyi-script` (Rhai) stays for rules. This
  language is a few operators a writer reads at a glance.

**Estimated size:** about 1,400 lines (code 700, tests 550, docs 150).

**Split:** past 1,000 lines, split off steps 7 and 8 (text boxes over a
story and the preview's switches) into a new ticket.

## Implementation steps

1. `crates/core/src/story.rs` (and `story/expr.rs`): `VarName`,
   `Value`, `StoryDecls`, `StoryState`, `Change`, `Test`, `Expr`, their
   parser and type check, and `holds` / `eval`. Re-export the main types
   at the crate's root and name the module in `lib.rs`'s docs.
2. `crates/assets/src/dialogue/parse.rs`: `@var` (top level only),
   `@set`, and `@if <test>` (one bare id keeps `Step::If`). Add
   `Step::Set` and `Step::IfTest` to `Step`, and mark it
   `#[non_exhaustive]`. Teach `has_text`, `print_scene`, `resolved`
   (an `IfTest` stays, its parts resolved inside), `conditions` and the
   `Lines` bookkeeping about the new steps.
3. `DialogueFile` and `Scripts` gain `story: StoryDecls`, merged by
   `ScriptsCheck` across files, with the duplicate error.
4. `check.rs`: the undeclared, type, number and depth checks, each with
   file and line.
5. `crates/ui/src/dialogue/player.rs`: a `story: StoryState` and the
   state it started from; `step_on` plays `Step::Set` and picks an
   `IfTest`'s part when it reaches it; `with_story`, `story`,
   `story_changes`.
6. The ADR, its row in `docs/adr/README.md`, and the amended note on
   ADR-0016's row.
7. `present.rs`: `text_boxes_with` and `text_box_at_with` walk the steps
   with the sets applied.
8. The preview's bool switches.
9. Docs: the module docs' format sample in `dialogue.rs` gains `@var`,
   `@set` and `@if $x`; `skills/shuyi-dialogue/SKILL.md` gets a
   *Story variables* section with a worked example (the farming game's
   shopkeeper) that `cargo xtask check-skills` runs; the book's
   dialogue section.

## Acceptance criteria

- [ ] `@var $paid false`, then `@set $paid = true` in one reply's
      reaction, then `@if $paid` after the choice: that reply plays the
      `then` part, the other reply the `otherwise` part.
- [ ] `@if scout` alone still settles at the start: a `@set` before it
      changes nothing, and its tests pass unchanged.
- [ ] `@set $gold = $gold + 1` at `i64::MAX` stays at `i64::MAX`.
- [ ] A test 16 levels deep is accepted; 17 is an error with its line.
- [ ] `$gold == true`, `@set $met = 3` (bool), `@var $x 1.5`, and a
      `$name` never declared are each one error with file and line.
- [ ] `$x` declared `0` in one file and `false` in another is one error
      naming both files and lines.
- [ ] A save written as RON with a `StoryState`, read back, gives the
      same `state_hash`; `complete` adds a newly declared default and
      changes no other value.
- [ ] `story_changes()` applied with `StoryState::apply` to the state
      the scene started from gives `story()`.
- [ ] `from_text_box` past a `@set` gives the same `story()` as reading
      up to that box.
- [ ] Evaluating a test over a state that lacks its variable returns
      false and doesn't panic.
- [ ] Each break has a `Breaking:` line and a `migration` block.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the expression parser and type check (each operator, each
  error), saturation, depth 16 and 17, a missing variable; `@var`,
  `@set` and `@if <test>` parse and print; each check error.
- Property: random tests and expressions print and parse back the same;
  `print_scene` then `parse_dlg` gives the same steps for scenes with
  the new lines; `apply(changes_since(before))` on `before` gives the
  state after any run of sets.
- Integration: a scene played in `DialoguePlayer` over both values of a
  bool variable, and the `text_boxes_with` order matching the player's
  boxes for each.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
