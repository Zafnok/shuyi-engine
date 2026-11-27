---
id: "3440"
title: "A forwarded MCP call runs as a task: a call another workspace's server answers waits on it off the loop"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: ["3436"]
nick_input: none
completed: 2026-11-27
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

- [x] A `playtest` forwarded to another workspace's server, on
      2026-07-28 with the extension, answers a task that is `working`
      while that server builds, then `completed` with the result the
      call answers without tasks.
- [x] A forwarded `start`'s task completing makes the session that
      server's: `step` goes there.
- [x] `tasks/cancel` of a working forwarded task drops that server; the
      next call starts it again.
- [x] Without the extension, or on the old protocol, a forwarded call
      answers as today.
- [x] ~~Checked by hand in Claude Code, in a worktree session (whose
      server is the main checkout's, forwarding to the worktree's: 3436
      couldn't check it there): `start` on a game in the worktree answers
      a task, Claude Code polls it, the session plays once it completes,
      and a stop in the client cancels the build. The session's own
      server must be this branch's: `claude --mcp-config` naming the
      branch's built `shuyi mcp`, started in the main checkout.~~ (not
      done: no second Claude Code could be started from this session; a
      scripted client checked the same over real processes, in the
      Completion notes)
- [x] All gates in the `run-gates` skill pass.

## Tests required

- `server/tests/tasks.rs`: each case with a fake `Forward` whose
  `ready` and `response` wait until the test releases them (the
  `Forwarded` fake of `server/tests.rs` extended), and one whose drop the
  cancel must cause.

## Completion notes

What was done:

- `Forward` (`crates/cli/src/mcp/forward.rs`) gained `send` and
  `response`, with defaults (`send` sends nothing and answers `None`:
  the server then calls `request` when it may wait), so `Failed` is
  unchanged. `Process` implements them over `lines`, and keeps a
  response read while it waits for another (`kept`), so a call that
  waits never loses a task's response.
- The task table's work is `server::Work`: `Here` (3436's `Pending`)
  or `There` (an `Away`: the workspace, the call, its id once sent, and
  the restart's fallback). `task_call` makes one for a forwarded call of
  a tasked tool or `start`; `advance` polls it (`poll_away`): nothing is
  sent while the server builds, the call once it is ready, and its
  response ends the task with what the call answers without tasks
  (`away_ended`, which the calls that wait share now).
- A forwarded `start` ends the session everywhere and cancels a `start`
  building; the session is that server's once its task completes `ok`.
  A session tool advances the tasks first, and meanwhile answers
  `SH0002` naming the task.
- `tasks/cancel` of a forwarded task drops its server (`Server::lost`);
  the next call starts it again.
- The engine-moved restart inside a task (`Server::completed`) sends its
  call to the fresh server without waiting, with this server's answer as
  the fallback if that one stops (`restart_to`, `not_restarted`).
- `polled` doesn't ask a server for its tools while a task's call is
  sent to it: the request would wait behind the call and hold the loop.
- ADR-0162 amends ADR-0159 §1 and extends ADR-0035 §2;
  `docs/contract.md`'s *Tasks* paragraph says a forwarded call is a task
  too.

Deviations:

- `Forward`'s defaults don't wait themselves (a trait default holds no
  state to send now and answer later): `send` answers `None`, and the
  server calls `request`, which waits, when it next looks, as 3436's
  `Link::send` does. `Failed` and the old fakes are unchanged, as the
  ticket wanted.
- `Server::polled`'s list waits while a call is sent to that server
  (ADR-0162 §6): not in the ticket, but without it the first poll after
  a forwarded server is ready would block the loop on its `tools/list`
  for the whole call.
- The fake `Forwarded` of `server/tests.rs` was extended with `send`,
  `response` (held until `answers` is set) and a drop count, as the
  ticket's *Tests required* said; the 3436 test of a `start` in another
  workspace now reads that start's task.

Checked by hand: not in Claude Code itself. This session couldn't start a
second Claude Code with `--mcp-config`. Instead a scripted 2026-07-28
client with the tasks extension and roots drove this branch's `shuyi
mcp` started in the main checkout (`D:\shuyi-engine`), rooted in this
worktree, over real processes. `start` on `examples/hello-grid` answered
task `shuyi-1` in 0.4 s. A `step` meanwhile answered `SH0002` naming it.
The task was `working` for 95 s, while the worktree's server and the
game built, then `completed` `ok`, and `step` played the game there. A
second `start` was `cancelled` by `tasks/cancel`, and a third started
that server again and completed in 6 s. The Claude Code criterion is
struck for that reason. No ticket: the same check in Claude Code
needs only a person to start it, and the scripted client drove the same
messages Claude Code sends (ADR-0159's *Context*).

What a call that waits still waits for (ADR-0162 *Consequences*): a
session tool, or the `stop` a `start` elsewhere sends, to a server busy
answering a task's call waits for that call, since the forwarded server
answers one call at a time on the old protocol. No ticket: that is the
ticket's Out (the forwarded server stays on the old protocol), and a
session tool there while a task's call runs is rare.

No break: `Forward` gained two methods with defaults; the game neither
implements nor calls it (`gh search code --repo Zafnok/visions-of-shuyi
'Forward'` and `'forwarded'`: only its xtask's own `FORWARDED`).

Behaviour: shuyi-cli: on 2026-07-28, a forwarded call of `start`
(without `link`), `playtest`, `autobalance`, `bake` or `export` from a
client with MCP's tasks extension answers a task instead of the result;
`tasks/cancel` of it stops that workspace's server.

Book: none, the addition is the MCP server's seam to its forwarded
servers, which no game implements; `docs/contract.md` says what a client
sees.

Skills: none, no schema, template, format or problem code changed.

Game: nothing to adopt, the game's MCP use is through Claude Code, which
gets tasks from a forwarded call once its server is a build of this
change.
