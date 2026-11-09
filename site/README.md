# The website

Shuyi Engine's website: a [Zola](https://www.getzola.org/) project, built
by Zola **0.21.0** and published on GitHub Pages by
[`pages.yml`](../.github/workflows/pages.yml)
([ADR-0048](../docs/adr/0048-the-website-zola-0-21-pinned-in-site-sections-of-our-own-templates-and-the-base-url-passed-by-the-build.md)).
Nothing in the engine's build needs Zola.

## Preview it

Install Zola 0.21.0, the version the site is built with. Later releases
are under the EUPL-1.2, which this repository doesn't use (ADR-0048).

```bash
winget install -e --id getzola.zola --version 0.21.0
```

On Linux or macOS, take `zola-v0.21.0-*` from
[its release](https://github.com/getzola/zola/releases/tag/v0.21.0).
Then, in this folder, write the problem codes' pages and the feature
cards' statuses, and serve the site:

```bash
bash errors.sh
cargo xtask site-data --site .
zola serve --drafts
```

and open <http://127.0.0.1:1111>. `--drafts` shows the posts marked
`draft = true`. An example's page shows how to build it: the Pages build
copies each example's web build there.

## What is where

| Path | What |
| ---- | ---- |
| `config.toml` | The site's settings. `base_url` is for `zola serve`; the build passes `--base-url` |
| `content/` | The pages: `_index.md` (the home page's text), `learn/`, `community.md`, `sponsor.md`, `news/` (one Markdown file a post) and `examples/` (one page an example, named as its folder in `examples/`) |
| `content/changelog.md`, `content/news/generated/` | **Never committed**: `cargo xtask changelog --site site/content` writes the Changelog page and a News post for each release (`release-vX-Y-Z.md`, or `changes-YYYY-MM.md` before the first tag), and `pages.yml` runs it before `zola build`. It needs the tags and the whole history (`git clone` in full, not shallow) |
| `content/news/intro/` | An optional introduction for a release post, `vX.Y.Z.md`, which goes above the generated notes. Start it with `+++` / `draft = true` / `+++`, so Zola leaves the file out of the site and the tool drops the front matter |
| `content/learn/` | Learn: `_index.md`, the one way in; `book/`, the book ([The book's order](#the-book-s-order)); and `errors/`, which `errors.sh` writes |
| `content/learn/book/` | The book ([ADR-0090](../docs/adr/0090-one-book-the-project-folded-into-the-chapters-a-newcomer-mark-around-a-passage-and-a-readers-filter-kept-in-the-browser.md)): `_index.md`, its front page, whose `extra.parts` gives its order; its flat pages, each naming its part in `extra.part` (`before-you-start.md`; the chapters, numbered by `extra.chapter`; the appendices, lettered by `extra.appendix`, the glossary the last); and `box-pusher/`, its project |
| `content/learn/book/box-pusher/` | The book's project: `_index.md`, its landing page (`extra.project = true`, its code's folder in `extra.code`), and a page per step, named as its folder in `tutorial/`, its number in `extra.step` and the chapter it comes after in `extra.follows` |
| `tutorial/` | The project's code: one folder per step, a whole game project and no member of the workspace, with its own `Cargo.lock` and a path dependency on the engine's crates. CI's `tutorial` job builds and tests each. `shuyi new`'s files for agents and CI, and the pinned Rust (the root's applies), are left out. It keeps its name while the book has one project (ADR-0090 §2) |
| `tutorial/tries.toml`, `static/tutorial-tries/` | The pictures of the book's project and its chapters: each try names a step, or `book` for Moon Garden (`snippets/`), its keys and, for a before-and-after, the edits a page asks of the reader. `cargo xtask tutorial-tries` renders them into `static/tutorial-tries/<name>/`, where they are checked in. CI's `tutorial-tries` job renders them again and compares |
| `snippets/` | The book's code: a crate of the workspace, `book-snippets`, which CI builds and tests with the rest |
| `data/features.toml` | The home page's feature cards, the first rows from `docs/ROADMAP.md`, *Bevy's homepage*. A card names its `tickets`, and its status is made from them; a card with none has its own `status` ([What follows the tickets](#what-follows-the-tickets)) |
| `data/tickets.toml` | **Never committed**: `cargo xtask site-data --site site` writes each card's status and link text made from its tickets, and `pages.yml` and CI run it before `zola build`. Without it, each card shows its own `status`, or "Planned" |
| `data/api.toml` | The crates whose API docs Learn links, at `/api/`, where `pages.yml` puts `cargo doc`'s output |
| `templates/` | The HTML, our own (no theme): `base.html` has the menu, the footer and a Learn page's link to its twin; `book.html` the book's front page and contents, `chapter.html` every page of the book, a step's too, `project.html` a project's landing page, and `book-macros.html` the book's order and the pager through it; `shortcodes/snippet.md` shows the book's code, and `shortcodes/newcomer.md` marks a passage for a newcomer ([Marking a passage](#marking-a-passage)) |
| `static/style.css` | The one stylesheet: system fonts, light and dark, and the newcomer's mark (`.newcomer`). It also has the code's colors: `--code-*` tokens, in light and dark, each at least 4.5:1 on its background, for inline code and for the `z-*` classes Zola writes for a code block (`highlight_theme = "css"`, so no theme file) |
| `syntaxes/` | Our own grammars for code blocks, read through `extra_syntaxes_and_themes`: `ron` (the rows after a `.map` file's `---` stay plain) and `text` (plain, and known, so it gives no warning). A fence in a language the build doesn't know fails the build checks: add its grammar here |
| `errors.sh` | Writes `content/learn/errors/`, a page per problem code from `crates/cli/errors/`, never kept in git |
| `check-build.sh` | The build checks CI runs |
| `llms-header.md` | The start of `llms.txt`: its title, its summary and what Shuyi is ([The docs for agents](#the-docs-for-agents)) |

## The rules

- Link to a page of the site with Zola's `@/` links (`[News](@/news/_index.md)`)
  in Markdown and `get_url` in templates, never with an address:
  changing `base_url` alone must move every link.
- Load nothing from another host: no fonts, scripts, stylesheets or
  images.
- A new example gets its page in `content/examples/`, or the Pages build
  fails.
- The book's code comes from `snippets/`, never written in a page: CI
  builds and tests that crate, so a change to the engine that breaks the
  book fails. In the crate, mark the lines with `// ANCHOR: <name>` and
  `// ANCHOR_END: <name>` around whole items, and show them with

  ```text
  {{ snippet(file="src/rules.rs", anchor="rules") }}
  ```

  `file` is from `snippets/`; `language` names the fence's language
  when it isn't Rust (`language="ron"` for a RON file of the game's).
  `step` reads the file from that step of the project instead
  (`step="04-push"`, `file` from `tutorial/04-push/`), and with no
  `anchor` it shows the whole file.
  The lines are shown as written, without the lines of other anchors
  inside them. A file or an anchor that isn't there fails the build.
- A page of a project writes no code of its own: its only code blocks
  written in the page are `bash`, the commands to type, and the build
  checks fail any other. A step's page links its web build at `play/`,
  which `pages.yml` puts there (`templates/chapter.html`), and CI's
  `web-render` job boots each.
- The book is for a reader new to programming: each word of Rust or of the
  engine is explained the first time a page uses it, or links the page that
  did, and its row goes in the one glossary, `book/glossary.md`, with the
  newcomer who needs it (*New to*), linking the step that first uses it and
  the chapter that says more.
- A passage that only a newcomer needs is marked with who it is for
  ([Marking a passage](#marking-a-passage)), a new one too.
- Each chapter stands on its own: it says what an idea is for before its
  name, explains each word of Rust or of the engine its code uses, or links
  where a step does, and has a section *When it goes wrong* with real
  messages, from running the mistake. Its code stays on Moon Garden, and the
  project is the one that builds a game in order.
- Name a chapter by its title and link it. Its number is its place in the
  book's order, which a new chapter or project can change.
- A picture of the project or a chapter is a try in `tutorial/tries.toml`,
  never a file made by hand. A book's try has `step = "book"`, and its edits'
  files are from `snippets/`. Add it, run `cargo xtask tutorial-tries <name>`,
  look at the picture, and show it with

  ```text
  {{ try_it(name="box-blue") }}
  ```

  A try's edit must be in its file exactly once and build, and every try
  must be shown by a page, or the `tutorial-tries` job fails. A code anchor
  that a page adds to a step changes nothing else in the step.
- A new engine crate gets its row in `data/api.toml`, or the build
  checks fail.

## Marking a passage

Every passage of the book says who it is for (ticket 2825,
[ADR-0090](../docs/adr/0090-one-book-the-project-folded-into-the-chapters-a-newcomer-mark-around-a-passage-and-a-readers-filter-kept-in-the-browser.md)
§4 and §5). An unmarked passage is for everyone. A passage that only a
newcomer needs is marked for one of four:

| `to` | Its label | For a reader who | Such as |
| ---- | --------- | ---------------- | ------- |
| `programming` | New to programming? | has never programmed | what a program, a terminal, a file or a variable is |
| `games` | New to making games? | has never made a game | how games are made whatever the engine: the game loop and frames, state, tiles, data kept apart from code |
| `rust` | New to Rust? | programs, but not in Rust | a feature of the language, on the line that first uses it |
| `shuyi` | New to Shuyi? | hasn't made a game with this engine | an idea of the engine the first time a reader meets it: the stack of screens, what `ctx` holds |

Mark it with the `newcomer` shortcode, and a few words saying what it
explains, which name the line a folded passage leaves (2826):

```text
{% newcomer(to="rust", topic="`match`") %}
`match` compares a value with each pattern in turn…

{{ snippet(step="05-undo", file="crates/game/src/game.rs", anchor="match") }}
{% end %}
```

- The opener and `{% end %}` each stand on a line of their own, at its
  start, with a blank line before and after. A mark holds whole blocks:
  paragraphs, lists, code, `snippet`s, `try_it`s, headings with their
  sections. Never inside a list, a table or a blockquote.
- Marks don't nest. `topic` is required, and can't hold a `"`.
- A sentence for a newcomer inside a paragraph everyone needs is split
  into a paragraph of its own first. The part everyone needs stays
  unmarked.
- **Leave unmarked** what a reader who knows the audience still needs. A
  `games` passage says how games are made, not how Shuyi makes them. A
  `shuyi` passage only introduces an idea a Shuyi veteran already has: a
  line to type, a rule the engine checks, a gotcha, or what an operation
  does that the page relies on stays unmarked, and so does a chapter's own
  text about its idea.
- **Read the page once with each audience left out.** What is left must
  read as whole sentences, and its code must still be explained for that
  reader.
- A page written whole for one newcomer says so in its front matter,
  `[extra] newcomer = "programming"`, and its top line says "For readers
  new to programming".

With no JavaScript every passage shows, under its label and a rule at its
side. `cargo xtask check-book` fails a mark of an audience not among the
four, one with no `topic`, one not closed, one inside another or not on
lines of its own, and a callout of the old hand-made form
(`> **Rust:**`, `> **Shuyi:**`, `> **Game idea:**`).

## The book's order

The book's `_index.md` lists its parts in `[extra] parts`, in order
(ADR-0090 §2):

```toml
parts = [
  { part = "start" },             # the flat pages with extra.part = "start"
  { chapters = "box-pusher" },    # the chapters, each followed by that project's steps
  { part = "appendices", title = "Appendices" },
]
```

- A `part` entry holds the flat pages that name it, by `weight`.
- The `chapters` entry holds the chapters by `weight` (10–100 by tens).
  After each come the project's steps whose `follows` names the chapter's
  file, by `weight`, and the project's landing page before its first step.
- A `section` entry (the guides, 2004) holds that folder's landing page,
  then its pages.

The contents (`book.html`), the pager (`book-macros.html`, through the
whole book) and each page's top line follow it. Zola's `lower` and
`higher` stay inside one section, so they aren't used. An old address goes
in the page's `aliases`, and the build checks that it sends a reader on.

## What follows the tickets

`pages.yml` builds the whole site again on every push to `main`. What it
shows is made from the repository at each build, but for the book's words
and the feature cards' text:

| Part | Made at each build from | Follows the tickets |
| ---- | ----------------------- | ------------------- |
| The Changelog and the release posts | `tickets/done/` and the merges | yes |
| The API docs at `/api/` | `cargo doc` | yes |
| The problem codes' pages | `crates/cli/errors/` | yes |
| The Examples | each example's web build | yes |
| The feature cards' statuses | the `tickets` each card names (`cargo xtask site-data`, ticket 2815) | yes: **Ready** when all are in `tickets/done/`, **In progress** when some are, **Planned** when none is |
| The book's code | `snippets/`, built and tested in CI | yes: a change that breaks it fails |
| The project's code and its playable steps | `tutorial/`, built and tested in CI, built for the web by `pages.yml` | yes: a change that breaks a step fails |
| The book's words | written by hand | when a PR changes them: CI's `api` job fails a change to the public API that changes no file of the book and has no `Book:` line in its ticket's notes saying why not |
| `llms.txt`, `llms-full.txt` and the twins | the Learn pages | as the book does |
| The feature cards' text | `data/features.toml`, written by hand | no |

A card that names a ticket in neither `tickets/open/` nor
`tickets/done/`, or has both `tickets` and `status`, fails the command,
and the build checks (`check-build.sh`) fail a home page that doesn't
show a card's made status. Name a done ticket that shipped the part a
card has today, so the card says it's partly there.

## The docs for agents

After `zola build`, and once the API docs are in, `cargo xtask llms`
(`crates/xtask/src/llms.rs`, ticket 2808) writes into the built site, in
the shape of <https://llmstxt.org/>:

- a Markdown twin of each Learn page that isn't a draft, at its URL with
  `index.md` on the end (`/learn/book/quick-start/index.md`): its title,
  a `Source:` line, and its text, each `snippet` shortcode replaced by the
  code it shows, each passage marked for a newcomer a blockquote under its
  label and topic (`` > **New to Rust?** `match` ``), and each link made absolute (a `@/` link to a Learn page
  goes to that page's twin). The page's HTML links its twin
  (`templates/base.html`);
- `llms.txt`: [`llms-header.md`](llms-header.md), written by hand, the
  version and the commit, then a list of links for each section of Learn,
  the contract, the API docs, the released versions' copies and the
  optional reading;
- `llms-full.txt`: every twin but a project's (a section whose `_index.md`
  has `extra.project = true`), which teaches people from nothing, and a page
  written whole for a newcomer to programming, to making games or to Rust
  (`extra.newcomer`), nor the passages marked for one: an agent has the
  chapters, the API docs and the steps' sources. It keeps the passages for a
  newcomer to Shuyi. It is one file, which fails above
  `FULL_BUDGET_BYTES` (400 000 bytes, about 100 000 tokens). Raising it is
  a change in a PR, on purpose;
- the contract's schemas, `crates/cli/schemas/`, at
  `/contract/schemas/v<N>/`, with an `index.md`.

Change Learn, never a twin: they are written at each build. The twins
follow Zola's URL rule only for names already in slug form, so a Learn
page that sets `slug` or `path`, or a file or folder named other than
lowercase letters, digits and `-`, fails the tool. A shortcode added to
Learn needs its expansion in `llms.rs` in the same PR, or the tool fails
on it. To try it on a local build:

```bash
zola build --output-dir public
cargo xtask llms --public public --base-url http://127.0.0.1:1111/ --commit HEAD
```

## The checks

CI's `site` job, on every pull request:

```bash
bash errors.sh
cargo xtask check-book
zola check --skip-external-links
bash check-build.sh
```

then builds the site once more and runs `cargo xtask llms` over it, so a
shortcode left unexpanded, a broken `llms.txt` link or a full file over
its budget fails before the merge. The API docs aren't built there, so
their links are checked by `pages.yml`.

`cargo xtask check-book` checks the newcomers' marks ([Marking a
passage](#marking-a-passage)). `zola check` checks the `@/` links and their
anchors. `check-build.sh`
fails on a fenced Rust block in the book, on a code block of a project's
that isn't `bash`, and on an engine crate with no row in `data/api.toml`;
then it builds the site with two base URLs, fails if either output names
the other's or lacks a page, if a step's page doesn't link its `play/` or
follows no chapter, if an old address (`aliases`) doesn't send the reader
on, or if the chapters aren't numbered 1 to the last and the appendices
lettered from A, and fails if
a page loads anything from another host, if a code block's language is
unknown, or if a page with a Rust block has no `z-keyword`, which is
highlighting turned off. The `test` job builds and
tests `snippets/`.
`cargo xtask check-links` reads `content/` like the rest of the
repository's Markdown, and leaves the `@/` links to Zola.
