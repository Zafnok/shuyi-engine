---
id: "3454"
title: "A game hears what changed in the engine by itself: shuyi changes reads the change feed from the game's pin, as an op and an MCP tool, and a new game's weekly workflow keeps one issue up to date"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3453"]
nick_input: none
completed:
---

# 3454 — A game reads the change feed

## Context

3453 publishes the change feed: `changes/changes.json` on the site
(`https://zafnok.github.io/shuyi-engine/changes/changes.json`), one
entry per Pages build, each a range of first-parent commits of `main`
with every commit's sha, the public API's lines `changed` (old and new),
`removed` and `added` per crate, the `Breaking:` and `Behaviour:` lines
and the `migration` blocks. It needs no token to read. Nick
(2026-11-28): other people's games must hear about engine changes
without giving us a token, and their agents should be able to "read
changes, query the codebase, make changes".

This ticket makes a game read the feed itself, from its own pin:

- **A game's pin** is in its `Cargo.lock`: the `[[package]]` entries
  named `shuyi` or `shuyi-<crate>` for each of `new::ENGINE_CRATES`
  (`crates/cli/src/contract/new.rs:68`). Their `source` line is
  `git+https://github.com/Zafnok/shuyi-engine…#<sha>` (a revision:
  every `shuyi new` game today, `Engine::Main`), `registry+…` or
  `sparse+…` (a release, from 2005), or absent (a path checkout).
- **No op reads the network today.** `docs` searches what the binary
  embeds (ADR-0154, `crates/cli/src/contract/docs.rs`). The `shuyi`
  binary runs a few ops with no game program
  (`contract::cli::without_project`,
  `crates/cli/src/contract/cli.rs:105`), and `shuyi mcp` makes a tool of
  each op in `Op::ALL`, answered by `mcp::host::one_off`
  (`crates/cli/src/mcp/host.rs:139`).
- `crates/xtask/src/publish.rs` already shells out to `curl`, which
  Windows 10 and later, macOS and the Linux runners have.
- A new game's workflows are `templates/shuyi-default/base/.github/workflows/`
  (`ci.yml`, `export.yml`, `release.yml`).

2523 (the `shuyi-upgrade` skill) builds on this op instead of a list of
its own.

## Nick input

None.

## Scope

