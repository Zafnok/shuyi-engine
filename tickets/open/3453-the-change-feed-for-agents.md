---
id: "3453"
title: "The change feed: every push to main publishes what a game sees change, its signatures before and after, its Breaking and Behaviour lines and its migration blocks, as JSON, Markdown and Atom on the site, found by commit"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3453 — The change feed for agents

## Context

Nick, 2026-11-28: other games need to hear about engine changes, not
only Visions of Shuyi, and "other people aren't gonna give me random
PATs to wire into their projects". He asked for a changelog for people
and one beside it that "straight up tells you the signature and
behavior differences in a very agent friendly way", so a game's agent
can read the changes, search its code and make the edits.

What there is (read on 2026-11-28, `main` at 5d79431e):

- **The push to dependents** ([ADR-0045](../../docs/adr/0045-a-push-to-main-starts-dependents-watchers-workflow-dispatch-with-an-actions-only-token-kept-in-an-environment-for-main.md)):
  `notify-dependents.yml` starts each workflow listed in the
  `DEPENDENT_WORKFLOWS` variable with a token of ours that has Actions:
  write on that repository. It works for repositories Nick owns only:
  another game would have to give us a token on its repository.
- **The lines a game reads**, all under a ticket's `## Completion
  notes`, read by `crates/xtask/src/tickets.rs`:
  - `Breaking:` ([ADR-0038](../../docs/adr/0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md) §6, `breaking_lines`);
  - `Behaviour:` ([ADR-0108](../../docs/adr/0108-a-behaviour-line-for-a-change-a-game-sees-without-a-break.md), `behaviour_lines`);
  - `migration` blocks (`tickets/README.md`, *Breaking changes*; the
    parser is `changelog::migrations` in `crates/xtask/src/changelog.rs`,
    `Migration { since, what, old, new, before, after, find }`).
- **`cargo xtask changelog`** (2804) writes them as Markdown, Zola pages
  (News) and `CHANGELOG.md`, grouped by release tag, or by month before
  the first tag. It is prose for people, grouped by version: a game
  pinned by git revision (every game today: no release yet, 2005) can't
  tell which entries are past its pin.
- **The API's signatures are computed and thrown away.** CI's `api` job
  (`cargo xtask api`, `crates/xtask/src/api.rs`) diffs each PR's
  `cargo-public-api --simplified` listing and shows it in the job
  summary only. The Pages build (`.github/workflows/pages.yml`, *Write
  the API as text*) writes each crate's listing with its doc sentences
  to `/api/<crate>.txt` (`cargo xtask api-text`, 3413) on every push to
  `main`, and the next deploy replaces it.
- **`llms.txt`** (`cargo xtask llms`, 2808) is where an agent starts on
  the site; it links the Learn twins, the contract and `/api/`.
- **The Pages workflow** runs on every push to `main`, one at a time
  (`concurrency: pages-…`, `cancel-in-progress: false`). A failed run
  leaves the last site up.
- `shuyi new` games follow `main` by git revision (`Engine::Main`,
  `crates/cli/src/contract/new.rs`), so their `Cargo.lock` names the
  engine's commit (`git+https://github.com/Zafnok/shuyi-engine…#<sha>`).

This ticket publishes one feed, made from those sources, that any game
or agent reads with no account and no token: a pull, not a push.
3454 makes a game read it by itself (an op, an MCP tool and a template
workflow). 2523's `shuyi-upgrade` skill reads it through 3454.

## Nick input

None.

## Scope

**In:**
- **`cargo xtask changes`** (`crates/xtask/src/changes.rs`, wired in
  `crates/xtask/src/main.rs`: the module doc list, `USAGE` and the match
  arm):
  - `--previous <dir>`: the folder holding the live site's
    `changes/changes.json` and `changes/listing.json`, as the workflow
    fetched them (either may be missing: below);
  - `--listings <dir>`: this build's listings, one `<crate>.txt` per
    crate of `api::API_CRATES`, plain `cargo-public-api --simplified`
    lines (as `api::listing` returns them, no doc sentences);
  - `--out <dir>`: where it writes the site's `changes/`;
  - `--base-url <url>` and `--commit <sha>`, as `cargo xtask llms` takes
    them.
- **An entry per build.** The new entry covers the first-parent commits
  from the previous build's commit (exclusive, `listing.json`'s
  `commit`) to `--commit` (inclusive), read with one
  `git log --first-parent <prev>..<commit>` through a function the tests
  replace, as `changelog.rs`'s `LOG_ARGS` does. A deploy that failed
  makes the next entry wider; nothing is lost.
