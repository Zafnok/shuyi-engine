---
id: "1133"
title: "An MCP server over the tool contract, with a session that keeps one headless game running"
type: feature
milestone: "Engine: public"
model: reasoning
effort: high
status: done
blocked_by: ["1129", "2509", "3040", "2810"]
nick_input: none
completed: 2026-10-22
---

# 1133 — MCP server over the tool contract

## Context

ADR-0005 §2. Every engine with agent support found in the research gives
agents the same handful of abilities:

- start the game headless;
- send it input;
- read the scene and the logs;
- take screenshots.

Godot's are community MCP servers. Unity and Unreal have official MCP
plugins. Bevy has `bevy_brp_mcp`.

Our contract (1129) already does all of that without a window. This
ticket exposes it over MCP (stdio). It keeps a session so an agent can
step a game, look at it, and step it again.

## Nick input

None.

## Scope

**In:**
- `shuyi mcp` is an MCP server over stdio. Each contract operation is a
  tool, with the JSON schemas from 1129 (`playtest`, from 2126, among
  them).
- A tool whose answer failed returns that answer as it is, with its
  `code` and `help` (2509), as an MCP error result, so a client
  branches on `code`, never on the message. `explain` (2509) is a tool
  like the other operations.
- Session tools: `start` (build and load a game headless), `step`
  (frames or a script, played as the contract's `frame_png::Script`, so
  steps later tickets add to it, such as 2524's `act`, reach `step`
  too), `scene`, `frame_png` (returns the image),
  `events`, `reset`, and `stop`.
- The session's `scene` and `step` tools answer with the `log` field
  3040 adds to the contract's `run` and `scene` answers (each line's
  frame, level, category and text). The server keeps no log of its own.
- **Either order with 2356** (debug commands): if it is done when this
  starts, add a `command` tool to the session (input: `line`, one line;
  answer: the reply and whether it ran) and let `step` take a script's
  `command` steps (`frame_png::Step::Command`). If not, 2356 adds them.
- `shuyi mcp install` registers the server with the agents it finds
  (Claude Code first; others whose config format is documented). It shows
  the diff and asks before writing, and keeps a backup, as REA does.
  Its `--dry-run` writes nothing and prints what it would write; for
  Claude Code that is the server's entry as a JSON file Claude Code
  reads with `--mcp-config`, which 2525's runs reuse.
- **Either order with 2513** (the dev container): if it is done, the
  engine's and every template's `.devcontainer/devcontainer.json`
  register the server after the setup script. Their `postCreateCommand`
  also runs `shuyi mcp install` without the question (for example with
  `--yes`), so an agent in a dev container or a Codespace finds it. The
  container has no installed `shuyi` (2513 leaves `cargo install` until
  2005), so it runs the CLI as 2513 runs `doctor`: `cargo run -q -p
  shuyi-cli --` in the engine, `cargo shuyi` in a template. If that
  can't work, the completion notes say why. If 2513 isn't done, write a
  ticket for these lines (`write-ticket`), blocked by 2513.
- **This repository uses it too.** A checked-in `.mcp.json` at the
  root registers the server for sessions that work on the engine.
  This repository has no installed `shuyi`, so the entry runs `cargo
  run -q -p shuyi-cli -- mcp`. Its content is what `shuyi mcp install
  --dry-run` writes for Claude Code, with that command in place of
  `shuyi`. The `work-ticket` skill's §4 (*Verify*) gains one line: a
  change a player sees is checked through the session tools on
  `examples/hello-grid` (`start`, `step`, `scene`, `frame_png`), not by
  reading the Harness's code to drive it. The docs map
  (`docs/README.md`, 2810) gets a row for `.mcp.json` under *Engine
  developers*, and *What each task reads* says to use the session
  tools for the `23xx` and `27xx` blocks.
- An MCP library under an allowed license (check with `cargo deny`), or
  a small hand-written JSON-RPC layer if none fits.
- A test drives the server with a scripted client over stdio against
  `hello-grid`: start, step to the battle, read the scene, render a
  frame, and one call that fails (`bots` with 0 games), checking its
  `code` is `SH0001`.

**Out (do not do):**
- a network transport for the MCP server itself (stdio only).
  Attaching the session tools to a running windowed build over the
  live link is 2521's, which waits for this ticket;
- editing files through MCP (agents already do that);
- frames over time from the session tool `frame_png` (`every`,
  `count`, `columns`, a sheet labelled with times): 2514 adds them to
  this tool after this ticket;
- debug commands a game declares and their console (2356), apart from
  the either-order item above;
