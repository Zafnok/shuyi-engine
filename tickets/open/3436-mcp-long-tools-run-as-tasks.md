---
id: "3436"
title: "The MCP server's long tools run as tasks: start's build, playtest, autobalance, bake and export answer a task the client polls"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3435"]
nick_input: none
completed:
---

# 3436 — The MCP server's long tools run as tasks

## Context

Anthropic's article *Skills over MCP, tasks and more* (2026-11-24) made
MCP **tasks** generally available in Claude and Claude Code: a
`tools/call` the server can't finish quickly answers a task at once, and
the client polls it. They are MCP's tasks extension,
`io.modelcontextprotocol/tasks` (SEP-2663; spec at
`github.com/modelcontextprotocol/ext-tasks`, overview at
`modelcontextprotocol.io/extensions/tasks/overview`). What Claude Code's
docs (`code.claude.com/docs/en/mcp`, *MCP tasks for server authors*,
read 2026-11-26) ask of a server:

- Only on a **2026-07-28** connection (3435), and only when the request's
  `_meta["io.modelcontextprotocol/clientCapabilities"].extensions` holds
  `io.modelcontextprotocol/tasks`: a task answered to a request without
  it is invalid. The server declares the extension in `server/discover`.
- The `tools/call` result is `resultType: "task"` with `taskId` (up to
  128 printable ASCII characters, no spaces), `status`, `createdAt`,
  `lastUpdatedAt`, `ttlMs` (may be `null`) and, optionally,
  `pollIntervalMs` (clamped to 250–30,000 ms; 2,000 by default). The
  task exists before the answer is sent.
- **`tasks/get`** (`{ taskId }`) answers its status: `working`,
  `input_required` (with `inputRequests`, answered by **`tasks/update`**
  `{ taskId, inputResponses }`), `completed` (with `result`, the
  `tools/call` result it stands for), `failed` (with `error`) or
  `cancelled`. **`tasks/cancel`** is sent when the user stops it while
  connected. Claude Code uses no `tasks/result`, `tasks/list` or
  `notifications/tasks`.
- Keep a finished result at least an hour; poll every 10–30 s for jobs
  past a minute.

Read the extension's spec again when you start: the overview and the
2026-07-28 changelog disagree on small points (the changelog removed
`tasks/result` and `tasks/list`).

**Why we want it.** Some of our tools run for minutes, and today the
client waits on one `tools/call` the whole time:

- `start` launches the game's tool program with `cargo run … mcp-host`
  (`Cargo::launch`, `crates/cli/src/mcp/child.rs`), so its first call in
  a fresh worktree is a cold build of the game. 3414 found the
  server's own cold build outlasting the client's 300 s `MCP_TIMEOUT`
  (`.claude/settings.json`, `.codex/config.toml`) in a new worktree; a
  game's build is of the same order.
- `playtest`, `autobalance` (bots over many seeds), `bake` and `export`
  (a release build of a target) run as long, and `run` and `replay` can.
- A call forwarded to another workspace's server (ADR-0035 §2) waits on
  that server's build first.

A client that gives up on a call loses the work, and the agent has no
way to see progress or stop it.

**Why it isn't small.** The server is one synchronous loop over stdin
(ADR-0031 §1; `serve` reads on a thread and wakes every 200 ms to
`Server::poll`, ADR-0035 §4). To answer `tasks/get` while a build runs,
the work of a task must run off that loop, and the session tools must
not race it: `step` while `start` builds has no game yet.

## Nick input

None.

## Scope

**In:**
- **Which tools are tasks**: `start` (with no `link`), `playtest`,
  `autobalance`, `bake`, `export`, and any forwarded call, on a
  2026-07-28 request that declared the extension. Every other tool, and
  any tool without the extension, answers as today. A list in one place
  (`mcp::tasked(op) -> bool`), with its reason in a comment.
