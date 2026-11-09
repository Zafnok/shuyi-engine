---
id: "2826"
title: "The reader's filter: a popup with three questions and a skip on a first visit, a bar on every page to change them, and skipping ahead past what they know"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2825"]
nick_input: sign-off
completed:
---

# 2826 — The reader's filter

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md), *one
book, read at the reader's level*): "The Book should then have an option
to skip ahead, to filter content, etc depending on familiarity", so that
experienced developers "can just filter to that experience level and get
rid of the novice crap". Asked what a first-time visitor sees, he wrote:
"short welcome page with skip to bring to full book & a bar to tune at top
to revisit if they picked wrong onboarding option or skipped".

2825 marked every passage of the book with its audience (new to
programming, to making games, to Rust, to Shuyi), and with no JavaScript every
passage shows. The site has no JS framework and loads nothing from
another host (`site/README.md`, *The rules*), so the filter is a plain
script of our own.

**From 2823 (done):** [ADR-0090](../../docs/adr/0090-one-book-the-project-folded-into-the-chapters-a-newcomer-mark-around-a-passage-and-a-readers-filter-kept-in-the-browser.md) §6 decides the filter. Its *The front
page's and the popup's text* is the popup's words, which Nick signed off
on 2026-11-08 after reading a working mock in the browser
(`docs/decisions.md`, *the book's project folds into its chapters, and a
fourth kind of reader*). He asked for a popup rather than a welcome
page, and that nothing else repeat where to start.

