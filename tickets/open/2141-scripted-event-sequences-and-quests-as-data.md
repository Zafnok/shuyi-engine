---
id: "2141"
title: A game runs scripted event sequences and quests as data over story variables
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2140"]
nick_input: none
completed:
---

# 2141 — Scripted event sequences and quests as data over story variables

## Context

Found by the blind-spot audit of 2026-11-22 ("No story state", high,
partial). Every engine a story game is made in has a way to write "the
guard walks to the door, says his line, waits a second, and the gate is
open from now on" as data: RPG Maker's event commands, GameMaker's
Sequences with Moments, Godot's `AnimationPlayer` method tracks and the
Dialogic and Quest plugins, Unity's Timeline signals, Ren'Py's script.
Most of them add quests on top: objectives whose state comes from the
story so far.

What Shuyi has today (checked 2026-11-23):

- `crates/assets/src/dialogue.rs`: `Step` has `Caption`, `Place`,
  `Clear`, `Say`, `Narrate`, `Choice`, `Music`, `Directive(D)` and
  `If`. `If` tests only whether a character is present (`@if
  <character>`). `crates/ui/src/dialogue/player.rs` (`DialoguePlayer`)
  plays one scene; what a game's directives set is its
  `DialogueHooks::Stage`, which lives only while the scene plays.
- `crates/assets/src/cinematic.rs`: a cinematic is timed shots over a
  length in milliseconds, drawn from the time alone, and `S` and `P`
  are the game's. Nothing waits on an actor arriving or a reply being
  read: a cinematic is a timeline, not a sequence of steps.
- `crates/core/src/timer.rs`: `Countdown` and `Every`, counted in steps
  or turns, kept in the rules' state so they save and replay.
- No module of `shuyi_core` (`crates/core/src/lib.rs`: `bots`,
  `character`, `ease`, `grid`, `hash`, `history`, `made`, `path`,
  `rng`, `save`, `text`, `threat`, `tick`, `timer`, `turn`) has story
  variables, a step runner or quests. A search of `tickets/open` and
  `tickets/done` for quests, cutscenes, scripted sequences and story
  flags finds no ticket that plans them; the parked genre tickets name
  "a story flag" only as something a game sets itself (2751).

This ticket builds on 2140, which adds the story variables in
`shuyi_core::story`: `StoryState` (typed values a game keeps in its
save), `Test` (a condition over them, parsed and type-checked against
`StoryDecls`, evaluated with `Test::holds`) and `Change` (one variable
set, applied with `StoryState::apply`), and the `.dlg` lines `@set` and
`@if <expr>` that write and read them. 2140's Out line leaves this
ticket the event sequences, quests and objectives over those variables.
This ticket builds the two things the audit put after it:

1. **An event sequence**: a list of steps a game's rules run one at a
   time, where a step may wait (for steps or turns, for the game to say
   an actor has arrived, for a dialogue scene to end) and may set or
   test a story variable.
2. **Quests as data**: objectives whose state is a condition over the
   story variables, and a log that says when each starts, finishes or
   fails, once.

**Who uses it.** Any game with a story outside its battles or levels: a
farming game's festival morning (the mayor walks to the square, speaks,
the stalls open), a platformer's sign post that opens a door the first
time it is read, an adventure game's fetch quest, a visual novel's
route flags, a tactics game's town between battles. The game, Visions
of Shuyi, has side quests in its own code (game#1072, game#1074;
`gh search code --repo Zafnok/visions-of-shuyi quest`, engine pin
`50472bbc`); this ticket only adds, and the game need not move to it.

**Rules it keeps** (ADR-0002): the runner is pure and deterministic, has
no clock and no floats, and lives in the state the rules own, so it
saves, hashes (`state_hash`) and replays with it. It moves only when
the rules advance it, from a command (turn games) or a step (tick
games). It never moves an actor or shows a line itself: it hands the
game a cue, as `DialogueHooks::direct` hands a directive.

**Names.** `Sequence` is taken twice in the plans: 2135's
`ease::Sequence` (tweens one after another) and the parked MIDI
module's `shuyi_midi::sequence::Sequence` (2205). So this ticket's
types are `EventSequence`, `EventStep` and `SequenceRunner`, in
`shuyi_core::sequence`, and none is called `Sequence` alone. 2135 and
2205 are named only for the clash: neither is needed first, and this
ticket leaves them nothing.

