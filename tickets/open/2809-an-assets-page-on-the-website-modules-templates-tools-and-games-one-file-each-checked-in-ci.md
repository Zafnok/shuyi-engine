---
id: "2809"
title: "An Assets page on the website: modules, templates, tools and games, one file per entry, checked in CI, with other people's work listed if Nick says yes"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2801", "2005", "2003", "2526"]
nick_input: decision
completed:
---

# 2809 — An Assets page on the website

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engine Defold.

- **Bevy's website** has Assets in its top menu, beside Learn, News,
  Community and Examples. The page is built from one TOML file per
  entry, added by pull request: a name, a description under 100
  characters, a link, an optional image under 2 MB and an optional crate
  (<https://github.com/bevyengine/bevy-assets>).
- **Godot** has its Asset Library, and **Defold** its Asset Portal.

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md),
*everything on Bevy's homepage, and a website like Bevy's*). 2801
builds the site with Home, Learn, News, Examples, Community and
Sponsor, and leaves out Assets:

- its Examples page holds the engine's own examples;
- its Community page links issues, `CONTRIBUTING.md`, the code of
  conduct and the security policy (1132).

Nothing on the site lists modules, templates or tools, the engine's or
anyone else's. Nothing lists games made with the engine. The README
welcomes "made with Shuyi Engine" (*The name*), but there is no place
to show one.

What this ticket builds on:

- `docs/modules.md` (2003): the catalog of the engine's own modules,
  formats and backends.
