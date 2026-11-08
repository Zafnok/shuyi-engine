---
id: "2257"
title: The text checks say cells where they count cells
type: bug
milestone: After MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] `grep -rn "characters; the limit" crates` finds nothing.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a wide-glyph line over its limit reports cells.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
