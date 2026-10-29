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
Then, in this folder, write the problem codes' pages and serve the site:

```bash
bash errors.sh
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
| `content/learn/` | Learn: `_index.md`, `book/` (the book, one page a chapter, in `weight` order, each numbered by its `extra.chapter`; a page with none is an appendix) and `errors/`, which `errors.sh` writes |
| `snippets/` | The book's code: a crate of the workspace, `book-snippets`, which CI builds and tests with the rest |
| `data/features.toml` | The home page's feature cards, the first rows from `docs/ROADMAP.md`, *Bevy's homepage* |
| `data/api.toml` | The crates whose API docs Learn links, at `/api/`, where `pages.yml` puts `cargo doc`'s output |
| `templates/` | The HTML, our own (no theme): `base.html` has the menu, the footer and a Learn page's link to its twin; `shortcodes/snippet.md` shows the book's code |
| `static/style.css` | The one stylesheet: system fonts, light and dark |
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
  The lines are shown as written, without the lines of other anchors
  inside them. A file or an anchor that isn't there fails the build.
- A new engine crate gets its row in `data/api.toml`, or the build
  checks fail.

## The docs for agents

After `zola build`, and once the API docs are in, `cargo xtask llms`
(`crates/xtask/src/llms.rs`, ticket 2808) writes into the built site, in
the shape of <https://llmstxt.org/>:

- a Markdown twin of each Learn page that isn't a draft, at its URL with
  `index.md` on the end (`/learn/book/quick-start/index.md`): its title,
  a `Source:` line, and its text, each `snippet` shortcode replaced by the
  code it shows and each link made absolute (a `@/` link to a Learn page
  goes to that page's twin). The page's HTML links its twin
  (`templates/base.html`);
- `llms.txt`: [`llms-header.md`](llms-header.md), written by hand, the
  version and the commit, then a list of links for each section of Learn,
  the contract, the API docs, the released versions' copies and the
  optional reading;
- `llms-full.txt`: every twin in one file, which fails above
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
zola check --skip-external-links
bash check-build.sh
```

then builds the site once more and runs `cargo xtask llms` over it, so a
shortcode left unexpanded, a broken `llms.txt` link or a full file over
its budget fails before the merge. The API docs aren't built there, so
their links are checked by `pages.yml`.

`zola check` checks the `@/` links and their anchors. `check-build.sh`
fails on a fenced Rust block in the book, and on an engine crate with
no row in `data/api.toml`; then it builds the site with two base URLs,
fails if either output names the other's or lacks a page, and fails if
a page loads anything from another host. The `test` job builds and
tests `snippets/`.
`cargo xtask check-links` reads `content/` like the rest of the
repository's Markdown, and leaves the `@/` links to Zola.
