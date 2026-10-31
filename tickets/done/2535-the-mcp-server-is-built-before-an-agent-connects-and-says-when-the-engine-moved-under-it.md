---
id: "2535"
title: "The MCP server is built before an agent connects, and says when the game's engine moved under it"
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["1133"]
nick_input: none
completed: 2026-10-22
---

# 2535 — The MCP server is ready before an agent connects, and says when it is out of date

## Context

1133 made `shuyi mcp`, the tool contract as an MCP server with a session
([ADR-0031](../../docs/adr/0031-the-mcp-server-hand-written-json-rpc-the-game-in-its-tool-program-and-session-tools-as-ops.md),
`docs/contract.md`, *The MCP server*). An agent's project file
registers it as a command that `cargo` builds and runs: `cargo run -q
-p shuyi-cli -- mcp` in this repository (`.mcp.json`), `cargo shuyi mcp`
in a game (what `shuyi mcp install` writes). That keeps the server on
the engine the project builds on, with no install to update: a game's
server moves with its pin. Two things are left to people, and they
should not be:

- **The first connection times out after the engine changes.** Claude
  Code waits a limited time for a stdio server to answer `initialize`.
  With nothing to compile, `shuyi mcp` answers in about 8 seconds on
  Nick's Windows machine. After a pull here, or a pin move in a game,
  `cargo run` compiles first and takes minutes, so the client gives up
  ("Connection closed"). This happened to the session that finished
  1133: `shuyi` failed to connect right after `shuyi-cli` changed.
- **A connected server doesn't know it is out of date.** The server is
  built when the client connects and runs until the client stops it.
  `start` builds the game's tool program afresh (the host,
  `mcp::child`), so the game's code is always the latest, but the
  server's own tool list and dispatch are the engine's as of the
  connection. After a pin move, a new operation is missing from
  `tools/list`, and a changed input is refused, until someone
  reconnects. Every host reply carries the host's `contract`
  (`Answer::contract`), which the server can compare with its own
  `contract::VERSION`.

## Nick input

None.

## Scope

**In:**
- **`shuyi mcp --ready`**: builds what the server needs and exits 0,
  printing `mcp: ready (contract N)`, or 1 with the build's error. It
  runs through the same command as the server, so running the
  registered entry with `--ready` builds the server. In a project whose
  `shuyi.ron` names a game tool program, it also runs `cargo build`
  with that program's arguments (a `GameProgram::cargo_build_args`
  beside `cargo_args`), so the first `start` doesn't wait for it
  either.
- **`shuyi mcp install` ends with a ready run**: after writing (not
  with `--dry-run`, and not when the user declined), it runs the entry
  it registered with `--ready` and prints how long it took. When that
  took longer than 20 seconds, it says which setting raises Claude
  Code's wait for a server to start. Check the setting's name in Claude
  Code's documentation first (`MCP_TIMEOUT` when this was written), and
  write it only if the documentation confirms it. The run goes through
  a closure, as `ask` does, so the tests use a fake.
- **A server older than the game's engine says so.** When a host's
  answer has a `contract` other than the server's `contract::VERSION`,
  the server adds a limit to that answer: `the game's engine speaks
  contract X and this server Y: the server was built before the engine
  changed; run "<command> mcp --ready", then reconnect it (in Claude
  Code: /mcp)`, with `<command>` the project's `command`. Its `start`
  answer says it once more. It checks only the contract: a pin move that
  changes no schema leaves the tools as they were, so a server that
  doesn't know about it is still right.
- **Docs**: `docs/contract.md`, *The MCP server*, says in two sentences
  that the entry builds the server from the project's engine, and that
  after updating the engine (a pull here, a pin move in a game) you run
  `<command> mcp --ready` and reconnect. The templates' `AGENTS.md` row
  for `mcp install` adds the same sentence. The usage text names
  `--ready`.

**Out (do not do):**
- a server that restarts itself, or proxies to a newer one: MCP over
  stdio has no way to hand the client a new process;
