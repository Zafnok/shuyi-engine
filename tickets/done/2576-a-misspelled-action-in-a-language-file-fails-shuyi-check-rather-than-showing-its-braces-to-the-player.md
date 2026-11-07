---
id: "2576"
title: A misspelled action in a language file fails `shuyi check`, rather than showing its braces to the player
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-06
---

# 2576 — A misspelled action in a language file fails `shuyi check`

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

- [x] `{Confirn}` in a language file fails `cargo shuyi check` with its file,
      line, code and the known actions.
- [x] `{seeds}` and `{Cursor}` still pass.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the validator on a good file, a misspelled action and an argument.
- Integration: `check` on a game with the mistake, as `crates/cli`'s tests do.

## Completion notes

- `shuyi_ui::text::check_key_placeholders::<A>(bundle, path, text)`
  checks a language file's placeholders: a `{Name}` whose first letter
  is a capital names a key, so it must be `{Cursor}` or an action of the
  game's `ActionSet` (what `key_text` fills); a lowercase `{name}` is a
  value a screen passes through `text_with`'s `args`, which no file can
  know, and passes. Why the first letter: action names are capitalized
  (`ActionSet::name`, the keymap's names), and the arguments every
  screen, template, example, tutorial step and the book pass are
  lowercase, so the letter tells the two apart without the file having
  to list its arguments. An error has the file, the key's line, code
  `SH2008` (`unknown-name`, as the keymap's unknown action) and the
  known actions.
- `BasicData::load` (the templates, the tutorial's steps, the book's
  game) and Hello, Grid's loader call it on `en.ron`, so `cargo shuyi
  check` fails on `{Confirn}`. A game with a loader of its own (one
  reading `lang/en/ui.ron`) calls it there; the rustdoc says how.
- Tests: the validator on a good file (actions, `{Cursor}`, `{seeds}`,
  `{}`), a misspelled action; `check` on `BasicTools` with the mistake
  (`crates/cli/src/basic/tests.rs`).
- Docs: the book's chapters 2 and 3, *When it goes wrong*, quote the
  real message, from `site/snippets`' tool program with `{Confirn}` put
  in its `en.ron`; chapter 3 says the capital-letter rule where it
  teaches placeholders. The `SH2008` page and the `shuyi-input` skill
  name the case.
- The check found the same mistake already shipped: Hello, Grid's
  `en.ron` and the tutorial's steps 08 to 10 had the toy's
  `"key_bindings.help_switch"`, naming `{CursorLeft}/{CursorRight}`,
  actions those games don't have (their players read the braces): now
  `{Left}/{Right}`. Hello, Grid also had the toy's
  `"dialogue.log_hint"` (`{DialogueLog}`), which no code of it reads and
  no action of it fills: removed. Hello, Grid's golden table blessed: its
  `en.ron` row and its files' fingerprint moved, and with it its stage
  session's content stamp.
- Deviation: the check is in `shuyi-ui` (beside `key_text` and
  `CURSOR`), not `crates/assets/src/lang*`: the assets crate doesn't know
  `{Cursor}`.

The public API adds `shuyi_ui::text::check_key_placeholders`; the book
changes with it.

Breaking: shuyi-ui: `BasicData::load`: a game's `en.ron` with a capitalized placeholder that is neither an action of its `ActionSet` nor `{Cursor}` now fails to load, and `check` names it (`SH2008`); fix the name, or write a value the screen passes in lowercase. Hello, Grid's golden "asset en.ron" and "fingerprint files" rows moved with its text.

```migration
since: 0.1.0
what: A capitalized placeholder in en.ron must be an action or {Cursor}; a value a screen passes is lowercase.
old: '"key_bindings.help_switch": "{CursorLeft}/{CursorRight} keyboard or controller"'
new: '"key_bindings.help_switch": "{Left}/{Right} keyboard or controller"'
before: |
  "key_bindings.help_switch": "{CursorLeft}/{CursorRight} keyboard or controller · {Cursor} move · {Cancel} back",
after: |
  "key_bindings.help_switch": "{Left}/{Right} keyboard or controller · {Cursor} move · {Cancel} back",
find: ["{CursorLeft}/{CursorRight}"]
```

Nick input: none.
