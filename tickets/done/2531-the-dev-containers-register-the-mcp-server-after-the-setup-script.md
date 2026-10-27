---
id: "2531"
title: "The dev containers register the MCP server after the setup script, so an agent in a container or a Codespace finds it"
type: feature
milestone: MVP
model: sonnet-5
effort: low
status: done
blocked_by: ["2513", "1133"]
nick_input: none
completed: 2026-10-27
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
1133. 1133 landed first, and 2513 is done, so this ticket does it.

**From 2513 (done):** the files are `.devcontainer/devcontainer.json`
and the template's `templates/shuyi-default/base/.devcontainer/devcontainer.json`
(the template's `base` part, so every short name's game has it). The
test is `every_dev_container_is_set_up_as_the_engines` in
`crates/cli/src/contract/new/tests.rs`: it checks that `image`,
`postCreateCommand`, `remoteEnv` and `customizations` equal the
engine's, and the game's `name` is the game's.

A container has no installed `shuyi`, before or after the first
release (2005), so it runs the CLI as 2513 runs `doctor`:
`cargo run -q -p shuyi-cli --` in the engine, `cargo shuyi` in a
template (the alias runs the game's own tool program, which serves
`mcp` too).

Since 2535, `mcp install` ends by running the entry it wrote with
`--ready`, which builds the server and the game's tool program. So the
container's creation also takes that first build (minutes, cold), and
the agent's first connection is then quick instead of timing out. That
is the point of 2535, so keep it; the README's line (Scope) also says
that the container's first creation takes that build.

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
- installing `shuyi` in the container: it runs from source, as
  Context says, and the first release (2005) changes nothing there;
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

- [x] The engine's and every template's `postCreateCommand` run `mcp
      install --yes` after the setup script.
- [x] CI's templates job still passes (a template's `devcontainer.json`
      is plain JSON with `{{name}}` its only placeholder).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: 2513's test in `crates/cli/src/contract/new/tests.rs` reads
  every `devcontainer.json` and checks that its `image` and
  `postCreateCommand` equal the engine's. The commands now differ on
  purpose, so change that check: the `image` still equals the engine's;
  every `postCreateCommand` equals the engine's up to the command that
  runs `mcp install` (the setup script, then `. "$HOME/.cargo/env"`);
  the engine's then ends with `mcp install --yes --command "cargo run
  -q -p shuyi-cli --"`, and each template's with `cargo shuyi mcp
  install --yes`. If 2513's test checks something else, change it to
  check the same.

## Completion notes

The engine's and the template's `postCreateCommand` now run the setup
script, source `"$HOME/.cargo/env"`, then `mcp install --yes` (the
engine's with `--command "cargo run -q -p shuyi-cli --"`, the template's
as `cargo shuyi mcp install --yes`). `every_dev_container_is_set_up_as_the_engines`
checks the exact strings; `image`, `remoteEnv` and `customizations` still
equal the engine's. The README's Codespaces paragraph and the template's
`AGENTS.md` `mcp install` row say the container registers the server.
No container runtime was at hand, so the container itself was not opened.
`--yes` asks nothing, so `mcp install` needed no change. No follow-ups.

Breaking: none.
