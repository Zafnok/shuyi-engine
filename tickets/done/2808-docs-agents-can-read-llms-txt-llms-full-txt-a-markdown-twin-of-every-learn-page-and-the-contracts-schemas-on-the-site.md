---
id: "2808"
title: "Docs agents can read: llms.txt, llms-full.txt, a Markdown twin of every Learn page, and the contract's schemas on the site"
type: infra
milestone: MVP
model: sonnet-5
effort: high
status: done
blocked_by: ["2801", "2802"]
nick_input: none
completed: 2026-10-28
---

# 2808 — Docs agents can read

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and with Defold among the 2D engines. An agent that
hasn't installed an engine learns it only from what it can fetch, and
some engines now publish their docs in the shape agents read best:

- Unity serves an `llms.txt` that points to one per product, over
  manual pages it also serves as Markdown
  (https://docs.unity.com/llms.txt).
- Defold has an `llms.txt` with Markdown indexes of its manuals, its
  API and its examples, and one `llms-full.txt` holding all of them
  (https://defold.com/llms.txt).
- Bevy has none (`bevy.org/llms.txt` answers 404), and its minor
  releases break its API, so agents often write Bevy code for a version
  that is gone.

The format is the proposal at https://llmstxt.org/: an `llms.txt` with
an H1, a summary in a blockquote and H2 sections of links, and a
Markdown copy of each page at the page's URL with `index.md` (or `.md`)
on the end.

What Shuyi has, and plans:

- 2801 builds the website with Zola, as HTML only. 2802 writes the book
  in `site/content/learn/book/` in Markdown, with every code block taken
  from `site/snippets/` by a `snippet` shortcode, and rustdoc HTML at
  `/api/`. 2803 adds the tutorial in `site/content/learn/tutorial/`.
  None of them writes anything for agents.
- The contract's JSON schemas are in `crates/cli/schemas/`:
  `versions.json` lists every contract version, and `contract::VERSION`
  is the last one (16 on 2026-10-24, in `crates/cli/src/contract.rs`).
  `shuyi schema` prints them, but only for someone who has installed
  the CLI. The site won't carry them.
- The templates' `AGENTS.md` (`templates/blank/AGENTS.md` line 9,
  `templates/grid/AGENTS.md` line 8) link the engine's `AGENTS.md` at
  `blob/main`, even for a game that `shuyi new --engine-rev` pins to a
  commit (`Engine::Rev` in `crates/cli/src/contract/new.rs`).
- 1134 (done) ships skills that `shuyi new` copies into each game, so
  they match the installed version. They don't help an agent that
  hasn't installed anything yet.

Why it matters:

- **The game.** Its sessions read the engine's checkout at the pin
  (`cargo xtask engine`), so this is for other games more than for it.
- **Other 2D games.** A young engine is missing from models' training
  data. Its docs on the web are the only way an agent learns it before
  `cargo install`, and Markdown with the code inline is what it reads
  best. Twins are cheap here because the book's source is already
  Markdown.
- **The end goal** ([`docs/decisions.md`](../../docs/decisions.md),
  2026-10-19): an agent told "make me a snake game with Shuyi Engine"
  starts by fetching the site. Snake waits for the real-time work
  (2026-10-20), but the same holds for the turn-based games the agent
  check measures until then. The home page (2801) lists the tool
  contract for agents as a feature, so the docs should read well to
  them.

**Version matching.** The site is built from `main`, while users will
install releases from crates.io (2005). So `llms.txt` says which commit
it documents, and from the first release on, each minor version keeps a
frozen `llms-full.txt` at `/v<x.y>/`: 2005 attaches it to the release,
and this ticket's `llms.txt` lists it.

## Nick input

None.

## Scope

**In:**
- **`cargo xtask llms`** (`crates/xtask/src/llms.rs`), run after `zola
  build`:
  - Arguments: `--public <dir>` (Zola's output, `site/public/` as 2801's
    `pages.yml` builds it), `--base-url <url>` (the value `zola build`
    got) and `--commit <sha>`. The version is `[workspace.package]
    version` in the root `Cargo.toml`. The commit's date comes from
    `git log -1 --format=%cs <sha>`, through one function the tests
    replace.
  - **The pages:** every `*.md` under `site/content/learn/`, `_index.md`
    included, except those with `draft = true`. The front matter (TOML
    between `+++` lines) gives `title`, `description`, `weight` and
    `draft`.
  - **The URL rule**, Zola's for names already in slug form:
    `learn/book/quick-start.md` is `/learn/book/quick-start/`, an
    `_index.md` is its folder's URL, and an `index.md` (a page with its
    files beside it) is its folder's URL too. Kept small on purpose, so
    these are errors naming the file: a page that sets `slug` or `path`,
    and a file or folder name that isn't lowercase ASCII letters, digits
    and `-`. A Learn page that needs one of them adds the rule here in
    its PR.
  - **The twins:** each page's Markdown is written to `<public>/<its
    URL>index.md`, starting with `# <title>` and a `Source: <HTML URL>`
    line, with:
    - the front matter dropped;
    - each `snippet` shortcode replaced by a fenced block holding the
      same lines the shortcode puts on the HTML page, with the same
      language tag (`rust` unless the shortcode sets another). Read
      2802's shortcode in `site/templates/shortcodes/` and match its
      path rule, its arguments and its handling of `// ANCHOR:` lines.
      An argument the expansion doesn't know is an error, like an
      unknown shortcode;
    - shortcodes found wherever Zola finds them: Zola reads them before
      the Markdown, so inside code blocks too. Its escaped form
      (`{{/*` … `*/}}`) becomes plain `{{` … `}}` in the twin;
    - then, links rewritten as absolute URLs. A Zola internal link
      (`@/learn/book/x.md#y`) to a page with a twin goes to the twin
      (`<base>/learn/book/x/index.md#y`), and one to any other page (News,
      the home page) goes to its HTML URL. A relative link to an image
      or a file is made absolute against the page's URL. Use
      `pulldown-cmark`'s offset iterator (already an xtask dependency)
      over inline links, images and reference definitions, and change
      only the target's bytes;
    - any other shortcode (`{{ name(…) }}` or `{% name(…) %}`) an error
      with its file and line. A shortcode added to Learn later needs its
      expansion here in the same PR.
  - **`llms.txt`** at `<public>/llms.txt`, in llmstxt.org's shape:
    - the H1, the blockquote and a short paragraph on what Shuyi is,
      from a hand-written `site/llms-header.md` (taken from the README's
      opening; no marketing words);
    - a paragraph the tool writes: the version, the commit and its date,
      that this copy follows `main`, that a game on a release should
      read its version's copy under *Released versions*, and the link
      to `llms-full.txt` with its size;
    - one H2 per section, that is per folder with an `_index.md`
      (`learn/` itself first, then the book, the tutorial and the rest
      as they come), sections and pages in `weight` order, then by file
      name. Each page is `- [title](twin URL): description`;
    - **Contract:** `docs/contract.md` at the commit
      (`https://raw.githubusercontent.com/Zafnok/shuyi-engine/<sha>/docs/contract.md`)
      and the schemas' index (below);
    - **API:** `/api/<lib name>/index.html` for each `shuyi-*` package in
      `crates/*/Cargo.toml`, with rustdoc's folder name (`shuyi_core`,
      with an underscore). These are 2802's rustdoc HTML pages; there is
      no Markdown API yet;
    - **Released versions:** each `<public>/v<x.y>/llms-full.txt` found,
      newest first, or "none yet";
    - **Optional:** the ADRs (`docs/adr/`) and `docs/ROADMAP.md`, at the
      commit.
  - **`llms-full.txt`** at `<public>/llms-full.txt`: every twin in
    `llms.txt`'s order, each from its `# title` and `Source:` line,
    split by `---` lines. The tool prints its size in bytes, and in
    tokens at about four bytes each, and fails above
    `FULL_BUDGET_BYTES` (400 000, about 100 000 tokens). Raising it is
    a change in a PR, on purpose.
  - **The schemas:** `crates/cli/schemas/*.json` copied to
    `<public>/contract/schemas/v<N>/`, N being the last version in
    `versions.json`, with an `index.md` listing each op's input and
    output files. A copy only: no `$id` in the schemas and no new op.
  - **Checks.** The command fails on any of these, listing them all:
    - a Learn page whose URL has no `index.html` in `<public>` (the URL
      rule here and Zola's disagree);
    - a shortcode left in a twin or in `llms-full.txt` (`{{` or `{%`
      outside a code block);
    - a link in `llms.txt` to the site that names no file in
      `<public>` (the API links are checked only when `<public>/api/`
      exists, and the output says when they weren't);
    - a link to the repository at the commit that names a path missing
      from the checkout;
    - `llms-full.txt` over the budget.
  - **`--full-only --out <dir>`** writes `llms-full.txt` alone, for
    2005's release workflow to attach to a release. It takes
    `--base-url` and `--commit` but no `--public`, and skips the checks
    that read `<public>`.
- **The site:**
  - each Learn page's `<head>` links its twin (`<link rel="alternate"
    type="text/markdown" href="index.md">`), in 2801's page and
    section templates, only for pages under `learn/` (the template
    tests the page's `relative_path`);
  - the Learn landing page and the footer get one line, "For agents:
    llms.txt", linked;
  - `site/README.md` gets a short section: what the tool writes, the
    twin rule, the URL rule, the budget, and that a new shortcode needs
    its expansion in `llms.rs`.
- **The docs map** (`docs/README.md`, 2810, done before 2802): a row
  for `llms.txt`, its twins and `site/llms-header.md` under *People
  making games*. They are generated from Learn, so the row says to
  change Learn and never a twin (the map's rule 2). 2801 gave
  `site/README.md` its row under *Engine developers*; add the llms
  tool to what that row says it holds.
- **`pages.yml`:** one `BASE_URL` value goes to `zola build --base-url`
  and to `cargo xtask llms --base-url`. The tool runs after `zola build`
  and 2802's `cargo doc` copy, before the upload, with `--commit ${{
  github.sha }}`.
- **`ci.yml`**, on pull requests, beside the site checks 2801 adds:
  `zola build --output-dir <tmp>`, then `cargo xtask llms --public <tmp>
  --base-url <the same URL> --commit ${{ github.sha }}`. A PR that
  leaves a shortcode unexpanded, breaks an `llms.txt` link or passes the
  budget fails before it merges.
- **`cargo xtask check-links`** skips `site/public`, since a local build
  writes twins there, and `crates/xtask/tests/fixtures/llms`, whose
  pages hold Zola's `@/` links and links broken on purpose (both in
  `SKIPPED_PATHS`, `crates/xtask/src/links.rs`).
- **New games point their agent at the engine they build with:**
  - in both templates' `AGENTS.md`, the sentence linking the engine's
    `AGENTS.md` at `blob/main` becomes a short list filled from one new
    placeholder, `{{engine.docs}}`;
  - `files()` in `crates/cli/src/contract/new.rs` fills it as one more
    pair beside the `{{engine.<crate>}}` ones, from the game's `Engine`:
    - `Main`: `AGENTS.md` at `blob/main`, and the site's `llms.txt` and
      `llms-full.txt`, from a new `ENGINE_SITE` constant
      (`https://zafnok.github.io/shuyi-engine`) beside `ENGINE_GIT`. The
      parked 2807 moves it to a custom domain;
    - `Rev`: `AGENTS.md` at `blob/<rev>`, and the book's Markdown at
      `tree/<rev>/site/content/learn`, saying its code is in
      `site/snippets/` at the same commit;
    - `Path`: the checkout's `AGENTS.md` and `site/content/learn/`, as
      paths;
  - the module's table of placeholders gains the row. Games on a
    released version come with 2005, which fills the same placeholder
    with that version's `/v<x.y>/llms-full.txt`.
- **From 2511 and 2501 (done):** the built-in template
  (`templates/shuyi-default/`) has replaced the two folders: change the
  sentence in its `base` part's `AGENTS.md` (its second paragraph), and
  fill `{{engine.docs}}` where the composer fills the
  `{{engine.<crate>}}` ones (`shuyi_cli::template::fill`, from a new
  field of `template::Fill` that `contract::new::fill_for` sets), with
  the test for each `Engine` variant there and its row in
  `docs/templates.md`'s table of placeholders.
- **Either order with 2803 and 2004:** 2803's pages are in
  `site/content/learn/tutorial/`, so they get twins and an H2 with no
  change here, unless 2803 adds an argument or a shortcode for its
  step folders or its diffs: if 2803 is done, read what it added and
  expand it too; if not, 2803 adds that expansion in its PR. 2004's
  guides (`docs/guides/`) reach Learn through the book. If they land in
  `site/content/learn/` as Markdown files before `zola build`, they get
  twins too, as long as `ci.yml` makes the same copy before its `cargo
  xtask llms`. If they come in through a shortcode, that ticket's PR
  adds its expansion here. **From 2802 (done):** 2509's error pages
  (`crates/cli/errors/`) land in `site/content/learn/errors/` as
  Markdown, written by `site/errors.sh` (never kept in git) before Zola
  runs in `ci.yml`'s `site` job and in `pages.yml`: run it before
  `cargo xtask llms` too, so they get twins. The book's `snippet`
  shortcode reads `site/snippets/<file>` and takes `file`, `anchor` and
  `language` (Zola's own `lang` is the page's).
- **Size:** if the diff, fixtures and snapshots aside, passes about 600
  changed lines, the `{{engine.docs}}` part (`new.rs` and the templates'
  `AGENTS.md`) becomes a follow-up ticket in this milestone, blocked by
  this one (the `write-ticket` skill). The PR says so, and points the
  mentions of that part in 2005 (which then waits for it too), 2501
  and 2807 at the new ticket.

**Out (do not do):**
- the frozen copy's upload and download (2005: its release workflow
  attaches the `llms-full.txt` that `--full-only` writes, and
  `pages.yml` puts each one in `/v<x.y>/`);
- the "Coming from another engine" chapter, mapping words from Godot,
  Unity, Bevy, GameMaker and PICO-8 to Shuyi's (2802 gains it);
- sending 1134's skills to these docs: they go through the game's
  `AGENTS.md`, and 1134 says so;
- a Markdown summary of the API from rustdoc JSON or
  `cargo-public-api`: rustdoc's JSON needs a nightly, so it waits until
  after the MVP, and only if agents are seen reading `/api/` badly;
- a `shuyi docs <query>` op with the book embedded in `shuyi-cli`. It
  overlaps 1134's skills and the crate sources in the cargo registry.
  It becomes a ticket only if the agent eval's runs (2525) show agents
  failing to find the docs;
- `$id` URLs in the schemas, or schemas of past contract versions (the
  repository keeps only the current files);
- hosted search, a chatbot, translations of the docs;
- twins of News, the blog and the home page.

## Implementation steps

1. Read 2801's `site/` (its config, its templates and `pages.yml`),
   2802's shortcode and snippets crate, and llmstxt.org.
2. Add `toml` to `crates/xtask/Cargo.toml` for the front matter:
   MIT OR Apache-2.0, both in `deny.toml`'s allow list (check its
   license on the day), with default features off and only what parsing
   into a `toml::Table` needs. It is xtask only and never ships. 2802
   wrote TOML front matter (`+++`); a chapter's number is its
   `extra.chapter`. Add `insta`
   (`insta.workspace = true`) to xtask's dev-dependencies for the
   snapshots.