- **The popup:** a `<dialog>` opened with `showModal()` on a reader's
  first visit to any book page, titled "Welcome to the book". It asks
  one question at a time:
  - "Have you programmed before?" (**No, I'm new to it** · **Yes**). A
    "No" passes over the Rust question;
  - "Have you written Rust?" (**No, or not much** · **Yes**);
  - "Have you made a game before?" (**No** · **Yes, in another engine**
    · **Yes, with Shuyi**), always asked.

  **Skip: show me the whole book** is in the popup, and Escape does the
  same. "Yes, in another engine" shows a line sending the reader to
  appendix A, then **Done**. The last answer closes the popup and folds
  the page at once, and the reader stays on the page they opened.
- **The answers** are kept in `localStorage` under `shuyi-book`, as
  `{"v": 1, "knows": [...], "engine": true}`, the key's presence meaning
  answered or skipped. A skip writes `"knows": []`, and 2829 adds
  `"version"`.
- **The front page** is the book's intro and contents only. Once the
  popup is answered, the contents carry tags: **Start here** on the first
  page the answers don't hide, **You can skip this** on a page whose
  page-level mark the reader knows, and **Read this first** on appendix A
  for a reader from another engine.
- **The first pages:** *Before you start* for a reader new to
  programming, and the quick start for everyone else.
- **The bar** has four toggles, one for each audience of ADR-0090 §4,
  independent of each other.
- **A page is skipped only for its page-level mark** (`extra.newcomer`).
  A page with passage marks folds them to stubs and is never skipped.
- **The script** reads the book's order from a JSON block that
  `book-macros.html` writes into each page: each page's URL, title and
  page-level mark. The bar and the popup go in `book-macros.html` too,
  so `chapter.html`, `project.html` and `book.html` share them.
  **From 2824 (done):** `book-macros.html`'s `order()` writes the book's
  order as one content path a line, and `pager(current)` and `where(p)`
  read it; write the JSON block from `order()`. The book's front page
  already says what the bar does (Nick's text in ADR-0090), so the bar
  makes that sentence true.
- **A link to an anchor** inside a hidden passage opens the passage.

**From 2825 (done):** what the filter folds.

- **A passage:** `<aside class="newcomer" data-newcomer="rust">`, its
  first child a `<p class="newcomer-label">` holding the label ("New to
  Rust?") and the topic in a `<span>` (the shortcode
  `site/templates/shortcodes/newcomer.md`). The stub's text is that
  label and topic.
- **A page marked whole:** `extra.newcomer`, which the top line already
  reads ("For readers new to programming", `book-macros.html`'s
  `where`). Only *Before you start* has one.
- **The labels' colour** is `--newcomer` in `style.css`, light and dark.
- *Before you start*'s note on the marks, and the book's front page, say
  that the bar folds the notes a reader doesn't need, and leaves a line to
  click: this ticket makes that true.

## Nick input

**Sign-off:** open the book as a first-time visitor, answer the popup's
three questions as a programmer who doesn't know Rust, read a page, then
change your answers with the bar. Say whether it reads as the mock you
signed off for 2823 did.

## Scope

**In:**
- **The popup**, on the first visit to any book page: the three
  questions as the ADR words them, each a set of buttons, and **Skip:
  show me the whole book** inside it. It is shown once: a reader who
  answered or skipped never sees it again unless they choose **Start
  over**. Focus goes into it when it opens, stays in it while it's open,
  and returns to the page when it closes.
- **The contents' tags** on the front page, as the ADR says.
- **The bar**, at the top of every book page: the four audiences, each
  a toggle changed in place, and **Start over**, which opens the popup
  again. It says how many passages of this page it hides ("3 notes
  hidden"), and **Show all** shows them for this visit.
- **A hidden passage** leaves the ADR's stub: one line naming what it was
  ("New to Rust? `match` · show", from the mark's `topic`), a button that
  shows it when clicked, so a reader who answered wrong loses nothing.
- **Skipping ahead:** the contents mark a page whose page-level mark the
  answers hide ("You can skip this"), and such a page's top says so with a link
  to the next page the reader needs. Previous and next go by the reader's
  path when the answers skip pages, with a line saying a page was skipped.
- **The script**, `site/static/book-filter.js`, plain JS with no build
  step, and its styles in `style.css`, light and dark. The answers are in
  `localStorage`, each read and write in `try`/`catch`; when storage
  throws, the popup opens on every page and the book still works.
  With no JavaScript there is no popup and no bar, and every passage
  shows.
- **Keyboard and narrow screens:** the buttons are buttons
  (`aria-pressed` for an answer), the stubs are buttons, focus is visible,
  and every page works at 375 px with no sideways scroll.
- **The twins** don't change: an agent reads the whole book, and the
  popup is in no twin.
- **`site/README.md`** and the `document` skill.

**Out (do not do):**
- what a passage is for, or new marks (2825);
- the version a reader knew (2829);
- accounts, a server, or anything kept past the reader's own browser;
- quick checks and exercises (2827, 2828).

## Implementation steps

1. Read ADR-0090, 2825's completion notes, the templates and
   `style.css`.
2. Write the popup, the contents' tags, the bar and the stubs in the
   templates, then the script and the styles. The mock Nick signed off,
   described in ADR-0090's *What was tried*, is a starting point for the
   popup's look.
3. Build the site and serve it. In the built-in browser, read as each kind
   of reader in ADR-0090 §6's table: new to programming, a programmer
   new to Rust and to games, a Rust programmer from another engine, and
   someone who knows Shuyi. For each, the contents' tags, a page with
   hidden passages, a stub opened, and next to the following page. Then
   skip, and then *Start over*. Open a step's page first, too, to see
   the popup there.
4. Try it with storage blocked and with JavaScript off, at 375 px, and
   with the keyboard alone.
5. Ask Nick for his sign-off, then run the `document` skill and the gates.

## Acceptance criteria

- [ ] A first visit to any book page opens the popup, and a later visit
      doesn't; *Start over* opens it again.
- [ ] For each of the four readers of step 3, the contents tag the page
      the ADR says, and the reader sees no passage of an audience they
      know but its stub.
- [ ] Nothing but the popup and the contents' tags says where to start.
- [ ] Skip shows the whole book with nothing hidden.
- [ ] The bar changes the answers on the page at once, and says how many
      passages it hides.
- [ ] A page the answers hide entirely is marked in the contents and links
      the next page needed.
- [ ] With storage blocked, and with JavaScript off, the book reads in
      full (said in the notes).
- [ ] The keyboard reaches every control, and every page works at 375 px.
- [ ] The page loads nothing from another host (`check-build.sh` passes).
- [ ] Nick's sign-off is in the completion notes.
- [ ] The book's chapter is updated (2802): this ticket is that change.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Site: the build checks fail a book page that doesn't load
  `book-filter.js`, or a mark the script has no audience for.
- By hand in the browser: steps 3 and 4, a row for each, in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
