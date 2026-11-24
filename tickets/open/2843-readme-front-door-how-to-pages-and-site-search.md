---
id: "2843"
title: The README is a front door, and the site has how-to pages and search
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: sign-off
completed:
---

# 2843 — The README is a front door, and the site has how-to pages and search

## Context

A blind-spot audit of the engine (2026-11-23) found three gaps in how a
newcomer finds their way in. Each was checked against the code.

1. **The root README is a map for insiders.** `README.md` is 289 lines
   and about 2,900 words. It has no picture and no Rust code block. It
   links the website once (line 216, *Fast compiles*) and never links
   the book's quick start, `/api/` or `/examples/`. Its repository map
   spends rows on Dependabot, the advisory check, CodeQL, Scorecard and
   SonarCloud (lines 118–119). *The crates* says "game ADR-0063 §2"
   (line 132), an ADR of another repository. About 15 lines explain
   `DEPENDENT_WORKFLOWS` to a game's maintainer (lines 167–181). A
   reader from crates.io or a search meets all this before one line of
   Rust. Bevy, macroquad, Godot and LÖVE all lead with a picture, a few
   lines of code and links. The material for a better first screen
   already exists: `examples/hello-grid/preview.png` is an animated
   picture that a test keeps current (`examples/hello-grid/shuyi.ron`,
   `preview`), and the book's quick start has its code in
   `site/snippets/src/quick_start.rs`, which CI builds and tests.
2. **Task knowledge reaches agents only.** The skill pack, `skills/`,
   has 18 task guides (about 174 KB of `SKILL.md`), each with a worked
   example CI runs on a new game ([ADR-0043](../../docs/adr/0043-the-skill-pack-skills-at-the-root-worked-examples-run-by-the-xtask-check-skills-in-the-cli-and-the-pack-folder-as-the-plugin.md)).
   The website never publishes them. A person who wants "add a screen"
   or "import art" has the book's concept chapters, and nothing task by
   task.
3. **The site can't be searched.** `site/config.toml` line 11 has
   `build_search_index = false`. 2801 left search as a follow-up "if
   Learn grows" (`tickets/done/2801-*.md`, line 160). Learn now holds
   the book (27 pages), the problem codes' pages (`site/errors.sh`) and
   the book's project. No open ticket plans site search: the only
   search in `tickets/open` is a module registry (2526, 2809).

The docs map (`docs/README.md`, the `document` skill) gives each fact one
home. The README's row is "What the engine is, its crates, and where to
start". The skill stays the home of a task's steps: the site's how-to
page is generated from it, as `cargo xtask llms` generates the twins
(2808) and `site/errors.sh` the problem codes' pages (2802).

## Nick input

**Sign-off:** open the README on GitHub, read its first screen, and
say whether a stranger would know what the engine is and how to start.

## Scope

**In:**
- **The README as a front door.** In this order, on its first screen:
  a one-line pitch; the status line, kept short; Hello, Grid's
  `preview.png`; a Rust block of about 15 to 20 lines; three commands
  (install the CLI, `shuyi new`, `cargo run`); and links to the quick
  start, the book, `/api/`, `/examples/`, the playground and `llms.txt`.
  Then, kept: what the engine is for, the crates in a short table, the
  first example, the license and the name.
- **The README's code is tested.** Its Rust block is an anchored region
  of a file in `site/snippets/` (the crate `book-snippets`, which CI
  builds and tests), named by a comment above the block. A new xtask
  check fails when the block and the region differ.
- **The insiders' material moves out.** The repository map, the upkeep
  rows, the dependents' dispatch (`DEPENDENT_WORKFLOWS`) and the
  `api:` labels go to a new `docs/repository.md`. `CONTRIBUTING.md` and
  `AGENTS.md` link it. The README links it once, under *Working on the
  engine*. The game ADR citation goes, and so does any other mention of
  this repository's process (tickets, the game's tickets, Nick): the
  docs map's rule 4.
- **How-to pages generated from the skill pack.** A new Learn section,
  `site/content/learn/how-to/`, titled *How-to*: one page a skill, made
  from its `SKILL.md` before Zola runs and never committed. It is named
  apart from the book's *Guides* part (2004's direction guides), which
  answers another question.