3. Write `llms.rs`: the page list and the URL rule; the twins, with the
   shortcode expansion and the link rewriting; `llms.txt`,
   `llms-full.txt` and the schemas; then the checks. Add the command to
   `main.rs`, its module comment and its usage text.
4. Write a fixture site in `crates/xtask/tests/fixtures/llms/`: two
   sections, four pages, one draft, a snippet with nested anchors, an
   internal link with an anchor, a link to a page outside Learn, and an
   image. Snapshot its outputs.
5. Change the site's templates, `site/README.md`,
   `site/llms-header.md`, `pages.yml`, `ci.yml` and `links.rs`.
6. Change `new.rs` and the two templates' `AGENTS.md`.
7. Run the gates. After the merge, fetch `/llms.txt`, one twin and
   `/llms-full.txt` from the live site, and say so in the completion
   notes.

## Acceptance criteria

- [x] ~~The live site serves `/llms.txt` with an H1, a blockquote, the
      version and the commit, and the H2 sections above, and every link
      in it resolves.~~ (not done before the merge: the live site changes
      only when `pages.yml` runs on `main`. A build of the real site at
      the Pages URL, with `cargo xtask llms` over it, gives that
      `llms.txt` and its link check passes, the API links aside, which
      only `pages.yml` builds and checks. Fetching the live files is
      the first thing to do after the merge.)
