---
id: "2824"
title: "One book: the tutorial's steps fold into the chapters, each after the chapter it uses, with one glossary, one Learn way in, and the old addresses sending readers on"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["2823"]
nick_input: none
completed: 2026-11-08
---

# 2824 — One book: the tutorial folds into the chapters

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md), *one
book, read at the reader's level*): "the Book will contain everything. The
Tutorial can be an example game or project". This ticket moves the pages
where [ADR-0090](../../docs/adr/0090-one-book-the-project-folded-into-the-chapters-a-newcomer-mark-around-a-passage-and-a-readers-filter-kept-in-the-browser.md)
puts them. It changes no page's teaching: marking who a passage is for is
2825's, and the filter is 2826's.

**From 2823 (done):** ADR-0090 decides what this ticket builds, and Nick
signed off its order and its front page's text on 2026-11-08
(`docs/decisions.md`, *the book's project folds into its chapters*).

- **The order** (§1 and its *The table of contents*, with each new and
  old address): *Before you start*, then the chapters in the project's
  order. Each chapter is followed by the steps that use its idea. Then
  come the guides (2004), the follow-up project (2830) and the
  appendices. The chapters are renumbered:

  | Chapter | New number | Was |
  | ------- | ---------- | --- |
  | Quick start | 1 | 1 |
  | The frame and skins | 2 | 4 |
  | Assets and formats | 3 | 5 |
  | Actions, never keys | 4 | 3 |
  | Rules as commands | 5 | 6 |
  | A game and its screens | 6 | 2 |
  | Saves and settings, Testing, Targets and export, Tools for agents | 7–10 | 7–10 |

- **The pages:**
  - `tutorial/00-start.md` becomes `book/before-you-start.md`;
  - `tutorial/glossary.md` becomes `book/glossary.md`;
  - the tutorial's `_index.md` and its ten steps go into a project
    section, `book/box-pusher/`, with their names kept;
  - every other book page stays where it is.
- **The front matter:**
  - each flat book page names its part: `[extra] part = "start"`,
    `"chapters"` or `"appendices"`;
  - the book's `_index.md` lists the parts in order, in
    `[extra] parts`: `{ part = "start" }`, `{ chapters = "box-pusher" }`,
    `{ part = "appendices", title = "Appendices" }`;
  - each step gains `follows`, its chapter's file name, as the ADR's
    table gives it (`follows = "rules-as-commands"` for steps 3–5);
  - the project's `_index.md` has `template = "project.html"`,
    `page_template = "chapter.html"`, `[extra] project = true`,
    `code = "tutorial"`, and `aliases = ["learn/tutorial/"]`;
  - each step has the aliases of its old page and of its old `play/`;
  - the chapters get their new `chapter` and `weight` (10–100 by tens);
  - *Coming from another engine* gives up `chapter = 11` for
    `appendix = "A"`. *Fast compiles* is `"B"` and the glossary `"C"`,
    with weights from 200. The chapters are 1–10, so the numbering
    check in `check-build.sh` still holds;
  - *Before you start* gets `[extra] newcomer = "programming"`. This
    ticket adds the key only, and 2825 gives it its look.
- **"Chapter N" everywhere:** every line naming a chapter by its old
  number is rewritten to the new one. That is 66 lines on 2026-11-08,
  in `site/content/`, `docs/README.md` (*What each task reads*),
  `docs/ROADMAP.md` and `tickets/open/`; grep `chapter [0-9]` again.
  The same goes for words that assume the old order ("the previous
  chapter").
- **The templates** (§2):
  - `chapter.html` is every book page's, steps included. A step has
    *Play this step* and *Its whole project* under `page.extra.step`. A
    chapter with steps after it ends with a line naming them;
  - `project.html` is a project's landing page;
  - `book.html` is the book's intro and contents, each chapter's steps
    listed under it;
  - `book-macros.html` holds the book's order and the pager through the
    whole book;
  - `tutorial.html` and `tutorial-step.html` go.

  The ADR's *What was tried* shows each piece working on Zola 0.21.
- **The front page** is the ADR's *The front page's and the popup's
  text*, the front page's part: the intro, then the contents. It has no
  list of where to start and no *Start here* heading (Nick, 2026-11-08).
- **`llms.rs`:** `FOR_PEOPLE` becomes a rule. `llms-full.txt` leaves out
  the pages of a section whose `_index.md` has `extra.project = true`,
  and a page whose `extra.newcomer` is `programming`, `games` or `rust`
  (§3, §5).
- **`site/tutorial/` keeps its path** (§2). The web builds move to
  `/learn/book/box-pusher/<step>/play/`.

Today Learn has two ways in (`site/content/learn/_index.md`): the book
(`site/content/learn/book/`) and the tutorial
(`site/content/learn/tutorial/`), each with its own landing page, its own
templates and its own glossary-like words. The book's landing page sends a
new programmer to the tutorial's *Before you start*. The tutorial's code
stays in `site/tutorial/<step>/` unless the ADR moved it.

## Nick input

None.

## Scope

**In:**
- **The pages move** to ADR-0090's places: *Before you start*, the ten
  steps, the tutorial's `_index.md` and the glossary into the book, as
  `git mv`, so their history follows. Each gets the front matter above,
  and an `aliases` entry for its old address.
- **One glossary** for the whole book: the tutorial's `glossary.md` takes
  the words the book's chapters explain, and each chapter's link to a word
  goes there.
- **The landing pages:** the book's `_index.md` is its intro and its
  contents, as the ADR's text says (2826 adds the popup). Learn's
  `_index.md` has one way in, the book, beside the API docs and the
  problem codes. The
  tutorial's `_index.md` becomes the project's landing page,
  `book/box-pusher/_index.md`, its text kept but for the sentences that
  name "the tutorial" and "the book" as two things.
- **The templates** as the ADR says: the project's steps keep their
  `play/` link and their *step N of 10* place, and previous and next run
  through the whole book, in the ADR's order: each chapter, then its
  steps.
- **Every link** to a moved page (`@/learn/tutorial/…`) in `site/content/`,
  the templates, `site/data/features.toml`, `site/README.md` and the rest
  of the repository's Markdown, found with `cargo xtask check-links` and
  `zola check`. Open tickets that name a moved page's path in backticks
  (2112, 2225, 2528 and 2777 on 2026-11-03; grep `tickets/open` again)
  get the new path in this PR.
- **What reads the old paths:** `pages.yml` (the steps' web builds copied
  to each step page's `play/`), CI's `web-render` job, `check-build.sh`
  (its rule that a tutorial page has only `bash` code blocks follows the
  project's pages), and `crates/xtask/src/llms.rs` (its rule leaving the
  tutorial out of `llms-full.txt` follows the project, as the ADR says),
  with their tests.
- **`site/README.md`**: *What is where* and *The rules* for one book, and
  the `document` skill for the docs map.

**Out (do not do):**
- changing what a page teaches, beyond the links and the sentences that
  name "the tutorial" or "the book" as two things;
- marking passages by audience (2825), the welcome page and the bar
  (2826), quick checks (2827), exercises (2828);
- moving `site/tutorial/` or renaming its step folders (ADR-0090 keeps
  them);
- the guides (2004).

**Either order with 2814:** if 2814 is done, the playground keeps its own
page, and the new Learn landing page and the book's landing page link it;
if not, nothing to do (2814 links it from the landing pages as this ticket
leaves them).

**Either order with 2820:** if 2820 is done, the steps' playground moves
with their pages and template, and its copied files keep their paths; if
not, nothing to do (2820 builds it on the steps' pages where this ticket
puts them).

**From 2817, through 2823:** 2817 comes before 2823, so every step has been
played at its `/learn/tutorial/` address before this ticket moves it.
Nothing is left here but the redirects this ticket adds anyway.

## Implementation steps

1. Read ADR-0090 and 2823's completion notes, `site/README.md`, the templates,
   `check-build.sh`, `llms.rs`, `pages.yml` and the CI jobs that read
   `site/`.
2. Move the pages with `git mv`, give each its front matter and
   `aliases`, and write the landing pages.
3. Merge or adapt the templates, and fix the links.
4. Change `pages.yml`, the CI jobs, `check-build.sh` and `llms.rs`, with
   their tests.
5. Build the site, serve it, and in the built-in browser read the
   contents, follow previous and next through all of it, open a step's
   `play/`, and open three old addresses.
6. Run `cargo xtask llms` on the built site, the `document` skill, then
   the gates.

## Acceptance criteria

- [x] Every page the ADR's table of contents names is at its new address,
      and Learn has one way in, the book.
- [x] Every old `/learn/tutorial/…` address reaches its page's new address.
- [x] Previous and next run through the whole book in the ADR's order,
      each chapter followed by its steps, and the contents list each
      step under its chapter.
- [x] No line in the site, `docs/` or `tickets/open/` names a chapter by
      its old number.
- [x] Each step's page links its web build, and `web-render` boots each.
- [x] One glossary; no page links the old one.
- [x] `zola check`, `check-build.sh` and `cargo xtask check-links` pass,
      and `cargo xtask llms` writes a twin of every page, with
      `llms-full.txt` as the ADR says.
- [x] `git log --follow` on a moved page shows its history (the glossary's with `-M20%`, see the notes).
- [x] The book's chapter is updated (2802): this ticket is that change.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- `llms.rs`: its unit tests updated for the project's new path, one that
  an aliased page gets one twin, at its new address, and one that
  `llms-full.txt` leaves out a project's pages and a page whose
  `extra.newcomer` is `programming`.
- Site: `check-build.sh` fails a non-`bash` code block on a project page
  (tried once by hand, said in the notes) and passes the site.
- By hand in the browser: step 5, listed in the notes.

## Completion notes

**What was done.** The pages are where ADR-0090 puts them, moved with
`git mv` so their history follows:

- `tutorial/00-start.md` is `book/before-you-start.md` (`part =
  "start"`, `newcomer = "programming"`, the key only: 2825 gives it its
  look); `tutorial/glossary.md` is `book/glossary.md`, appendix C; the
  tutorial's `_index.md` and its ten steps are `book/box-pusher/`, the
  project, with `project = true`, `code = "tutorial"`, and
  `template = "project.html"`. Each step has `follows`, its chapter's
  file, as the ADR's table gives it.
- The chapters are renumbered and weighted 10–100 in the project's
  order. *Coming from another engine* is appendix A, *Fast compiles* B.
- Every moved page has its old address in `aliases`, and each step its
  old `play/` too. The redirects keep the `#anchor`.
- **One glossary:** it took a link to the chapter that says more for each
  word a chapter explains, and rows for the words only the chapters
  explain (seed, tween, property test, golden table, headless, export,
  debug view and gizmos, widget, chord and layout, asset and bundle,
  plugin, layers and mod, pack, profile, session, session record,
  platform backend, tool contract, MCP, skill pack, problem code, RON,
  build script, semver, linker). Each definition is the chapter's own
  words.
- **The landing pages:** the book's `_index.md` is Nick's front-page text
  and the contents; Learn's has one way in, the book, beside the API docs
  and the problem codes; the project's landing page keeps its text but for
  the sentences naming the tutorial and the book as two things.
- **The templates:** `book-macros.html` builds the book's order from
  `extra.parts` (`order()`, a content path a line) and makes the pager
  through the whole book, the top line ("The book · Chapter 5", "· Appendix
  A", "· Project: a box-pushing puzzle · Step 4 of 10") and the line ending
  a chapter with its steps ("Now use it in the project: step 3, *Walking*,
  …"). `chapter.html` is every page's, with a step's *Play this step* and
  *Its whole project* (from its project's `extra.code`). `book.html` lists
  each chapter's steps under it, the appendices lettered, and links the
  problem codes and the API docs last. `project.html` lists the steps, each
  with the chapter it follows. `tutorial.html` and `tutorial-step.html`
  are gone. The build takes about 1 s here, not the trial's 10 s.
- **"Chapter N":** every line of `site/content/`, `site/snippets/`,
  `docs/README.md` and `tickets/open/` naming a chapter by its old number
  was rewritten in one pass (each number mapped at once, so none chained),
  `from-another-engine.md`'s table of bare numbers too. 2108's planned
  "chapter 12 after chapter 11" is now chapter 11 after chapter 10, and
  its table is appendix A's. The quick start's *Next* now names chapter 2.
  `docs/ROADMAP.md`'s and `docs/decisions.md`'s "Chapter 1" are the
  game's, and stay.
- **What read the old paths:** `pages.yml` copies each step's web build
  to `/learn/book/box-pusher/<step>/play/`; `web-render` boots the builds
  from `site/tutorial/` as before (only its comment changed);
  `check-build.sh` finds the projects by `project = true` and fails a
  non-`bash` block in one, a step that doesn't link its `play/` or follows
  no chapter, an alias that doesn't redirect, and appendices not lettered
  from A; `tries.rs` reads the pages of `book/` and `book/box-pusher/`;
  `llms.rs` leaves out of `llms-full.txt` a project's pages and a page
  written whole for `programming`, `games` or `rust`, and its first lines
  and `llms.txt`'s say so.
- **Docs:** `site/README.md` (*What is where*, *The rules*, a new *The
  book's order*, *The docs for agents*, *The checks*) and the docs map's
  two rows for the book and its project (the `document` skill;
  `check-docs` and `check-links` pass). 2819's done notes had a link to
  the old glossary, now the new one.

**Tests.** `llms.rs`: the front matter's `project` and `newcomer`; a
project keeps its twins and `llms.txt` lines and stays out of
`llms-full.txt`; an aliased page gets one twin at its new address and
none at the old one; a `programming` page stays out of `llms-full.txt`
and a `shuyi` one stays in. The two snapshots changed by the new sentence.
`check-build.sh` failed, tried once by hand, on a ```` ```ron ```` block
added to `box-pusher/04-push.md` ("the project box-pusher has a code
block of its own"), and passes the site.

**By hand in the built-in browser** (the site built at a local base URL,
step 1's web build copied to its `play/`):

- the front page's contents, each step under its chapter;
- next from *Before you start* through all 25 pages to the glossary, and
  previous back, the exact reverse, in the ADR's order;
- step 1's *Play this step*: the game's title screen at
  `/learn/book/box-pusher/01-new/play/`;
- `/learn/tutorial/04-push/#winning`, `/learn/tutorial/`,
  `/learn/tutorial/00-start/` and `/learn/tutorial/01-new/play/` each
  landed on the new page, the anchor kept.

`git log --follow` shows the history of each moved page but the glossary:
every row of it changed (each link's path, and its new chapter link), so
git finds that rename only with `git log --follow -M20%`.

`cargo xtask llms` over the build: a twin for every page at its new
address, none under `learn/tutorial/`, `llms-full.txt` 237 028 bytes of
400 000.

**Gates run:** the skills' sync check, `cargo fmt --all --check`,
`cargo clippy -p xtask -p book-snippets --all-targets -- -D warnings`,
`cargo test -p xtask -p book-snippets`, `cargo xtask tutorial-tries
--check`, `zola check`, `check-build.sh`, `check-links`, `check-docs`,
`changelog`, `ticket-lint` and `typos`. No engine crate changed (only
`xtask`, and comments in `book-snippets`), so the workspace-wide gates are
CI's.

**Deviations:**

- The glossary links each word to the step and the chapter in one
  *Where* column, rather than a column each.
- `book.html` lists the problem codes and the API docs at the end of the
  appendices' list.
- The front page says what the bar does, as Nick signed it off; the bar
  is 2826's, and its ticket now says so.

**Handed over:** 2826 (the JSON order is to be written from
`book-macros.html`'s `order()`); 2820 builds its playground in
`chapter.html`'s step block; 2814 links the playground from Learn's list
and the contents; 2004 adds `{ section = "guides" }` to `extra.parts`;
2005's note about the quick start's place is gone. 2825 already says it
gives *Before you start*'s mark its look.

**Follow-up tickets:** none.

No break: no public API, format or golden value changed.

Book: this ticket is the book's change.