- 2526: `shuyi add` of an outside module from crates.io, git or a
  folder, and `docs/writing-a-module.md`. It says how to add a module,
  not how to find one. ADR-0013 §7 and 2526 both refuse a registry
  service, and point to docs and links instead. Once 2526 is done, the
  engine also has:
  - `shuyi_assets::credits::CODE_LICENSES`, ADR-0002 §8's 14 code
    licenses (from 2223, or from 2526 if 2223 wasn't done);
  - `credits::license_allowed(expression: &str, list: &[&str]) ->
    Result<bool, String>`, an evaluator of SPDX expressions. 2526 put it
    there so this ticket's check can call it;
  - Nick's answer on other people's crate names (`shuyi-foo` or
    `foo-shuyi`) in `docs/decisions.md`, and a "Naming your crate"
    paragraph in the README.
- `targets/*.ron` at the repository's root: the target ids are the file
  stems (`desktop`, `web`, `playdate`, `steam-deck`, …).
- `crates/xtask/src/`: `ticket-lint`, `check-links` (relative links
  only; it never fetches the web) and `check-no-std`. The last two are
  each a `run(root: &Path) -> Vec<String>` of errors. CI's `tickets`
  job runs the first two. 2808 adds `toml` to the xtask for the Learn pages' front
  matter.

**Why it matters:**

- **The game** needs nothing here. Once it has a store page, it may be
  the first game listed, if Nick says yes (below). That is a name, a
  line and a link: no picture, and nothing of its content.
- **Other 2D games:** ADR-0009 §7 has the engine write a module only
  when a kind of game needs it, so the modules for most kinds of game
  will come from other people. A developer, or their agent, needs one
  page that lists them.
- **The end goal:** Nick wants "a wide variety of game types"
  (2026-10-16), each made quickly by an agent (2026-10-19). That depends
  on finding what others built.

A list reviewed by pull request is lighter than a registry, but it is
still moderation work. So this ticket builds the page, the entries'
format and its check, with the engine's own entries. Taking other
people's entries is a follow-up this ticket writes only if Nick says
yes.

## Nick input

**Decision**, with the `ask-nick` skill, at the start. Record the
answers in `docs/decisions.md`. The names of other people's crates are
2526's question, answered before this ticket starts.

1. **Should the page list other people's modules, templates and tools?**
   **Recommended: yes**, with these rules:
   - a submission is a GitHub issue form, so it works under any
     contribution policy (1132);
   - a Claude session checks it against a written checklist and adds
     the entry by PR;
   - Nick may remove any entry;
   - a listed entry is found, not vouched for: the page says so;
   - only the ADR-0002 §8 code licenses are listed, so anything listed
     may ship in a paid game.

   The cost: someone answers each submission.
2. **Should it list games made with Shuyi Engine?** **Recommended: yes**,
   as text and links only, with no screenshots, so no game's art enters
   this repository. A game may be under any license, paid or free.
   Visions of Shuyi may be the first once it has a store page.

What each answer changes:

- yes to either: this ticket writes the follow-up that takes
  submissions (Scope, below);
- no to 1: an entry of kind module, template or tool that isn't the
  engine's fails the check;
- no to 2: `game` is not a kind, `play` is not a key, and the page has
  no Games group;
- no to both: the page lists the engine's own entries only, and there
  is no follow-up.

The community channel (a forum, Discord or GitHub Discussions) is
1132's question. A game jam is a question for after the first release,
not this ticket's.

## Scope

**In:**
- **Entries, one file each:** `site/content/assets/<id>.md`, a Zola page
  whose `<id>` is lowercase kebab case. Zola's `load_data` reads one
  file, not a folder, so a folder of entries is a section of pages. The
  front matter:
  ```toml
  +++
  title = "foo-shuyi"
  description = "One plain line, under 100 characters."
  [extra]
  kind = "module"            # module | template | tool | game
  by = "Someone"             # who made it; "Shuyi Engine" for ours
  link = "https://crates.io/crates/foo-shuyi"
  source = "https://github.com/someone/foo-shuyi"   # optional for games
  license = "MIT OR Apache-2.0"                     # optional for games
  crate = "foo-shuyi"        # optional, its crates.io name
  engine = "0.2"             # the engine version it was last checked with
  targets = ["desktop", "web"]                      # optional for tools
  play = "https://someone.itch.io/foo"              # games only, optional
  +++
  ```
  - Format plugins and platform backends are modules here, as in the
    catalog (ADR-0009 §4 and §6).
  - The body is optional: a few lines of plain Markdown, with no raw
    HTML and no image.
  - This is site content, not a game asset, so ADR-0007 doesn't apply.
- **The page** (`site/content/assets/_index.md`, with `template =
  "assets.html"` and `page_template = "asset.html"`):
  - `site/templates/assets.html`: entries grouped as Modules, Templates,
    Tools and Games. A group with no entries is not shown;
  - each card shows the name, the line, who made it, the license, the
    engine version, the targets and its links (`link`, `source`,
    `play`);
  - the engine's own entries come first in each group, marked as the
    engine's;
  - `site/templates/asset.html`: an entry's own page, with the card and
    the body;
  - "Assets" joins the menu of 2801's base template, after Examples;
  - a short text at the top says what is listed. If Nick said yes to
    either question, it also says that an entry not made by the engine
    is found, not vouched for.
- **The first entries,** the engine's own (`by = "Shuyi Engine"`,
  `license = "MIT OR Apache-2.0"`):
  - the `grid` and `blank` templates, each with the targets in its
    `shuyi.ron`. Their `link` and `source` are
    `https://github.com/Zafnok/shuyi-engine/tree/main/templates`, which
    holds them as short names of one template made of parts
    (`templates/shuyi-default`, 2501);
  - the `shuyi` command line (`crate = "shuyi-cli"`);
  - the MCP server, `shuyi mcp` (1133);
  - the skill pack (1134).

  The Modules group opens with three lines, whether or not it has
  entries:
  - a link to the catalog, `docs/modules.md` on GitHub, for the
    engine's own modules, formats and backends;
  - how to add an outside module: `shuyi add <crate>@<version>` (2526);
  - a link to `docs/writing-a-module.md` (2526).