- [x] Every page under `site/content/learn/` that isn't a draft has its
      twin at its URL with `index.md` on the end, and its HTML links
      the twin.
- [x] `/llms-full.txt` holds every twin in `llms.txt`'s order with
      snippets as code. The Pages run prints its size, and a size over
      the budget fails (fixture test).
- [x] A twin's code block is the same text as the anchored lines of its
      snippet file, with nested anchor lines dropped (fixture test).
- [x] An unknown shortcode, a page with `slug` or `path`, a page whose
      URL Zola didn't build, or a broken `llms.txt` link fails the
      command with the file and line (fixture tests), and so fails CI
      on a pull request.
- [x] `/contract/schemas/v<N>/` holds every file of
      `crates/cli/schemas/` for the current contract version, and
      `llms.txt` links its index.
- [x] `llms.txt` lists a `v<x.y>/llms-full.txt` put in the output
      folder, and says "none yet" without one (fixture tests).
- [x] `--full-only --out <dir>` writes `llms-full.txt` alone.
- [x] ~~`shuyi new` with `--engine-rev`, with the default and with
      `--engine-path` writes an `AGENTS.md` whose engine links name
      that commit, `main` and the site, or the checkout, and no file of
      a new game holds `{{`.~~ (not done: split into 2812, as the
      ticket's *Size* line says)
- [x] `cargo xtask check-links` passes with a local `site/public` and
      the fixture site in place.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`llms.rs`): the front matter (each key, a missing title, bad
  TOML); URLs (a page, an `_index.md`, an `index.md` in its own folder,
  and the errors for `slug`, `path` and a name not in slug form); the
  snippet expansion, with a language other than `rust` and an unknown
  argument; an escaped shortcode; link rewriting with anchors, images,
  a reference definition and a link outside Learn; section order by
  `weight`; drafts skipped; the budget; released copies listed newest
  first; each check failing on a fixture broken for it.