**In:**
- **`shuyi changes [--from <rev>] [--to <rev>] [--feed <file or URL>]
  [--json]`**, a contract op (`crates/cli/src/contract/changes.rs`,
  `Op::Changes`) that needs no game program:
  - in `contract::cli::without_project`, beside `docs` and `doctor`, so
    it runs while the game doesn't build; it reads the game from the
    working folder (the first folder up that has a `Cargo.lock`);
  - **the feed:** `--feed` a file is read as it is; a URL, or no
    `--feed` (`FEED_URL`, the site's `changes/changes.json`), is fetched
    with `curl -fsSL --max-time 30 <url>` through a `Fetch` trait the
    tests replace. A feed whose `schema` isn't 1 is an error that says
    to update the engine's tools. The op's types mirror 3453's entry
    (`crates/xtask/src/changes.rs`), deserialized here: `xtask` is no
    dependency of `shuyi-cli`, and a test reads 3453's snapshot feed to
    keep the two equal;
  - **from:** `--from`, else the lock's pin. A revision is a sha or a
    prefix of at least 7 hex characters, matched against every entry's
    `commits`; a version `X.Y.Z` is the commit whose `tag` is `vX.Y.Z`.
    The lock gives a sha (`git+`), a version (`registry+`, `sparse+`) or
    nothing (a path: the error names `--from`). Engine crates at two
    pins are an error;
  - **to:** `--to` (as `from`), else the feed's newest commit;
  - **the entries:** each entry after the one holding `from`, up to the
    one holding `to`. The entry holding `from` is in too when `from`
    isn't its last commit, marked `partial: true` ("some of it may
    already be in your engine"). Quiet entries are counted, not listed;
  - **older than the feed:** a `from` in no entry but older than the
    feed's first (the first entry's `from` is `null`, and the sha isn't
    found) answers every entry, with a limit: "your engine is older
    than the feed, which starts at <sha> (<version>); the changelog
    says what came before". A `from` the caller gave that is in no
    entry and not a sha shape is an error;
  - **call sites:** for each migration block's `find` strings, every
    place one occurs in the game's `*.rs` and `Cargo.toml` files: the
    file from the root, `/`-separated, the 1-based line and column (in
    characters), and the line's text. The walk skips `target/`, every
    folder whose name starts with `.`, and every folder below the root
    whose `Cargo.toml` has its own `[workspace]` (an engine checkout
    kept inside the game). API lines carry no call sites: the compiler
    finds those once the pin moves, and the answer says so once;
  - `ChangesInput { from, to, feed: Option<String> }`; `ChangesOutput {
    from, to, quiet: u32, entries: Vec<Entry>, limits: Vec<String> }`;
    `Entry { range, version, prs: Vec<u32>, partial, api, breaking,
    behaviour, migrations }`, each migration with `found:
    Vec<CallSite>`, `CallSite { file, line, column, text }`. The output
    types are `#[non_exhaustive]` (ADR-0039, kind 6: the feed grows
    fields);
  - `ok` is true when no listed entry has a `breaking` or `behaviour`
    line, an API line or a call site: the game's engine is current;
  - the text answer is Markdown an agent or a person reads: a first
    line `the engine: <from> to <to>, N entries (M quiet)`, then per
    entry its range and PRs, its `changed` lines as a fenced `diff`, its
    `removed` and `added`, its lines, and each migration's `what`,
    `before`, `after` and call sites as `path:line:column: <text>`;
  - the op is in `Op`, `Op::ALL`, `Op::name`, `schemas()`, the op table
    in `crates/cli/src/contract.rs`'s module docs, the usage in
    `crates/cli/src/lib.rs`, the lines that list the ops needing no
    project (`crates/cli/src/lib.rs`'s module docs,
    `contract/cli.rs`'s, the usage), and `docs/contract.md`'s op table
    (needs: "a `Cargo.lock`, or `--from`; the network, or `--feed`");
  - **MCP:** its arm in `mcp::host::one_off` and in `mcp::description`;
    the tool's description says to call it before moving the engine's
    pin, and that its answer is the plan;
  - the schemas are written again (`crates/cli/schemas/changes.*.json`)
    and `crates/cli/schemas/versions.json` gets the fingerprint. A new
    op breaks no client: `contract::VERSION` stays (ADR-0150). `Op` is
    `#[non_exhaustive]` (2086), so the new variant is no break and needs
    no `Breaking:` line.
- **A new game's workflow,
  `templates/shuyi-default/base/.github/workflows/engine-changes.yml`:**
  - on `schedule` (weekly, Monday 06:17 UTC) and `workflow_dispatch`;
    `permissions: contents: read, issues: write`; the actions pinned by
    sha as `ci.yml` pins them, with its set-up steps;
  - runs `cargo shuyi changes` into a file, and with the game's own
    `GITHUB_TOKEN` (`gh`, `GH_TOKEN: ${{ github.token }}`):
    - not `ok`: edits the one open issue labelled `engine-changes`
      (`gh issue list --label engine-changes --state open`), or creates
      it with the label (`gh label create engine-changes --force`
      first), titled "Engine changes since this game's pin", its body
      the text answer (`--body-file`, never the text in the command
      line);
    - `ok`: closes that issue, if open, with a comment "the engine pin
      is current";
  - a comment at its top says what it does, that it reads only the
    engine's public site, and that deleting the file turns it off;
  - no input of an event reaches a `run:` line (zizmor's template
    injection rule); if CI's zizmor job audits `templates/`, the file
    passes it, else run `zizmor` on it once by hand and say so in the
    notes.
- **The templates' `AGENTS.md`** (`templates/shuyi-default/base/AGENTS.md`):
  a row in the table of tools, "`cargo shuyi changes` | what changed in
  the engine since this game's pin, and where the game's code uses
  what a migration renames", and one line saying the weekly workflow
  keeps an `engine-changes` issue.
- **A short ADR** (the next free number): `changes` is the first op
  that reads the network; it reads only the engine's own site, through
  `curl`, never in a game's run-time crates, and `--feed` makes it
  offline; a game's pin is found by its sha in the feed, a release by
  its tag; the template's workflow uses the game's own token, which is
  how the engine reaches games with none of ours (extends 3453's ADR and
  ADR-0022).

**Out (do not do):**
- The feed itself and its Pages steps: 3453.
- Moving the pin, editing the game's code or the `shuyi-upgrade` skill:
  2523, which runs this op first.
- Call sites for API lines (a guessed search for a removed item's
  name): a follow-up once real entries show which searches are worth
  it.
- A pull request opened by the workflow that moves the pin: an agent
  or a person does that from the issue, with 2523's skill.
- Changing Visions of Shuyi's watcher or `notify-dependents.yml`: the
  game's own ticket.
- An HTTP client crate in `shuyi-cli`: `curl` is enough, and a client
  would weigh on every game's tools.

**Estimated size:** about 1,400 lines (code 650, tests 550, docs and
the workflow 200).

**Split:** past 1,000 lines, split off the template workflow and the
templates' `AGENTS.md` line into a new ticket blocked by this one.

## Implementation steps

1. Read 3453's `crates/xtask/src/changes.rs` and its ADR,
   `crates/cli/src/contract/docs.rs` and `doctor.rs` (ops with no
   project), `contract/cli.rs`, `mcp/host.rs`, and the template's
   `ci.yml`.
2. Write the lock reader (a revision, a release, a checkout, two pins,
   none) and the pin lookup over a feed.
3. Write the op: the fetch behind `Fetch`, the entries' range, the
   walk, the answer, its text. Add it to `Op` and every list above.
   Regenerate the schemas and `versions.json`.
4. Wire MCP.
5. Write the template workflow and the `AGENTS.md` line; run a game
   from the template through CI's `templates` job.
6. Write the ADR and `docs/contract.md`'s row. Run the gates.

## Acceptance criteria

- [ ] Over a scratch game (made under `std::env::temp_dir()`, as
      `crates/cli/src/contract/import/tests.rs` makes one) whose
      `Cargo.lock` pins the engine by git at the middle commit of a fake
      feed's second entry, `changes --feed <file>` answers the second
      entry `partial` and the entries after it, not the first; the
      counts of quiet entries are right; a migration's `find` string in
      `src/lib.rs` is a call site with the right line and column, and
      matches under `target/`, `.git/` and a nested `[workspace]` are
      not.
- [ ] The same game pinned by release `0.2.0` starts after the commit
      tagged `v0.2.0`; pinned by path, it fails naming `--from`;
      `--from` a sha older than the feed answers every entry and the
      limit.
- [ ] A feed with `schema: 2` fails with the message to update; a fetch
      that fails is an error naming the URL, through the fake `Fetch`.
- [ ] `ok` is true for a pin at the newest commit, and false when a
      listed entry has only a `Behaviour:` line.
- [ ] 3453's snapshot feed deserializes into the op's types (a test).
- [ ] `shuyi mcp` lists a `changes` tool, and a call with `feed`
      answers as the CLI does (an MCP test).
- [ ] The schemas have their line in `versions.json`, and
      `contract::VERSION` didn't rise (`cargo xtask contract` passes).
- [ ] A game made by CI's `templates` job has
      `.github/workflows/engine-changes.yml`, and `cargo shuyi changes
      --feed <a fixture>` runs green in it.
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the lock reader's five cases; the pin lookup by sha, prefix and
  tag; the entries' range and `partial`; the walk's skips; the column
  after non-ASCII text; the text answer; `ok`.
- Integration: the scratch game through `changes` and through the CLI
  with `--json`; the MCP tool.
- CI: the `templates` job's fresh game has the workflow and runs the op
  over a fixture feed.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
