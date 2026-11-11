---
id: "2826"
title: "The reader's filter: a popup with three questions and a skip on a first visit, a bar on every page to change them, and skipping ahead past what they know"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["2825"]
nick_input: sign-off
completed: 2026-11-09
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

- [x] A first visit to any book page opens the popup, and a later visit
      doesn't; *Start over* opens it again.
- [x] For each of the four readers of step 3, the contents tag the page
      the ADR says, and the reader sees no passage of an audience they
      know but its stub.
- [x] Nothing but the popup and the contents' tags says where to start.
- [x] Skip shows the whole book with nothing hidden.
- [x] The bar changes the answers on the page at once, and says how many
      passages it hides.
- [x] A page the answers hide entirely is marked in the contents and links
      the next page needed.
- [x] With storage blocked, and with JavaScript off, the book reads in
      full (said in the notes).
- [x] The keyboard reaches every control, and every page works at 375 px.
- [x] The page loads nothing from another host (`check-build.sh` passes).
- [x] Nick's sign-off is in the completion notes.
- [x] The book's chapter is updated (2802): this ticket is that change.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Site: the build checks fail a book page that doesn't load
  `book-filter.js`, or a mark the script has no audience for.
- By hand in the browser: steps 3 and 4, a row for each, in the notes.

## Completion notes

**What was done.**

- **The popup:** `book-macros.html`'s `filter()` writes a `<dialog>`
  titled "Welcome to the book", with ADR-0090's words. The questions show
  one at a time below the ones answered, each answer a button with
  `aria-pressed`, so an earlier answer can be changed. "No" to
  programming passes over Rust. "Yes, in another engine" shows the line
  sending the reader to appendix A, then **Done**. **Skip: show me the
  whole book** and Escape write `"knows": []`. The last answer closes it,
  and the reader stays on the page. Focus goes to the first answer, stays
  in the modal, moves to each new question, and goes back to what had it
  (**Start over**) on closing.
- **The answers:** `localStorage`, `shuyi-book`,
  `{"v": 1, "knows": [...], "engine": …}`, every read and write in
  `try`/`catch`.
- **The bar,** on every book page: four toggles, "N notes hidden", **Show
  all** (for this visit) and **Start over**.
- **The contents' tags** on the front page: **Start here**, **You can
  skip this** and **Read this first**, as ADR-0090 §6 says.
- **Skipping ahead:** *Before you start*'s top says "You can skip this
  page. Next for you: Quick start →" to a programmer, the pager passes over
  it with "Skipped: *Before you start*", and the contents tag it.
- **The script,** `site/static/book-filter.js`, plain JS loaded in the
  `<head>`: it sets `data-book` and `data-book-fold` on `<html>` before the
  page is drawn, so nothing flashes, and `style.css` folds by them. It
  reads the book's order from a JSON block that `filter()` writes from
  `order()`: each page's URL, title and page-level mark, and appendix A.
  `base.html` has a new `head` block for it. The words are all in the
  templates, but for the counts and the tags.
- **The build checks** (`check-build.sh`) fail a built book page that
  doesn't load `book-filter.js`, and a passage's mark that the script's
  `AUDIENCES` or `style.css`'s fold rules don't have. Each was seen to
  fail: with `project.html`'s script left out, and with `shuyi` taken out
  of `AUDIENCES`.
- **Docs:** `site/README.md` has a new section, *The reader's filter*,
  and the filter in *What is where*, *The rules* and *The checks*. The docs
  map's row for the book names the filter. The `document` skill holds no
  copy of the map, so it needed no change.

**Nick's sign-off (2026-11-09),** in the desktop app's browser, recorded
in `docs/decisions.md`, *the reader's filter leaves out what a reader
knows*. He read it twice before signing off:

1. "notes should have both show and hide": an opened note had no way to
   fold again. Every note shown now has " · hide" on its label.
2. A folded note's line "still takes up a lot of space": a reader who
   says they know an audience is trusted, and its notes are not drawn at
   all. Show and hide are only for the notes a reader sees.
3. Shown that, he answered **"Yes, signed off"**.