- Snapshot (`insta`): `llms.txt`, one twin and `llms-full.txt` of the
  fixture site, with a fixed commit and date.
- Unit (`new.rs`): `{{engine.docs}}` for each `Engine` variant; each
  template filled with each variant has no `{{` left.
- CI: the pull-request run over the real site.

## Completion notes

Done on 2026-10-28.

- **`cargo xtask llms`** (`crates/xtask/src/llms.rs`, its tests in
  `crates/xtask/src/llms/tests.rs`, the fixture site in
  `crates/xtask/tests/fixtures/llms/` and three insta snapshots) writes
  the twins, `llms.txt`, `llms-full.txt` and the schemas, and runs every
  check of the scope, listing all the errors. The real site today: 46
  pages (Learn, the book and the 31 problem codes), and `llms-full.txt`
  is 104 400 bytes, about 26 100 tokens, a quarter of the budget.
- **The site:** `base.html` links a Learn page's twin in its `<head>`
  (one test of `page.relative_path` or `section.relative_path`, so both
  page and section templates get it, and nothing outside Learn), and
  its footer and `learn.html` have the "For agents: llms.txt" line.
  `site/llms-header.md` is the start of `llms.txt`. `site/README.md`
  has *The docs for agents*.
- **Workflows:** `pages.yml` runs the tool after the API docs and the
  examples are in, with the same `BASE_URL` value as `zola build`;
  `ci.yml`'s `site` job builds the site again into `$RUNNER_TEMP` and
  runs it there (it now sets up Rust for the xtask).