`Condition` below stands for 2140's `Test`, and a `Set` step writes a
2140 `Change` into the `StoryState`. If 2140 settled on other names
when it was done, use those.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::sequence`** (`crates/core/src/sequence.rs`, tests in
  `crates/core/src/sequence/tests.rs`), `no_std` like the rest of the
  crate:
  - `EventSequence<A>`: an id and its `EventStep<A>`s. `A` is the
    game's own step (move an actor, play a sound, shake the screen).
  - `EventStep<A>`: `Do(A)` (the game does it and the sequence goes
    on), `Await(A)` (the game does it and the sequence waits until the
    game says it is done), `Dialogue(scene id)` (waits until the game
    says the scene ended), `Wait(n)` (whole steps or turns, never 0),
    `Set { var, value }` (writes a story variable through 2140),
    `If { when: Condition, then, otherwise }`, and `Run(id)` (another
    sequence, then back).
  - `SequenceRunner`: where a running sequence is (a stack of sequence
    id and step path, at most `MAX_DEPTH` deep), plus what it waits on.
    `Serialize`, `Deserialize`, `Clone`, `Eq`, `Hash`, `Debug`, so a
    save made mid-sequence resumes at the same step.
  - `SequenceRunner::run(&mut self, sequences, story, out)`: runs steps
    until one waits or the sequence ends, pushing a `Cue<A>` for each
    `Do`, `Await` and `Dialogue` into `out`; `advance(n)` counts down a
    `Wait`; `finish()` ends an `Await` or `Dialogue`; `is_done()`.
    `finish()` while nothing waits is an error value, not a panic.
  - A game runs several sequences at once with several runners. There
    is no parallel step.
- **The format** (ADR-0007): asset kind `sequence`,
  `SequenceRon<A>` in `crates/assets/src/sequence.rs`, extension
  `ron`, one sequence per file, its id the file stem, unknown fields
  refused. `check` refuses with a `ContentError` (file and line, as
  `cinematic::check` does): a `Run` of an unknown id, a cycle of
  `Run`s, nesting past `MAX_DEPTH`, `Wait(0)`, a `Dialogue` of a scene
  the game doesn't have (the game passes its scene ids, as cinematics
  get the audio manifest), and a variable or a value 2140's checks
  refuse. A golden row `sequence: sequence`.
- **`shuyi_core::quest`** (`crates/core/src/quest.rs`, tests beside):
  - `QuestDef`: an id, a title and a summary as text keys (ADR-0020),
    `starts_when: Condition`, an optional `fails_when: Condition`, its
    `ObjectiveDef`s, and an optional sequence id run when it is done.
  - `ObjectiveDef`: an id, a text key, `done_when: Condition`,
    `optional: bool`, an optional `shown_when: Condition`, and an
    optional `progress: (var, target)` so a view can say "3 of 5".
  - `QuestLog`: each quest's state (`Hidden`, `Active`, `Done`,
    `Failed`) and its objectives' done or not. `update(&mut self, defs,
    story) -> Vec<QuestEvent>` works the states out from the variables,
    quests in the order of their ids, and reports `Started`,
    `ObjectiveDone`, `Done` and `Failed` once each. A done or failed
    quest stays so. `Serialize`, `Deserialize`, `Eq`, `Hash`, so it
    saves beside 2140's variables.
  - A quest is done when every objective that isn't optional is done.
    "Defeat five wolves" is the game counting a variable and the
    objective's `done_when` reading it: quests read only story
    variables.
- **The quest format**: asset kind `quest`, `QuestRon` in
  `crates/assets/src/quest.rs`, a list of quests per file, checked for
  duplicate ids, empty objective lists, text keys missing from the
  English text the game passes to `check`, unknown sequence ids and
  what 2140 refuses in a condition. A golden row `quest: quest`.
- New public types follow ADR-0039: `EventSequence`, `QuestDef`,
  `ObjectiveDef` and `QuestLog` are `#[non_exhaustive]` with
  constructors; `EventStep`, `Cue` and `QuestEvent` are
  `#[non_exhaustive]` enums.
- An ADR (the `write-adr` skill): how a sequence advances inside the
  rules (a cue out, `finish` back in through a command or a step), why
  quests are conditions and not counters of their own, and the names.
- Docs: rows for both kinds in `docs/formats.md`'s table of generic
  kinds, the crate docs, a section in the book's
  `site/content/learn/book/rules-as-commands.md`, and a section in
  `skills/shuyi-dialogue/SKILL.md` (a sequence that runs a scene and
  sets a variable after it).
- **What changes for the game:** nothing. The API grows (CI's `api`
  job lists the additions). The PR says so.

**Out (do not do):**
- story variables, `@set` and `@if <expr>` in `.dlg`: 2140;
- the Yarn and Ink import (2202);
- cinematic files and `cinematic::Timeline` stay as they are. Keyframed
  tracks and cues on a timeline are 2364 (parked). A game whose
  cinematic `actions` should be a sequence can make its `P` a sequence
  id itself;
- a quest log screen or widget, and any view of a sequence on screen:
  the game draws them from `QuestLog` and its own cues;
- starting a sequence when a player steps on a tile or touches an
  area: the game's rules do that, from its own map;
- sequences or quests written in a script (`shuyi-script`; 2146,
  parked) and visual scripting (1135);
- hot reload of a running sequence: a reloaded file applies to runners
  started after it;
