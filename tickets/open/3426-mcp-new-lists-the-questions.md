---
id: "3426"
title: "The MCP new tool lists the wizard's questions, and its description says what it takes"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3426 — The MCP `new` tool lists the wizard's questions, and its description says what it takes

## Context

Found by the docs audit of 2026-11-25.

3424 made the wizard's questions data: `shuyi new <dir> --questions
--json` answers them for the answers given so far, each with who asks
it, its answers with their status and tickets, its default, its flag and
why it is `skipped` (schema `new.questions.json`). That is how an agent
is meant to learn what to ask its person before making a game. But it is
the CLI's alone:

- `--questions` is a field of `Flags`
  (`crates/cli/src/contract/new/questions.rs`, about line 75), not of
  `NewInput` (`crates/cli/src/contract/new.rs`, about lines 96–131);
- the MCP server's `new` calls `new::new(cwd, &input)`
  (`crates/cli/src/mcp/host.rs`, about line 134) and never reaches
  `questions::listed`.

`docs/contract.md`'s `new` row (about line 16) presents `--questions` as
part of the operation and doesn't say MCP lacks it. Everything else the
wizard takes does reach MCP (its flags through `set`, `answers`, `engine`,
`id`, `private_url`, `force`), and `command` and `created_with` come back.

The `new` tool's description (`mcp::description`, `crates/cli/src/mcp.rs`,
about line 405) also says `dir` is "from the server's working folder",
while `Server::absolute` (`crates/cli/src/mcp/server.rs`, about line 526)
resolves it from the session's folder, the client's first root. It
doesn't name `name`, `id`, `engine`, `private_url` or the `command` the
answer gives. And `absolute` makes `dir` absolute but not `answers`, so a
relative answers file in a call the server forwards to a fresh one may be
read from that server's folder.

## Nick input

None.

## Scope

**In:**

- `NewInput` gains `questions: bool` (`#[serde(default)]`; the type is
  `#[non_exhaustive]`, so the field is additive, ADR-0039). With it, `new`
  makes nothing and answers the questions, through the same
  `questions::listed` the CLI's `--questions` uses: the CLI's flag sets
  the field, and one path serves both. The MCP tool's answer for it is
  the `new.questions.json` shape. Read how the contract versions an
  input that answers in another shape (`docs/contract.md`, *The schemas*)
  and do what it says: the input schema is regenerated, and
  `contract::VERSION` and `versions.json` move if the rule asks.
- `answers` is made absolute from the session's folder, as `dir` is, in
  `Server::absolute`.
- **Either order with 3429:** if 3429 is done, take `--questions` off
  its list of the CLI's own flags, so its test checks the field; if not,
  nothing to do (3429's test finds the field in the input).
- The `new` tool's description: `dir` from the session's folder; the
  fields it takes, in one line; `questions` to list the questions first;
  that the answer gives `command` and keeps the answers in `shuyi.ron`.
- Docs: `docs/contract.md`'s `new` row (the field, and that MCP has it);
  the book's chapter 10 (`site/content/learn/book/tools-for-agents.md`):
  one passage on asking the questions first, `shuyi new <dir> --questions
  --json` or the MCP `new` with `questions`, with its answer's shape in a
  line; the `shuyi-new-game` skill (`skills/shuyi-new-game/SKILL.md`), if
  it tells an agent to list the questions, names the MCP route too.

**Out (do not do):**

- the wizard's prompts in a terminal (3423);
- the repository flags (`--github`, `--owner` and the rest), which the
  contract leaves out on purpose;
- the other tools' descriptions and the MCP docs' other drift (3427).

## Implementation steps

1. Read `contract/new.rs`, `new/questions.rs`, `wizard.rs`'s `listed`,
   `mcp/host.rs`'s `new` arm, `mcp/server.rs`'s `absolute`, and
   `docs/contract.md`, *The schemas*.
2. Move `questions` into `NewInput` and route both callers through it.
3. Make `answers` absolute; test it on a forwarded call (see the
   forwarding tests in `crates/cli/src/mcp/tests.rs`).
4. The description; the schemas (`cargo xtask` regenerates them, as
   `docs/contract.md` says).
5. The docs and the skill.

## Acceptance criteria

- [ ] The MCP `new` tool with `questions: true` answers the questions
      and writes nothing, and its answer equals the CLI's `--questions
      --json` for the same answers.
- [ ] A relative `answers` in an MCP `new` call is read from the
      session's folder, forwarded or not.
- [ ] The `new` description says where `dir` is from and names
      `questions`.
- [ ] `docs/contract.md`, chapter 10 and the skill say it.
- [ ] `cargo xtask check-skills` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `NewInput` with `questions` round-trips; the MCP `new` tool with
  `questions` writes no file.
- Integration: the MCP answer equals the CLI's for the same `set`.
- The forwarded `answers` path.

## Completion notes

*(Filled in by the session that completes the ticket.)*
