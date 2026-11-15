---
id: "2081"
title: "The repository's MCP server command runs the shuyi binary with `--bin`, reusing the workspace's build"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2081 — The repository's MCP server command runs the shuyi binary with `--bin`, reusing the workspace's build

## Context

Ticket 2079 moved the `.cargo/config.toml` aliases from `run -q -p
shuyi-cli --` to `run -q --bin shuyi --`. With `-p`, cargo unifies that
package's features alone, so the dependencies it shares with the
workspace build again under other hashes; it measured 7 units for
`cargo run -p shuyi-cli -- --help` after `cargo build --workspace`, and
none for `--bin shuyi`.

The repository's own MCP server is started the old way:

- `.mcp.json` (Claude Code): `cargo run -q -p shuyi-cli -- mcp`;
- `.codex/config.toml` (Codex): the same args;
- `.devcontainer/devcontainer.json`: `mcp install --command "cargo run
  -q -p shuyi-cli --"`.

Each session's first MCP start builds those copies.

## Nick input

None.

## Scope

**In:**
- The three commands above with `--bin shuyi` instead of `-p
  shuyi-cli`, if measuring confirms they add no unit after `cargo build
  --workspace`.

**Out (do not do):**
- The command `shuyi mcp install` writes into a game's settings
  (`crates/cli/src/mcp/install`, its tests, `crates/cli/tests/it/mcp.rs`):
  a game's workspace is another one, with its own features.
- The game templates' aliases.

## Implementation steps

1. From a fresh target: `cargo build --workspace`, then `cargo run -q
   --bin shuyi -- mcp --help` and `cargo run -q -p shuyi-cli -- mcp
   --help`; count the `Compiling` lines of each (in the notes).
2. Change `.mcp.json`, `.codex/config.toml` and
   `.devcontainer/devcontainer.json`.
3. Start the MCP server from a new session and call `doctor`.

## Acceptance criteria

- [ ] After `cargo build --workspace`, the new command compiles nothing
      (notes).
- [ ] A new session's `shuyi` MCP server starts and answers `doctor`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Manual: the unit counts and the MCP start, in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
