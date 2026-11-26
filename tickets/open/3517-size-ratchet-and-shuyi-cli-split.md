---
id: "3517"
title: File and crate sizes have a ratchet, and shuyi-cli's tool contract becomes a crate of its own
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3517 — File and crate sizes have a ratchet, and shuyi-cli's tool contract becomes a crate of its own

## Context

A blind-spot audit of the engine (2026-11-23) found two very large
crates and nothing that stops them growing. It was checked again on
2026-11-23 with `find crates/<crate> -name '*.rs' | xargs cat | wc -l`.

- `shuyi-ui` has 83,025 lines and `shuyi-cli` has 82,193. Tests take a
  large share. Without files under `tests/` or `benches/`, `tests.rs`
  and `*_tests.rs`, `shuyi-ui` has 49,837 lines and `shuyi-cli` 52,517.
  Next come `shuyi-assets` (46,341 in all) and `xtask` (35,842).
- 17 non-test files have more than 1,500 lines. The largest are
  `crates/xtask/src/tickets.rs` (2,849),
  `crates/platform/src/audio/player.rs` (2,765) and
  `crates/ui/src/frame/glyph_buffer.rs` (2,109).
- Nothing checks the size of a file or a crate. `clippy.toml` only allows
  `unwrap` and `expect` in tests.
- A game links the whole of `shuyi-cli`. The template's tool program
  calls `shuyi_cli::main_with`
  (`templates/shuyi-default/base/crates/game/examples/tools.rs:10`). Its
  tests use `shuyi_cli::contract::{check, run, playtest, session}` and
  `shuyi_cli::project`. The game does the same. A search with
  `gh search code --repo Zafnok/visions-of-shuyi 'shuyi_cli'`, at the
  game's pin `50472bbc`, finds `crates/tools/src/*.rs`. Those files use
  `shuyi_cli::{main_with, run, project, contract::{session, replay,
  playtest, autobalance}, frame_png, codec, skills, check_keys,
  app_info}`.
- A one-line edit anywhere in `shuyi-cli` rebuilds the whole crate. A
  change to the MCP server or the export rebuilds the contract under it
  too. Parallel agents also edit the same large files and conflict more
  often. The repository already budgets a game's rebuild (ADR-0049,
  ADR-0120, `tools/compile-budget.ron`).

**What the audit got wrong.** It said `shuyi-cli` mixes in "dev tooling
only this repository's agents use". That is not true:

- `worktrees` (seed and trim) is a command of a game's `cargo shuyi`
  (ticket 3403) as well as of `xtask`.
- `repos` is what `shuyi new` and `shuyi repos` run
  (`crates/cli/src/contract/new/questions.rs`).
- The MCP server is a game's own (ADR-0031).

So the split below keeps every command, and draws its line between the
contract and the commands built over it. The audit also gave
`player.rs` as 2,395 lines. It has 2,765.

**The modules' edges inside `shuyi-cli`** (`grep -rhoE 'crate::[a-z_]+'`
per module):

- `mcp` uses `contract` (36 times), `project`, `toy`, `frame_png`,
  `game`, `cargo` and `codec`. `contract` uses `mcp` in two places:
  - `contract/cli.rs:107` dispatches `shuyi mcp`;
  - `contract.rs:439` (`op_schemas`) imports the session tools' input
    and output types from `crate::mcp`.
- `export` (11,534 lines with its tests) uses `project`, `contract`,
  `web`, `cargo`, `game`, `notices` and `codec`. It is used by
  `contract/bake.rs`, `contract/export.rs` and `bench.rs`.
- `web.rs` (2,069) uses `export`, and `export` uses `web`.
- `repos` is used only by `contract/new/questions.rs`. Nothing inside the
  crate uses `worktrees`; `lib.rs:390` dispatches it, and so does `xtask`
  (`crates/xtask/src/main.rs:350`, `trim.rs`).