- **The task's run**: the work runs on its own thread; the loop answers
  `tasks/get` from the task's state. A task id is the server's own
  (`shuyi-<n>`, counting from 1 per process). `pollIntervalMs` 2,000
  for `start` and 10,000 for the rest; `ttlMs` one hour after it ends.
- **The session while `start` runs**: a session tool called while a
  `start` task is `working` answers today's "no session" failure with a
  `why` that names the task; a second `start` cancels the first and
  starts again (decide in the ADR, with the reason).
- **`tasks/cancel`**: stops the work: the child's `cargo` process is
  killed (the `Process` the forwarding and the host already hold), and
  the task is `cancelled`. A cancel of a finished or unknown task is an
  empty result, as the spec says.
- **`failed`** carries the tool's error as `tools/call` would have
  answered it, its `code` (`SH0001`) kept, so an agent still branches on
  it.
- **An ADR** (the `write-adr` skill): tasks in the server, which tools,
  the thread per task, the session's rule while `start` runs. It amends
  ADR-0031 §1 (the loop is no longer the only thread that works).
- **Docs**: `docs/contract.md`'s *The MCP server* (which tools may answer
  a task, and that a client without tasks waits as before); the book's
  *Tools for agents* (one sentence).
- **Estimated size:** about 1,100 lines (code 500, tests 450, docs 150).
- **Split:** past 900 lines, split off the forwarded calls as tasks into
  a new ticket: a forwarded call then answers as today.

**Out (do not do):**
- The 2026-07-28 protocol, `server/discover` and `_meta`: 3435.
- `input_required` inside a task (`tasks/update`): no tasked tool asks
  the person anything. `new`'s questions (3437) are asked on the call
  itself, not in a task.
- `notifications/tasks`, `subscriptions/listen` for tasks,
  `tasks/list` and `tasks/result`.
- Keeping tasks across a server restart: a task lives in its process.
- The server's own build before it can answer at all (first contact):
  3414's.
- A task on the old protocol.

## Implementation steps

1. Read the tasks extension's spec and Claude Code's *MCP tasks for
   server authors*; `crates/cli/src/mcp/server.rs` (`serve`, `handle`,
   `poll`, `tool_call`, `call`, `start`), `mcp/child.rs`,
   `mcp/forward.rs`; ADR-0031 and ADR-0035.
2. The task table in `server.rs` (or a new `mcp/tasks.rs`): id, status,
   timestamps, result, the handle to stop it.
3. `tool_call` answers a task for a tasked tool when the request allows
   it; `tasks/get` and `tasks/cancel` arms in `handle`.
4. The session's rule while `start` runs.
5. Tests first (below), then the ADR and the docs.
6. **Check by hand**: in Claude Code on 2026-07-28, `start` on a game
   in a fresh worktree answers a task, Claude Code polls it, and the
   session plays once it completes; a stop in the client cancels the
   build. Write what you saw in the completion notes.

## Acceptance criteria

- [ ] `server/discover` declares `io.modelcontextprotocol/tasks`.
- [ ] `playtest` on a 2026-07-28 request with the extension answers
      `resultType: "task"` with every field Claude Code requires, then
      `tasks/get` answers `working`, then `completed` with the same
      `result` the call answers without tasks.
- [ ] The same call without the extension, or on 2025-11-25, answers
      the result directly, as today.
- [ ] A tool not in the list (`scene`, `docs`) never answers a task.
- [ ] `tasks/cancel` of a working `start` kills its build and makes it
      `cancelled`; `step` then answers "no session".
- [ ] A failing `start` (a game that doesn't build) is `failed` with the
      `SH` code `start` answers today.
- [ ] `tasks/get` of an unknown id is `-32602`.
- [ ] Checked by hand in Claude Code (step 6).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `server/tests.rs`: each acceptance case through `handle` and `poll`,
  with a fake launcher whose build blocks until the test releases it
  (so `working` is seen, then `completed`), and one that the cancel
  must stop.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