- **`cargo xtask check-assets`** (`crates/xtask/src/assets_page.rs`, a
  `run(root: &Path) -> Vec<String>` like `links::run`). It reads the
  front matter (between the first two `+++` lines) of every `.md` in
  `site/content/assets/` except `_index.md` into a `toml::Table`, so an
  unknown key can be named. It fails, naming the file and the rule,
  when:
  - the folder holds a file that isn't `.md` (no images), or a file
    name isn't lowercase kebab case;
  - a file has no front matter, a top-level key isn't `title`,
    `description` or `extra`, or `[extra]` has a key not listed above;
  - a required key is missing: `title`, `description`, `kind`, `by`,
    `link` and `engine` always; `source` and `license` for a module,
    template or tool; `targets` for all but a tool;
  - `kind` isn't one of the kinds (four, or three if Nick said no to
    games);
  - `description` is over 100 characters, holds a line break, or holds
    Markdown (`*`, `` ` `` or `[`);
  - `link`, `source` or `play` isn't an `https://` URL, or `play` is set
    on something that isn't a game;
  - `engine` isn't a version like `0.2` or `0.2.1`;
  - a `targets` id isn't the stem of a file in `targets/`;
  - a module, template or tool has a `license` that
    `credits::license_allowed(license, &credits::CODE_LICENSES)` (2526)
    rejects or can't read. A game may have any license, or none;
  - `by` is "Shuyi Engine" ignoring case and spaces, and either isn't
    written exactly so, or its `source` isn't
    `https://github.com/Zafnok/shuyi-engine` (alone or followed by `/`
    or `#`), or its `link` is neither that nor
    `https://crates.io/crates/<name>` or `https://docs.rs/<name>` for a
    package of `crates/*/Cargo.toml`. Only the engine's own work may say
    it is the engine's;
  - if Nick's naming answer (2526) keeps `shuyi-` for the engine's
    crates: an entry that isn't the engine's has a `crate` starting with
    `shuyi-` or `shuyi_` (crates.io treats the two as one name);
  - if Nick said no to 1: an entry of kind module, template or tool
    isn't the engine's;
  - the body holds raw HTML or an image (a `<` outside a code span, or
    `![`).
- **The xtask's dependencies:**
  - `toml`, if 2808 didn't add it: `toml = { version = "1",
    default-features = false, features = ["std", "parse", "serde"] }`.
    `toml` is MIT OR Apache-2.0. It brings `serde_core`,
    `serde_spanned`, `toml_datetime` and `toml_parser` (MIT OR
    Apache-2.0) and `winnow` (MIT), as read from toml 1.1.8's manifest
    on 2026-10-20. `deny.toml` allows both licenses. Check again on the
    day with `cargo deny check licenses`;
  - `shuyi-assets` by path, as `crates/cli/Cargo.toml` has it, for
    `credits`. The xtask is never published, and 2015's `check-layers`
    rules cover `shuyi-*` crates only.
- **Wiring:** `main.rs` (the match, `USAGE` and the module doc's list),
  the xtask's `description` in its `Cargo.toml`, a step in `ci.yml`'s
  `tickets` job after `check-links`, and a line in the `run-gates`
  skill after `check-links`.
- **The follow-up, if Nick said yes to either question:** write it with
  the `write-ticket` skill (After MVP, the next free id in the `28xx`
  block, `blocked_by` this ticket, `nick_input: none`). It holds:
  - an issue form, `.github/ISSUE_TEMPLATE/assets-entry.yml`, with the
    entry's fields, for the kinds Nick said yes to, and an "Add yours"
    link to it on the page;
  - a section "Listing an entry" in `site/README.md`: the checklist a
    session follows to turn an issue into an entry. The links open, the
    license fits, the line is plain and under 100 characters, `by` is
    who made it, and nothing claims to be the engine's. A PR from
    outside, when 1132's policy allows them, goes through the same
    checklist. Nick may remove any entry;
  - an ADR and a line in `AGENTS.md` hard rule 1, beside `sonar`: an
    issue labelled `assets-entry` or `site-links` is its own ticket,
    worked with that checklist, one issue per PR;
  - `.github/workflows/site-links.yml`: weekly (a `schedule` like
    `mutants.yml`'s) and `workflow_dispatch`, never on pull requests. It
    runs `zola check` with external links at 2801's pinned Zola version
    and checksum, so an entry whose site went away shows up. On failure
    it opens one issue labelled `site-links`, or comments on the open
    one, as 2011's `security.yml` does for `advisory`.

**Out (do not do):**
- taking other people's entries: the issue form, the checklist, the
  rule 1 exception and the weekly dead-link check are the follow-up's
  (above);
- the naming question and the README's "Naming your crate" (2526);
- screenshots or any image in entries, and an image-size check;
- a "Made with Shuyi Engine" badge;
- ratings, download counts, comments or paid listings;
- a hosted registry, a search service, a JSON feed of the entries for
  agents, or a snapshot of them in `shuyi modules` (2526 leaves it out
  too). A follow-up, once the page has entries by other people;
- the Assets page in `llms.txt` (2808 covers the Learn pages);
- an entry for Visions of Shuyi: a PR of its own once the game has a
  store page and Nick said yes to games;
- a game jam (ask Nick after the first release);
- a forum, Discord or Discussions (1132's decision);
- the engine's own examples, which stay on the Examples page (2801);
- checking that a listed module builds (its own CI does that).

## Implementation steps

1. Read 2801's `site/` (`config.toml`, the base template, the CSS and
   `pages.yml`), `credits::license_allowed` and `CODE_LICENSES` in
   `crates/assets/src/credits.rs`, and Nick's naming answer in
   `docs/decisions.md`.
2. Ask Nick (above) and record the answers.
3. Write `_index.md`, `assets.html`, `asset.html`, the card's CSS beside
   2801's, the menu item, and the five first entries.
4. Add the dependencies. Write `assets_page.rs` and its unit tests, then
   the wiring and the CLI test.
5. If Nick said yes to either question, write the follow-up ticket.
6. Run the gates. Look at the page and an entry's page at 1280 px and
   375 px wide with `zola serve` in `site/`.

## Acceptance criteria

- [ ] The site's menu has Assets after Examples. The page shows the
      engine's own entries grouped by kind, with no empty group, and
      each entry has its own page. `zola check` passes in CI.
- [ ] `cargo xtask check-assets` passes on the repository, and fails on
      a temporary entry for each rule, naming the file and the rule.
- [ ] An entry with `by = "Shuyi Engine"` and a `source` outside the
      engine's repository fails, and so does one whose `source` is
      `https://github.com/Zafnok/shuyi-engine-fork`.
- [ ] A module entry under `GPL-3.0-only` fails, one under `MIT OR
      Apache-2.0` passes, and a game entry with no license passes (if
      games are listed).
- [ ] An entry whose body holds `<script>` or an image fails.
- [ ] 2801's checks still pass: no page loads anything from another
      host, changing `base_url` moves every link, and no page scrolls
      sideways at 375 px.
- [ ] `ci.yml`'s `tickets` job and the `run-gates` skill run
      `check-assets`.
- [ ] Nick's answers are in `docs/decisions.md`. If he said yes to
      either question, the follow-up ticket is in `tickets/open/` and
      the PR names it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`assets_page.rs`, on temporary roots as `tickets.rs`'s tests
  make them, with a `targets/`, a `crates/*/Cargo.toml` and entries):
  - each rule, failing on one bad entry and passing on a good entry of
    each kind;
  - the front matter split: a file with none, a file with one `+++`;
  - the target ids read from `targets/`.
- CLI (`crates/xtask/tests/cli.rs`): `check-assets` passes on the
  repository, as `check_links_passes_on_the_repository` does.
- CI: `check-assets` in the `tickets` job on pull requests.

## Completion notes

*(Filled in by the session that completes the ticket.)*
