---
id: "2834"
title: The book's pages have a folding contents beside the text
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: in-progress
blocked_by: []
nick_input: sign-off
completed:
---

# 2834 — The book's pages have a folding contents beside the text

## Context

Nick's feedback on the book (2026-11-12): a page of the book has no way to
see the rest of the book, or the rest of the page, without going back to
the front page. He wants a collapsible navigation, table of contents or
index on the left or the right side, and the two sides compared.

Today a book page (`site/templates/chapter.html`) has the line saying where
it is (`book::where`), the text, and the pager to the page before and after
(`book::pager`, `site/templates/book-macros.html`). The whole contents is
only on the book's front page (`site/templates/book.html`), built from
`extra.parts` in the book's order (ADR-0090 §2), with the reader's filter
(`site/static/book-filter.js`) tagging its links once the reader has
answered its questions.

## Nick input

**Sign-off:** the PR shows the two layouts below, side by side, as
screenshots of the same page, wide and on a phone. Comment which you
want, or that both are right as built.

## Scope

**In:**
- **The book's contents, on the left:** every page of the book in its
  order (chapters numbered, each project's steps under the chapter they
  follow, appendices lettered), the page being read marked
  (`aria-current="page"`), as a `<nav aria-label="The book's contents">`.
  It folds: a button hides and shows it, and the choice is kept in
  `localStorage` for the next page. Folded by default on a narrow screen,
  where it opens over the text.
- **This page's headings, on the right**, on wide screens only: the page's
  `##` and `###` headings from Zola's `page.toc`, as links, with the one
  being read marked as the reader scrolls (an `IntersectionObserver`).
  Folds with its own button, kept in `localStorage` too. On a narrow screen
  it sits folded at the top of the text, in a `<details>`.
- The comparison Nick asked for: build it this way (left = the book, right =
  this page, the common docs-site layout), then put the alternatives in
  the PR as screenshots: both lists on the left (the page's headings nested
  under the page being read). Keep whichever Nick signs off; if he picks
  the alternative, the PR changes before merge.
- The reader's filter tags the sidebar's links as it tags the front page's
  contents (Start here, You can skip this), from the same data.
- With no JavaScript, both lists are there, unfolded on wide screens.

**Out (do not do):**
- No search box; that would be its own ticket.
- No change to the front page's contents, the pager, or the book's order.
- Pages outside the book (news, examples, the playground page) don't get it.

## Implementation steps

1. `site/templates/book-macros.html`: a `contents(current)` macro writing
   the book's contents from `extra.parts`, in the same order and with the
   same numbering as `book.html`. Have `book.html` call it too if that
   keeps one source of the order without changing the front page's look;
   otherwise share `self::order()` as the pager does.
2. A `page_toc(p)` macro from `p.toc` (two levels).
3. `site/templates/chapter.html`: a three-column layout around the
   `<article>` (`book-side` left, `book-toc` right), the fold buttons with
   `aria-expanded` and `aria-controls`.
4. `site/static/style.css`: the grid (the text keeps its 44rem measure);
   below a breakpoint, the left list becomes an overlay drawer and the
   right a `<details>` above the text. Respect `prefers-reduced-motion`.
5. `site/static/book-filter.js` (or a small `book-side.js`): the folds and
   their `localStorage` keys, the scroll marking, and the filter's tags
   applied to `.book-side a` as to `.contents a`.

## Acceptance criteria

- [ ] On every page of the book (chapters, steps, appendices), the left list shows the whole book in the front page's order, with the current page marked.
- [ ] Folding either list and moving to another page keeps it folded; unfolding keeps it open.
- [ ] On a 375-pixel-wide screen, the text is full width; the book's list opens over it from a button and closes with the button or Escape, returning focus to the button.
- [ ] The right list's marked heading follows scrolling on a long chapter.
- [ ] Every link is reachable and the folds work with the keyboard alone.
- [ ] With JavaScript off, both lists show and work as plain links.
- [ ] The PR has the screenshots Nick signs off on (both layouts, wide and phone).
- [ ] `zola check` passes (no broken link from the new lists).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Manual, in the built-in browser on `zola serve`: the criteria above, wide
  and at the `mobile` preset, light and dark.
- The build fails if a page in the book's order is missing from the
  sidebar: the macro walks the same order the pager does, so this holds by
  construction; say so in the macro's comment.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
