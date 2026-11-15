---
id: "2257"
title: The text checks say cells where they count cells
type: bug
milestone: After MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2257 — The text checks say cells where they count cells

## Context

2252 (#267) made the text checks measure with the font atlas's wide
glyphs: a wide character takes two cells. Its notes said the messages
still say "characters" where they now count cells, and no ticket was
written. The retro of 2026-11-08 found the wording still on `main` in
`crates/assets/src/dialogue/check.rs`:

- line 488: `text is {len} characters; the limit is {limit}`;
- line 496: `text takes {lines} lines of {} characters; the box shows
  {box_lines}`;
- line 614: `option text is {len} characters; the limit is {max_option},
  so it fits the menu`.

A translator told "12 characters" for a line of six wide characters is
misled.

## Nick input

None.

## Scope

**In:**
- Messages that count cells say "cells", or "cells (a wide character is
  two)" the first time in a message. Do the same anywhere else in
  `crates/assets` and `crates/cli` that reports a width counted in cells
  (`grep -rn "characters;" crates`).
- Update the snapshots and the error docs (`crates/cli/errors/*.md`)
  that quote the old text.

**Out (do not do):**
- Changing how width is counted.

## Implementation steps

1. Change the three messages above and any others the grep finds.
2. Update the tests and snapshots that pin them.
3. Say in the completion notes that the message text changed, since
   the game's dialogue tests pin exact errors, and name the game's PR
   that moves the pin.

## Acceptance criteria

- [x] `grep -rn "characters; the limit" crates` finds nothing.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a wide-glyph line over its limit reports cells.

## Completion notes

- The dialogue checks (`crates/assets/src/dialogue/check.rs`) now say
  `text is N cells; the limit is L`, `text takes N lines of W cells; the
  box shows B` and `option text is N cells; the limit is L, so it fits
  the menu`.
- The grep found two more that count cells (`text_width`) and said
  characters: the tip checks (`crates/assets/src/tip.rs`), now `the
  title must be 1 to N cells` and `a text line is over N cells: …`.
- The wording is plain "cells", as the language pack's check
  (`lang/lines.rs`) already says it, not "cells (a wide character is
  two)": one word for the same measure across the checks.
- Left alone: the other "characters" messages count `char`s or bytes
  (a language tag, a repository's name, a skill's description, a
  profile's name, a recipient's code), and the `@if`/`@choice` messages'
  "characters on screen" means the scene's characters. The public fields
  `TipLimits::line_chars` and `title_chars` keep their names (a rename is
  a break, and out of scope); their docs already say cells.
- The test the ticket asks for already existed: the wide-glyph case in
  `dialogue/tests.rs` (41 of `あ` against a limit of 80) now pins
  `text is 82 cells; the limit is 80`, as the tip test pins
  `a text line is over 10 cells: "あいうえおか"`.
- No error doc under `crates/cli/errors/` and no snapshot quoted the old
  text; the unit tests that pinned it are updated.

Behaviour: shuyi-assets: the dialogue checks' errors say "cells" where they said "characters" (`text is N cells; the limit is L`, `text takes N lines of W cells; the box shows B`, `option text is N cells; the limit is L, so it fits the menu`), and the tip checks' (`the title must be 1 to N cells`, `a text line is over N cells: …`).

Game: the search `gh search code --repo Zafnok/visions-of-shuyi "characters; the limit"` finds the game's dialogue tests pinning the engine's text (`crates/content/src/dialogue/tests.rs`, `tests/choice.rs`, `tests/names.rs`); the game's PR that next moves the pin past this change updates them. No game PR exists yet. The game's `lesson.rs` and `lang/data.rs` hits for the tip wording are the game's own messages, not the engine's, and don't change.

Book: none, the chapter (`assets-and-formats.md`) already says a wide character takes two cells and quotes none of these messages; no public API changed.