- **An entry holds** (`#[derive(Serialize, Deserialize)]`, the fields in
  this order):
  - `from`: the previous commit, or `null` for the first entry;
  - `to`: `--commit`;
  - `version`: the workspace `version` of the root `Cargo.toml` at `to`;
  - `commits`: each first-parent commit in the range, oldest first:
    `{ sha, date, subject, pr, ticket, tag }`. `pr` is the `(#NNN)` on
    the subject's end, `ticket` the `[NNNN]` on its start
    (`changelog.rs` reads both already: reuse its parsing), `tag` the
    `vX.Y.Z` decoration, each `null` when absent. Every commit is listed,
    so a game finds its pin's entry by its sha alone;
  - `api`: per crate whose listing changed, `{ crate, removed, added,
    changed }`. `changed` pairs a removed line with an added line of
    the same item, `{ path, old, new }`, and those two leave `removed`
    and `added`. An item's path is the listing line's path: the first
    word after the item's kind that contains `::`, cut before its first
    `(`, `<`, `:` followed by a space, or space (`pub fn
    shuyi_core::rng::Rng::next(&mut self) -> u64` is
    `shuyi_core::rng::Rng::next`). A path removed or added more than
    once (overloads of a trait's `impl`) is never paired. A crate new
    in this build lists every line as `added`;
  - `breaking`, `behaviour`: `{ ticket, crates, text }` for each
    `Breaking:` and `Behaviour:` line of each ticket in `commits`, read
    from `tickets/done/<ticket>-*.md` with `breaking_lines` and
    `behaviour_lines`; `crates` is the comma list before the line's
    second `:`, `text` the rest;
  - `migrations`: each `migration` block of those tickets, with its
    `ticket`, read with `changelog::migrations`;
  - `quiet`: `true` when `api`, `breaking`, `behaviour` and
    `migrations` are all empty.
- **The first entry.** With no `listing.json` in `--previous` (a 404 on
  the live site: the first build), the entry has `from: null`, the one
  commit `--commit`, no `api` (there is nothing to diff against) and a
  `note`: "The feed starts here: earlier changes are in the changelog".
  Any other failure to fetch fails the workflow step: a feed that lost
  its history is worse than a late one.
- **What it writes** in `--out`:
  - `changes.json`: `{ "schema": 1, "repo":
    "https://github.com/Zafnok/shuyi-engine", "entries": [...] }`, the
    new entry first, then the previous file's entries unchanged. A
    previous file of another `schema` fails the command;
  - `listing.json`: `{ "commit": <sha>, "crates": { "<crate>": [lines]
    } }`, the next build's `--previous`;
  - `index.md`, for agents: a header (what the feed is, that entries
    are newest first, how to find a pin: "search this file for the
    first 12 characters of your `Cargo.lock`'s engine commit; every
    entry above it is new to you", and the link to `changes.json`),
    then one section per entry that isn't `quiet`, headed with its
    range, its version and its PRs as links. Under it, each `changed`
    as a fenced `diff` (`- old` / `+ new`), each `removed` and `added`
    likewise, then the `Breaking:` and `Behaviour:` lines, then each
    migration block's `what`, `before` and `after`. Cut to a budget,
    `INDEX_BUDGET_BYTES` (200 000, beside `llms::FULL_BUDGET_BYTES`),
    newest entries kept, with a last line saying the older ones are in
    `changes.json`. A quiet entry is a line with its range only;
  - `atom.xml`: an Atom feed for people (a feed reader, a chat's RSS
    app), one `<entry>` per entry that has a `breaking` or `behaviour`
    line, a `removed` or a `changed`, newest 50, each linking its
    section of `index.md`. Written with plain escaping (the five XML
    entities), no new dependency.
- **The Pages workflow**, after *Write the API as text* and before
  *Write the docs for agents*:
  1. fetch `<base_url>changes/changes.json` and `listing.json` into a
     scratch folder with `curl -fsS`, treating a 404 as missing and any
     other failure as fatal;
  2. write the plain listings (a flag of `cargo xtask api-text`, `--plain
     <dir>`, that writes `api::listing`'s lines as they are: the nightly
     is already installed there);
  3. run `cargo xtask changes`, `--out site/public/changes`.
- **`llms.txt`** links `changes/index.md` and `changes/changes.json`
  in a section of its own, *Changes*, after the contract
  (`crates/xtask/src/llms.rs`); `site/llms-header.md` gets one sentence
  saying an agent moving a game to a newer engine starts there.
- **The site:** the News page (`site/content/news/_index.md` or its
  template) links `changes/atom.xml` beside its own feed, with one
  sentence: "every change a game sees, as it lands on `main`".
- **Docs:** the feed's fields, where to find it, and how a game finds
  its pin, in the one place the docs map gives a format that games read
  (the `document` skill decides the file); `docs/releasing.md` says the
  feed is per push and the changelog per release.
- **An ADR** (the next free number, `write-adr`): the feed is the
  engine's one public channel to dependents; it is a pull, published
  on the site, with no token; an entry is a range of first-parent
  commits, so a failed build widens the next one and loses nothing;
  the live site is its store, and a lost store restarts it with a
  first entry, the changelog keeping everything before. It extends
  ADR-0038 and ADR-0108 (who reads the lines) and stands beside
  ADR-0045, which stays for Nick's own repositories.
- **No public API change**: `xtask` is never shipped, and the site is
  not a crate.

**Out (do not do):**
- Reading the feed from a game (an op, an MCP tool, a template
  workflow): 3454.
- The `shuyi-upgrade` skill and the check that every `Breaking:` line
  has a `migration` block: 2523.
- Changing `notify-dependents.yml` or ADR-0045, or moving Visions of
  Shuyi onto the feed: the game's own ticket, if it wants it.
- `cargo-semver-checks` findings in the feed: they need the previous
  tree built, and the `Breaking:` lines, checked by CI's `api` job on
  each PR, already say what breaks.
- Rebuilding the feed's history before its first entry from old
  commits: the changelog holds it.
- A release's copy of the feed on its GitHub release: 2005's release
  workflow, if it wants one.
- Detecting behaviour changes: ADR-0108 §4.

**Estimated size:** about 1,300 lines (code 600, tests 500, docs and
the workflow 200).

**Split:** past 1,000 lines, split off `atom.xml` and the News link
into a new ticket blocked by this one.

## Implementation steps

1. Read `crates/xtask/src/changelog.rs` (its `git log` call and the
   subject parsing), `tickets.rs`'s `breaking_lines`, `behaviour_lines`
   and `notes_lines`, `api.rs`'s `listing` and `API_CRATES`,
   `api_text.rs`, `llms.rs` and `.github/workflows/pages.yml`.
2. Add `--plain <dir>` to `cargo xtask api-text`, with a test.
3. Write `changes.rs`: the entry types, the range read through a
   replaceable `git` function, the listing diff and its pairing, the
   ticket lines, `changes.json` and `listing.json`. Tests over a fake
   history and fake listings.
4. Write `index.md` and its budget, then `atom.xml`. Tests on each.
5. Wire the workflow steps, with the 404 rule as a small shell function
   in `site/` (as `site/check-build.sh` is) or inline. Link the feed
   from `llms.txt`, the header and News.
6. Write the docs and the ADR. Run the gates.

## Acceptance criteria

- [ ] Over a fake history of four first-parent commits (a ticket with a
      `Breaking:` line and a `migration` block, one with a `Behaviour:`
      line, a sonar PR, a commit with a `v0.2.0` tag) and fake listings
      before and after, `cargo xtask changes` writes one entry whose
      `commits` are the four, oldest first, with `pr`, `ticket` and `tag`
      read right, and whose `breaking`, `behaviour` and `migrations` hold
      exactly the two tickets' lines and block.
- [ ] A listing line whose signature changed is one `changed` with
      `path`, `old` and `new`; a removed item is in `removed` only; a
      new crate's lines are all `added`; two removed lines of one path
      are not paired.
- [ ] With no `listing.json`, the entry has `from: null`, no `api` and
      the note; with a previous `changes.json`, its entries follow the
      new one unchanged; a previous file of `schema: 2` fails.
- [ ] A range with nothing a game sees is `quiet: true`, a range line
      in `index.md` and no `atom.xml` entry.
- [ ] `index.md` stays under `INDEX_BUDGET_BYTES` on 500 fake entries,
      keeps the newest, and ends with the line naming `changes.json`.
- [ ] `atom.xml` parses as XML (a test with an entry whose text holds
      `<`, `&` and `"`) and has at most 50 entries.
- [ ] The Pages workflow fetches the previous files, fails on an error
      other than a 404, and its build has `changes/changes.json`,
      `changes/listing.json`, `changes/index.md` and `changes/atom.xml`;
      `llms.txt` links the first and third (its existing check that each
      link names a file of the site passes).
- [ ] After the first two deploys from `main`, the live
      `changes/changes.json` has two entries, the second's `from` the
      first's `to` (checked by hand once, in the completion notes).
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the subject parsing (reused); the path of a listing line (a
  `fn`, a method, a `struct` with generics, a `const` with a type, an
  `impl` line with no path); the pairing; the ticket lines' `crates`
  and `text`; the first entry; the previous file kept; the schema
  refusal; `index.md`'s budget and its quiet lines; `atom.xml`'s
  escaping and its cap; `api-text --plain`.
- Integration: the fake history and listings above, through
  `cargo xtask changes` with temporary folders, compared with a
  snapshot of the four files.
- CI: the Pages workflow's new steps on the first push to `main`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
