---
id: "2804"
title: "News with an automatic changelog: cargo xtask changelog from the tickets done and the merges, a changelog page, and a post for each release"
type: infra
milestone: "Engine: public"
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2801"]
nick_input: none
completed:
---

# 2804 — News with an automatic changelog

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): "a newspage
with automatic changelog". Bevy's News has a post for each release and
a migration guide.

Everything a changelog needs is already in the repository:

- each merged ticket is in `tickets/done/`, with its `id`, `title` and
  `completed` date;
- its block names its area (`tickets/README.md`, *Numbering*: `21xx`
  core, `22xx` assets, and so on);
- `main`'s first-parent history has one merge or squash per PR, titled
  `[NNNN] …` for a ticket, `Ticket NNNN: …` or `Game NNNN: …` for
  ticket-writing and small game PRs, and a Sonar title for
  `sonar` issues (ADR-0017);
- release tags will be `vX.Y.Z` (2005).

So nobody writes the changelog by hand: a tool writes it, the site runs
the tool when it builds, and the release workflow writes `CHANGELOG.md`.

## Nick input

None.

## Scope

**In:**
- **`cargo xtask changelog`** (`crates/xtask/src/changelog.rs`):
  - reads `tickets/done/*.md` frontmatter, the `vX.Y.Z` tags and their
    dates (`git tag --list 'v*' --format`), and `git log
    --first-parent main` subjects with their PR numbers;
  - groups the entries by release (a tag, newest first; "Unreleased"
    for after the last tag; before any tag, by month), then by area
    from the ticket's block;
  - one line per ticket: its title, linked to the ticket file and its
    PR. PRs with no ticket (Sonar fixes, small game PRs) are counted on
    one line per release ("12 Sonar fixes");
  - **breaking changes:** a ticket's completion notes may have a line
    starting `Breaking:` (under `## Completion notes`, after an optional
    `- `, in 2010's form `Breaking: <crate>[, <crate>…]: <item>: <what a
    game changes>`; read with 2010's `breaking_lines` in
    `crates/xtask/src/tickets.rs`, the one reader of these lines,
    ADR-0038 §6). Each such line is listed
    first, under "Breaking changes", for the game and other users to
    read before a pin bump (`AGENTS.md`, *Working with the game
    repository*);
  - **migration blocks:** under a `Breaking:` line, a fenced block with
    the info string `migration`, in YAML (parsed with `serde_norway`,
    already an xtask dependency). Its fields:
    - `since`: the engine version that first has the change, `X.Y.Z`
      (the workspace `version` after the PR);
    - `what`: one sentence;
    - `old` and `new`: the old and new path or signature;
    - `before` and `after`: a game's code before and after;
    - `find` (optional): literal strings that occur in `before` and not
      in `after`, and so mark a call site in a game's code.

    A ticket may have several. Each release's notes get a "Migration
    guide" section after "Breaking changes", in the Markdown, Zola and
    keep-a-changelog outputs: each block's `what`, `old` → `new`, and
    `before` and `after` as Rust code blocks. A block that doesn't parse
    is an error naming the file and line. The shape is documented in
    `tickets/README.md`, in a new section *Breaking changes*;
  - writes Markdown to stdout, or with `--site <dir>` Zola pages:
    `changelog.md`, and one `release-vX-Y-Z.md` post per tag with the
    release's date, the `release` tag, and the generated notes;
  - `--format keep-a-changelog` writes `CHANGELOG.md` in the [Keep a
    Changelog](https://keepachangelog.com/en/1.1.0/) shape;
  - the `git` calls go through one function the tests replace with a
    fake history.
- **The site:** `pages.yml` runs `cargo xtask changelog --site
  site/content/news/generated/` before `zola build` (with a full clone,
  `fetch-depth: 0`, so tags and history are there). The generated files
  are never committed (`.gitignore`). News lists them with the
  hand-written posts (2805), newest first; the Changelog page is linked
  from News and the footer.
- **Either order with 2805:** News's list, its tag pages and its feeds
  are 2805's. Each generated post has a one-line `description` and
  `tags = ["release"]` under `[taxonomies]`, and no author. If 2805 is
  done, its templates show these posts (with no author line); add the
  Changelog link only. If not, write a plain News list (date, title
  and description, newest first) and declare the `tags` taxonomy, with
  a feed, in `site/config.toml`; 2805 extends that list.
- **A release post can have an introduction:** if
  `site/content/news/intro/vX.Y.Z.md` exists, its text goes above the
  generated notes. Nick or Claude writes it when they want one; with
  none, the post is the notes alone.
- **The `work-ticket` skill:** its completion notes step says to add a
  `Breaking:` line for each API change the game must follow, and under
  it a `migration` block (`tickets/README.md`, *Breaking changes*).
  **With 2010** (done): the step is already there (`work-ticket` §5
  step 2; 2010 added it with CI's `api` job, which fails a breaking PR
  without the line), so keep one step and add the `migration` block to
  it.
  **Either order with 2014:** if 2014 is done, the step already says
  each engine item CI's `game` report lists as `new` has a `Breaking:`
  line; add the `migration` block beside that sentence. If not, nothing
  to do for the report: 2014 adds that sentence to the same step.
- **2005** (the first release) writes `CHANGELOG.md` with this tool in
  its release workflow, and uses its notes as the GitHub release's body.

**Out (do not do):**
- the release workflow itself (2005);
- posting anywhere but the site (social media, Discord);
- weekly digests (a follow-up if Nick wants one);
- CI's check that a breaking PR has a `Breaking:` line (2010), the
  check that each line has a `migration` block (2523), and building the
  game against a PR (2014);
- the machine-readable copy of the blocks shipped in `shuyi-cli`, and
  the `shuyi migrations` op and `shuyi-upgrade` skill that read it
  (2523).

## Implementation steps

1. Write the parser of ticket frontmatter (reuse `tickets.rs`'s), the
   history reader behind its trait, the grouping, and the `Breaking:`
   reader (reuse 2010's `breaking_lines`). Write the `migration`
   block parser `pub(crate)`, so 2523 reuses it.
2. Write the three outputs, each with its migration guide section.
3. Add it to `pages.yml` and `.gitignore`, and the News templates.
4. Write the *Breaking changes* section of `tickets/README.md`, and
   change the `work-ticket` skill and 2005.
5. Run the gates.

## Acceptance criteria

- [ ] On a fake history with two tags, tickets in four blocks, a
      `Breaking:` line, a `migration` block and three Sonar PRs, the
      output matches a committed snapshot in each of the three formats,
      and the snapshot shows the block's migration guide section.
- [ ] The live News page lists release posts (or, before the first tag,
      monthly ones) and the Changelog page lists every ticket in
      `tickets/done/`.
- [ ] A ticket moved to `done/` shows on the site after its PR merges,
      with no other step.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: grouping by tag and by area; the Sonar count; `Breaking:` lines;
  a `migration` block parsed; a bad one reported with file and line; a
  ticket in `done/` with no PR found (listed without a PR link).
- Snapshot: the three formats on the fake history.

## Completion notes

*(Filled in by the session that completes the ticket.)*