- **How-to pages for the cookbook.** The how-to pages for people are
  also made from `examples/cookbook/` (2786): one page a recipe, from
  its `//!` task line and its code.
  **Either order with 2786:** if 2786 is done, generate them; if not,
  nothing to do for the cookbook pages (2786 writes the recipes, and
  this ticket's generator picks them up).
- If 2844 is done, the README's contributor links include
  `docs/architecture.md`.
- **Site search** over Learn, the problem codes and the how-to pages,
  with Zola's built-in index, a search box in the menu and a link to
  the API docs' own search.

**Out (do not do):**
- The `shuyi` crate root's whole-game example, the entry-point doc
  examples and docs.rs metadata are 2842's. The README may reuse the
  crate root's game.
- The examples gallery and writing the cookbook: 2786. The how-to pages
  pick up any skill it adds with no change here.
- An in-session docs search for agents (the `docs` contract operation
  over the CLI and MCP, and MCP resources) is 3412's; this ticket's
  search is the website's.
- The architecture overview, `docs/architecture.md`: 2844.
- Links from the book's item names to the API: 2845.
- `SECURITY.md`, the code of conduct and the issue forms: 2849.
- Each crate's `readme` metadata for crates.io: 2005.
- Whether this joins the first release (2005's `blocked_by`): that is
  Nick's call, in 3528.
- Rewriting any skill. The skill is the source; a page that reads badly
  is fixed in the skill, in its own ticket.
- A search service, a hosted index, or a JavaScript build step.

**Estimated size:** about 1,200 lines (code 500, tests 300, docs 400).

**Split:** past 900 lines, split off steps 6–8 (site search) into a new
ticket.

## Implementation steps

1. **The README's code.** Pick the anchor in
   `site/snippets/src/quick_start.rs` that shows a whole small game
   (`game` is lines 37–58 today), or add a `readme` anchor there. Above
   the README's block write `<!-- snippet: src/quick_start.rs#<anchor> -->`.
   Add `crates/xtask/src/readme.rs`, run as `cargo xtask check-readme`
   and from `cargo xtask gates` (`crates/xtask/src/gates.rs`, the
   gates' table, and the `run-gates` skill's list). It fails, with the
   file and line:
   - a Rust block without the comment, or whose text differs from the
     region the comment names (read with the same anchor rule as the
     `snippet` shortcode, `crates/xtask/src/llms.rs`'s `anchored`);
   - a README with no image, or without links to the quick start,
     `/api/`, `/examples/` and `llms.txt`;
   - the words `game ADR`, `game's ticket` or `ticket NNNN` in the
     README.
2. **`docs/repository.md`.** Move there the *Map of the repository*
   table, the upkeep rows, the paragraphs on the `api:` labels and on
   `DEPENDENT_WORKFLOWS`, and any other part only a maintainer of the
   engine or of a dependent game needs. Grep the repository for links
   to the README's anchors that move (`README.md#`) and repoint them.
   `cargo xtask check-links` finds a broken one.
3. **The README.** Rewrite it in the order of the Scope. The commands
   are the quick start's own (`site/content/learn/book/quick-start.md`:
   `cargo install --locked --git … shuyi-cli`, `shuyi new …`, `cargo
   run`). Site links are absolute (`https://zafnok.github.io/shuyi-engine/…`),
   as line 216's is today. Keep the crates table, and its rule "adds a
   crate, or changes what one is for" in the docs map. Aim for under
   150 lines.
4. **The docs map.** In `docs/README.md`: the README's row says what it
   holds now; a new row for `docs/repository.md` under *Engine
   developers*; a row for the how-to pages (generated from `skills/`,
   never edited). `cargo xtask check-docs` passes.
5. **The how-to pages.** Add `crates/xtask/src/how_to.rs`, run as
   `cargo xtask how-to --site site`, beside the problem codes' script:
   in `pages.yml` and `ci.yml`'s `site` job before `zola build` (and
   before `cargo xtask llms`), and in `site/check-build.sh`. The folder
   `site/content/learn/how-to/` goes in `.gitignore`. For each
   `skills/<name>/SKILL.md` it writes `<name>.md`:
   - front matter: `title` from the skill's first `# ` heading,
     `description` from its `description`, a `weight` in the pack's
     order (`skills/README.md`'s list, or by name);
   - a first line saying the page is written for a coding agent, that a
     person follows the same steps by hand, and linking its source on
     GitHub;
   - the body after the skill's front matter. A link to another skill's
     `SKILL.md` becomes that how-to page. A link to a file in the skill's
     folder (`example.ron`, `example/…`, `references/…`) becomes its
     GitHub URL at the commit being built. `{{` and `{%` are escaped
     for Zola: `skills/shuyi-balance-bots/SKILL.md` quotes a workflow's
     `${{ … }}`, which Zola would read as a shortcode.
   If 2786 is done, it also writes one page for each
   `examples/cookbook/examples/<task>.rs`: the title from the `//!`
   first line, the rest of the `//!` comment as text, then the code in a
   Rust block and a link to the file on GitHub.
   And an `_index.md`: a short paragraph on what the section is, then
   the list. Add a *How-to* heading to Learn's page
   (`site/templates/learn.html`).
6. **The twins.** The how-to pages get their twins and `llms.txt` lines
   like every Learn page. Leave them out of `llms-full.txt`, as the
   book's project is (`for_agents` in `crates/xtask/src/llms.rs`): an
   agent has the skills themselves, and 174 KB would crowd the 400 KB
   budget. Say so in the `NOT_FOR_AGENTS` comment and in
   `site/README.md`, *The docs for agents*.
7. **The search index.** In `site/config.toml`: `build_search_index =
   true`, and a `[search]` table with `index_format =
   "elasticlunr_javascript"` (Zola 0.21 writes `search_index.en.js` and
   `elasticlunr.min.js`). Turn `in_search_index` off for the playground
   and the News feed pages if they crowd the results. Check
   `elasticlunr.min.js`'s license from the file Zola 0.21.0 writes
   (elasticlunr.js is MIT), commit its text under `site/static/`, and add
   its row to `THIRD_PARTY_ASSETS.md`.
8. **The search box.** `site/static/search.js`, plain JavaScript with
   no build step, as the site's other scripts: a box in `base.html`'s
   menu that loads the index on first focus, lists the top results with
   their section (book, problem code, how-to), works from the keyboard
   alone, and ends with "Search the API docs for …" linking
   `/api/shuyi/?search=<query>`. Without JavaScript the box is hidden.
   In `site/check-build.sh`, check that the index exists and is under a
   budget: measure it once, set the budget at 1.5 times that, and write
   both in `site/README.md`.
9. **Docs.** `site/README.md`: the how-to section and the search (its
   rows in *What is where*). `skills/README.md`: one line saying each
   skill is also a page of the site.

## Acceptance criteria

- [ ] The README's first screen has the pitch, Hello, Grid's picture, a
      Rust block, the three commands and the links of the Scope.
- [ ] `cargo xtask check-readme` passes, and fails on a README whose
      Rust block differs by one character from its anchor, on one with
      no image, and on one that says "game ADR".
- [ ] The README cites no game ADR, game ticket or ticket number, and is
      under 150 lines.
- [ ] `docs/repository.md` holds the map and the upkeep material; the
      docs map has its row; `cargo xtask check-links` and `cargo xtask
      check-docs` pass.
- [ ] The built site has `/learn/how-to/` and one page for each of the
      18 skills, each linking its source; a link between two skills goes
      from page to page; `zola check` passes.
- [ ] `llms-full.txt` holds no how-to page and stays under its budget;
      each how-to page has its twin.
- [ ] Searching the built site for "quick start" lists the quick start
      first, "SH0001" lists that problem code's page, and "rebinding"
      lists the input how-to page.
- [ ] `search_index.en.js` is under the budget written in
      `site/README.md`, and `check-build.sh` fails past it.
- [ ] `elasticlunr.min.js` has its row in `THIRD_PARTY_ASSETS.md` and its
      license text committed.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask/src/readme.rs`): a README whose block matches its
  anchor passes; one character off fails with its line; a block with no
  comment fails; a missing image or link fails; "game ADR" fails.
- Unit (`crates/xtask/src/how_to.rs`): the title from the heading; a
  link to another skill becomes its page; a link to `example.ron`
  becomes a GitHub URL; `${{ x }}` comes out escaped; the skill's front
  matter is gone.
- Unit (`crates/xtask/src/llms.rs`): a page under `learn/how-to/` keeps
  its twin and is out of the full file.
- Integration: `site/tests/search.mjs`, run with Node by CI's `site`
  job beside `ron-paint.mjs`, loads the built `search_index.en.js` and
  checks the three searches of the acceptance criteria.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