So the contract can move down into a crate of its own once three edges
move:
- the `mcp` dispatch;
- the session tools' types;
- the `bake` and `export` operations.
The MCP server, the export, the web build, `repos`, `worktrees`,
`reveal` and `bench` stay above it in `shuyi-cli`.

**Test helpers.** The audit counted 24 `fn temp_repo`, `fn temp_dir` and
`fn scratch` helpers (confirmed: 19 in `shuyi-cli`, 3 in `xtask`, 1 in
`shuyi-app`, 1 in `shuyi-embed`). This ticket merges the ones in the
crates it touches.

Related tickets, none of which plans a size check or this split
(`grep -rn 'check-sizes\|size-budget\|shuyi-contract' tickets/open`
finds nothing):

- 3520 puts the genre kits in `shuyi-ui` and `shuyi-core` behind
  default-on features. A build without them compiles less, but the
  crates' line counts stay the same, so their budgets here do not fall
  from it.
- 3415 makes the compile-time gate time an edit to a library and to the
  rules.
- 3414 adds `crates/mcp-launch` and edits `crates/cli/src/mcp`, which
  stays where it is here.
- **From 3510 (done):** the publish list is the workspace's engine
  crates (ADR-0152). `shuyi-contract` is published once it is a member:
  it takes `version.workspace = true` and `publish.workspace = true`, an
  entry in `[workspace.dependencies]` pinned `=` the workspace's version
  (`shuyi-cli` depends on it), and `cargo xtask check-publish` holds it.
  CI's `check-publish --index` fails until its name is on crates.io.

## Nick input

None.

## Scope

**In:**
- `cargo xtask check-sizes`, a ratchet over a new data file,
  `tools/size-budget.ron`:
  - It counts each `.rs` file's lines under `crates/`. It leaves out test
    files as `gates::test_only` does, and also leaves out `*_tests.rs`.
  - A non-test file over the default limit (1,500 lines) fails, unless
    the file is listed with its own limit. Each file over 1,500 today is
    listed at its count today. An exception that falls to 1,500 or below
    fails as stale, as the flaky ledger's stale entries do.
  - Each crate's non-test lines have a budget at today's count. A crate
    over its budget fails.
  - A file or crate more than 2 % under its number also fails, with the
    number to write. `--write` lowers every number to its count. It
    never raises one. A raise is a hand edit with a `//` reason on the
    line, as in `compile-budget.ron`.
  - It runs in CI's `tickets` job beside `check-paths`, and is a gate in
    `gates.rs` that `--fast` runs.
- A new crate, `shuyi-contract` (`crates/contract`), holds the tool
  contract and what it stands on. That means the operations except
  `export` and `bake`, plus `project`, `game`, `codec`, `sha256`,
  `pool`, `frame_png`, `font_atlas`, `glyph_tiles`, `catalog`,
  `template`, `skills`, `notices`, `app_info`, `check_keys`,
  `check_text`, `lang_status`, `private_assets` and `toy`. The session
  tools' types move into it too. The exact cut is the session's to
  make, with one rule: `shuyi-contract` never depends on `shuyi-cli`.
  Each module moves with `git mv`, so `pr-size` (`-M`) counts the edits,
  not the move.
- `shuyi-cli` keeps the `shuyi` binary, `main_with`, the command
  dispatch, `mcp`, `export`, `web`, `bench`, `repos`, `reveal`,
  `worktrees`, `watch` and `basic`, plus the `export` and `bake`
  operations. It re-exports each moved module at its old path
  (`pub use shuyi_contract::project;`, and a `contract` module that
  re-exports `shuyi_contract::contract::*` and adds `export` and
  `bake`). Every path the template and the game use stays the same.
- An ADR (`write-adr`) records the split: why the line sits there, the
  rule that nothing points up, and that a new command needing only the
  contract goes in `shuyi-contract`.
