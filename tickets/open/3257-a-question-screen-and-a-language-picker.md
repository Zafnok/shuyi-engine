---
id: "3257"
title: "Two ready-made screens: a full-screen question that pops with the answer, and a first-launch language list that marks a machine-made pack"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3257 — A question screen and a language list

## Context

A request from the game.

1. **The game ticket:** game#1418, which moves the game's five
   question screens and its language list onto these once they land
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1418-question-and-language-screens-from-the-engine.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** ask the player one question on a screen
   of its own (a title, a few answers, a help line) and give the screen
   below the answer they picked; and, on the very first launch, ask
   which language to play in, each language by its own name, a
   machine-made one marked as such.
3. **The friction numbers:** sweep game#1397 (2026-11-27). The game would
   keep about 15 lines per question (its text keys and values) and its
   rule for when the language list is asked. At its pin 230080d it
   measures about 220 code lines: a `pick_one` frame helper
   (`screens/mod.rs`) used by five screens (`voices_question.rs` 46,
   `mode_select.rs` 82, `familiarity.rs` 79, `game_over.rs`, the save
   prompt in `save.rs`), a shared `SavePromptView` and its painter
   (`save/view.rs`, `save/glyph.rs`) that four of them borrow, and
   `language_list.rs` (93), whose `in_use` copies this engine's
   `OptionRow::language()` lookup and whose `names` exist only to put a
   machine-made pack's note after its name. About 3×.
4. **The generic shape:**
   - `screens::question::QuestionScreen<T>`: a title key, answers as
     `(T, text key)`, an optional help key, `.without_cancel()`, a focus;
     `update` gives `Transition::pop_with(value)` on a pick and `Pop` on
     Cancel; its view (`QuestionView`: title, `MenuView`, help) is plain
     data and `question::glyph::paint` paints it, replaceable with
     `.painted_by` like `ProfilesScreen`. It is the full-screen sibling
     of `widgets::Ask`, which draws over another screen.
   - `screens::language::LanguageScreen`: every language
     `Ctx::languages_offered` gives, the one in use focused, picked and
     saved to `settings.language`, then popped. A pack whose
     `LangInfo::made_by` is `MadeBy::Machine` shows its own
     `language.machine_note` text after its name; `OptionRow::language()`
     shows the same note, so a game needs no row of its own.
   - `language::asks(ctx)`: no language saved yet and a pack to pick
     besides the source, for the game's start to decide.
5. **An example from a game that isn't tactics:** a visual novel asks
   "Play with voices?" and "Skip text you have read?" before its first
   scene, and a farming game translated by machine into three languages
   asks for one at first launch, marked so the player knows.

## Nick input

None.

## Scope

**In:**
- `crates/ui/src/screens/question.rs` (with `view.rs`, `glyph.rs`) and
  `crates/ui/src/screens/language.rs`, beside `options`, `profiles` and
  `key_bindings`.
- `OptionRow::language()` shows the machine note.
- The text key `language.machine_note` as a constant; English in the
  engine's English file.
- New public types `#[non_exhaustive]` with constructors (ADR-0039).
  Additive: no break to the public API.

**Out (do not do):**
- Which questions a game asks and when (the game's flow).
- A layout or key question after the language: `layout_picker` already
  is one.
- One-bit painters of these two screens: a game passes its own with
  `.painted_by` until the engine's screens get a one-bit look.
- Labelling machine-made text anywhere but beside a language's name.

## Implementation steps

1. `QuestionScreen<T>` from the game's `pick_one` and `SavePromptView`
   (behavior unchanged: the menu's own sounds through
   `handle_with_sound`).
2. `LanguageScreen` and `asks` from the game's `language_list.rs`, its
   lookup of the language in use shared with `OptionRow::language()`.
3. The machine note in `OptionRow::language()`.
4. Tests on a toy `Ctx` with a source and two packs, one made by
   machine; snapshots of both screens on 100 × 32.
5. Docs: the ready-made screens in `skills/shuyi-screen/SKILL.md`, the
   book's *The engine's own screens and widgets*.
6. The PR says the game's PR that moves the pin (game#1418) follows it.

## Acceptance criteria

- [ ] `QuestionScreen::new("q.key", [(true, "yes"), (false, "no")])`:
      Confirm on `No` pops with `false`; Cancel pops with nothing; with
      `.without_cancel()` Cancel does nothing.
- [ ] `LanguageScreen` lists the source and both packs by their own
      names, the machine-made one with its note, focused on the one in
      use; picking one saves it and pops.
- [ ] `OptionRow::language()` shows the same note.
- [ ] `asks` is false once a language is saved, and with no pack.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: picks, cancel, focus, the note, `asks`.
- Snapshot: both screens, 100 × 32.

## Completion notes

*(Filled in by the session that completes the ticket.)*