- a parallel step, a jump or a label, and loops other than `Run`ning
  oneself (refused as a cycle).

**Estimated size:** about 1,700 lines (code 750, tests 700, docs 250).

**Split:** past 900 lines, split off steps 6–8 (quests: `shuyi_core::quest`,
the quest format and their docs) into a new ticket blocked by this one.

## Implementation steps

1. Read 2140's completion notes: the names of the variables, the value
   type and the condition, how a condition is checked at load and how
   the variables sit in a save. Use them as they are.
2. `crates/core/src/sequence.rs`: `EventSequence<A>`, `EventStep<A>`,
   `Cue<A>`, `SequenceRunner` and `MAX_DEPTH` (16), with a doc example
   of a tick game that runs a sequence of a `Do`, a `Wait(30)` and a
   `Set`. Add `pub mod sequence;` to `crates/core/src/lib.rs`.
3. The runner: `start(id)`, `run`, `advance`, `finish`, `is_done`. A
   step path, not a pointer, so the runner serializes. `run` on a
   finished runner pushes nothing. `run` never loops more steps than
   the sequence holds before it waits or ends (no step can repeat
   without a `Wait`, `Await` or `Dialogue` between, as cycles are
   refused at load).
4. `crates/assets/src/sequence.rs`: `SequenceRon<A>` (`Format`,
   `AssetKind` with `NAME = "sequence"`), `parse`, and `check(file,
   scenes, story_decl)`, with `pub mod sequence;` in
   `crates/assets/src/lib.rs`. Add it beside `CinematicRon` in
   `crates/assets/src/format/builtin_tests.rs` (its id, extensions and
   limit, and the fuzzed decode) and its golden row in
   `crates/assets/src/format/golden_tests.rs` and
   `crates/assets/tests/golden/formats.ron`.
5. The ADR, then the docs for sequences (formats table, crate docs, the
   book section, the skill section).
6. `crates/core/src/quest.rs`: `QuestDef`, `ObjectiveDef`, `QuestState`,
   `QuestLog`, `QuestEvent`; `update` as the Scope says.
7. `crates/assets/src/quest.rs`: `QuestRon`, `parse`, `check`, its
   tests beside `SequenceRon`'s and its golden row.
8. The docs for quests: its formats row, the book and skill sections
   extended.
9. Run `cargo xtask check-layers`: `shuyi-core` gains no dependency.

## Acceptance criteria

- [ ] A sequence of `Do(a)`, `Wait(3)`, `Set { var, value }` and
      `Do(b)` gives cue `a` on the first `run`, nothing after
      `advance(2)`, and cue `b` with the variable set after one more
      `advance(1)`.
- [ ] `Await(a)` and `Dialogue(id)` each stop the runner until
      `finish()`; `finish()` with nothing waiting returns an error and
      changes nothing.
- [ ] `If` takes `then` when 2140's condition holds and `otherwise`
      when it doesn't, read at the moment the runner reaches it.
- [ ] `Run(id)` plays the other sequence and comes back to the next
      step. Nesting of exactly `MAX_DEPTH` loads; one deeper is refused
      at load.
- [ ] A runner saved to RON mid-`Wait` and mid-`Await`, read back and
      run on, gives the same cues and the same `state_hash` as one that
      was never saved.
- [ ] `check` refuses each of: an unknown `Run` id, `a` running `b`
      running `a`, `Wait(0)`, an unknown scene, an unknown variable,
      and an unknown field, each with the file and line.
- [ ] A quest with two objectives, one optional, is `Hidden` until
      `starts_when` holds, `Active` after, and `Done` when the one that
      isn't optional is done; `update` reports `Started`,
      `ObjectiveDone` and `Done` once each, however often it is called.
- [ ] A quest whose `fails_when` holds is `Failed` and stays so when
      the condition stops holding; a `Done` quest never becomes
      `Failed`.
- [ ] The quest `check` refuses a duplicate quest id, a quest with no
      objectives, an unknown text key and an unknown sequence id.
- [ ] Golden rows `sequence: sequence` and `quest: quest` pass on x64
      and wasm32 (CI's `determinism` job).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: every `EventStep` through the runner; `finish` and `advance`
  when nothing waits; `MAX_DEPTH` and one more; each `check` refusal;
  every quest state change and that each event fires once.
- Property: for random sequences of `Do`, `Wait` and `Set` over a
  fixed variable set, saving and loading the runner at any point gives
  the same cues and final variables as running straight through; for
  random variable values, `QuestLog::update` called twice in a row
  reports nothing the second time.
- Fuzz-style: `SequenceRon` and `QuestRon` join the fuzzed decodes in
  `crates/assets/src/format/builtin_tests.rs` and never panic.
- Snapshot / integration: a toy tick game (`crates/core/src/toy.rs`'s
  pattern) that runs a sequence from a step, finishes an `Await` from a
  later step, and replays to the same `state_hash`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
