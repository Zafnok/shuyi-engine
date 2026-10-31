---
id: "2035"
title: "cargo xtask golden --breaks: a changed or removed golden value fails CI's determinism job without a Breaking: line naming its table"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2116"]
nick_input: none
completed:
---

# 2035 — `cargo xtask golden --breaks`: a moved golden value needs a `Breaking:` line

## Context

2116 added golden tables (`shuyi_assets::golden`): values a build gives
(a record's state hashes, a decoded asset's `state_hash`), pinned in
`**/tests/golden/*.ron` and checked on x64, ARM64 and wasm32 by CI's
`test` and `determinism` jobs. It added `cargo xtask golden --bless`,
which rewrites the tables a run gives (`crates/xtask/src/golden.rs`).

2116 also planned `--breaks`: the CI check that a PR which changes or
removes a golden value names it in a `Breaking:` line. Its **Size** rule
moved it here: 2116's diff was past 450 lines before `--bless` (the
module's docs and tests, the two tables' tests and the xtask).

The rule itself is written. 2005's version policy counts a changed or
removed golden value as a breaking change: in an engine crate's table,
or in an example's table when that example's own `src/`, `files/`,
`tests/records/` or `Cargo.toml` didn't change in the same PR. Today
nothing checks it: a PR can bless a moved value and say nothing, and the
game finds it at its next pin bump. The `api` job (2010, ADR-0038)
checks the same kind of rule for the public API; this ticket reuses its
pieces.

## Nick input

None.

## Scope

**In:**

- **`cargo xtask golden --breaks --base <rev> [--pr-branch <branch>]`**
  in `crates/xtask/src/golden.rs`, beside `--bless`, with its lines in
  `main.rs`'s `USAGE` and `match`:
  - reads `git diff -U0 --no-renames <base> -- ':(glob)**/tests/golden/*.ron'`
    through 2010's `Runner` (`crates/xtask/src/api.rs`), which the tests
    fake. A removed row line (a `-` line holding a `"name": ` entry) is a
    changed or removed value; added rows are not;
  - then:
    1. none: pass;
    2. a table of an example (`examples/<name>/tests/golden/`) whose
       `src/`, `files/`, `tests/records/` or `Cargo.toml` changed in the
       same diff (`git diff --name-only <base>`): its rows moved with its
       own rules, not with the engine. Pass, and say so;
    3. otherwise, when `--pr-branch` names a ticket
       (`ticket_id_from_branch`, `pub(crate)` in
       `crates/xtask/src/tickets.rs`), that ticket's file in
       `tickets/done/` must have, under `## Completion notes`, a
       `Breaking:` line (`breaking_lines`, same file) that contains each
       changed table's file name. If not, it fails, printing the rows and
       an example line: `Breaking: shuyi-assets: formats.ron "lights:
       lights": <what a game sees>`;
    4. a branch that names no ticket (`sonar/…`, `dependabot/…`): fail.
       A PR without a ticket doesn't move a golden value; the message
       says to write one.

    Without `--pr-branch` (a local run) it prints what it found and
    passes. Reuse `api.rs`'s `done_ticket` (make it `pub(crate)`)
    rather than a copy.
- **The CI step**, in the `determinism` job of
  `.github/workflows/ci.yml`:
  - the checkout gets `fetch-depth: 2`;
  - the job's `env:` gets `HEAD_REF: ${{ github.head_ref }}` and
    `EVENT_NAME: ${{ github.event_name }}`, never `${{ }}` inside `run:`,
    as the `tickets` job does;
  - a last step, on a pull request only (`if [ "$EVENT_NAME" =
    "pull_request" ]`, as the `tickets` job does): `cargo xtask golden
    --breaks --base HEAD^1 --pr-branch "$HEAD_REF"`. As in
    `changes.yml`, `HEAD` is the PR's merge commit and `HEAD^1` the
    base's tip;
  - the job's comment says so.
- **Docs:** the module docs of `shuyi_assets::golden`
  (`crates/assets/src/golden.rs`, *A moved value is a break*) and the
  `run-gates` skill's section *Golden tables (CI's `determinism` job)*
  say CI now checks the `Breaking:` line; `main.rs`'s `USAGE` line for
  `golden`.

**Out (do not do):**

- The golden tables, `--bless` and the `determinism` job's test steps
  (2116, done).
- The rule's wording in the version policy (2005 holds it).
- The public API's `Breaking:` rule and the `api` job (2010).

## Implementation steps

1. Read `crates/xtask/src/golden.rs` (`--bless`, its `Runner` use and
   tests), `crates/xtask/src/api.rs` (`rule`, `done_ticket`,
   `changed_files`) and `tickets.rs`'s `breaking_lines` and
   `ticket_id_from_branch`.
2. Write the diff reader (rows changed per table) and the rule as pure
   functions over the diff's text, the changed files and a
   `done_ticket` closure, as `api::rule` takes one; then `breaks(root,
   options, runner)` that runs `git` and calls them.
3. Wire `--breaks` into `main.rs`, with its options parsed as
   `api::Options::parse` parses its own.
4. Add the CI step and the job's `env:` and `fetch-depth`.
5. Prove it on a demo PR, closed unmerged, from a branch
   `t2035-breaks-demo` cut from this ticket's branch once this ticket is
   in `done/` there (`work-ticket` §5), so the ticket lint passes on it.
   Its base is this ticket's branch, not `main`. On it:
   - change the input of one row in
     `crates/assets/src/format/golden_tests.rs` and bless: `--breaks`
     fails, asking for a `Breaking:` line;
   - add the line to this ticket's completion notes on that branch:
     `--breaks` passes.
   Link both runs from this PR's body.
6. Update the docs (Scope), run the gates.

## Acceptance criteria

- [ ] `cargo xtask golden --breaks` fails on a PR that changes or
      removes a golden value without a `Breaking:` line naming its
      table, and passes with one, or when only rows were added (unit
      tests and the demo PR).
- [ ] An example's table whose example's own files changed in the same
      diff passes without a line (unit test).
- [ ] A branch that names no ticket fails on a moved value (unit test).
- [ ] CI's `determinism` job runs it on every pull request.
- [ ] The demo PR's two runs are linked from this PR.
- [ ] The book needs no change (the completion notes have a `Book:` line
      if the public API changed; it shouldn't: this is the xtask).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, `crates/xtask/src/golden.rs`, on fake diffs: only added rows; a
  changed row with and without the line; a removed row; an example whose
  own `src/` changed; a branch with no ticket; no `--pr-branch` (prints
  and passes).
- CI: the step itself, shown by the demo PR.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
