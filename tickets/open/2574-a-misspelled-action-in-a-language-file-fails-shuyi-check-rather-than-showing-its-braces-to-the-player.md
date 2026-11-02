---
id: "2574"
title: A misspelled action in a language file fails `shuyi check`, rather than showing its braces to the player
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2574 — A misspelled action in a language file fails `shuyi check`

## Context

Found while 2819 provoked each mistake a beginner makes, to quote its real
message in the book. Two mistakes in `en.ron` pass `cargo shuyi check`:

- `"title.help": "{Confirn} plant"`: `check` says OK, and the player reads
  `{Confirn} plant` on the screen. No tool says anything.
- deleting `"title.name"`: `check` says OK, and a debug build panics on the
  first frame that asks for it, naming the key
  (`crates/ui/src/ctx.rs`). Which keys a game's code asks for isn't known to
  `check`, so this half is not in this ticket.

The book's chapters 2 and 3 say so under *When it goes wrong*. The first is
a hole the validators can close (2530 gave them problem codes).

## Nick input

None.

## Scope

**In:**
- `shuyi check` reports a `{Name}` placeholder in a language file that can't
  be filled, with its file and line and a problem code. Decide how to tell a
  misspelled action from an argument a screen passes (`{seeds}`, filled by
  `text_with`'s `args`), and say why in the notes: for example, an argument
  is lowercase by rule, so a capitalized placeholder must be an action of the
  game's `ActionSet`, or `{Cursor}`.

**Out (do not do):**
- a text key that code asks for and the file lacks, as above;
- changing what `text_with` does at run time.

## Implementation steps

1. Read how `text_with` fills placeholders and how `check` validates the
   language file (`crates/assets/src/lang*`, `crates/ui/src/ctx.rs`).
2. Add the check beside the language validators, naming the known actions as
   the keymap's `SH2008` does.
3. Make sure the rule holds for the templates, the examples, the tutorial's
   steps and the book's game.
4. Update the book: chapters 2 and 3, *When it goes wrong*, with the real
   message.

## Acceptance criteria

- [ ] `{Confirn}` in a language file fails `cargo shuyi check` with its file,
      line, code and the known actions.
- [ ] `{seeds}` and `{Cursor}` still pass.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the validator on a good file, a misspelled action and an argument.
- Integration: `check` on a game with the mistake, as `crates/cli`'s tests do.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
