---
id: "3429"
title: "Every flag of an operation's CLI is a field of its input or is listed as the CLI's alone, checked by a test"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-25
---

# 3429 — Every flag of an operation's CLI is a field of its input or is listed as the CLI's alone, checked by a test

## Context

The docs audit of 2026-11-25 found that `shuyi new --questions` (3424)
never reached the MCP server: the flag was put in the CLI's `Flags`
(`crates/cli/src/contract/new/questions.rs`), not in `NewInput`, and
the MCP tool takes only the input (3426 fixes that one). The MCP
server's tools are made from each op's input schema (`mcp::tools`), so
any flag that lives outside the input is the CLI's alone, whether or not
someone meant it to be. 3427's tests keep the tool list and the docs'
lists of operations in step. Nothing compares an op's flags with its
input. 3427 left that check out, so it is this ticket's.

Some flags are the CLI's alone on purpose: output ones (`--json`,
`--report`, `--brief`) and `new`'s repository ones (`--github`,
`--owner`, `--visibility`, …), which `docs/contract.md`'s `new` row says
MCP refuses.

## Nick input

None.

## Scope

**In:**

- **One list of each op's flags.** Find where each op's CLI arguments
  are parsed (`crates/cli/src/contract/cli.rs` and each op's `parse`,
  e.g. `new::parse`) and its usage text. Make the flags an op accepts
  readable by a test: a `const` slice per op that the parser and the
  usage text both use. If the parser is too spread out for that, a test
  can drive the parser with each flag the usage text names. The session
  picks one and says why in the completion notes.
- **A list of the CLI's own flags**, per op, each with why in a few words
  (output only; a terminal's repository step; …), in one place in
  `crates/cli/src/contract/`, and in `docs/contract.md` where it lists
  the CLI-only parts.
- **The test:** for each op in `Op::ALL`, every flag is a property of its
  input schema (by name, with `-` read as `_`, or by an explicit mapping
  where the names differ, such as `--engine-rev` to `engine`), or is in
  the CLI's own list. A new flag in neither fails, with a message that
  says to add the field to the input or the flag to the list with why.
- If the test finds a flag that should reach MCP and doesn't (other than
  `--questions`, which is 3426's), write a ticket for it, or fix it here
  if it takes only a few lines.

**Out (do not do):**

- `new --questions` itself (3426);
- the session tools (`start`, `step`, …), which have no CLI flags;
- the commands that aren't operations (`bench`, `web`, …: 3427 lists
  them).

## Implementation steps

1. Read `contract/cli.rs`, each op's argument parser and usage text,
   and `mcp::tools`.
2. Pick how the flags are listed, and do it.
3. Write the CLI-only list and the test, and see the test fail on a
   flag you add on purpose. **Either order with 3426:** if 3426 is done,
   `questions` is a field of `NewInput` and the test passes for it; if
   not, list `--questions` among the CLI's own flags with "3426 moves it
   into the input", and 3426 takes it off the list.
4. The docs line.

## Acceptance criteria

- [x] The test covers every op in `Op::ALL`, and fails on a flag that
      is in neither the input nor the CLI-only list.
- [x] Every CLI-only flag has a reason beside it.
- [x] `docs/contract.md` lists the CLI-only flags or links where they
      are.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- The parity test; a unit test of the name mapping.

## Completion notes

- **How the flags are listed: the usage text, and a test that drives
  the parser.** An op's flags are the ones its usage text names
  (`contract::flags::usage`, which calls each op's `usage`). The test
  checks that each op's parser takes each of them: alone or with a
  value, it doesn't answer what it answers a flag it doesn't know. A
  `const` slice that the parser also reads would have meant rebuilding
  a dozen match-arm parsers, and some flags aren't fixed: `run`,
  `playtest` and `autobalance` take the game's start options, and `new`
  takes the catalog's concerns. The usage is what people read, so it
  is the one list. The cost: a flag the parser takes and the usage
  doesn't name isn't checked. Two were like that, and this PR adds
  them to the usage: `run`/`scene --drop` and `attach --crank`.
- `crates/cli/src/contract/flags.rs` (compiled for tests only) holds:
  - `CLI_ONLY` (op, flag, why);
  - `RENAMED` (op, flag, field): `--entrant` to `entrants`, the steps
    to `script`/`steps`, `new`'s wizard flags and `--examples` to
    `set`, `--engine-rev`/`--engine-path` to `engine`, `attach --for`
    to `follow_for`, `--any-stamp` to `session`;
  - `BEFORE_THE_PARSER`: `run --watch`, which `lib.rs` reads before
    `run`'s parser sees it;
  - its tests: the parity test over `Op::ALL`, the parser test, a
    test that the tables name only real flags and fields, the name
    mapping's unit test and the usage scraper's unit test.

  Checked by hand: a `bake --made-up` added to the parser and the usage
  failed the parity test, with its message.
- The test found no flag that should reach MCP and doesn't, other than
  `--questions` (3426's). It is in `CLI_ONLY` as "3426 moves it into
  the input", and 3426's clause now says to take it off the list.
- `doctor`, `replay` and `import` had their usage inline in `cli.rs`.
  Each now has a `usage` function, so every op's usage is in one.
- Docs: `docs/contract.md`, *The MCP server*, lists the CLI's own flags
  and links `flags.rs`.
- The ticket was written today; nothing in it had gone stale.

No break.

Book: none, three `usage` functions are added beside the ones the other ops already have. The book never names an op's usage function.

Behaviour: shuyi-cli: the usage of `run` and `scene` names `--drop`, and the usage of `attach` names `--crank`. Searched the game for `let that many seconds pass` and `usage(`: no hit.