- starting a session from a session record: 2519 adds `session` and
  `any_stamp` to `start`;
- going back in a session (`seek`): 2357 adds it once 2518's session
  records exist;
- several peers in one session, a `peer` argument and a `net` tool: the
  parked 2529 adds them after 1140. Keep the session's game in one
  field, so 2529 can put a game's `Peers` there.

## Implementation steps

1. Write the server and the session state.
2. Write the install command with a dry run.
3. Do the either-order items whose other ticket is done (2356, 2513).
4. Write the scripted client test.

## Acceptance criteria

- [x] The scripted client test passes in CI.
- [x] `shuyi mcp install --dry-run` prints what it would write.
- [x] The root `.mcp.json` registers the server, and a Claude Code
      session in this repository lists its tools (a manual check noted
      in the PR).
- [x] `cargo deny check` passes.
- [x] All gates pass.

## Tests required

- Integration: the stdio client test.

## Completion notes

- **What was done.** `shuyi mcp` (`crates/cli/src/mcp.rs` and
  `mcp/`): a hand-written JSON-RPC 2.0 layer over stdio (`initialize`,
  `ping`, `tools/list`, `tools/call`), no new dependency. Every operation
  of `Op::ALL` is a tool with its 1129 schemas (`playtest` among them),
  and the session tools `start`, `step`, `scene`, `frame_png`, `events`,
  `reset` and `stop`. A failed answer is an MCP error result carrying the
  answer as it is (`code`, `help`); `explain` is a tool. `step` answers
  as `run` and `scene` as `scene`, both with 3040's `log`; the server
  keeps no log. `shuyi mcp install [--dry-run] [--yes] [--command …]`
  writes Claude Code's `.mcp.json` always and Cursor's, VS Code's and
  Gemini CLI's files where it finds them, showing a diff, asking, and
  keeping `<file>.bak`. The root `.mcp.json` is the dry run's Claude
  Code file with `cargo run -q -p shuyi-cli --` (a test checks). The
  `work-ticket` skill's §4 and the docs map gained their lines.
  ADR-0031 records the design.
- **How the session runs.** The server runs no game: a tool that runs
  the game goes to the game's tool program, `cargo run` as the hidden
  `shuyi mcp-host` (JSON lines on stdin and stdout). The session keeps
  that program running from `start` to `stop`, so `start` rebuilds the
  game after an agent edits it; a build error or a panic comes back as
  the new code `SH3005` with the program's last output. The session's
  game is one field on each side (`Server::game`, the host's
  `Live::game`) for 2529.
- **Deviations.**
  - The contract's one-off `scene` and the session's `scene` share a
    name: in MCP, `scene` is the session's (no input), and `run`
    answers with a one-off run's scene. The session tools are `Op`
    variants (`Op::SESSION`), so each answer has its own `op`; their
    schemas are checked in as `session.<tool>.*.json`, and the
    contract's version is 9.
  - Painting the session's frame needed `Session::picture` (default:
    not offered, `SH3001`) and `Headless::with_picture`; Hello, Grid and
    both templates wire it.
  - Two new problem codes: `SH0002` (`no-session`) and `SH3005`
    (`game-program`), with their pages.
  - The stdio test lives in `crates/cli/tests/it/mcp.rs` and runs the
    real `shuyi` binary from the repository's root against
    `examples/hello-grid`, as the root `.mcp.json` does; `start` builds
    Hello, Grid's tool program with cargo inside the test.
- **Either-order items.** 2356 isn't done: its MCP clause now says what
  to add (a `command` session tool; `step` already plays the contract's
  `Script`). 2513 isn't done: 2531 registers the server in the dev
  containers, blocked by 2513. The clauses of the open tickets that
  named 1133 are settled (2225, 2350, 2356, 2361, 2406, 2513, 2515,
  2517, 2520, 2523, 2524, 2527, 2907).
- **Follow-up tickets.** 2531.
- **The game.** No breaking change: `Op` gains variants (the game
  doesn't match on it), and `Session` gains a defaulted method. The
  game's tools can wire `Headless::with_picture` when it moves its pin,
  and run `cargo shuyi mcp install` to get the server; no game ticket
  needed.
- **Manual check.** `claude mcp list` in this repository lists `shuyi:
  cargo run -q -p shuyi-cli -- mcp`, pending approval: Claude Code asks
  before it runs a project's server. Running `claude` here and
  approving it shows the tools; the stdio test drives the same command
  through `initialize`, `tools/list` and the session.
- **Nick.** Nothing to decide.