- a global `shuyi` binary registered for every project: it wouldn't
  follow a game's pin (2005 releases one; the docs keep the
  per-project entry);
- the game's side: game#1181
  (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1181-the-engines-mcp-server-on-the-game-a-session-registered-and-reconnected-on-a-pin-move.md`)
  sets the server up there and reconnects it after a pin move. If 1181
  is done when this lands, write a game ticket that adds `cargo shuyi
  mcp --ready` before the reconnect in its `engine-change` skill's step
  6, and name it in the completion notes.

## Implementation steps

1. Read `crates/cli/src/mcp.rs`, `mcp/server.rs`, `mcp/install.rs` and
   `mcp/child.rs`, and `docs/contract.md`'s *The MCP server*.
2. `--ready` in `mcp::main`: parse it, build the game tool program if
   the working folder's project names one (`cargo build` with
   `cargo_build_args`, its output passed through), print the line, and
   exit. Test the arguments in `project.rs`'s tests, and `--ready` with
   no project through the binary (`crates/cli/tests/it/binary.rs`).
3. The stale check in `server.rs`: one function from a host's answer
   and the server's version to the limit, called where a host's reply
   becomes the tool's answer (`answer_of`). Unit tests with a `Link`
   that answers with `contract: VERSION + 1` and with `VERSION`.
4. `install`: the ready run after writing, through a closure; tests for
   a fast run, a slow one (the timeout hint), a failed one (exit 1 and
   the build's error), and none on `--dry-run` or a declined answer.
5. The docs above, then the `document` skill.

## Acceptance criteria

- [x] `shuyi mcp --ready` builds and prints `mcp: ready (contract N)`;
      in Hello, Grid's folder it builds its tool program too.
- [x] `shuyi mcp install` runs the entry with `--ready` after writing,
      and not on a dry run.
- [x] A host answer of another contract gets the limit naming both
      versions and the command to run.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the stale limit (another contract, the same contract); the ready
  run in `install` (fast, slow, failed, skipped); `cargo_build_args`.
- Integration: `shuyi mcp --ready` through the binary, outside a
  project.

## Completion notes

- `shuyi mcp --ready` (`mcp::ready`): builds the game's tool program
  with `GameProgram::cargo_build_args` (the same package, program and
  features as `cargo_args`, so `start`'s `cargo run` has nothing left to
  compile), cargo's output passed through, then prints `mcp: ready
  (contract N)`; 1 if the program doesn't build, 2 for any other
  argument. In Hello, Grid's folder, `cargo run -q -p shuyi-cli -- mcp
  --ready` built its tools and printed `mcp: ready (contract 9)`.
- `mcp install` runs the entry it wrote with `--ready` after writing,
  through a `ready` closure (the real one is `install::run_ready`, its
  output passed through), and says how long it took. Over 20 s
  (`install::SLOW`) it names `MCP_TIMEOUT` (milliseconds), which Claude
  Code's MCP docs confirm sets the startup wait. Nothing written (a
  dry run, a declined answer, or files that have the server already)
  runs nothing. The closure is handed what install has said so far, so
  the "wrote" lines are printed before the build's output.
- `server::stale`: a host answer whose `contract` isn't the server's
  `VERSION` gets the limit, with the project's `command` (the session
  keeps the command of the project it started). It is added in
  `answer_of`, so `start`, the session tools and one-off operations all
  say it.
- Docs: `docs/contract.md`, *The MCP server*; the usage of `shuyi mcp`
  and `shuyi`; both templates' `AGENTS.md` row for `mcp install`. The
  clause in 1134 that waited on this ticket now says to run `cargo shuyi
  mcp --ready` before reconnecting.
- game#1181 is still open, so no game ticket was written: it can add
  `cargo shuyi mcp --ready` before the reconnect itself when it is done.
- No API a game calls changed (`GameProgram` gained a method).
