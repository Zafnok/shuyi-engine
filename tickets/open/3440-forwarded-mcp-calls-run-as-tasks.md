---
id: "3440"
title: "A forwarded MCP call runs as a task: a call another workspace's server answers waits on it off the loop"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["3436"]
nick_input: none
completed:
---

# 3440 — A forwarded MCP call runs as a task

## Context

3436 made the MCP server's long tools (`start`, `playtest`,
`autobalance`, `bake`, `export`; `mcp::tasked`) answer a task on a
2026-07-28 request that declares MCP's tasks extension, and split off
the forwarded calls at its size valve (ADR-0159). Today a call whose
project is in another cargo workspace goes to that workspace's own
server (ADR-0035 §2, `Server::forwarded_call` in
`crates/cli/src/mcp/server.rs`), and the client still waits on one
`tools/call` while that server builds itself and then the game, which
in a fresh worktree outlasts the client's 300 s `MCP_TIMEOUT`.

**From 3436 (done):**

- `Server::task_call` answers `None` for a forwarded call
  (`self.forward_to(&root).is_some()`), so `tool_call` answers it as
  before.
- A task's work is a `Pending` (its host, a `Link`), polled by
  `Server::advance` from `tasks/get`, `poll` and the session tools; a
  `Link` replies without blocking through `send` and `reply`.
- The engine-moved restart (`Server::answered`) still calls the fresh
  server and waits on it, inside the task's completion.

`forward::Process` already reads its server's stdout on a thread
(`lines: Receiver<String>`), so a request can be sent and its response
picked up later.

## Nick input

None.

## Scope

**In:**
- `Forward` gains a request that doesn't wait (`send` of a method and
  params returning its id, and a `response` that tries the lines), with
  default methods that wait, so `Failed` and the tests' fakes are
  unchanged; `Process` implements them over its `lines`.
- A forwarded call of a tasked tool (`mcp::tasked`), and of `start` to
  another workspace, answers a task, on the same conditions as 3436's
  (2026-07-28, the extension declared). A `start` there sets
  `session_at` once its task completes `ok`, as `forwarded_tool` does
  now; while it works, a session tool answers 3436's "no session" with
  the task named.
- The server still building (`Forward::ready` none) is part of the task:
  the call is sent once it is ready.
- `tasks/cancel` of a forwarded task drops that workspace's server
  (it is started again on the next call), as a host's cancel kills it.
- The engine-moved restart in `Server::answered` sends its call to the
  fresh server without waiting, when the task allows it.
- A short ADR amending ADR-0159 §1 (a forwarded call answers as it
  is).
- `docs/contract.md`'s *The MCP server*: a forwarded call may answer a
  task too.
- **Estimated size:** about 650 lines (code 250, tests 300, docs 100).

**Out (do not do):**
- Tasks on the old protocol, `tasks/update`, `notifications/tasks`
  (3436's Out).
- Forwarding the client's `tasks/get` to the other server: the server
  answers its own tasks, and the forwarded server is spoken to on the
  old protocol, as now.

## Implementation steps

1. Read ADR-0035, ADR-0159 and `crates/cli/src/mcp/server.rs`
   (`task_call`, `advance`, `completed`, `forwarded_call`,
   `forwarded_tool`, `answered`) and `mcp/forward.rs` (`Process`).
2. `Forward::send` and `Forward::response` with waiting defaults;
   `Process`'s over `lines`, matching the response's `id`.
3. A second kind of task work beside `Pending`: the workspace, the
   request's id, and whether it was sent (the server may still build).
4. `task_call` makes it for a forwarded call; `advance` polls it;
   `cancel` drops the workspace's server.
5. Tests first, then the docs.

## Acceptance criteria

- [ ] A `playtest` forwarded to another workspace's server, on
      2026-07-28 with the extension, answers a task that is `working`
      while that server builds, then `completed` with the result the
      call answers without tasks.
- [ ] A forwarded `start`'s task completing makes the session that
      server's: `step` goes there.
- [ ] `tasks/cancel` of a working forwarded task drops that server; the
      next call starts it again.
- [ ] Without the extension, or on the old protocol, a forwarded call
      answers as today.
- [ ] Checked by hand in Claude Code, in a worktree session (whose
      server is the main checkout's, forwarding to the worktree's: 3436
      couldn't check it there): `start` on a game in the worktree answers
      a task, Claude Code polls it, the session plays once it completes,
      and a stop in the client cancels the build. The session's own
      server must be this branch's: `claude --mcp-config` naming the
      branch's built `shuyi mcp`, started in the main checkout.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `server/tests/tasks.rs`: each case with a fake `Forward` whose
  `ready` and `response` wait until the test releases them (the
  `Forwarded` fake of `server/tests.rs` extended), and one whose drop the
  cancel must cause.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
