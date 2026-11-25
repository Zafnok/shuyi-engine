---
id: "3429"
title: "Every flag of an operation's CLI is a field of its input or is listed as the CLI's alone, checked by a test"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] The test covers every op in `Op::ALL`, and fails on a flag that
      is in neither the input nor the CLI-only list.
- [ ] Every CLI-only flag has a reason beside it.
- [ ] `docs/contract.md` lists the CLI-only flags or links where they
      are.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The parity test; a unit test of the name mapping.

## Completion notes

*(Filled in by the session that completes the ticket.)*