**Deviations, both from Nick's answers:**

- **A passage of an audience the reader knows leaves no stub.** The
  ticket's *Scope* and ADR-0090 §6 said it leaves a one-line stub.
  [ADR-0098](../../docs/adr/0098-the-readers-filter-leaves-a-known-audiences-notes-out-whole-and-show-and-hide-are-for-the-notes-a-reader-sees.md)
  amends §6. The stub ("New to Rust? `match` · show") is left by a note
  the reader hides. The bar's count and **Show all** stay, so nothing is
  lost for good, and a change in the bar or **Start over** resets each
  note's own state.
- **The front page and *Before you start*** said "a folded note leaves a
  line you can click to open it". They now say the bar leaves out the
  notes you don't need, and a note you see can be hidden and shown again.
  The front page's text was Nick's (ADR-0090); he read the new sentence
  before signing off.

**By hand in the built-in browser** (`zola serve`), in this order:

| Reader | Answers | Contents' tags | A page with marks | Next |
| ------ | ------- | -------------- | ----------------- | ---- |
| new to programming | No · — · No | *Before you start*: Start here | *Before you start*: no notes hidden, no skip line; then the programming toggle off shows "You can skip this page. Next for you: Quick start →" | Quick start → |
| a programmer new to Rust and to games | Yes · No · No | *Before you start*: You can skip this; Quick start: Start here | Quick start: 5 programming notes; a note opened, focus on it | Project → with "Skipped: *Before you start*"; the quick start's ← gone |
| a Rust programmer from another engine | Yes · Yes · another engine, Done | the same, and appendix A: Read this first | Step 3: 12 of 13 notes left out (11 Rust, 1 games), the Shuyi one shown | ← Rules as commands, Pushing → |
| knows Shuyi | Yes · Yes · with Shuyi, answered on step 4 first | — | Step 4 stays open after the popup: 9 notes hidden; step 3: 13 of 13 | — |

- **Skip:** Escape on step 4's popup, after **Start over**: `"knows": []`,
  no notes hidden.
- **Start over** opens the popup, clears the answers, and gives focus
  back to itself on closing.
- **A step's page first:** with storage cleared, step 4 opened the popup.
  A later visit to any page didn't.
- **A link into a hidden passage:** `03-walk/#the-big-idea-commands`, in a
  Shuyi note, for a reader who knows Shuyi, opened that note at the anchor.
  A `hashchange` to it opened it too.
- **The bar:** "New to Rust" off on the quick start: 12 notes hidden, and
  `knows` written at once; **Show all**: none, and the button gone; on
  again: 4. **Hide** on a shown note: its line with "show", focus on it,
  the count unchanged; **show** brings it back.

**Step 4's checks:**

- **Storage blocked** (the page in a frame whose `localStorage` throws):
  the popup opened, all 13 notes of step 3 showed, the answers folded the
  page, and the bar worked, for that page only. The book reads in full.
- **JavaScript off** (a sandboxed frame, no scripts): no popup, no bar,
  13 of 13 notes shown under their labels, the pager as built.
- **375 px:** every page of the book (25 and the front page), for a
  programmer and for a reader new to everything, with every note hidden
  to its line: none scrolls sideways. The popup fits at 375 px too.
- **Keyboard:** Tab and Enter answered the popup, focus moved to each new
  question, and a visible ring showed on every button.

**Gates run:** `typos`; `cargo xtask check-book`, `check-docs`,
`check-links`, `check-pins` and `ticket-lint`; `zola check`;
`site/check-build.sh`; `cargo xtask llms` (the twins carry no popup,
`llms-full.txt` 219 960 bytes); the engine's `cargo shuyi lint`. No Rust
changed, so the workspace gates are CI's.

**Handed over:**

- 2827 has a "From 2826 (done)" note: what the filter leaves out, the
  hide and the stub, and what a check needs to be treated the same.
- 2829's note on the filter is now "From 2826 (done)": where the popup's
  questions are, and that `read()` and `write()` must carry its
  `"version"`.

**Follow-up tickets:** none.

No break: no public API, format or golden value changed.

Book: this ticket is the book's change.
