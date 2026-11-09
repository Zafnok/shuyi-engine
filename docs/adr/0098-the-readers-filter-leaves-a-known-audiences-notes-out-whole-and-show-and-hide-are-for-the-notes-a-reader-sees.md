# ADR-0098: The reader's filter leaves a known audience's notes out whole, and show and hide are for the notes a reader sees

- **Status:** Accepted; amends ADR-0090 §6
- **Date:** 2026-11-09
- **Related tickets:** 2826 (this), 2823 (ADR-0090), 2827 (quick checks)

## Context

ADR-0090 §6, *What a hidden passage leaves*, decided that a passage of an
audience the reader knows folds to a one-line stub, a button such as "New
to Rust? `match` · show", so that "nothing disappears silently". 2826
built it that way.

Nick read the built filter in the desktop app's browser on 2026-11-09,
while signing off 2826 (`docs/decisions.md`, *the reader's filter leaves
out what a reader knows*):

- An opened note had **show** and no way back: he asked for **hide** too.
- Then: a stub still takes a lot of space. A reader who says they know
  Rust is to be trusted, and their Rust notes are not drawn at all. Show
  and hide are for the notes a reader is reading, within what they don't
  know.

On step 3 (*Walking*), for a Rust programmer, 11 of the page's 13 notes
are Rust's: 11 stub lines, each with a margin above and below.

## Decision

1. **A passage of an audience the reader knows is left out whole.**
   Nothing stands in its place. `style.css` hides it by `<html>`'s
   `data-book-fold`, as before; the stub is no longer shown for it.
2. **Every passage shown has hide.** `book-filter.js` adds " · hide", a
   button, to the passage's label. Hiding folds the passage to its stub
   (" · show"), the one ADR-0090 §6 describes. The stub shows only for a
   passage the reader hid (`is-shut`).
3. **The bar still counts what it leaves out** ("12 notes hidden") and
   **Show all** shows those passages for the visit, each with its hide.
   The count leaves out the passages the reader hid, which have their own
   stubs.
4. **A passage's own state lasts until the answers change.** A change in
   the bar, and **Start over**, start every passage again from the
   answers.
5. **A link to an anchor inside a passage left out** still opens that
   passage (ADR-0090 §6), with its hide.

The rest of ADR-0090 §6 stands: the popup, the answers in `localStorage`,
the contents' tags, skipping a page by its page-level mark, and no bar or
popup without JavaScript.

## Consequences

- A reader who knows an audience reads the page as if its notes weren't
  written. A page for a Rust programmer is as short as the book can make
  it.
- A reader who answered wrong sees nothing on the page saying a note was
  left out. The bar says how many, and **Show all** or a toggle brings
  them back.
- The front page and *Before you start* said a folded note leaves a line
  to click. They now say the bar leaves out the notes a reader doesn't
  need, and a note can be hidden and shown again.
- 2827's quick check, folded with its audience, is left out the same way.

## Alternatives considered

- **ADR-0090 §6 as it was: a stub for every passage left out.** Nothing
  disappears silently, but Nick found the stubs take too much space, and
  a reader who said they know an audience doesn't need them.
- **One line per page naming what was left out** ("11 notes on Rust left
  out · show"). It keeps the page short and the reader informed, but the
  bar already says it, at the top of the same page.
- **Stubs only, with no hide on a shown passage.** That was the first
  build. Nick asked for hide.
