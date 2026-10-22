# ADR-0031: The MCP server: hand-written JSON-RPC, the game in its tool program, and session tools as operations

- **Status:** Accepted
- **Date:** 2026-10-22
- **Related tickets:** 1133 (this), 2356 (debug commands: a `command`
  tool), 2514 (frames over time from `frame_png`), 2519 (`start` from a
  session record), 2357 (`seek`), 2521 (the live link), 2529 (several
  peers, parked), 2525 (scheduled agent runs, which reuse the dry run)
- **Extends:** ADR-0005 §2 (three surfaces), ADR-0022 (the contract and
  its session), ADR-0028 (problem codes)

## Context

ADR-0005 §2 says `shuyi mcp` serves each contract operation as a tool
and keeps a session of one headless game between calls. Building it
raised questions:

- **A library or our own.** The Rust MCP SDK (`rmcp`, MIT) brings tokio
  and a stack of macros for a server whose transport is stdio and whose
  methods are four (`initialize`, `ping`, `tools/list`, `tools/call`).
  serde_json is already a dependency.
- **Where the game runs.** The `shuyi` binary knows no game: the
  operations that run it run in the game's tool program (ADR-0022 §2).
  A session must keep a game between calls, and an agent changes the
  game's code between sessions; a server that ran the game in its own
  process would need to be restarted by the client to see a change.
- **Names.** The ticket names the session tools `start`, `step`,
  `scene`, `frame_png`, `events`, `reset` and `stop`. The contract
  already has a one-off `scene`. Answers carry an `op`.
- **What the session paints.** `GameTools::frame` paints a fresh
  Harness after a script; the session's frame is the frame of a game
  that already ran.

## Decision

### 1. A small JSON-RPC layer of our own

`shuyi_cli::mcp::server` reads one JSON-RPC 2.0 message per line and
writes one response per line, as MCP's stdio transport says. It answers
`initialize` (the client's protocol version if it is one it knows, else
its newest), `ping`, `tools/list` and `tools/call`; a notification gets
no answer, an unknown method `-32601`, an unknown tool `-32602`. No new
dependency: a picture is base64 by a function of ten lines.

### 2. The game runs in its tool program, as `shuyi mcp-host`

The server runs no game. A tool that runs the game is sent to a host:
`cargo run` of the program `shuyi.ron` names, with the hidden command
`mcp-host`, which reads a request (`{"op": …, "input": …}`) per line on
stdin and writes a reply (`{"answer": …, "png": …}`) per line on stdout
(`mcp::host`). A one-off operation gets a host of its own, which ends
when its stdin closes; the session keeps one from `start` to `stop`. So
`start` builds the game's latest code, and a compile error or a panic
comes back as `SH3005` (`game-program`) with the last of the program's
stderr, which also goes on to the server's stderr.

The server works the same whether it runs in the `shuyi` binary (this
repository's `.mcp.json`) or in a game's tool program (`cargo shuyi
mcp` in a new game). Its links to hosts are a trait (`Launch`, `Link`),
and `InProcess` is a host in the same process, for tests.

The session's game is one field on each side (`Server::game`, the
host's `Live::game`), so several peers (2529) can take its place.

### 3. Session tools are operations

`Op` gains `Start`, `Step`, `FramePng` (named `frame_png`), `Events`,
`Reset` and `Stop`, outside `Op::ALL` (the CLI's operations) and listed
in `Op::SESSION` with `Scene`. Every session answer is an `Answer` with
its `op`, so a client reads one shape. `step` answers as `run` does,
`scene` as `scene` does. Their schemas are generated and checked in as
`session.<tool>.input.json` and `.output.json`; the contract's version
went to 9.

The tools are made from `Op::ALL` and `Op::SESSION`, so a new operation
is a tool once its arm is added to `host::one_off`, which the compiler
asks for. The one-off `scene` is not listed: in MCP, `scene` is the
session's, and `run` answers with a one-off run's scene.

### 4. A failed answer is an error result, as it is

A tool's result is its answer as `structuredContent` and as JSON text,
with the tool's output schema; `isError` is set when the answer isn't
`ok`. The answer keeps its `code` and `help` (ADR-0028), so a client
branches on the code. Wrong input is an answer with `SH0001`, not a
JSON-RPC error. A session tool with no game running answers `SH0002`
(`no-session`).

### 5. A session paints its own frame

`Session::picture(scale)` paints the session's frame; its default says
the game doesn't offer it (`SH3001`). `Headless::with_picture` gives it
from the game's `frame-png` painter; Hello, Grid and both templates
wire it.

### 6. `install` writes project files, shows a diff, and keeps a backup

`shuyi mcp install` writes the server's entry into each agent's project
file of MCP servers: Claude Code's `.mcp.json` always, Cursor's,
VS Code's and Gemini CLI's when it finds their folders or programs. It
merges into what is there, shows a line diff, asks, and copies the old
file to `<file>.bak`. The entry runs the project's `command` (or
`--command`) with `mcp`. `--dry-run` prints each file; Claude Code's is
what `claude --mcp-config` reads.

## Consequences

- An agent can start a game, step it, read its scene, see its frame and
  change its code, then `start` again, with no window and no restart of
  its client.
- Every one-off call that runs the game costs a `cargo run`, which is a
  no-op build once built; the first `start` in a fresh checkout builds
  the game.
- A server whose engine is newer than a game's (an installed `shuyi`
  over an old pin) gets a reply it can't use, and says so with
  `SH3005`.
- The engine's own test runs `cargo run` of Hello, Grid's tools from
  inside `cargo test`, so CI builds that program once more.
- The server is synchronous: a long tool call blocks the next message,
  and cancellation is ignored.

## Alternatives considered

- **`rmcp`** — an async runtime and macros for four methods over stdio.
- **The game in the server's own process** — the client would have to
  restart the server to see a change to the game's code, and a panic in
  the game would end the server.
- **Session tools with prefixed names (`session_scene`)** — the ticket
  and the tickets after it name the bare tools; the prefix moved to the
  schemas' file names instead.
- **Answers of session tools with the `op` of the operation they
  resemble (`step` as `run`)** — a client couldn't tell a step's answer
  from a run's.
- **`frame_png` replaying the session's steps in a fresh Harness** — it
  costs the whole session's frames on every picture, and needs the
  waits to come out to the same frames.
