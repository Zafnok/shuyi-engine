---
id: "2826"
title: "The reader's filter: a welcome page with three questions and a skip, a bar on every page to change them, and skipping ahead past what they know"
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
programming, new to Rust, new to Shuyi), and with no JavaScript every
passage shows. 2823's ADR decides the questions, the answers each takes,
where they are kept, what a hidden passage leaves behind, and what
"skip ahead" means. The site has no JS framework and loads nothing from
another host (`site/README.md`, *The rules*), so the filter is a plain
script of our own.

## Nick input

**Sign-off:** open the book as a first-time visitor, answer the three
questions as a programmer who doesn't know Rust, read a page, then change
your answers with the bar. Say whether it reads well, and whether the
welcome page's words fit.

## Scope

**In:**
- **The welcome page**, at the book's first address: the three questions
  as the ADR words them (for example "Have you programmed before?",
  "Have you written Rust?", "Have you made a game with Shuyi?"), each a set
  of buttons, and **Skip: show me the whole book**. Answering opens the
  first page the answers don't hide; skipping opens the book's first page
  with nothing hidden. The page is shown once: a reader who answered or
  skipped goes straight to the contents next time, and the bar brings it
  back.
- **The bar**, at the top of every book page: the three answers, each
  changed in place, and **Start over**, which shows the welcome page
  again. It says how many passages of this page it hides ("3 passages
  hidden for you"), and **Show all** shows them for this visit.
- **A hidden passage** leaves the ADR's stub: one line naming what it was
  ("For readers new to Rust: `match`"), which shows it when clicked, so a
  reader who answered wrong loses nothing.
- **Skipping ahead:** the contents mark a page whose passages the answers
  all hide ("You can skip this"), and such a page's top says so with a link
  to the next page the reader needs. Previous and next go by the reader's
  path when the answers skip pages, with a line saying a page was skipped.
- **The script**, `site/static/book-filter.js`, plain JS with no build
  step, and its styles in `style.css`, light and dark. The answers are in
  `localStorage`, each read and write in `try`/`catch`; when storage
  throws, the welcome page shows each time and the book still works.
  With no JavaScript the welcome page shows its text and a link into the
  whole book, and there is no bar.
- **Keyboard and narrow screens:** the buttons are buttons
  (`aria-pressed` for an answer), the stubs are buttons, focus is visible,
  and every page works at 375 px with no sideways scroll.
- **The twins** don't change: an agent reads the whole book. The welcome
  page's twin says what it asks.
- **`site/README.md`** and the `document` skill.

**Out (do not do):**
- what a passage is for, or new marks (2825);
- the version a reader knew (2829);
- accounts, a server, or anything kept past the reader's own browser;
- quick checks and exercises (2827, 2828).

## Implementation steps

1. Read 2823's ADR, 2825's completion notes, the templates and
   `style.css`.
2. Write the welcome page, the bar and the stubs in the templates, then
   the script and the styles.
3. Build the site and serve it. In the built-in browser, read as each kind
   of reader: new to programming, a programmer new to Rust, a Rust
   programmer new to Shuyi, and a Shuyi developer. For each, the first
   page it opens, a page with hidden passages, a stub opened, and next to
   the following page. Then skip, and then *Start over*.
4. Try it with storage blocked and with JavaScript off, at 375 px, and
   with the keyboard alone.
5. Ask Nick for his sign-off, then run the `document` skill and the gates.

## Acceptance criteria

- [ ] A first visit to the book shows the welcome page, and a later visit
      doesn't; *Start over* shows it again.
- [ ] Each of the four readers of step 3 opens on the page the ADR says,
      and sees no passage of an audience they know but its stub.
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
