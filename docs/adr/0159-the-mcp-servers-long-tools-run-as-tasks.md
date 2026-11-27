# ADR-0159: The MCP server's long tools run as tasks: a host's reply awaited off the loop, and no session until `start`'s task completes

- **Status:** Accepted
- **Date:** 2026-11-27
- **Related tickets:** 3436 (this); 3440 (forwarded calls as tasks)
- **Amends:** ADR-0031 §1 (the loop is no longer the only place a call's
  work waits)
- **Extends:** ADR-0156 (2026-07-28)

## Context

MCP's tasks extension (`io.modelcontextprotocol/tasks`, SEP-2663) lets a
`tools/call` the server can't finish quickly answer a task at once,
which the client polls. Read on 2026-11-27: the specification
(`tasks.extensions.modelcontextprotocol.io/specification/2026-07-28/`),
its schema (`ext-tasks`, `schema/2026-07-28/schema.json`), the overview
at `modelcontextprotocol.io/extensions/tasks/overview`, and Claude
Code's *MCP tasks for server authors* (`code.claude.com/docs/en/mcp`).
What they ask of a server:

- The server declares the extension in `server/discover`
  (`capabilities.extensions`, an empty object). It answers a task only
  to a request whose `_meta` client capabilities declare it too, and
  decides alone which calls are tasks: a client asks for nothing.
- The task answer (`resultType: "task"`) and `tasks/get`'s
  (`resultType: "complete"`) carry, flat, `taskId`, `status`,
  `createdAt`, `lastUpdatedAt` (ISO 8601) and `ttlMs` (from creation,
  `null` for no end), and may carry `pollIntervalMs` and
  `statusMessage`. A completed task carries `result`, the `tools/call`
  result; a failed one `error`, a JSON-RPC error. `failed` is **not**
  for a tool's error result (`isError: true`): that is `completed`.
- `tasks/cancel` is acknowledged with an empty result; the server need
  not stop the work. An unknown task is `-32602`; `tasks/get`,
  `tasks/cancel` from a client without the extension `-32021`.
- Claude Code polls at `pollIntervalMs` (clamped to 250–30,000 ms, 2,000
  if absent), asks results be kept at least an hour, sends
  `tasks/cancel` when the person stops a task while connected, and uses
  no `tasks/result`, `tasks/list` or `notifications/tasks`.

The server is one loop over stdin (ADR-0031 §1, ADR-0035 §4): a call's
work runs inside `handle`, and a long one (`start`'s cold build of the
game, `playtest`'s many runs, `export`'s release build) holds the client
on one `tools/call` past its 300 s timeout, and holds the loop too.

The hosts the work waits on are trait objects that are not `Send`:
`Launch<'a>` and `Link` borrow, in `InProcess`, the game's tools
(`&'a dyn GameTools`), which games' own tests of the server use. The
long work of every tasked tool is a host's reply: `Cargo` starts a
`cargo run … mcp-host` process at once, and the build and the run are
what its first reply waits on. What a tasked tool does without a host
(a project with no game program, a launch refused) answers at once.

## Decision

### 1. Which tools are tasks

`mcp::tasked(op)` lists them: `start` (without `link`), `playtest`,
`autobalance`, `bake` and `export`, each with the reason in its comment.
A call of one is a task on a 2026-07-28 request that declares the
extension (`server::declares_tasks`) and that this server answers
itself. Every other call, every call without the extension and every
call on the old protocol answers as before. A call forwarded to another
workspace's server answers as before too (3440: the ticket's size valve
split it off).

### 2. A host's reply is awaited off the loop, not the work moved to a thread

`Link` gains two methods with defaults: `send` (the request, without
waiting; `false` if the host only answers as `call` waits, the default)
and `reply` (the reply once it has come, none before). `child::Program`
reads its stdout on a thread of its own (`read_replies`), so its `reply`
is a `try_recv`, and its `call` is `send` and a blocking `recv`.

A tasked call is prepared as a call is (`Server::prepared`,
`Server::start_prepared`): a host launched and its request made, a
`Pending`. The request is sent, the `Pending` is the task's work, and
the task exists before its answer is written. `Server::advance` looks at
each working task: a reply that came ends it with the result
`tools/call` would have answered (`Server::finish`, then
`Server::answered`, the engine-moved restart included). The loop
advances on `tasks/get`, `tasks/cancel`, a session tool and each
`poll` (every 200 ms, ADR-0035 §4). A host whose `send` is `false`
(`InProcess`) is called when the server next looks: in a test, at the
first `tasks/get`.

This amends ADR-0031 §1: a call's work may wait outside `handle`, on
the host's own reader thread, while the loop goes on answering.

### 3. The task table

`mcp::tasks::Tasks` holds each task: its id (`shuyi-<n>`, counted from 1
in the process), its tool, when it was made and last changed, and its
state (`Working` with its work, `Completed` with the `tools/call` result,
`Failed` with a JSON-RPC error, `Cancelled`). `pollIntervalMs` is 2,000
for `start`, whose build is often seconds, and 10,000 for the others,
which run for minutes. `ttlMs` is `null` while it works (it is never
forgotten then), and once it ends the time it ran plus an hour, so it is
forgotten an hour after it ended (`KEEP`); a later `tasks/get` is
`-32602`. A task lives in its process.

A tool's error result (`SH3005`, a game that doesn't build) is
`completed`, its `code` kept in `structuredContent`, as the specification
says and as an agent branches on it today. `failed` carries the JSON-RPC
error a call would have answered: the internal error of a fresh server
that stopped (`-32603`).

### 4. The session while `start` builds

The session has no game until `start`'s task completes `ok` (or with a
session record's result, as today), when the host becomes the session's
game. A session tool called while it works answers `SH0002` (no
session) with a `why` that names the task and says to call again once it
completed. A second `start`, a task or not, attached or not, here or
forwarded to another workspace's server, cancels the working one (one
here says so in its `limits`): `start`
already replaces the game that runs, and the first's build would only
hold cargo's lock against the second.

### 5. A cancel drops the host

`tasks/cancel` of a working task drops its `Pending`: `Program`'s drop
kills `cargo` (on Windows, cargo's job object takes its `rustc` and the
game with it; elsewhere a running `rustc` finishes on its own and the
game ends when its stdin closes). The task is `cancelled`. A cancel of a
task that ended changes nothing and is an empty result.

## Consequences

- A client with tasks no longer times out on a cold build, sees the
  task's state, and can stop it; one without them waits as before.
- The traits stay as they are for games: the two methods are additive,
  with defaults, and `InProcess` needs neither.
- A `Link` that blocks in `reply` would block the loop; only `Program`
  sends, and its `reply` never blocks.
- The engine-moved restart still waits on the fresh server inside a
  task's completion, and a forwarded call still waits on its server:
  3440.
- The work of a one-off with no host (none is long) runs in the call, as
  before; it is a task that is `completed` at once.

## Alternatives considered

- **A thread per task, running the call.** The ticket's plan. The
  hosts and launchers are not `Send` (`InProcess` borrows the game's
  tools), so a thread would need `Send + 'static` bounds on `Launch`
  and `Link`: a break for every game's tests of the server, for work
  that is a wait on a process already running beside the server. The
  reader thread `Program` gains is that thread, without the bound.
- **Answer every tasked tool's task `completed` at once, after the
  call.** The client would still wait on the call.
- **`failed` for a tool's error result, as the ticket said.** The
  specification forbids it, and Claude Code reports a completed error
  result as a failure with its text and code anyway.
- **Let a session tool wait for `start`'s task.** It would hold the loop
  on the build the task exists to take off it.