- **Dependencies:** `toml` 1.1 (MIT OR Apache-2.0, read from its
  manifest on the day; `cargo deny check` passes), with `parse`,
  `serde` and `std` only: `toml::Table` and `toml::Value` need `serde`
  in 1.1. `insta` in xtask's dev-dependencies.

Deviations:

- **Split, by the ticket's *Size* line:** the diff passed 600 lines
  (fixtures and snapshots aside, `llms.rs` and its tests alone are
  about 2 100), so the `{{engine.docs}}` part (`new.rs`, the template's
  `AGENTS.md`, `template::Fill`) is **2812**. 2005 now waits for it too,
  and the mentions in 2005, 2807 (parked: its `blocked_by` gains 2812)
  and the done 2501 point at it. Its acceptance box here is struck.
- `llms-full.txt` starts with a line of its own before the first twin:
  the version, the commit and its date, and where `llms.txt` is, so a
  frozen copy at `/v<x.y>/` says what it documents.
- `llms.txt`'s *Optional* links the ADRs' index (`docs/adr/README.md`)
  rather than each ADR: its relative links resolve on the raw host too,
  and `llms.txt` stays short.
- The API links carry each crate's `description` from its `Cargo.toml`.
- A name that starts with a date (`2026-10-20-x`) is an error of the URL
  rule too: Zola takes the date off such a name.
- The schemas' index isn't a twin, so it isn't in `llms-full.txt`.

Follow-up: 2812 (above). After the merge, fetch `/llms.txt`, one twin
and `/llms-full.txt` from the live site.

No break: the xtask is never published, and no public crate changed.
