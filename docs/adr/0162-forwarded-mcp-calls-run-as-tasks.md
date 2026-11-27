# ADR-0162: A forwarded MCP call runs as a task: the other workspace's server built and its call answered off the loop

- **Status:** Accepted
- **Date:** 2026-11-27
- **Related tickets:** 3440 (this); 3436 (tasks)
- **Amends:** ADR-0159 §1 (a forwarded call answered as it was)
- **Extends:** ADR-0035 §2 (the forwarded server per workspace)

## Context

ADR-0159 made the long tools (`mcp::tasked`) answer a task on a
2026-07-28 request that declares MCP's tasks extension, but left out a
call whose project is in another cargo workspace: that call goes to the
workspace's own server (ADR-0035 §2), and the client waited on one
`tools/call` while that server built itself and then the game. In a
fresh worktree that outlasts Claude Code's 300 s `MCP_TIMEOUT`, and it
is the case agents hit most: a session in a worktree whose server is the
main checkout's.

A forwarded server (`forward::Process`) already reads its stdout on a
thread (`lines`), so a request can be written and its response picked up
later. The server speaks the old protocol to it, and the forwarded
server answers each call itself, one at a time: while it works on one,
a second request waits behind it.

## Decision

### 1. `Forward` sends without waiting

`Forward` gains `send` (a request written, its id returned; `None` by
default: nothing is sent, the server calls `request` when it may wait)
and `response` (the response to an id once it has come; none before).
`Failed` and the tests' fakes keep the defaults. `Process` implements
both over `lines`, and keeps a response read while waiting for another
(`kept`), so a call that waits (`tools/list`, a session tool) never
loses a task's response.

### 2. A task's work is here or there

The task table's work is `server::Work`: `Here` (a `Pending`, ADR-0159
§2) or `There` (an `Away`: the workspace, the `tools/call` parameters,
the id once sent, and the restart's fallback, §4). A forwarded call of a
tasked tool, and a forwarded `start` without `link`, on the conditions
of ADR-0159 §1, makes one. `Server::advance` polls it
(`poll_away`): while the server builds (`ready(false)` none) nothing is
sent; once it is ready the call is sent, and its response ends the task
with the response the call answers without tasks (`away_ended`, shared
with the call that waits). A server that stopped is forgotten, and the
task completes with its `SH3005` error result, as the call did.

### 3. `start` there

A forwarded `start` ends the session's game anywhere (here, attached,
another workspace's) and cancels a `start` building, as ADR-0159 §4
says; the session has no game until its task completes `ok`, when the
session is that server's (`session_at`). A session tool meanwhile
answers `SH0002` naming the task. A session tool advances the tasks
first, so it sees a `start` that completed since the last poll.

### 4. The engine-moved restart

When a task's host answers with another contract (ADR-0035 §5), the
call goes to a fresh server of the server's own workspace as an `Away`
with this server's answer as its fallback: the task works on until that
server answers, and answers the fallback with why if it stops. The call
that waits does the same, waiting.

### 5. A cancel drops the server

`tasks/cancel` of a working `There` task drops that workspace's server,
which stops it and its build (its stdin closes, and `Process`'s drop
kills it), and forgets a session there. The next call starts it again.
A second `start` cancels one building there the same way.

### 6. The list waits for a busy server

`Server::polled` doesn't ask a server for its tools while a task's call
is sent to it: the request would wait behind the call and hold the loop.
It asks once the call ended.

## Consequences

- A forwarded `start` in a fresh worktree no longer times out the
  client, and a stop in the client cancels the build.
- The forwarded server is still spoken to on the old protocol and runs
  one call at a time: a call that waits on a server answering a task's
  call (a session tool there, the `stop` a `start` elsewhere sends)
  still waits for it. Only the list is held back (§6).
- A cancel costs that server's own restart, which rebuilds little: its
  build is cargo's, kept on disk.

## Alternatives considered

- **Forward the client's `tasks/get` to the other server.** The ticket's
  Out: that server would need tasks on the old protocol, and the task's
  id would be another process's. The server answers its own tasks.
- **Speak 2026-07-28 with tasks to the forwarded server.** It would end
  the wait behind a call too, but the forwarded server may be an older
  engine's, without tasks; the old protocol is the one every forwarded
  server speaks.
- **Cancel by sending the forwarded server `stop`.** It doesn't stop a
  build, and the server would answer it only after the call.
