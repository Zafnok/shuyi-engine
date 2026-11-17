---
id: "2834"
title: The book's pages have a folding contents beside the text
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: []
nick_input: sign-off
completed: 2026-11-13
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

- [x] On every page of the book (chapters, steps, appendices), the left list shows the whole book in the front page's order, with the current page marked.
- [x] Folding either list and moving to another page keeps it folded; unfolding keeps it open.
- [x] On a 375-pixel-wide screen, the text is full width; the book's list opens over it from a button and closes with the button or Escape, returning focus to the button.
- [x] The right list's marked heading follows scrolling on a long chapter.
- [x] Every link is reachable and the folds work with the keyboard alone.
- [x] With JavaScript off, both lists show and work as plain links.
- [x] The PR has the screenshots Nick signs off on (both layouts, wide and phone).
- [x] `zola check` passes (no broken link from the new lists).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Manual, in the built-in browser on `zola serve`: the criteria above, wide
  and at the `mobile` preset, light and dark.
- The build fails if a page in the book's order is missing from the
  sidebar: the macro walks the same order the pager does, so this holds by
  construction; say so in the macro's comment.

## Completion notes

What was done:

- `book-macros.html`: `contents(current)` writes the book's contents
  from `order()`, the order the pager walks, so a page of the order
  can't be missing from it (said in its comment): chapters numbered,
  each project's landing page and steps in a list under the chapter they
  follow, appendices lettered under the part's title, the page being read
  `aria-current="page"`. `page_toc(toc)` writes a page's `##` and `###`
  headings. `book_open` and `book_close` lay the three columns out around
  the text, with the fold buttons (`aria-expanded`, `aria-controls`), a
  "Skip to the text" link for the keyboard, and the headings in a
  `<details>` for narrower screens. `script()` loads the new
  `static/book-side.js` beside `book-filter.js`.
- `chapter.html` and `project.html` (a project's landing page is a page
  of the book too) call them; `base.html`'s `<main>` takes a class from
  a `main_class` block, so a book page's `main` is wider (82rem) while the
  text keeps its 44rem measure.
- `style.css`: one column below 960 px, the contents on the left from
  960 px, the headings on the right too from 1240 px, both sticky. A
  folded column leaves the others centred. Below 960 px the contents is
  a drawer over the text, sliding in unless `prefers-reduced-motion`, with
  a scrim, and the Contents button's row stays at the top as the text
  scrolls. The fold buttons wear the small-button look of ticket 2831.
- `book-side.js`: the folds, kept in `localStorage` under
  `shuyi-book-side` and `shuyi-book-toc` and set on `<html>` before the
  page is drawn; the drawer (Close, Escape or a click on the scrim
  closes it and gives the focus back to Contents; the rest of the page is
  `inert` while it is open); the heading being read, marked
  `aria-current="location"` through an `IntersectionObserver`, with a
  `scrollend` listener for a jump that carries no heading across the
  line.
- `book-filter.js` tags `.book-side a` as it tags `.contents a`.
- `check-build.sh` fails a page of the book, but the front page, that
  doesn't load `book-side.js` or has no contents beside it marking the
  page itself. `site/README.md` has a section, *Beside each page*.

Deviations:

- The book's front page doesn't get the sidebar: its own contents is
  the whole page, and the ticket keeps it unchanged. The macro walks
  `order()` rather than being called by `book.html`, since the front
  page's contents carries descriptions and extra links the sidebar
  doesn't.
- On a phone the drawer covers the Contents button, so it has its own
  Close button at its top; Escape and a click beside it close it too, and
  each gives the focus back to Contents.
- Between 960 and 1240 px the page's headings are the `<details>` above
  the text, as on a phone: three columns don't fit there.
- The alternative layout (both lists on the left, the page's headings
  under the page being read) was built in the templates for its
  screenshots and taken out again; it is a small change to `contents` and
  `book_open` if Nick picks it.

Manual evidence, on `zola serve` at this branch as the commit that moves this ticket has it, on
`main` at 4a56e985 (no later commit on `main`), in the built-in browser
and headless Chrome: wide (1440 px) and the `mobile` preset, light and
dark. Every link of the contents and the headings was reached with Tab;
Enter and Space fold both columns; the folds held on the next page
(Rules as commands, then step 4); on the phone, Contents opened the
drawer with the focus on the current page, and Escape and Close shut it
with the focus back on Contents; the marked heading followed scrolling on
chapter 5 (History and undo, Rules in fixed steps, Check yourself at the
end, none above the first). With the scripts stripped from the page,
both lists show on a wide screen and the contents follows the text on a
phone. `zola check` and `site/check-build.sh` pass.

Book: the book's front page (`site/content/learn/book/_index.md`) says
what is beside each page and how it folds.

No break: only the website changed, nothing a game uses.

Nick input: sign-off on the layout, in the PR (both layouts, wide and
phone).
