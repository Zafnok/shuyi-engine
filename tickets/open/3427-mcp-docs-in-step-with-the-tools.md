---
id: "3427"
title: "The MCP server's docs and descriptions say what its tools take, and tests keep the docs' lists of operations in step"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3427 — The MCP server's docs and descriptions say what its tools take, and tests keep the docs' lists of operations in step

## Context

Found by the docs audit of 2026-11-25. The tool list itself is right:
`mcp::tools` builds it from `Op::ALL` and `Op::SESSION`, and
`crates/cli/src/mcp/tests.rs` checks each op is a tool once, with the
right `project`. What drifted is the words around it, and two lists no
test reads.

- **Who takes `project`.** `docs/contract.md`, *The MCP server* (about
  lines 509–512): "The tools that run the game, and `lint`, take
  `project` … `lint` runs in the server". `modules` and `add` take it
  too and also run in the server (`takes_project` in
  `crates/cli/src/mcp.rs`, its test in `mcp/tests.rs`; `one_off` in
  `mcp/host.rs`). The server's instructions (`INSTRUCTIONS`,
  `crates/cli/src/mcp/server.rs`, about line 35) and `mcp.rs`'s module
  doc (about line 7) say only "the tools that run the game take
  `project`".
- **Stale descriptions** in `mcp::description`: `export` (`EXPORT`,
  `mcp.rs` about line 381) lists `windows`, `linux`, `macos`, `web`, not
  `playdate`, and names neither `edition` nor `with`; `doctor` doesn't
  name its optional `gh` and `ssh-keygen` checks. (`new`'s is 3426's.)
- **What isn't a tool.** contract.md says "Each operation is a tool"
  (about line 504), yet `schema` is a row of *The operations* (about
  line 36) and no tool, and only `repos`, `reveal`, `trim-targets` and
  `seed-target` are said to be the CLI's alone (about lines 72–76).
  `bench`, `web`, `preview`, `frame-png` (one-off, to a file),
  `font-atlas`, `glyph-tiles`, `app-info`, `lang-status`,
  `check-keys`/`check-text` (covered by `lint`) and `private-assets` are
  commands an agent may look for as tools. The template's `AGENTS.md`
  (`templates/shuyi-default/base/AGENTS.md`, about line 96) says of
  `mcp install`: "each command is a tool".
- **The book's chapter 10** (`site/content/learn/book/tools-for-agents.md`,
  about lines 54–56) lists the operations without `export`. Its session
  table (about lines 125–137) is right, but its `frame_png` row doesn't
  name `every` and `count`.
- **No test** reads the book's list of operations or either session
  table (contract.md's and the book's) against `Op::ALL` and
  `Op::SESSION`. And `Op::ALL` is a hand-written slice
  (`crates/cli/src/contract.rs`, about line 137): nothing checks that a
  new `Op` variant is in `ALL` or `SESSION`, so a new op could miss the
  MCP server with every test passing. `tests/it/docs.rs` (about lines
  82–118) already checks contract.md's table and the README's row.

## Nick input

None.

## Scope

**In:**

- contract.md *The MCP server*: every tool that takes `project`, and
  which run in the server; a short list of the commands that are the
  CLI's alone, each with why in a few words (to a file, a server of its
  own, covered by `lint`, …), and `schema` among them or out of *The
  operations*, whichever the docs test allows without weakening it.
- The server's `INSTRUCTIONS` and `mcp.rs`'s module doc: who takes
  `project`, in the same words.
- `EXPORT` and `doctor`'s descriptions.
- The template's `AGENTS.md` line: "each operation is a tool".
- Chapter 10: `export` in the list; `every` and `count` in `frame_png`'s
  row.
- **Tests** (in `crates/cli/tests/it/docs.rs` beside the others, or the
  crate's own):
  - every `Op` is in `Op::ALL` or `Op::SESSION` (a `match` with no
    wildcard arm, so a new variant fails to compile until it is placed);
  - chapter 10's list of operations equals `Op::ALL`'s names, as
    contract.md's table is checked;
  - both session tables list exactly `Op::SESSION`'s tool names;
  - each tool's description names every required property of its input
    schema (the optional ones are left to judgment).

**Out (do not do):**

- the `new` tool's description and its `questions` (3426);
- making any CLI-only command a tool;
- checking that every CLI flag reaches the op's input (3429).

## Implementation steps

1. Read `crates/cli/src/mcp.rs` (`tools`, `takes_project`,
   `description`), `mcp/server.rs`, `mcp/host.rs`'s `one_off`,
   `contract.rs`'s `Op`, and `tests/it/docs.rs`.
2. Write the tests first and see them fail on today's docs.
3. Fix contract.md, the instructions, the descriptions, the template's
   line and chapter 10.
4. Run the gates.

## Acceptance criteria

- [ ] contract.md, the server's instructions and `mcp.rs`'s doc name
      every tool that takes `project`.
- [ ] contract.md lists the CLI-only commands with why, and says what
      `schema` is.
- [ ] `export`'s description names `playdate`, `edition` and `with`;
      `doctor`'s names its optional checks.
- [ ] Chapter 10 lists `export`.
- [ ] The four tests exist and pass, and each fails when its list loses
      a name (shown in the completion notes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The four tests above.

## Completion notes

*(Filled in by the session that completes the ticket.)*
