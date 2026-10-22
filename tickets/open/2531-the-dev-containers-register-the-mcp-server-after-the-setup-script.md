---
id: "2531"
title: "The dev containers register the MCP server after the setup script, so an agent in a container or a Codespace finds it"
type: feature
milestone: MVP
model: sonnet-5
effort: low
status: todo
blocked_by: ["2513", "1133"]
nick_input: none
completed:
---

# 2531 — The dev containers register the MCP server

## Context

1133 made `shuyi mcp` (the tool contract as an MCP server, with a
session that keeps a headless game running) and `shuyi mcp install`,
which writes the server's entry into each agent's project file
(Claude Code's `.mcp.json` always; Cursor's, VS Code's and Gemini CLI's
where it finds them) after showing a diff and asking, or without asking
with `--yes` ([ADR-0031](../../docs/adr/0031-the-mcp-server-hand-written-json-rpc-the-game-in-its-tool-program-and-session-tools-as-ops.md),
`docs/contract.md`, *The MCP server*).

2513 adds `.devcontainer/devcontainer.json` to the engine's root and to
every template, whose `postCreateCommand` runs `bash
scripts/setup-linux.sh`. 1133's scope said the dev containers should
register the server once both exist, and 2513's **Out** leaves it to
1133. 1133 landed first, so this ticket does it once 2513 is done.

A container has no installed `shuyi` (2513 leaves `cargo install` to
2005), so it runs the CLI as 2513 runs `doctor`: `cargo run -q -p
shuyi-cli --` in the engine, `cargo shuyi` in a template (the alias runs
the game's own tool program, which serves `mcp` too).

## Nick input

None.

## Scope

**In:**
- The engine's `.devcontainer/devcontainer.json`: `postCreateCommand`
  runs the setup script, then `cargo run -q -p shuyi-cli -- mcp install
  --yes --command "cargo run -q -p shuyi-cli --"`. The setup script's
  `cargo` may not be on the command's `PATH` yet: source
  `"$HOME/.cargo/env"` between them, as 2513's README text says.
- Each template's `.devcontainer/devcontainer.json`
  (`templates/*/.devcontainer/`): the same, with `cargo shuyi mcp install
  --yes` (the project's `command` is `cargo shuyi`, so no `--command`).
- The README's dev container paragraph (2513's) and each template's
  `AGENTS.md` *The tools* table say, in one line each, that the
  container registers the server.

**Out (do not do):**
- installing `shuyi` in the container (2005);
- a network transport for the server (1133: stdio only);
- changing `mcp install` itself. If it can't work in a container (it
  asks on a terminal the container has none of: `--yes` doesn't ask),
  say why in the completion notes and write a ticket.

## Implementation steps

1. Read 2513's `devcontainer.json` files and `scripts/setup-linux.sh`,
   and `crates/cli/src/mcp/install.rs`.
2. Change each `postCreateCommand` as **In** says, keeping it a single
   string of plain JSON.
3. Check the engine's by hand if a container runtime is at hand: open
   it, then `cat .mcp.json` (it is checked in already, so `install`
   says it has the server) and, in VS Code, `.vscode/mcp.json`. If none
   is at hand, say so in the completion notes.
4. Write the two lines of text.

## Acceptance criteria

- [ ] The engine's and every template's `postCreateCommand` run `mcp
      install --yes` after the setup script.
- [ ] CI's templates job still passes (a template's `devcontainer.json`
      is plain JSON with `{{name}}` its only placeholder).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: if 2513 added a test that reads the `devcontainer.json` files
  (that each parses as JSON, names the setup script), extend it: each
  `postCreateCommand` ends with `mcp install --yes` and its `--command`
  as **In** says.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
