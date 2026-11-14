# ADR-0090: One book: the project folded into the chapters, a `newcomer` mark around a passage, and the reader's filter kept in the browser

- **Status:** Accepted; amended by ADR-0098 (§6: a known audience's passages left out whole, and hide on every passage shown); §8 extended by ADR-0117 (what an exercise is and how the game checks it)
- **Date:** 2026-11-08
- **Related tickets:** 2823 (this), 2824 (the move), 2825 (the marks),
  2826 (the filter), 2827 (quick checks), 2828 (exercises), 2829 (what
  changed since a version), 2830 (the follow-up project), 2004 (the
  guides), 2814 and 2820 (the playground)

## Context

Nick, 2026-11-03 (`docs/decisions.md`, *one book, read at the reader's
level*): the book and the tutorial fold into one book that holds
everything. The tutorial becomes the book's project. Every passage says
who it is for. A reader answers questions on a first visit, with a skip,
and a bar at the top of every page changes the answers. A reader who knew
an earlier Shuyi sees what changed. On 2026-11-05 the exercises' design
moved to 2828.

Nick, 2026-11-08 (*the book's project folds into its chapters, and a
fourth kind of reader*), reading a first mock of this design in the
browser:

- A project placed between the start and the chapters "looks like you
  appended when it should be a fold". He chose both kinds of fold: each
  chapter ends with the project's step that uses its idea, and a
  follow-up project comes after the whole book.
- The questions open as a popup on a first visit, with the skip in the
  popup. A list of where to start, or a part called *Start here*, would
  say the same thing again.
- A fourth kind of reader: someone new to making games (game loops,
  sprites, ECS). The game question is "Have you made a game before?",
  answered No, Yes in another engine, or Yes with Shuyi.

He then signed off the revised mock: the contents and the popup below.

What there is on 2026-11-08 (`site/README.md`, *What is where*):

- **The book**, `site/content/learn/book/`: a flat section, `book.html`
  for its contents and `chapter.html` for its pages. Chapters 1–10 have
  `extra.chapter`. *Coming from another engine* has `chapter = 11`, and
  *Fast compiles* has none, so it lists as an appendix. Its code is
  `site/snippets/`, Moon Garden.
- **The tutorial**, `site/content/learn/tutorial/`: `tutorial.html` and
  `tutorial-step.html`. It holds *Before you start* (`00-start`), ten
  steps with `extra.step`, and `glossary`. Each step's code is a whole
  project in `site/tutorial/<step>/`. Five places read that path:
  - `tries.toml`;
  - the `snippet` shortcode's `step`;
  - CI's `tutorial`, `web-render` and `tutorial-tries` jobs;
  - `pages.yml`, which copies each step's web build to
    `/learn/tutorial/<step>/play/`;
  - `check-build.sh`.
- **The hand-made marks**, counted on 2026-11-08: 75 `> **Rust:**`,
  21 `> **Shuyi:**` and 8 `> **Game idea:**`. The legend of *Before
  you start* explains the three.
- **"Chapter N" by number:** 66 lines of `site/content/`,
  `docs/README.md` (*What each task reads*), `docs/ROADMAP.md` and
  open tickets name a chapter by its number.
- **`crates/xtask/src/llms.rs`** writes a twin for each Learn page. It
  leaves every page under `learn/tutorial/` out of `llms-full.txt`
  (`FOR_PEOPLE`; Nick, 2026-11-01: the tutorial is written for people).
  It refuses a page that sets `slug` or `path`, and any shortcode it has
  no expansion for. `llms-full.txt` fails above 400 000 bytes.
- **`/hello-grid/`** already redirects with Zola's `aliases`.

How other books do it:

- **The Rust book** numbers its chapters straight through. Its projects
  are chapters among the concept chapters: the guessing game is chapter
  2, the command-line project is chapter 12, and the web server is the
  last. Brown University's edition adds quizzes in the page, each answer
  explained once chosen.
- **Rust by Example** (mdBook) has settings for the theme and search
  only, and no reader levels. Some pages end with an *Activity*, a task
  to do.
- **MDN** keeps two places: *Learn web development* for beginners and
  the reference for everyone. Nothing in a page is filtered, and a
  beginner is sent to the other place.
- **Godot's docs** put a project, *Your first 2D game*, between
  *Getting started* and the manual's concept pages, in one book.

None of them filters one text by what its reader knows. This book does.
The other things come from them:

- a chapter is followed by the project work that uses it, and a
  project comes at the end, as in the Rust book;
- a check sits in the page, as in Brown's edition;
- a task ends a page, as Rust by Example's do.

### What was tried

Ticket 2823 step 3. The trial ran in a scratch worktree of this
repository, on 2026-11-08, with Zola 0.21.0. It did these things:

- moved the pages as §2 says;
- built `chapter.html`, `project.html`, `book.html` and a macro file;
- added `aliases`;
- wrapped a passage in a body shortcode, with a `snippet` inside;
- ran `zola build` and `cargo xtask llms` over the result.

Then it gave each step a `follows` and put the chapters in §1's order.

It found:

- **A nested section works.** `book/box-pusher/` is a section of its
  own, with `template = "project.html"` and `page_template =
  "chapter.html"`. Its pages render with the book's page template.
- **The fold works, and a pager can run through it.** Zola's
  `page.lower` and `page.higher` stay inside one section. The trial
  instead built the book's order in a Tera macro:
  1. it took the book's `extra.parts`;
  2. for the chapters' entry, it took each chapter, then the project's
     steps whose `follows` names it, with the project's landing page
     before its first step;
  3. it found the current page's index in that list.

  The pager went from *Before you start* to the quick start, then the
  project's landing page and step 1, then chapters 2 and 3, then step 2,
  and so on to the glossary. The contents listed each step under its
  chapter. The build took 10.3 s, against 6.6 s before. Most of the
  difference is the nested loop, which runs once a page.
- **A body shortcode works around Markdown and a `snippet`.** The trial
  used `{% newcomer(to="rust", topic="…") %}` … `{% end %}`. Its text was
  a `.md` shortcode's output: `<aside …>`, a blank line, `{{ body }}`, a
  blank line, `</aside>`. Zola expanded the `snippet` inside it, and the
  Markdown inside rendered as Markdown: a paragraph, a list and a
  highlighted Rust block. A pair of inline shortcodes, `newcomer_start()`
  and `newcomer_end()`, worked too. It lost because the body form has
  one closer that can't be left out, and the check reads it more simply.
- **`aliases` work on pages and on sections.** The aliases were
  `learn/tutorial/04-push/` and `learn/tutorial/04-push/play/` on the
  step, and `learn/tutorial/` on the project's `_index.md`. Each wrote a
  redirect page at the old address, to the new one through the base URL.
  The redirect keeps the URL's `#anchor`, so an old link to a heading
  still lands on it.
- **`cargo xtask llms` takes the moved pages.** It wrote one twin for each
  moved page, at its new address, and none at an alias, since it reads
  the sources and not the output. It listed the project as a section of
  its own in `llms.txt`. It refused the three new shortcodes, as it
  should: each shortcode needs an expansion (2825).
- **The budget.** With `FOR_PEOPLE` no longer matching, `llms-full.txt`
  took the project: 386 728 bytes of its 400 000. The project's twins are
  149 441 bytes of that. Without them it is about 237 000.
- **The popup.** A mock of the book's front page, with the site's own
  stylesheet, was opened in the desktop app's browser. It had a
  `<dialog>` opened with `showModal()`, the questions shown one at a
  time, the bar, and the contents' tags. Nick read it, and signed off
  the second version.

## Decision

### 1. The order: the project folded into the chapters

The book reads in this order:

1. ***Before you start***, for a reader new to programming (§5).
2. **The chapters, each followed by the project's steps that use its
   idea.** A chapter teaches its idea on Moon Garden. The steps after it
   use the idea in the project, a box-pushing puzzle. The project's
   landing page comes once, before step 1.
3. **Guides**, the direction and move guides of 2004, when they land.
4. **The follow-up project**, a second and bigger game built with
   everything the book taught (2830), when it lands.
5. **Appendices:** A *Coming from another engine*, B *Fast compiles*,
   C *Glossary*. Under them, the contents also link the problem codes
   (`/learn/errors/`) and the API docs. Neither is a page of the book,
   and the pager doesn't reach them.

**The chapters go in the project's order.** A step comes after the
chapter whose idea it mostly uses, and the steps keep their order.

| New | Chapter | Was | Then the project's steps |
| --- | ------- | --- | ------------------------ |
| 1 | Quick start | 1 | 1 A new game |
| 2 | The frame and skins | 4 | — |
| 3 | Assets and formats | 5 | 2 A level on the board |
| 4 | Actions, never keys | 3 | — |
| 5 | Rules as commands | 6 | 3 Walking, 4 Pushing, 5 Undo |
| 6 | A game and its screens | 2 | 6 Levels and a menu |
| 7 | Saves and settings | 7 | 7 Saving progress, 8 Polish (the options and rebinding screens are settings) |
| 8 | Testing | 8 | 9 Bots, replays and a frame |
| 9 | Targets and export | 9 | 10 Shipping it |
| 10 | Tools for agents | 10 | — |

A step uses a little more than its chapter. Step 2's level is held as
the rules' state before chapter 5 says what a state is, and step 1 opens
a screen before chapter 6. A step already explains what it uses on the
line that uses it (2818), with a link to the chapter that says more. Each
chapter stands on its own (`site/README.md`, *The rules*), so the new
order changes only the words "chapter N" and "the previous chapter".

**Numbering:**

- Chapters are numbered 1–10, in the new order.
- A step keeps its number inside its project: "Project, step 4".
- The appendices take letters. *Coming from another engine* leaves
  `chapter = 11` for `appendix = "A"`. It was always reference, and the
  popup sends a reader from another engine there.
- *Before you start* has no number.
- Every "chapter N" in `site/content/`, `docs/` and the open tickets is
  rewritten to the new number in the same PR (2824).

**More projects.** Two kinds of project fit:

- **A folded project**, as the box-pusher is. Its steps each `follows`
  a chapter.
- **A project after the whole book**, as the follow-up is. It is a part
  of its own, after the guides.

A second project of either kind adds:

- its section, `site/content/learn/book/<name>/`, with its `_index.md`
  as §2 says;
- its entry in the book's `extra.parts`;
- its steps' code in `site/projects/<name>/<step>/`. The same PR moves
  the box-pusher's code from `site/tutorial/` to
  `site/projects/box-pusher/`, so the projects sit side by side. The
  `snippet` shortcode's `step` and a try's `step` then name
  `<project>/<step>`;
- its tries in `tries.toml`;
- its CI: the `tutorial` job's matrix, `web-render`'s loop, `pages.yml`'s
  copy to `play/`, and `check-build.sh`'s `play/` check, each looping
  over the projects.

None of that is done for one project.

### 2. The Zola layout

```text
site/content/learn/book/
  _index.md               the front page: the book's intro and its contents
                          (book.html); [extra] parts gives the order
  before-you-start.md     from tutorial/00-start.md
  quick-start.md …        the chapters, in place, renumbered
  from-another-engine.md  as it is, now appendix A
  fast-compiles.md        as it is, appendix B
  glossary.md             from tutorial/glossary.md, appendix C
  box-pusher/
    _index.md             from tutorial/_index.md: the project's landing page
    01-new.md … 10-ship.md  from tutorial/, names kept
```

- **Only a project, and the guides, are folders.** A part that is a
  folder is a Zola section. *Before you start*, the chapters and the
  appendices stay flat in `book/`, so their addresses and every link to
  them stay. Each names its part in `[extra] part`: `"start"`,
  `"chapters"` or `"appendices"`.
- **The book's order is data.** The book's `_index.md` lists its parts:

  ```toml
  [extra]
  parts = [
    { part = "start" },
    { chapters = "box-pusher" },   # the chapters, with that project folded in
    { section = "guides" },        # added by 2004
    { section = "…" },             # the follow-up project, added by 2830
    { part = "appendices", title = "Appendices" },
  ]
  ```

  - A `part` entry holds the flat pages that name it, by `weight`, under
    its `title` if it has one.
  - The `chapters` entry holds the chapters by `weight`. After each
    chapter come the project's steps whose `follows` names that
    chapter's file, by `weight`. The project's landing page goes before
    its first step.
  - A `section` entry holds that folder's landing page, then its pages,
    by `weight`. Its heading is its own `_index.md`'s title.
  - Weights order pages inside one part only: the chapters by tens in
    the new order (10–100), the appendices from 200.
- **A step's page** keeps `extra.step` and its file name, which is its
  code's folder, and gains `follows`, its chapter's file name
  (`follows = "rules-as-commands"`).
- **A project's `_index.md`:**
  - `template = "project.html"` and `page_template = "chapter.html"`;
  - `[extra] project = true`;
  - `code = "tutorial"`, the folder under `site/` with one folder per
    step;
  - its alias.
- **The templates:**
  - `book.html`: the book's front page, its intro and the contents. A
    chapter's steps are listed under it, as "↳ Project, step 3:
    Walking".
  - `chapter.html`: every page of the book, steps included.
    - Its top line says where the page is: "The book · Chapter 5", "The
      book · Appendix A", or "The book · Project: a box-pushing puzzle ·
      Step 4 of 10".
    - A step also gets **Play this step** (its `play/`) and **Its whole
      project** (`site/<code>/<step>` on GitHub). These come from
      `tutorial-step.html`, which goes. 2820's playground goes in the
      step's block here.
    - A chapter with steps ends with a line naming them: "Now use it in
      the project: step 3, *Walking*".
  - `project.html`: a project's landing page, its text and its steps. It
    comes from `tutorial.html`, which goes.
  - `book-macros.html`: the book's order. From it come the pager
    (previous and next through the whole book), the part line, and the
    order as JSON for 2826's script. 2826's bar and popup go here too,
    so every book page has the same ones.
- **`site/tutorial/` keeps its path** for the one project. Moving it now
  would touch five readers (`tries.toml`, `snippet`, three CI jobs and
  `pages.yml`) and give a reader nothing. Two projects are the reason to
  move it (§1).
- **Learn's front page** (`learn/_index.md`) has one way in, the book,
  beside the playground (2814), the API docs and the problem codes.
- **The playground** (2814) stays its own page, `/learn/playground/`. It
  isn't a page of the book and isn't in its order. The quick start
  links it.

### 3. The old addresses

Every page that leaves `learn/tutorial/` keeps its old address as an
`aliases` entry. The full list is in [the table of contents](#the-table-of-contents).

- `/learn/tutorial/` goes to `/learn/book/box-pusher/`. The alias is in
  the project's `_index.md`.
- Each step page also has `learn/tutorial/<step>/play/` as an alias, to
  the step's page. The web build itself moves to
  `/learn/book/box-pusher/<step>/play/` (`pages.yml`).
- **The twins:** a moved page gets one twin, at its new address. An old
  twin address (`/learn/tutorial/01-new/index.md`) isn't kept: a
  Markdown file can't redirect, and `llms.txt` lists the new ones. No
  released version links the old twins yet, so no frozen copy breaks.
  `aliases` is not `slug` or `path`, and `llms.rs` lets it through as it
  is.
- **`llms-full.txt`** leaves out the pages under a section whose
  `_index.md` has `extra.project = true`. It also leaves out a page
  marked whole for a newcomer to programming, games or Rust (§5). That
  rule replaces the path `FOR_PEOPLE` in 2824.

### 4. The audiences

There are exactly four. An unmarked passage is for everyone.

| Id | Label | For a reader who | Today's callout |
| -- | ----- | ---------------- | --------------- |
| `programming` | New to programming? | has never programmed: what a program, a terminal, a file or a variable is | — (in ordinary paragraphs) |
| `games` | New to making games? | has never made a game: how games are made whatever the engine (the game loop and frames, state, sprites, data kept apart from code, ECS) | `> **Game idea:**` |
| `rust` | New to Rust? | programs, but not in Rust: a feature of the language, on the line that first uses it | `> **Rust:**` |
| `shuyi` | New to Shuyi? | hasn't made a game with this engine: an idea of the engine, the first time the reader meets it (what the stack of screens is, what `ctx` holds) | `> **Shuyi:**` |

- **A mark hides its passage only from a reader who knows its
  audience.** A reader new to all four sees everything.
- **The `shuyi` mark is narrow: a Shuyi veteran must lose nothing they
  need.** A passage about the engine is unmarked when a reader who knows
  Shuyi still needs it:
  - a line to type or a file to change;
  - a rule the engine checks (the key check's `key_files` line in step
    5's first `> **Shuyi:**` note is one);
  - a gotcha;
  - what an operation does that the page relies on;
  - anything new since a release (§7).

  A `shuyi` passage only introduces an idea a veteran already has. In a
  chapter, the idea is the chapter's subject, so its text is unmarked.
  Only an opening that introduces the idea to a newcomer gets `shuyi`.
- **A `games` passage explains how games are made, not how Shuyi makes
  them.** "A game draws a frame many times a second" is `games`. "In
  Shuyi a screen draws into the glyph buffer" is `shuyi`, or unmarked.
- **A callout that does both is split.** The part everyone needs stays
  unmarked (2825).

### 5. The mark

**A passage:** a body shortcode, `site/templates/shortcodes/newcomer.md`:

```text
{% newcomer(to="rust", topic="`match`") %}
`match` compares a value with each pattern in turn…

{{ snippet(step="05-undo", file="crates/game/src/game.rs", anchor="match") }}
{% end %}
```

- **`to`** is one of the four ids. **`topic`** says in a few words what
  the passage explains. It is required, since it names the stub that a
  hidden passage leaves behind (§6).
- **Block-level only.** A mark holds whole blocks: paragraphs, lists,
  code, `snippet`s, `try_it`s, and headings with their sections. It
  stands on its own lines, with a blank line before and after. A
  sentence inside a paragraph is split into a paragraph of its own
  first.
- **Marks don't nest.** A passage for programming inside a passage for
  Rust would hide with the outer one anyway. The check fails a mark
  inside a mark, an unclosed mark, an unknown `to`, and a missing or
  empty `topic` (2825). So does a callout of the old form.
- **Its HTML:**

  ```text
  <aside class="newcomer" data-newcomer="rust">
  <p class="newcomer-label">New to Rust? <span>match</span></p>

  …the passage…

  </aside>
  ```

  The `topic` goes through Zola's `markdown(inline=true)` filter, since
  the label sits in an HTML block, where Markdown isn't read.
- **With no JavaScript,** every passage shows, each under its label and
  set off by a rule at its side, in light and dark.
- **A whole page:** `[extra] newcomer = "programming"`. *Before you
  start* is the one page marked whole on 2026-11-08. The page's top line
  says "For readers new to programming". The page's title is its topic.
  Its *Install the tools* stays: a reader new to programming installs
  there, and every other reader installs in the quick start.
- **The twin** (`llms.rs`): a mark becomes a blockquote. Its first line
  is the label and the topic, `` > **New to Rust?** `match` ``, and then
  each line of the passage follows with `> ` in front, the snippets
  expanded. A page marked whole has the label's line under its title.
- **`llms-full.txt`** takes every page it takes today, the glossary too.
  It leaves out:
  - the project's pages, as today (Nick, 2026-11-01: an agent reads the
    steps' sources);
  - the pages marked whole for `programming`, `games` or `rust`;
  - the `programming`, `games` and `rust` passages of the rest, since
    an agent programs, knows how games are made, and knows Rust.

  It keeps the `shuyi` passages, since an agent is new to the engine.
  Its first lines say what it leaves out. Taking the project would leave
  the file 13 000 bytes under its budget (*What was tried*).

### 6. The reader's filter

**The popup.** On a reader's first visit to any page of the book, a
`<dialog>` opens with `showModal()`. Its title is "Welcome to the book".
It asks its questions one at a time. **Skip: show me the whole book**
is in the popup and closes it with nothing hidden. Escape does the same.
Nothing on the front page or any other page says where to start besides
the popup and the tags below (Nick, 2026-11-08).

| Question | Answers | What it records |
| -------- | ------- | --------------- |
| Have you programmed before? | **No, I'm new to it** · **Yes** | "Yes": knows `programming`. "No" passes over the Rust question |
| Have you written Rust? | **No, or not much** · **Yes** | "Yes": knows `rust` |
| Have you made a game before? | **No** · **Yes, in another engine** · **Yes, with Shuyi** | "In another engine": knows `games`. "With Shuyi": knows `games` and `shuyi` |

- **The game question** is always asked: a reader new to programming may
  have made games, in a tool with no code.
- **"Yes, in another engine"** shows one more line before the popup
  closes: "Coming from another engine? Appendix A, *Coming from another
  engine*, puts the words of Godot, Unity, Bevy, GameMaker and PICO-8 in
  Shuyi's", then **Done**.
- The last answer, or **Done**, closes the popup and folds the page at
  once. On the front page the reader stays on the contents, which now
  carry the tags below. On any other page the reader stays on that page.

**Where the answers are kept:**

- `localStorage`, key `shuyi-book`, as JSON:
  `{"v": 1, "knows": ["programming", "games"], "engine": true}`.
  - A skip writes `"knows": []`.
  - The key being there means the popup has been answered or skipped.
  - `engine` is true after "Yes, in another engine".
  - 2829 adds `"version"`.
- Every read and write is in `try`/`catch`. When storage throws, the
  popup opens on every page, and a page shows everything until it's
  answered there.
- Nothing goes to a server, a cookie or a URL.

**The front page,** `/learn/book/`, is the book's intro and its
contents, nothing more. Once the popup is answered, the contents carry
small tags:

- **Start here** on the first page the answers don't hide. That is
  *Before you start* for a reader new to programming, and the quick start
  for anyone else.
- **You can skip this** on a page whose page-level mark the reader knows.
- **Read this first** on appendix A for a reader from another engine.

**The bar,** on every book page, from `book-macros.html`:

- "Notes for: **New to programming** · **New to making games** · **New to
  Rust** · **New to Shuyi**". Each is a toggle button:
  `aria-pressed="true"` shows that audience's passages. A change applies
  to the page at once, and the four are independent.
- How many passages of this page it hides: "3 notes hidden". **Show
  all** shows them for this visit.
- **Start over** clears the answers and opens the popup.

**With no JavaScript:** no popup and no bar. Every passage shows,
labelled, and the contents have no tags.

So the readers start here:

| Reader | Answers | Start here | Folded |
| ------ | ------- | ---------- | ------ |
| new to programming | No · — · No | *Before you start* | nothing |
| a programmer new to Rust and to games | Yes · No · No | the quick start | `programming` |
| a Rust programmer from another engine | Yes · Yes · another engine | the quick start, appendix A read first | `programming`, `games`, `rust` |
| knows Shuyi | Yes · Yes · with Shuyi | the quick start | all four |

**What a hidden passage leaves:**

- A one-line stub in its place, a `<button>`: "New to Rust? `match` ·
  show". Clicking it opens that one passage. Nothing disappears
  silently.
- A link to an anchor inside a hidden passage opens the passage.

**Skipping ahead:**

- **A page** is hidden whole only by its page-level mark. Its own top
  says "You can skip this page", with a link to the next page the reader
  needs.
- **The pager** follows the reader's path. Previous and next pass over a
  hidden page, with a line naming it ("Skipped: *Before you start*").
- **Inside a page,** a page is never skipped for its passage marks. Its
  hidden passages fold to stubs, so the page opens at the first passage
  the reader needs, and nothing scrolls by itself.
- **The script** reads the order from the JSON that `book-macros.html`
  writes into each page: each page's URL, title and page-level mark. It
  is plain JS, `site/static/book-filter.js`, with nothing from another
  host (2826).

### 7. The version marks (sketched for 2829)

- **A passage** that arrived or changed after the first release is
  wrapped in `{% since(version="0.4.0") %}` … `{% end %}`.
  - The version is as Cargo writes it. The compare link goes to its
    `x.y` frozen copy (`llms.rs`, *Released versions*).
  - A `since` may hold `newcomer` marks, and none of its own kind. A
    `newcomer` mark never holds a `since`.
  - It is block-level, as §5's mark is.
- **A whole page** new in a version: `[extra] since = "0.4.0"`.
- **The migration block** gets an optional field, `book`: the page, and
  an anchor if wanted, as an `@/` path
  (`book: "@/learn/book/actions.md#the-keymap"`).
  - `cargo xtask changelog` checks the page exists.
  - The `Book:` check takes a migration block with `book` as the book's
    change.
- **The reader's version** is a `"version"` field in `shuyi-book`, asked
  in the popup and the bar once there are two releases.
- Before the first release no passage carries `since`: everything is
  "since the first release".

### 8. Quick checks (sketched for 2827), and where an exercise sits

**Where:** a step or a chapter ends with a section, `## Check yourself`,
holding its quick checks. A step's exercises (2828) come after them, in
the same section. A chapter with steps after it ends with its checks,
then the line naming its steps (§2). The appendices have none.

**The shape:** a body shortcode, `check`. Its first paragraph is the
question. Then comes a list, one item per answer: `- [x]` marks a right
answer and `- [ ]` a wrong one, and ` — ` comes before why:

```text
{% check(to="rust") %}
What does the `?` at the end of `self.check(command)?` do when the check refuses?

- [ ] It stops the game. — No: nothing stops; `apply` hands the refusal back.
- [x] It returns the refusal from `apply` at once. — Yes: `?` returns an `Err` early.
{% end %}
```

- **`to`** is optional, an audience as in §5. The filter folds a check
  as it does a passage, with its stub.
- **Its twin** is the question, the answers, and which is right and why,
  as text.
- 2827 tries the list's parsing in Tera, and writes the check that fails
  a malformed one.

### The table of contents

Every page of the book and of the tutorial on 2026-11-08, in its new
place and order. Nick signed off this order on 2026-11-08.

| Order | Number | Page | New address (`/learn/book/…`) | From | Old address |
| ----- | ------ | ---- | ----------------------------- | ---- | ----------- |
| — | — | The book: its intro and contents | `/` | `book/_index.md` | (same) |
| 1 | — | Before you start (`programming`, whole page) | `before-you-start/` | `tutorial/00-start.md` | `/learn/tutorial/00-start/` |
| 2 | Chapter 1 | Quick start | `quick-start/` | `book/quick-start.md` | (same) |
| 3 | — | Project: a box-pushing puzzle, its landing page | `box-pusher/` | `tutorial/_index.md` | `/learn/tutorial/` |
| 4 | Step 1 | A new game | `box-pusher/01-new/` | `tutorial/01-new.md` | `/learn/tutorial/01-new/` |
| 5 | Chapter 2 (was 4) | The frame and skins | `frame-and-skins/` | `book/frame-and-skins.md` | (same) |
| 6 | Chapter 3 (was 5) | Assets and formats | `assets-and-formats/` | `book/assets-and-formats.md` | (same) |
| 7 | Step 2 | A level on the board | `box-pusher/02-board/` | `tutorial/02-board.md` | `/learn/tutorial/02-board/` |
| 8 | Chapter 4 (was 3) | Actions, never keys | `actions/` | `book/actions.md` | (same) |
| 9 | Chapter 5 (was 6) | Rules as commands | `rules-as-commands/` | `book/rules-as-commands.md` | (same) |
| 10 | Step 3 | Walking | `box-pusher/03-walk/` | `tutorial/03-walk.md` | `/learn/tutorial/03-walk/` |
| 11 | Step 4 | Pushing | `box-pusher/04-push/` | `tutorial/04-push.md` | `/learn/tutorial/04-push/` |
| 12 | Step 5 | Undo | `box-pusher/05-undo/` | `tutorial/05-undo.md` | `/learn/tutorial/05-undo/` |
| 13 | Chapter 6 (was 2) | A game and its screens | `games-and-screens/` | `book/games-and-screens.md` | (same) |
| 14 | Step 6 | Levels and a menu | `box-pusher/06-levels/` | `tutorial/06-levels.md` | `/learn/tutorial/06-levels/` |
| 15 | Chapter 7 | Saves and settings | `saves-and-settings/` | `book/saves-and-settings.md` | (same) |
| 16 | Step 7 | Saving progress | `box-pusher/07-save/` | `tutorial/07-save.md` | `/learn/tutorial/07-save/` |
| 17 | Step 8 | Polish | `box-pusher/08-polish/` | `tutorial/08-polish.md` | `/learn/tutorial/08-polish/` |
| 18 | Chapter 8 | Testing | `testing/` | `book/testing.md` | (same) |
| 19 | Step 9 | Bots, replays and a frame | `box-pusher/09-test/` | `tutorial/09-test.md` | `/learn/tutorial/09-test/` |
| 20 | Chapter 9 | Targets and export | `targets-and-export/` | `book/targets-and-export.md` | (same) |
| 21 | Step 10 | Shipping it | `box-pusher/10-ship/` | `tutorial/10-ship.md` | `/learn/tutorial/10-ship/` |
| 22 | Chapter 10 | Tools for agents | `tools-for-agents/` | `book/tools-for-agents.md` | (same) |
| 23 | — | Guides (2004) | `guides/…` | `docs/guides/`, copied at build | (new) |
| 24 | — | The follow-up project (2830) | its folder | (new) | (new) |
| 25 | Appendix A | Coming from another engine | `from-another-engine/` | `book/from-another-engine.md` (was chapter 11) | (same) |
| 26 | Appendix B | Fast compiles | `fast-compiles/` | `book/fast-compiles.md` | (same) |
| 27 | Appendix C | Glossary, the one for the whole book | `glossary/` | `tutorial/glossary.md` | `/learn/tutorial/glossary/` |
| — | — | Problem codes, and the API docs (links only) | `/learn/errors/`, `/api/` | `errors.sh`, `cargo doc` | (same) |

The steps' web builds move from `/learn/tutorial/<step>/play/` to
`/learn/book/box-pusher/<step>/play/`. The old address goes to the
step's page.

### The front page's and the popup's text

Nick signed off this text on 2026-11-08. 2824 writes the front page, and
2826 the popup. `[…]` is a button.

The front page, above the contents:

> # The book
>
> This book teaches you to make games with Shuyi Engine, from installing
> it to putting a finished game online. Each chapter takes one idea the
> engine is built on and shows it on a small game, Moon Garden. Then you
> use it in the book's project: a box-pushing puzzle you build a step at
> a time, from an empty folder to a game anyone can play in their
> browser.
>
> A note that only a newcomer needs says who it is for: new to
> programming, to making games, to Rust or to Shuyi. The bar at the top
> of each page folds the notes you don't need, and a folded note leaves
> a line you can click to open it.

The popup:

> ## Welcome to the book
>
> It's written for every reader at once. Three questions fold away what
> you already know. You can change them at any time in the bar at the
> top of each page.
>
> **Have you programmed before?** [No, I'm new to it] [Yes]
>
> **Have you written Rust?** [No, or not much] [Yes]
>
> **Have you made a game before?** [No] [Yes, in another engine] [Yes,
> with Shuyi]
>
> *(after "Yes, in another engine":)* **Coming from another engine?**
> Appendix A, *Coming from another engine*, puts the words of Godot,
> Unity, Bevy, GameMaker and PICO-8 in Shuyi's. [Done]
>
> [Skip: show me the whole book]

## Consequences

- The project is read where its ideas are taught, so a reader uses each
  idea right after meeting it.
- A second project is a folder, an entry in `extra.parts`, and the code
  move of §1.
- Every address of the book stays, and the tutorial's old addresses
  redirect, anchors included. But the chapters' numbers change: 2824
  rewrites every "chapter N" in the site, the docs and the open tickets,
  and later text should name a chapter by its title and link.
- The book's pages are built by one page template, so the bar, the
  popup, the stubs and the pager are written once.
- The pager builds the book's order once a page, through `get_section`
  and a loop over the steps for each chapter. On 65 pages that costs
  about 4 s of a 10 s build. A book of hundreds of pages would need the
  order computed once, for example as a data file written before the
  build.
- Writers have more to do: each newcomer passage is marked with a topic,
  and the check fails a passage left in the old form (2825). The
  `document` skill and `site/README.md`, *The rules*, say so.
- A reader whose browser blocks storage sees the popup on every page,
  but never loses the book.
- `llms-full.txt` gets smaller as passages are marked, and still leaves
  the projects out, so it stays well under its budget.
- Open tickets change in this PR to match: 2824, 2825, 2826, 2827, 2828,
  2829 and 2004. The follow-up project is a new ticket, 2830.

## Alternatives considered

- **The project as a part of its own, between the start and the
  chapters.** This was the first design, and it was simpler: no
  `follows` and no renumbering. Nick read it in the browser and said it
  "looks like you appended when it should be a fold".
- **A keystone project only, after all the chapters.** Cheapest: the
  parts are reordered and nothing renumbers. But a beginner reads ten
  chapters before building anything. Nick chose the fold *and* a
  follow-up project.
- **A welcome page with the questions, a list of where to start, and a
  *Start here* part.** That was the first design too. The three said the
  same thing three times (Nick, 2026-11-08). The popup asks, and the
  contents' tags answer.
- **Four questions, one each for Shuyi and for other engines.** That is
  one more click for the same four answers. A game made with Shuyi is a
  game made, so one question with three answers holds both.
- **The chapters keep their numbers and order, and steps follow
  whichever chapter comes first after their idea.** Step 6 (a menu
  screen) would need chapter 2, before steps 2–5, which need chapters 3
  to 6. The steps can't be reordered, since each builds on the last.
- **Every part a folder, the chapters in `book/chapters/`.** One rule for
  all parts, but it moves eleven pages that don't need it, and every
  link to them.
- **Keep `tutorial-step.html` for the project's pages.** It would carry
  a copy of the bar, the popup, the stubs and the pager. The step's few
  lines are an `if` in `chapter.html`.
- **Zola's `lower`/`higher`, and hand-made links between sections.** They
  don't cross sections, and hand-made links go stale when a step or a
  part is added.
- **A pair of inline shortcodes around a passage** (`newcomer_start()`,
  `newcomer_end()`). They worked in the trial, but a pair can be left
  half closed, and the check would then have to match them.
- **An inline mark around a sentence.** Its HTML would break the
  paragraph it sits in, and a filter that hides half a sentence leaves
  text that doesn't read.
- **Marks that nest.** "New to Rust" inside "New to programming" hides
  with the outer one anyway. Nesting buys nothing and makes the check
  and the twin harder.
- **The project in `llms-full.txt`, its `shuyi` and unmarked passages
  only.** That reverses Nick's rule of 2026-11-01, which is his to
  change, and it would leave the file near its budget.
- **A page skipped when every passage in it is marked.** The contents
  and the pager are built before any reader comes, so they can't know
  which marks a reader knows. A page-level mark can be known when the
  site is built.
- **The answers in the URL, or in a cookie.** A URL carries them into
  every shared link. A cookie goes to the server with each request.
  `localStorage` stays in the reader's browser.
- **Quick checks in a TOML file beside the pages, as the tries are.**
  Validating them would be easy, but the question would sit far from
  the text it asks about. 2827 may still choose it if the list can't be
  parsed in Tera.
