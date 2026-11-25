---
id: "2851"
title: "The book says what scales the window, and the glossary gets the words the last chapters bolded"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2851 — The book says what scales the window, and the glossary gets the words the last chapters bolded

## Context

Found by the docs audit of 2026-11-25.

- **Scale.** Chapter 9 (`site/content/learn/book/targets-and-export.md`,
  about lines 33–36) says each target in `shuyi.ron` has "how it scales
  (`integer`, whole multiples so pixels stay crisp, or `fit`, as big as
  the window allows)". Since 2333 the window draws the game's
  `GameSettings::DEFAULT_SCALE`, and a target's `scale` is read only by
  `check --target` (the template's `shuyi.ron`, about lines 57–59, says
  the target's scale "matches the game's `GameSettings::DEFAULT_SCALE`").
  So a reader who writes `scale: fit` in `shuyi.ron` gets a window that
  doesn't fit. `DEFAULT_SCALE` appears nowhere in the book. 2333's
  `Book:` line left it for the player's pick (3240), which doesn't
  change this chapter either.
- **The glossary** (`site/content/learn/book/glossary.md`) has no row for
  words the chapters now bold or lean on: **stream** (a named stream of
  the seeded random numbers, `rules-as-commands.md`, about line 171),
  **sound effect** (`.sfx.ron`, `assets-and-formats.md`, about line 41),
  **template** (bolded in `quick-start.md`, about line 115), and the
  terms **pointer** (mouse and touch, `actions.md`), **graphics tier**
  (`targets-and-export.md`) and **cue** (the audio manifest's,
  `saves-and-settings.md`). No commit of the last 60 touched the
  glossary.

## Nick input

None.

## Scope

**In:**

- Chapter 9's targets passage: a target's `scale` is what the game
  promises on that target and what `check --target` reads; the window
  draws `GameSettings::DEFAULT_SCALE`, which the game sets (a short
  snippet in `site/snippets/` if the chapter shows the setting, built by
  the `test` job like the book's other code); and the two should match.
  **Either order with 3242:** if 3242 is done, say in one sentence that
  `check --target` compares the two; if not, leave that sentence out
  (3242 adds it).
- Chapter 7 (`saves-and-settings.md`) only if it describes
  `GameSettings` without `DEFAULT_SCALE` where a reader would look for it:
  one line linking chapter 9.
- The glossary: a row for each word above that the book uses as a term
  (check each against the glossary's rule in `docs/README.md`: each word
  links the step that first uses it and the chapter that says more), in
  the glossary's order and voice. A word the book uses only in passing
  gets no row; say which in the completion notes.

**Out (do not do):**

- the player's pick of scale in the options (3240) and its words;
- the box pusher's steps (2850);
- chapter 10 (3426 and 3427).

## Implementation steps

1. Read `docs/README.md`'s rows for the book, `site/README.md` and the
   glossary's head.
2. Read `crates/ui`'s `GameSettings::DEFAULT_SCALE` and `ScaleMode`
   rustdoc, and 2333's ticket (`tickets/done/2333-*`), so the chapter
   says what the code does.
3. Write chapter 9's passage, and the snippet if one is needed.
4. Write the glossary's rows.
5. Run `cargo xtask check-book`, `check-links` and the gates.

## Acceptance criteria

- [ ] Chapter 9 no longer says a target's `scale` scales the window, and
      names `DEFAULT_SCALE` and where a game sets it.
- [ ] The glossary has a row for each of the words above the book uses
      as a term, each linking where it is first used and where it is
      explained.
- [ ] Any snippet added builds in the `test` job.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `cargo xtask check-book` and `check-links`; the `book-snippets` crate
  if a snippet is added.

## Completion notes

*(Filled in by the session that completes the ticket.)*
