---
id: "2081"
title: "The repository's MCP server command runs the shuyi binary with `--bin`, reusing the workspace's build"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2081 — The repository's MCP server command runs the shuyi binary with `--bin`, reusing the workspace's build

## Context

Ticket 2079 measured `cargo run -p shuyi-cli -- --help` at 7 units after
`cargo build --workspace`, and `cargo run --bin shuyi -- --help` at none:
with `-p`, cargo unifies that package's features alone, so the
dependencies it shares with the workspace build again under other
hashes. The `cargo shuyi` alias kept `-p shuyi-cli`, because it runs from
a member's folder (`examples/hello-grid`), where `--bin` looks only in
that folder's package. A command run from the root can use `--bin`.

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
  shuyi-cli`, if each runs from the workspace's root and measuring
  confirms they add no unit after `cargo build --workspace`.

**Out (do not do):**
- The command `shuyi mcp install` writes into a game's settings
  (`crates/cli/src/mcp/install`, its tests, `crates/cli/tests/it/mcp.rs`):
  a game's workspace is another one, with its own features.
- The game templates' aliases, and the `cargo shuyi` alias (2079).

## Implementation steps

1. From a fresh target: `cargo build --workspace`, then `cargo run -q
   --bin shuyi -- mcp --help` and `cargo run -q -p shuyi-cli -- mcp
   --help`; count the `Compiling` lines of each (in the notes).
2. Change `.mcp.json`, `.codex/config.toml` and
   `.devcontainer/devcontainer.json`.
3. Start the MCP server from a new session and call `doctor`.

## Acceptance criteria

- [x] After `cargo build --workspace`, the new command compiles nothing
      (notes).
- [x] A new session's `shuyi` MCP server starts and answers `doctor`.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Manual: the unit counts and the MCP start, in the notes.

## Completion notes

The repository's own MCP server now runs as `cargo run -q --bin shuyi --
mcp`, in `.mcp.json`, `.codex/config.toml` and the dev container's
`postCreateCommand` (its `--command`, and the `mcp install` it runs).

Measured on a target of its own (`CARGO_TARGET_DIR=target/fresh-2081`),
after `cargo build --workspace`, counting the `Compiling` lines of each
command run without `-q` (which hides them; the units are the same):

- `cargo run --bin shuyi -- mcp --help`: 0 units;
- `cargo run -p shuyi-cli -- mcp --help`: 8 units (`memchr`,
  `serde_json`, `schemars`, `shuyi-core`, `shuyi-assets`, `shuyi-ui`,
  `shuyi-platform`, `shuyi-cli`).

The MCP start: `cargo run -q --bin shuyi -- mcp`, read from the new
`.mcp.json` and run from the root as a new session's client runs it, was
sent `initialize` and a `tools/call` of `doctor` over stdin; it answered
both, `doctor` with `"ok": true`.

Deviations:

- `crates/cli/tests/it/mcp.rs`'s `ENGINE_COMMAND` changed with
  `.mcp.json`: its test checks that `.mcp.json` is what `mcp install`
  writes for that command. Only the constant changed, not the install
  tests the **Out** line keeps. So did the engine's command in
  `crates/cli/src/contract/new/tests.rs`
  (`every_dev_container_is_set_up_as_the_engines`), which pins the dev
  container's `postCreateCommand`.
- The dev container's own first `cargo run` (the `mcp install` it runs)
  also uses `--bin shuyi`, so the container builds one copy, not two.
- `docs/README.md`'s row for the MCP server names the new command.

No break; no `Behaviour:` line: only this repository's own settings
changed, which no game reads (a game's `mcp install` writes its own).

Book: none, no public API, contract or game-facing command changed; the
book never names this repository's own MCP settings.