- `tools/layers.ron` gets a `shuyi-contract` line, and `shuyi-cli`'s
  line gains it (AGENTS.md rule 5).
- In the crates this ticket touches, one `#[cfg(test)]` temp-folder
  helper per crate replaces its copies.
- No new public type. The moved types keep their shape and their
  `#[non_exhaustive]` marks (ADR-0039). If CI's `api` job finds a change
  in a path, it gets a `Breaking:` line in the completion notes, and the
  game's PR that moves the pin follows (ADR-0038).

**Out (do not do):**
- Splitting `shuyi-ui` or `shuyi-core`. 3520 puts their genre kits
  behind features, which leaves their source sizes as they are.
- Making today's large files smaller. The ratchet records them as they
  are, and each one that shrinks later lowers its number.
- Moving the MCP server, the export or `worktrees` into crates of their
  own. They stay in `shuyi-cli` above the contract. A later ticket may
  split one further when the ratchet or a rebuild time asks for it.
- A shared test-support crate across `xtask`, `shuyi-app` and
  `shuyi-embed`. Five helpers outside `shuyi-cli` do not need a crate.
- Claiming the new crate's name on crates.io: Nick's (a 0.0.0
  placeholder, asked with the `ask-nick` skill before the PR merges,
  since CI's `check-publish --index` fails until it is held).
- Counting lines in Markdown, data or snapshots. Only `.rs` files count.
- Byte budgets on the Markdown files agents read whole (AGENTS.md, the
  docs map, the work-ticket, run-gates, write-ticket and document
  skills): 3513 (`tools/context-budget.ron`, `cargo xtask
  check-context`).

**Either order with 2005:** if 2005 is done, give `shuyi-contract` its
crates.io metadata and a place in the release workflow's publish order
before `shuyi-cli`; if not, nothing to do here (2005 publishes
`shuyi-contract` with the others, before `shuyi-cli`).

**Estimated size:** about 1,700 lines (code 850, tests 450, docs 400).
The moved files count only their edited lines.

**Split:** past 900 lines, split off steps 6–11 (the `shuyi-contract`
split) into a new ticket, and land the ratchet alone.

## Implementation steps

1. Add `crates/xtask/src/sizes.rs` with `run(root, write: bool)`. List
   the `.rs` files under `crates/` the way `paths.rs` lists tracked
   files (`git ls-files`). Leave out a file when `gates::test_only` is
   true or its name ends in `_tests.rs`. Make `test_only` `pub(crate)` if
   it is not already. Count lines. Group them by crate with `cargo
   metadata` (a package's `manifest_path` folder).
2. Add `tools/size-budget.ron`: `Budget(file_lines: 1500, files: {...},
   crates: {...})`, with a header comment naming this ticket and
   `sizes.rs`. Fill `files` with each non-test file over 1,500 lines
   today (17 on 2026-11-23) and `crates` with each crate's non-test
   lines, both from the first run of `--write`.
3. Write the failures. Each one names the file or the crate, its count,
   its number and what to do. Over: make it smaller, or raise the number
   with a reason. Stale or more than 2 % under: the number to write, or
   `cargo xtask check-sizes --write`. A `files` entry for a file that no
   longer exists also fails. `--write` only lowers numbers and removes
   stale entries.
4. Wire `check-sizes` into `crates/xtask/src/main.rs` (the usage text
   and the match, beside `check-paths`). Add a gate to `gates.rs` with
   `Fast::Yes` (or the `IfChanged` filter on `.rs` that fits) and a
   hint. Add a step `cargo xtask check-sizes` to `ci.yml`'s `tickets`
   job after `check-paths`. Add a line to the run-gates skill's list
   beside `check-paths`.
5. Document the ratchet where the docs map puts `xtask` checks
   (`document` skill).
6. Map the edges again with the grep in *Context* and pick the cut. The
   session tools' types used by `op_schemas` (`EventsOutput`,
   `FramePngInput`, `FramePngOutput`, `NoInput`, `PerfInput`,
   `PerfOutput`, `SceneInput`, `StartInput`, `StartOutput`, `StepInput`,
   `StopOutput`) move into `contract::session`, and `mcp` imports them
   from there. Or `op_schemas` moves up to `shuyi-cli`. Choose the one
   that keeps `shuyi op-schemas` (or whatever lists the schemas)
   unchanged.
7. Create `crates/contract` (`shuyi-contract`) with the workspace's
   package fields and `[lints] workspace = true`. Move its dependencies
   over from `crates/cli/Cargo.toml` with their comments, and its
   features (`toy`, `jis`). `git mv` the modules. Rewrite `crate::` paths
   that now cross the line.
8. Move the `mcp` dispatch out of `contract/cli.rs` into `shuyi-cli`'s
   `main_with`, through a hook or by matching `mcp` before the contract's
   own commands. Move `contract/bake.rs` and `contract/export.rs` into
   `shuyi-cli`'s `contract` module.
9. In `crates/cli/src/lib.rs`, re-export every moved public module at
   its old path. Build the template's game (`shuyi new` in the gates) and
   the toy example. Every `shuyi_cli::` path in *Context* must compile
   unchanged.
10. Update `tools/layers.ron`, the `xtask` dependency (`shuyi_cli::worktrees`
    stays in `shuyi-cli`), `shuyi-embed`'s `build.rs` if it writes into
    `shuyi-cli`'s folder, and `crates/cli/Cargo.toml`'s description.
    Replace the duplicated `temp_dir` and `temp_repo` helpers in
    `shuyi-cli` and `shuyi-contract` with one helper per crate.
11. Measure how long `shuyi-cli` takes to rebuild after a one-line edit
    in `crates/cli/src/mcp.rs`, before and after (`cargo build -p
    shuyi-cli --timings`, median of three). Put both numbers in the
    completion notes. Run `cargo xtask check-sizes --write` to record the
    two crates' new budgets. Write the ADR.

## Acceptance criteria

- [ ] `cargo xtask check-sizes` passes on the branch, and
      `tools/size-budget.ron` lists every non-test `.rs` file over 1,500
      lines and every crate.
- [ ] Adding 20 lines to a listed file, or 1,600 lines to a new non-test
      file, fails it with the file, its count and its number (a test in
      `sizes.rs`).
- [ ] A file at exactly 1,500 lines with no entry passes. A listed file
      cut to 1,500 lines fails as stale. A crate 3 % under its budget
      fails with the number to write. One 1 % under passes.
- [ ] `--write` lowers numbers and removes stale entries, and never
      raises a number (a test).
- [ ] A test file (`tests/`, `tests.rs`, `*_tests.rs`, `benches/`) of any
      length passes.
- [ ] CI's `tickets` job and `cargo xtask gates --fast` run the check.
- [ ] `shuyi-contract` exists, `cargo tree -p shuyi-contract` has no
      `shuyi-cli`, and `cargo xtask check-layers` passes with its new line.
- [ ] The template's game, the toy example and the game's paths listed in
      *Context* compile with no edit, or each break has a `Breaking:` line.
- [ ] `shuyi --help`, `shuyi mcp`, `shuyi export`, `shuyi bake` and
      `cargo xtask trim-targets` behave as before (their existing tests
      pass unchanged).
- [ ] The completion notes give the rebuild times before and after.
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask/src/sizes/tests.rs`): counting over a fixture tree
  with test and non-test files; each failure (over, stale, more than 2 %
  under, missing file); `--write` only lowering; the crate grouping.
- Integration: the existing tests of `shuyi-cli`, now split across the
  two crates, pass with only their import paths changed. The template
  game's `tests/it` builds and passes in the gates.
- A layers test: `check-layers` fails a `shuyi-contract` that depends on
  `shuyi-cli` (the existing layers check, with the new line).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
