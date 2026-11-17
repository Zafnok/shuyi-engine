---
id: "2833"
title: A quick check's right answer isn't always first
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-13
---

# 2833 — A quick check's right answer isn't always first

## Context

Nick's feedback on the book (2026-11-12): in the **Check yourself**
sections, the first answer is the right one, so a reader learns to pick
the first button without reading.

The quick checks are ticket 2827's (ADR-0090 §8):
`{% check() %}…{% end %}` in a page's Markdown, painted by
`site/templates/shortcodes/check.html`, made into buttons by
`site/static/book-check.js`, checked by `cargo xtask check-book`
(`crates/xtask/src/book.rs`, `read_check`) and written as text into the
Markdown twins by `cargo xtask llms`. Each shows its answers in the order
they are written, and of the 61 checks under `site/content/` on
2026-11-12, the right answer is written first in 43, second in 16 and
third in 1.

## Nick input

None.

## Scope

**In:**
- In the page, `book-check.js` shuffles each check's buttons when the page
  loads (Fisher–Yates, `Math.random`: this is the page, not a simulation),
  so the right one's place changes from reader to reader and load to load.
  Each button keeps its own why; the `role="status"` line is unchanged.
- The written order is spread too, because the reader with no JavaScript
  (the `<details>` list) and the twins read it: reorder the answers in the
  checks under `site/content/` so the right one is not mostly first. Only
  the order changes; no question, answer or why is rewritten.
- `cargo xtask check-book` keeps it spread: it fails when, across the
  book, more than half of the checks with one right answer have it in the
  same place, and the message says how many are where.

**Out (do not do):**
- No build-time shuffle in the shortcode: Tera has no random, and a
  hashed order would make the twins differ from the page anyway.
- No change to a check's shape, its count of answers, or its audiences.
- Ticket 2828 (exercises) is not this ticket's job.

## Implementation steps

1. `site/static/book-check.js`, in `start()`: before wiring the buttons,
   pair each button with its why (`whys[i]`), shuffle the pairs, and append
   the buttons back into `.check-choices` in the new order, before the
   `.check-result` line. Wire the click handlers from the pairs, so a
   button always gets its own why.
2. Reorder the answers of the checks in `site/content/learn/book/**`
   (move whole `- [x]`/`- [ ]` lines with their why). Aim for the right
   answer's place spread about evenly over the places each check has.
3. `crates/xtask/src/book.rs`: in `run`, count the right answer's place
   (0-based) over every check with exactly one right answer, and push a
   problem when one place holds more than half of them, e.g.
   `the right answer is first in 43 of 61 quick checks: spread them (2833)`.
   Document the rule in the module's doc comment beside the check's shape,
   and in `check.html`'s comment.

## Acceptance criteria

- [x] On a built page, reloading a few times shows a check's buttons in different orders, and choosing the right one still says **Right.** with its own why; a wrong one says **Not quite.** with its own why.
- [x] With JavaScript off, the `<details>` list is in the written order.
- [x] After step 2, no place holds more than half of the single-right checks; `cargo xtask check-book` passes.
- [x] A test fails `check-book` when 3 of 4 single-right checks have it first, and passes at 2 of 4.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (xtask): the spread rule's two cases above, on checks written
  inline in the test.
- Manual, in the built-in browser on `zola serve`: the shuffle and the
  verdicts, with a screenshot of two loads in the PR.

## Completion notes

- `site/static/book-check.js` pairs each button with its why, shuffles the
  pairs (Fisher–Yates, `Math.random`) and puts the buttons back before the
  `role="status"` line; the click handlers come from the pairs.
- The answers of the 20 chapters' and steps' checks were reordered (whole
  lines only; each file's lines are the same set as before): of the 60
  checks with one right answer, it is now first in 20, second in 21 and
  third in 19. The one check with two right answers is unchanged.
- `cargo xtask check-book` reads all of Learn's pages, then fails when one
  place holds the right answer of more than half of the single-right
  checks: `the right answer is first in 43 of 60 quick checks, second in
  16, third in 1: spread them (2833)` on `main`'s pages. `book::check_pages`
  is the new entry the tests use; the rule is in the module's doc comment,
  `check.html`'s comment and `site/README.md`, *Quick checks*.
- Tests: `the_right_answers_are_spread_across_the_book` (3 of 4 first
  fails, 2 of 4 passes, a check with two right answers counts for neither).
- Manual, in the built-in browser on `zola serve` at a434b973 (`main`'s
  tip, nothing newer to merge) plus this diff: on *Walking*, two loads
  gave the checks' right answers at places `Rww/wRw/wwR` and
  `wRw/wwR/Rww`; clicking every button gave **Right.** or **Not quite.**
  with the why its list item holds (9 of 9). The served `<details>` list
  is in the written order (it is never moved; only the buttons are).
- Deviation: the ticket counted 61 checks; the rule counts 60, those with
  exactly one right answer.

No break. No `Behaviour:` line: the site and `xtask` aren't API a game uses.

Book: none, the change is to the book's checks themselves and its authors'
guide (`site/README.md`), not to an engine item a chapter teaches.
