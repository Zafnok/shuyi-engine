---
id: "2086"
title: "Before the first release, shuyi-cli's contract types that open tickets grow move under ADR-0039, Op::ALL becomes a slice, and check takes what it reads in its input"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2086 — shuyi-cli's contract types that open tickets grow move under ADR-0039 before the first release

## Context

The last of five tickets that move, before 2005, every exhaustive public
type an open ticket plans to grow (2082 has the reasons and Nick's words
of 2026-11-14). A game depends on `shuyi-cli` for its tool program (the
template's `tools` feature, `templates/shuyi-default/base/crates/game/Cargo.toml:28,44`),
so its public API is a game's too. 2025 moved the contract's inputs onto
constructors. The audit found that the operations and the answers are
still exhaustive, and that about half the breaking tickets in the
tools' block come from `Op` alone.

| Type or item | Where | Grown by |
| ------------ | ----- | -------- |
| `Op` and `Op::ALL: [Op; 16]` | `crates/cli/src/contract.rs:81,133` | 2225, 2505, 2515, 2517, 2520, 2523, 2527, 2552, 2232, 2529 |
| `CheckOutput` | `crates/cli/src/contract/check.rs:56` | 2516, 2517 |
| `check(game, input)` | `check.rs:67` | 2223 and 2517 each add an argument |
| `ReplayStep`, `ReplayOutput` | `crates/cli/src/contract/replay.rs:89,116` | 2520 |
| `AddOutput` | `crates/cli/src/contract/add.rs:59` | 2526 |
| `ModuleEntry` (the `modules` answer's) | `crates/cli/src/contract/modules.rs:44` | 2526 |
| `LintCheck` | `crates/cli/src/contract/lint.rs:82` | 2579 |
| `Concern`, `Status` (the catalog's) | `crates/cli/src/catalog.rs:61,94` | 2502; seven module tickets add `Status::Building` |
| `Part` (`lang-status`'s) | `crates/cli/src/lang_status.rs:28` | 2351 |
| `Event` (the session's) | `crates/cli/src/contract/session.rs:165` | 2356 |
| `font_atlas::run` | `crates/cli/src/font_atlas.rs:666` | 2224 adds a parameter |
| `Project::rules: Option<String>` | `crates/cli/src/project.rs:100` | 2771 wants one or a list |

2552 says it breaks no API, but it adds an `Op`. This ticket's PR
corrects that line.

## Nick input

None.

## Scope

**In:**
- `Op` `#[non_exhaustive]`, and `Op::ALL` as `&'static [Op]`.
- The answers and enums in the table `#[non_exhaustive]` (ADR-0039
  kinds 3 and 6), and their call sites in the CLI, the MCP server and
  the tests.
- `check` reads the project root and the project from `CheckInput`
  (already `#[non_exhaustive]`), so later inputs are fields, not
  arguments.
- `font_atlas::run` takes an options struct of kind 4.
- `Project::rules` becomes a list (`Vec<String>`) that still reads a
  lone string from `shuyi.ron` (a serde `untagged` of one or many), so
  every `shuyi.ron` keeps loading.
- The listed tickets' planned `Breaking:` lines for these items become
  additive, and 2552's line is corrected.
- A `Breaking:` line in the completion notes; the template's tools
  example and the game's tools program are the callers to search
  (`gh search code --repo Zafnok/visions-of-shuyi 'Op::'`).

**Out (do not do):**
- The contract's JSON and `contract::VERSION`: what the version
  promises is 2088's.
- Any operation or field the listed tickets add.
- `check_keys::run`, `check_text::run_names` and `codec::decode_png`
  (2543 and 2565 change them).

**Estimated size:** about 700 lines (code 350, tests 250, docs 100).

## Implementation steps

1. The attributes; `Op::ALL` as a slice, and its users (the MCP tool
   list, the usage text, the schemas' writer).
2. `CheckInput` gains the root and the project; `check`'s callers move.
3. `font_atlas::run`'s options struct.
4. `Project::rules` as a list read from one or many; the lint's fence
   reads each.
5. Build the workspace, the templates and the tools examples; fix each
   literal and exhaustive match.
6. Edit the listed tickets' planned `Breaking:` lines, and 2552's.

## Acceptance criteria

- [ ] Every item in the table is changed as above, and the contract's
      schemas and JSON answers are byte for byte the same (the schema
      tests pass with no file regenerated).
- [ ] A `shuyi.ron` with `rules: "rules"` and one with a list both load.
- [ ] `cargo xtask api` lists each change, and the `Breaking:` line names
      each item.
- [ ] No open ticket still plans a `Breaking:` line for one of these
      items.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Project::rules` from a string and from a list.
- The existing contract, schema and MCP tests pass unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
